use std::collections::VecDeque;
use std::iter::{FlatMap, FusedIterator};

use crate::curve::curve_types::CurveType;
use crate::curve::Curve;
use crate::iterators::curve::CurveSplitIterator;
use crate::iterators::{CurveIterator, JoinAdjacentIterator};
use crate::server::{
    ActualServerExecution, AvailableServerExecution, ConstrainedServerDemand, Server,
};
use crate::time::{TimeUnit, UnitNumber};
use crate::window::{Demand, Window};

/// `CurveIterator` for `ActualServerExecution`
///
/// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
///
/// For the server with priority `server_index` calculate th actual execution
/// given the unconstrained execution and the constrained demand
#[derive(Debug, Clone)]
pub struct ActualServerExecutionIterator<'a, AC, DC> {
    /// internal Iterator
    iter: JoinAdjacentIterator<
        InternalActualExecutionIterator<'a, AC, DC>,
        <ActualServerExecution as CurveType>::WindowKind,
        ActualServerExecution,
    >,
}

impl<'a, AC, DC> ActualServerExecutionIterator<'a, AC, DC> {
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
        AC: CurveIterator<
            <AvailableServerExecution as CurveType>::WindowKind,
            CurveKind = AvailableServerExecution,
        >,
        DC: CurveIterator<
            <ConstrainedServerDemand as CurveType>::WindowKind,
            CurveKind = ConstrainedServerDemand,
        >,
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
        ActualServerExecutionIterator { iter: outer }
    }
}

impl<AC, DC> CurveIterator<<ActualServerExecution as CurveType>::WindowKind>
    for ActualServerExecutionIterator<'_, AC, DC>
where
    AC: CurveIterator<
        <AvailableServerExecution as CurveType>::WindowKind,
        CurveKind = AvailableServerExecution,
    >,
    DC: CurveIterator<
        <ConstrainedServerDemand as CurveType>::WindowKind,
        CurveKind = ConstrainedServerDemand,
    >,
{
    type CurveKind = ActualServerExecution;
}

impl<AC, DC> FusedIterator for ActualServerExecutionIterator<'_, AC, DC>
where
    Self: Iterator,
    AC: FusedIterator,
    DC: FusedIterator,
{
}

impl<AC, DC> Iterator for ActualServerExecutionIterator<'_, AC, DC>
where
    AC: CurveIterator<
        <AvailableServerExecution as CurveType>::WindowKind,
        CurveKind = AvailableServerExecution,
    >,
    DC: CurveIterator<
        <ConstrainedServerDemand as CurveType>::WindowKind,
        CurveKind = ConstrainedServerDemand,
    >,
{
    type Item = Window<<ActualServerExecution as CurveType>::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

/// type alias for the type used in `InternalActualExecutionIterator`
/// for easier naming
type FlattenedSplitAvailableSupply<AC> = FlatMap<
    CurveSplitIterator<<AvailableServerExecution as CurveType>::WindowKind, AC>,
    Curve<AvailableServerExecution>,
    fn((UnitNumber, Curve<AvailableServerExecution>)) -> Curve<AvailableServerExecution>,
>;

/// `CurveIterator` for calculating the actual execution of a Server
///
/// The resulting windows are in order and either adjacent or non-overlapping
///
#[derive(Debug)]
pub struct InternalActualExecutionIterator<'a, AC, CDC> {
    /// the server for which to calculate the actual execution
    server: &'a Server<'a>,
    /// the remaining available execution
    available_execution: FlattenedSplitAvailableSupply<AC>,
    /// the peek of the remaining available execution that is not yet consumed
    execution_peek: VecDeque<Window<<AvailableServerExecution as CurveType>::WindowKind>>,
    /// the group spend_budget is referring to
    current_group: UnitNumber,
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
        AC: CurveIterator<
            <AvailableServerExecution as CurveType>::WindowKind,
            CurveKind = AvailableServerExecution,
        >,
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

impl<AC, CDC> FusedIterator for InternalActualExecutionIterator<'_, AC, CDC>
where
    Self: Iterator,
    FlattenedSplitAvailableSupply<AC>: FusedIterator,
    CDC: FusedIterator,
{
}

impl<AC, CDC> Iterator for InternalActualExecutionIterator<'_, AC, CDC>
where
    AC: CurveIterator<
        <AvailableServerExecution as CurveType>::WindowKind,
        CurveKind = AvailableServerExecution,
    >,
    CDC: CurveIterator<
        <ConstrainedServerDemand as CurveType>::WindowKind,
        CurveKind = ConstrainedServerDemand,
    >,
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

                    vec![result.remaining_supply_head, result.remaining_supply_tail]
                        .into_iter()
                        .filter(|window| !window.is_empty())
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
