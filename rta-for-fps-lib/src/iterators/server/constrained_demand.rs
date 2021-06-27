//! Module for the implementation of the `CurveIterator`s used to calculate
//! the constrained demand curve of a Server

use std::cmp::Ordering;
use std::iter::FusedIterator;

use crate::curve::curve_types::CurveType;
use crate::curve::{Curve, PartitionResult};
use crate::iterators::curve::{AggregationIterator, CurveSplitIterator};
use crate::iterators::peek::Peeker;
use crate::iterators::CurveIterator;
use crate::server::{AggregatedServerDemand, ConstrainedServerDemand, ServerProperties};
use crate::time::TimeUnit;
use crate::window::WindowEnd;
use crate::window::{Demand, Window};

/// Type alias for the `WindowKind` of the `AggregatedServerDemand` `CurveType`
/// to reduce type complexity
type AggregateDemandWindow = <AggregatedServerDemand as CurveType>::WindowKind;

/// `CurveIterator` for `ConstrainedServerDemand`
///
/// used to calculate a Servers constrained demand curve,
/// using the aggregated server demand curve
/// based on the Algorithm 1. from the paper and described in Section 5.1 of the paper
#[derive(Debug, Clone)]
pub struct ConstrainedServerDemandIterator<I> {
    /// The Server for which to calculate the constrained demand
    server_properties: ServerProperties,
    /// The remaining aggregated Demand of the Server
    demand:
        Peeker<Box<CurveSplitIterator<AggregateDemandWindow, I>>, Window<AggregateDemandWindow>>,
    /// The spill from the previous group
    spill: Option<Window<<AggregatedServerDemand as CurveType>::WindowKind>>,
    /// Remaining windows till we need to process the next group
    remainder: Vec<Window<<ConstrainedServerDemand as CurveType>::WindowKind>>,
}

impl<'a, I> ConstrainedServerDemandIterator<I>
where
    I: CurveIterator<CurveKind = AggregatedServerDemand>,
{
    /// Create a new `InternalConstrainedServerDemandIterator`
    /// the main part for calculating the Constraint Server Demand Curve
    pub fn new(server_properties: ServerProperties, aggregated_demand: I) -> Self {
        // Algorithm 1. (1)
        let split = CurveSplitIterator::new(aggregated_demand, server_properties.interval);
        ConstrainedServerDemandIterator {
            server_properties,
            demand: Peeker::new(Box::new(split)),
            spill: None,
            remainder: Vec::new(),
        }
    }
}

impl<I: CurveIterator> FusedIterator for ConstrainedServerDemandIterator<I>
where
    Self: Iterator,
    CurveSplitIterator<<AggregatedServerDemand as CurveType>::WindowKind, I>: FusedIterator,
{
}

impl<I> CurveIterator for ConstrainedServerDemandIterator<I>
where
    I: CurveIterator<CurveKind = AggregatedServerDemand>,
{
    type CurveKind = ConstrainedServerDemand;

    // Algorithm 1. (2)
    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        #![allow(clippy::option_if_let_else)] // false positive, can't use map_or as the same value is moved in both branches

        if let Some(window) = self.remainder.pop() {
            Some(window)
        } else {
            let next_group = self.demand.peek_ref();
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

                            let mut windows = vec![group_head.take()];

                            for window in &mut self.demand {
                                if window.budget_group(self.server_properties.interval)
                                    == k_group_head
                                {
                                    windows.push(window);
                                } else {
                                    self.demand.restore_peek(window);
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
                            // spill not spilled into group, next group consists only of spill
                            let curve = Curve::new(spill);
                            self.process_group(k_spill, curve)
                        }
                    }
                }
                (Some(group_head), None) => {
                    let k_group_head = group_head.start / self.server_properties.interval;
                    // no spill, only next group

                    let mut windows = vec![group_head.take()];

                    for window in &mut self.demand {
                        if window.budget_group(self.server_properties.interval) == k_group_head {
                            windows.push(window);
                        } else {
                            self.demand.restore_peek(window);
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

impl<I> ConstrainedServerDemandIterator<I>
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
