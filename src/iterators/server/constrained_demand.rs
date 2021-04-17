//! Module for the implementation of the `CurveIterator`s used to calculate
//! the constrained demand curve of a Server

use std::cmp::Ordering;
use std::iter::FusedIterator;

use crate::curve::curve_types::CurveType;
use crate::curve::{Curve, PartitionResult};
use crate::iterators::curve::{AggregationIterator, CurveSplitIterator};
use crate::iterators::{CurveIterator, JoinAdjacentIterator};
use crate::server::{AggregatedServerDemand, ConstrainedServerDemand, ServerProperties};
use crate::time::TimeUnit;
use crate::window::WindowEnd;
use crate::window::{Demand, Window};

/// `CurveIterator` for `ConstrainedServerDemand`
#[derive(Debug, Clone)]
pub struct ConstrainedServerDemandIterator<I> {
    /// internal Iterator
    iter: Box<
        JoinAdjacentIterator<
            InternalConstrainedServerDemandIterator<I>,
            Demand,
            ConstrainedServerDemand,
        >,
    >,
}

impl<I> ConstrainedServerDemandIterator<I>
where
    I: CurveIterator<CurveKind = AggregatedServerDemand>,
{
    /// Create a new `ConstrainedServerDemandIterator`
    pub fn new(server_properties: ServerProperties, aggregated_demand: I) -> Self {
        let internal =
            InternalConstrainedServerDemandIterator::new(server_properties, aggregated_demand);
        let outer = unsafe {
            // Safety:
            // `InternalConstrainedServerDemandIterator` guarantees that the windows are in order and
            // either non-overlapping or adjacent
            JoinAdjacentIterator::new(internal)
        };
        ConstrainedServerDemandIterator {
            iter: Box::new(outer),
        }
    }
}

impl<'a, I> CurveIterator for ConstrainedServerDemandIterator<I>
where
    I: CurveIterator<CurveKind = AggregatedServerDemand>,
{
    type CurveKind = ConstrainedServerDemand;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.iter.next_window()
    }
}

/// Iterator used internally in the `ConstrainedServerDemandIterator`
///
/// When iterating returns windows in order that are either non-overlapping or adjacent
///
/// used to calculate a Servers constrained demand curve,
/// using the aggregated server demand curve
/// based on the Algorithm 1. from the paper and described in Section 5.1 of the paper
#[derive(Debug, Clone)]
pub struct InternalConstrainedServerDemandIterator<I> {
    /// The Server for which to calculate the constrained demand
    server_properties: ServerProperties,
    /// The remaining aggregated Demand of the Server
    demand: Box<CurveSplitIterator<<AggregatedServerDemand as CurveType>::WindowKind, I>>,
    /// The next group
    demand_peek: Option<Window<<AggregatedServerDemand as CurveType>::WindowKind>>,
    /// The spill from the previous group
    spill: Option<Window<<AggregatedServerDemand as CurveType>::WindowKind>>,
    /// Remaining windows till we need to process the next group
    remainder: Vec<Window<<ConstrainedServerDemand as CurveType>::WindowKind>>,
}

impl<'a, I> InternalConstrainedServerDemandIterator<I>
where
    I: CurveIterator<CurveKind = AggregatedServerDemand>,
{
    /// Create a new `InternalConstrainedServerDemandIterator`
    /// the main part for calculating the Constraint Server Demand Curve
    pub fn new(server_properties: ServerProperties, aggregated_demand: I) -> Self {
        // Algorithm 1. (1)
        let split = CurveSplitIterator::new(aggregated_demand, server_properties.interval);
        InternalConstrainedServerDemandIterator {
            server_properties,
            demand: Box::new(split),
            demand_peek: None,
            spill: None,
            remainder: Vec::new(),
        }
    }
}

impl<I: CurveIterator> FusedIterator for InternalConstrainedServerDemandIterator<I>
where
    Self: Iterator,
    CurveSplitIterator<<AggregatedServerDemand as CurveType>::WindowKind, I>: FusedIterator,
{
}

impl<I> Iterator for InternalConstrainedServerDemandIterator<I>
where
    I: CurveIterator<CurveKind = AggregatedServerDemand>,
{
    type Item = Window<<ConstrainedServerDemand as CurveType>::WindowKind>;

    // Algorithm 1. (2)
    fn next(&mut self) -> Option<Self::Item> {
        #![allow(clippy::option_if_let_else)] // false positive, can't use map_or as the same value is moved in both branches

        if let Some(window) = self.remainder.pop() {
            Some(window)
        } else {
            let next_group = self.demand_peek.take().or_else(|| self.demand.next());
            let spill = self.spill.take();

            match (next_group, spill) {
                (None, None) => None,
                (Some(group_head), Some(spill)) => {
                    let k_group_head = group_head.start / self.server_properties.interval;
                    let k_spill = spill.start / self.server_properties.interval;

                    match k_group_head.cmp(&k_spill) {
                        Ordering::Less => {
                            unreachable!("Groups are processed in order and spill can only go into the future")
                        }
                        Ordering::Equal => {
                            // spill spilled into next_group

                            let mut windows = vec![group_head];

                            for window in &mut self.demand {
                                if window.budget_group(self.server_properties.interval)
                                    == k_group_head
                                {
                                    windows.push(window);
                                } else {
                                    self.demand_peek = Some(window);
                                    break;
                                }
                            }

                            // collect next_group
                            let next_group: Curve<AggregatedServerDemand> =
                                unsafe { Curve::from_windows_unchecked(windows) };

                            // Handle next_group and spill
                            let curve: Curve<_> = AggregationIterator::new(vec![
                                next_group.into_iter(),
                                Curve::new(spill).into_iter(),
                            ])
                            .collect_curve();

                            self.process_group(k_spill, curve)
                        }
                        Ordering::Greater => {
                            // restore demand_peek
                            // then process only spill
                            self.demand_peek = Some(group_head);

                            // spill not spilled into group, next group consists only of spill
                            let curve = Curve::new(spill);
                            self.process_group(k_spill, curve)
                        }
                    }
                }
                (Some(group_head), None) => {
                    let k_group_head = group_head.start / self.server_properties.interval;
                    // no spill, only next group

                    let mut windows = vec![group_head];

                    for window in &mut self.demand {
                        if window.budget_group(self.server_properties.interval) == k_group_head {
                            windows.push(window);
                        } else {
                            self.demand_peek = Some(window);
                            break;
                        }
                    }

                    // collect next_group
                    let next_group: Curve<AggregatedServerDemand> =
                        unsafe { Curve::from_windows_unchecked(windows) };

                    let curve = next_group;

                    self.process_group(k_group_head, curve)
                }
                (None, Some(spill)) => {
                    // only spill remaining

                    let k = spill.start / self.server_properties.interval;

                    let curve = Curve::new(spill);

                    self.process_group(k, curve)
                }
            }
        }
    }
}

impl<I> InternalConstrainedServerDemandIterator<I>
where
    I: CurveIterator<CurveKind = AggregatedServerDemand>,
{
    /// Process the group with index `k_group_head` and `demand `curve`
    fn process_group(
        &mut self,
        k_group_head: usize,
        curve: Curve<AggregatedServerDemand>,
    ) -> Option<Window<Demand>> {
        let PartitionResult { index, head, tail } =
            curve.partition(k_group_head, self.server_properties);

        let mut windows = curve.into_windows();

        self.remainder.reserve(windows.len().min(index) + 1);

        self.remainder.extend(
            windows
                .drain(..index)
                .chain(std::iter::once(head).filter(|window| !window.is_empty()))
                .rev(),
        );

        let delta_k: WindowEnd = tail.length()
            + windows
                .into_iter()
                .skip(1)
                .map(|window| window.length())
                .sum::<WindowEnd>();

        if delta_k > TimeUnit::ZERO {
            let spill_start = (k_group_head + 1) * self.server_properties.interval;
            self.spill = Some(Window::new(spill_start, spill_start + delta_k));
        }

        let result = self.remainder.pop();
        assert!(result.is_some());
        result
    }
}
