use crate::curve::curve_types::{CurveType, UnspecifiedCurve};
use crate::curve::{Curve, PartitionResult};
use crate::iterators::curve::{AggregatedDemandIterator, CollectCurveExt, CurveSplitIterator};
use crate::iterators::{CurveIterator, JoinAdjacentIterator};
use crate::server::{
    ActualServerExecution, AggregatedServerDemand, AvailableServerExecution,
    ConstrainedServerDemand, Server,
};
use crate::time::TimeUnit;
use crate::window::{Demand, Window};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::iter::{FlatMap, FusedIterator};

/// `CurveIterator` for `ConstrainedServerDemand`
#[derive(Debug, Clone)]
pub struct ConstrainedServerDemandIterator<'a, I> {
    /// internal Iterator
    iter: JoinAdjacentIterator<
        InternalConstrainedServerDemandIterator<'a, I>,
        Demand,
        ConstrainedServerDemand,
    >,
}

impl<'a, I: CurveIterator<'a, AggregatedServerDemand>> ConstrainedServerDemandIterator<'a, I> {
    /// Create a new `ConstrainedServerDemandIterator`
    pub fn new(server: &'a Server, aggregated_demand: I) -> Self {
        let internal = InternalConstrainedServerDemandIterator::new(server, aggregated_demand);
        let outer = unsafe {
            // Safety:
            // `InternalConstrainedServerDemandIterator` guarantees that the windows are in order and
            // either non-overlapping or adjacent
            JoinAdjacentIterator::new(internal)
        };
        ConstrainedServerDemandIterator { iter: outer }
    }
}

impl<'a, I> CurveIterator<'a, ConstrainedServerDemand> for ConstrainedServerDemandIterator<'a, I> where
    I: CurveIterator<'a, AggregatedServerDemand>
{
}

impl<'a, I> FusedIterator for ConstrainedServerDemandIterator<'a, I>
where
    Self: Iterator,
    JoinAdjacentIterator<
        InternalConstrainedServerDemandIterator<'a, I>,
        Demand,
        ConstrainedServerDemand,
    >: FusedIterator,
{
}

impl<'a, I> Iterator for ConstrainedServerDemandIterator<'a, I>
where
    I: CurveIterator<'a, AggregatedServerDemand>,
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
pub struct InternalConstrainedServerDemandIterator<'a, I> {
    /// The Server for which to calculate the constrained demand
    server: &'a Server,
    /// The remaining aggregated Demand of the Server
    groups: CurveSplitIterator<
        <AggregatedServerDemand as CurveType>::WindowKind,
        AggregatedServerDemand,
        I,
    >,
    /// The next group
    group_peek: Option<(usize, Curve<AggregatedServerDemand>)>,
    /// The spill from the previous group
    spill: Option<Window<<AggregatedServerDemand as CurveType>::WindowKind>>,
    /// Remaining windows till we need to process the next group
    remainder: VecDeque<Window<<ConstrainedServerDemand as CurveType>::WindowKind>>,
}

impl<'a, I: CurveIterator<'a, AggregatedServerDemand>>
    InternalConstrainedServerDemandIterator<'a, I>
{
    /// Create a new `InternalConstrainedServerDemandIterator`
    /// the main part for calculating the Constraint Server Demand Curve
    pub fn new(server: &'a Server, aggregated_demand: I) -> Self {
        // Algorithm 1. (1)
        let split = CurveSplitIterator::new(aggregated_demand, server.interval);
        InternalConstrainedServerDemandIterator {
            server,
            groups: split,
            group_peek: None,
            spill: None,
            remainder: VecDeque::new(),
        }
    }
}

impl<'a, I: CurveIterator<'a, AggregatedServerDemand>> FusedIterator
    for InternalConstrainedServerDemandIterator<'a, I>
where
    CurveSplitIterator<
        <AggregatedServerDemand as CurveType>::WindowKind,
        AggregatedServerDemand,
        I,
    >: FusedIterator,
{
}

impl<'a, I> Iterator for InternalConstrainedServerDemandIterator<'a, I>
where
    I: CurveIterator<'a, AggregatedServerDemand>,
{
    type Item = Window<<ConstrainedServerDemand as CurveType>::WindowKind>;

    // Algorithm 1. (2)
    fn next(&mut self) -> Option<Self::Item> {
        #![allow(clippy::option_if_let_else)] // false positive, can't use map_or as the same value is moved in both branches

        if let Some(window) = self.remainder.pop_front() {
            Some(window)
        } else {
            let next_group = self.group_peek.take().or_else(|| self.groups.next());
            let spill = self.spill.take();

            match (next_group, spill) {
                (None, None) => None,
                (Some((group_index, next_group)), spill)
                    if (group_index
                        == spill
                            .as_ref()
                            .map_or(group_index, |spill| spill.start / self.server.interval)) =>
                {
                    // Handle only next_group or spill into next_group
                    let curve = if let Some(spill) = spill {
                        AggregatedDemandIterator::new(
                            next_group.into_iter(),
                            Curve::new(spill).into_iter(),
                        )
                        .collect_curve()
                    } else {
                        next_group
                    };

                    let PartitionResult { index, head, tail } =
                        curve.partition(group_index, self.server);

                    let mut windows = curve.into_windows();

                    let keep = windows
                        .drain(..index)
                        .chain(std::iter::once(head).filter(|window| !window.is_empty()))
                        .collect();

                    let delta_k = tail.length()
                        + windows
                            .into_iter()
                            .skip(1)
                            .map(|window| window.length())
                            .sum();

                    if delta_k > TimeUnit::ZERO {
                        let spill_start = (group_index + 1) * self.server.interval;
                        self.spill = Some(Window::new(spill_start, spill_start + delta_k));
                    }

                    self.remainder = keep;

                    let result = self.remainder.pop_front();
                    assert!(result.is_some());
                    result
                }
                (Some(_), None) => unreachable!("handled in previous case!"),
                (next_group, Some(spill)) => {
                    self.group_peek = next_group;
                    // only spill remaining or spill not spilled into next_group

                    let k = spill.start / self.server.interval;

                    let curve = Curve::<UnspecifiedCurve<_>>::new(spill);

                    let PartitionResult { index, head, tail } = curve.partition(k, self.server);

                    let keep = curve
                        .into_windows()
                        .drain(..index)
                        .chain(std::iter::once(head).filter(|window| !window.is_empty()))
                        .collect();

                    self.spill = (!tail.is_empty()).then(|| {
                        let spill_start = (k + 1) * self.server.interval;
                        Window::new(spill_start, spill_start + tail.length())
                    });

                    self.remainder = keep;
                    let result = self.remainder.pop_front();
                    assert!(result.is_some());
                    result
                }
            }
        }
    }
}

/// `CurveIterator` for `ActualServerExecution`
///
/// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
///
/// For the server with priority `server_index` calculate th actual execution
/// given the unconstrained execution and the constrained demand
#[derive(Debug, Clone)]
pub struct ActualExecutionIterator<'a, AC, DC> {
    /// internal Iterator
    iter: JoinAdjacentIterator<
        InternalActualExecutionIterator<'a, AC, DC>,
        <ActualServerExecution as CurveType>::WindowKind,
        ActualServerExecution,
    >,
}

impl<'a, AC, DC> ActualExecutionIterator<'a, AC, DC> {
    /// Create a new `ActualExecutionIterator`
    /// for the server with priority `server_index`
    /// its `available_execution` and its `constrained_demand`
    pub fn new(
        servers: &'a [Server],
        server_index: usize,
        available_execution: AC,
        constrained_demand: DC,
    ) -> Self
    where
        AC: CurveIterator<'a, AvailableServerExecution>,
        DC: CurveIterator<'a, ConstrainedServerDemand>,
    {
        let inner = InternalActualExecutionIterator::new(
            servers,
            server_index,
            available_execution,
            constrained_demand,
        );
        let outer = unsafe {
            // Safety:
            // `InternalActualExecutionIterator` guarantees that the windows are in order and
            // either non-overlapping or adjacent
            JoinAdjacentIterator::new(inner)
        };
        ActualExecutionIterator { iter: outer }
    }
}

impl<'a, AC: 'a, DC: 'a> CurveIterator<'a, ActualServerExecution>
    for ActualExecutionIterator<'a, AC, DC>
where
    AC: CurveIterator<'a, AvailableServerExecution>,
    DC: CurveIterator<'a, ConstrainedServerDemand>,
{
}

impl<'a, AC, DC> FusedIterator for ActualExecutionIterator<'a, AC, DC>
where
    Self: Iterator,
    AC: FusedIterator,
    DC: FusedIterator,
{
}

impl<'a, AC, DC> Iterator for ActualExecutionIterator<'a, AC, DC>
where
    AC: CurveIterator<'a, AvailableServerExecution>,
    DC: CurveIterator<'a, ConstrainedServerDemand>,
{
    type Item = Window<<ActualServerExecution as CurveType>::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

/// type alias for the type used in `InternalActualExecutionIterator`
/// for easier naming
type FlattenedSplitAvailableSupply<'a, AC> = FlatMap<
    CurveSplitIterator<
        <AvailableServerExecution as CurveType>::WindowKind,
        AvailableServerExecution,
        AC,
    >,
    Curve<AvailableServerExecution>,
    fn((usize, Curve<AvailableServerExecution>)) -> Curve<AvailableServerExecution>,
>;

/// `CurveIterator` for calculating the actual execution of a Server
///
/// The resulting windows are in order and either adjacent or non-overlapping
///
#[derive(Debug)]
pub struct InternalActualExecutionIterator<'a, AC, CDC> {
    /// the server for which to calculate the actual execution
    server: &'a Server,
    /// the remaining available execution
    available_execution: FlattenedSplitAvailableSupply<'a, AC>,
    /// the peek of the remaining available execution that is not yet consumed
    execution_peek: VecDeque<Window<<AvailableServerExecution as CurveType>::WindowKind>>,
    /// the group spend_budget is referring to
    current_group: usize,
    /// the spend budget of the current group
    // remembering one group is enough as we go through them in order
    spend_budget: TimeUnit,
    /// remaining constrained demand
    constrained_demand: CDC,
    /// unused peek of teh remaining constrained demand
    demand_peek: Option<Window<Demand>>,
}

impl<'a, AC: Clone, CDC: Clone> Clone for InternalActualExecutionIterator<'a, AC, CDC> {
    fn clone(&self) -> Self {
        InternalActualExecutionIterator {
            server: self.server,
            available_execution: self.available_execution.clone(),
            execution_peek: self.execution_peek.clone(),
            current_group: self.current_group,
            spend_budget: self.spend_budget,
            constrained_demand: self.constrained_demand.clone(),
            demand_peek: self.demand_peek.clone(),
        }
    }
}

impl<'a, AC, CDC> InternalActualExecutionIterator<'a, AC, CDC> {
    /// Create a new `ActualExecutionIterator`
    #[must_use]
    pub fn new(
        servers: &'a [Server],
        server_index: usize,
        available_execution: AC,
        constrained_demand: CDC,
    ) -> Self
    where
        AC: CurveIterator<'a, AvailableServerExecution>,
    {
        let server = &servers[server_index];

        // Algorithm 4. (1)
        let split_execution = CurveSplitIterator::new(available_execution, server.interval)
            .flat_map((|(_, curve)| curve) as fn(_) -> _);

        InternalActualExecutionIterator {
            server,
            available_execution: split_execution,
            execution_peek: VecDeque::new(),
            current_group: 0,
            spend_budget: TimeUnit::ZERO,
            constrained_demand,
            demand_peek: None,
        }
    }
}

impl<'a, AC, CDC> FusedIterator for InternalActualExecutionIterator<'a, AC, CDC>
where
    Self: Iterator,
    FlattenedSplitAvailableSupply<'a, AC>: FusedIterator,
    CDC: FusedIterator,
{
}

impl<'a, AC, CDC> Iterator for InternalActualExecutionIterator<'a, AC, CDC>
where
    AC: CurveIterator<'a, AvailableServerExecution>,
    CDC: CurveIterator<'a, ConstrainedServerDemand>,
{
    type Item = Window<<ActualServerExecution as CurveType>::WindowKind>;

    // 4.
    fn next(&mut self) -> Option<Self::Item> {
        let demand = self
            .demand_peek
            .take()
            .or_else(|| self.constrained_demand.next());

        // as we typically deal with limited demand but endless supply
        // check demand first
        if let Some(demand_window) = demand {
            loop {
                let supply = self
                    .execution_peek
                    .pop_front()
                    .or_else(|| self.available_execution.next());

                if let Some(supply_window) = supply {
                    let window_group = supply_window.budget_group(self.server.interval);

                    // (a)
                    if supply_window.end <= demand_window.start {
                        // supply is useless for remaining demand
                        continue;
                    }
                    if window_group != self.current_group {
                        // entered new budget group
                        // reset spend budget
                        self.spend_budget = TimeUnit::ZERO;
                        self.current_group = window_group;
                    } else if self.spend_budget >= self.server.capacity {
                        // budget exhausted skip supply window
                        continue;
                    }

                    // (b)

                    let remaining_budget = self.server.capacity - self.spend_budget;

                    let valid_demand_segment = if demand_window.length() > remaining_budget {
                        let valid = Window::new(
                            demand_window.start,
                            demand_window.start + remaining_budget,
                        );
                        let residual = Window::new(valid.end, demand_window.end);

                        self.demand_peek = Some(residual);
                        valid
                    } else {
                        demand_window
                    };

                    // (d)
                    let result = Window::delta(&supply_window, &valid_demand_segment);

                    // (e)
                    self.spend_budget += result.overlap.length();

                    result
                        .remaining_supply
                        .into_windows()
                        .into_iter()
                        .rev()
                        .for_each(|window| self.execution_peek.push_front(window));

                    break Some(result.overlap);
                } else {
                    assert!(
                        self.demand_peek
                            .take()
                            .or_else(|| self.constrained_demand.next())
                            .is_none(),
                        "While calculating the actual execution the supply dried up before the demand"
                    );
                    // out of demand and supply
                    break None;
                }
            }
        } else {
            // out of demand, end of execution
            None
        }
    }
}
