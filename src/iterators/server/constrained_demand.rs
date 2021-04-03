//! Module for the implementation of the `CurveIterator`s used to calculate
//! the constrained demand curve of a Server

use std::iter::FusedIterator;

use crate::curve::curve_types::{CurveType, UnspecifiedCurve};
use crate::curve::{Curve, PartitionResult};
use crate::iterators::curve::{AggregationIterator, CurveSplitIterator};
use crate::iterators::{CurveIterator, JoinAdjacentIterator};
use crate::server::{AggregatedServerDemand, ConstrainedServerDemand, ServerProperties};
use crate::time::{TimeUnit, UnitNumber};
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
    I: CurveIterator<Demand, CurveKind = AggregatedServerDemand>,
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

impl<'a, I> CurveIterator<<ConstrainedServerDemand as CurveType>::WindowKind>
    for ConstrainedServerDemandIterator<I>
where
    I: CurveIterator<Demand, CurveKind = AggregatedServerDemand>,
{
    type CurveKind = ConstrainedServerDemand;
}

impl<'a, I> FusedIterator for ConstrainedServerDemandIterator<I>
where
    Self: Iterator,
    JoinAdjacentIterator<
        InternalConstrainedServerDemandIterator<I>,
        Demand,
        ConstrainedServerDemand,
    >: FusedIterator,
{
}

impl<'a, I> Iterator for ConstrainedServerDemandIterator<I>
where
    I: CurveIterator<Demand, CurveKind = AggregatedServerDemand>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
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
    groups: Box<CurveSplitIterator<<AggregatedServerDemand as CurveType>::WindowKind, I>>,
    /// The next group
    group_peek: Option<(UnitNumber, Curve<AggregatedServerDemand>)>,
    /// The spill from the previous group
    spill: Option<Window<<AggregatedServerDemand as CurveType>::WindowKind>>,
    /// Remaining windows till we need to process the next group
    remainder: Vec<Window<<ConstrainedServerDemand as CurveType>::WindowKind>>,
}

impl<'a, I> InternalConstrainedServerDemandIterator<I>
where
    I: CurveIterator<
        <AggregatedServerDemand as CurveType>::WindowKind,
        CurveKind = AggregatedServerDemand,
    >,
{
    /// Create a new `InternalConstrainedServerDemandIterator`
    /// the main part for calculating the Constraint Server Demand Curve
    pub fn new(server_properties: ServerProperties, aggregated_demand: I) -> Self {
        // Algorithm 1. (1)
        let split = CurveSplitIterator::new(aggregated_demand, server_properties.interval);
        InternalConstrainedServerDemandIterator {
            server_properties,
            groups: Box::new(split),
            group_peek: None,
            spill: None,
            remainder: Vec::new(),
        }
    }
}

impl<I: CurveIterator<AggregatedServerDemand>> FusedIterator
    for InternalConstrainedServerDemandIterator<I>
where
    Self: Iterator,
    CurveSplitIterator<<AggregatedServerDemand as CurveType>::WindowKind, I>: FusedIterator,
{
}

impl<I> Iterator for InternalConstrainedServerDemandIterator<I>
where
    I: CurveIterator<Demand, CurveKind = AggregatedServerDemand>,
{
    type Item = Window<<ConstrainedServerDemand as CurveType>::WindowKind>;

    // Algorithm 1. (2)
    fn next(&mut self) -> Option<Self::Item> {
        #![allow(clippy::option_if_let_else)] // false positive, can't use map_or as the same value is moved in both branches

        if let Some(window) = self.remainder.pop() {
            Some(window)
        } else {
            let next_group = self.group_peek.take().or_else(|| self.groups.next());
            let spill = self.spill.take();

            match (next_group, spill) {
                (None, None) => None,
                (Some((group_index, next_group)), spill)
                    if (group_index
                        == spill.as_ref().map_or(group_index, |spill| {
                            spill.start / self.server_properties.interval
                        })) =>
                {
                    // Handle next_group and potentially some spill into next_group
                    let curve = if let Some(spill) = spill {
                        AggregationIterator::new(vec![
                            next_group.into_iter(),
                            Curve::new(spill).into_iter(),
                        ])
                        .collect_curve()
                    } else {
                        next_group
                    };

                    let PartitionResult { index, head, tail } =
                        curve.partition(group_index, self.server_properties);

                    let mut windows = curve.into_windows();

                    self.remainder.reserve(windows.len().min(index) + 1);

                    self.remainder.extend(
                        windows
                            .drain(..index)
                            .chain(std::iter::once(head).filter(|window| !window.is_empty()))
                            .rev(),
                    );

                    let delta_k = tail.length()
                        + windows
                            .into_iter()
                            .skip(1)
                            .map(|window| window.length())
                            .sum();

                    if delta_k > TimeUnit::ZERO {
                        let spill_start = (group_index + 1) * self.server_properties.interval;
                        self.spill = Some(Window::new(spill_start, spill_start + delta_k));
                    }

                    let result = self.remainder.pop();
                    assert!(result.is_some());
                    result
                }
                (Some(_), None) => unreachable!("handled in previous case!"),
                (next_group, Some(spill)) => {
                    self.group_peek = next_group;
                    // only spill remaining or spill not spilled into next_group

                    let k = spill.start / self.server_properties.interval;

                    let curve = Curve::<UnspecifiedCurve<_>>::new(spill);

                    let PartitionResult { index, head, tail } =
                        curve.partition(k, self.server_properties);

                    self.remainder
                        .reserve(curve.as_windows().len().min(index) + 1);

                    self.remainder.extend(
                        curve
                            .into_windows()
                            .drain(..index)
                            .chain(std::iter::once(head).filter(|window| !window.is_empty()))
                            .rev(),
                    );

                    self.spill = (!tail.is_empty()).then(|| {
                        let spill_start = (k + 1) * self.server_properties.interval;
                        Window::new(spill_start, spill_start + tail.length())
                    });

                    let result = self.remainder.pop();
                    assert!(result.is_some());
                    result
                }
            }
        }
    }
}
