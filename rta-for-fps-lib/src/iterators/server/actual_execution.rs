//! Module for the implementation of the `CurveIterator`s used to calculate
//! the actual execution curve of a Server

use std::iter::{FlatMap, FusedIterator};

use crate::curve::curve_types::CurveType;
use crate::curve::Curve;
use crate::iterators::curve::CurveSplitIterator;
use crate::iterators::join::JoinAdjacentIterator;
use crate::iterators::peek::Peeker;
use crate::iterators::{CurveIterator, CurveIteratorIterator};
use crate::server::{
    ActualServerExecution, ConstrainedServerDemand, ServerProperties, UnconstrainedServerExecution,
};
use crate::time::{TimeUnit, UnitNumber};
use crate::window::WindowEnd;
use crate::window::{Demand, Window};

/// Type alias for the `WindowKind` of the `ActualServerExecution` `CurveType`
/// to reduce type complexity
type ActualExecutionWindow = <ActualServerExecution as CurveType>::WindowKind;

/// `CurveIterator` for `ActualServerExecution`
///
/// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
///
/// For a server and its unconstrained execution curve as well as its constrained demand calculate th actual execution
///
#[derive(Debug, Clone)]
pub struct ActualServerExecutionIterator<AC, DC> {
    /// internal Iterator
    iter: Box<
        JoinAdjacentIterator<
            InternalActualExecutionIterator<AC, CurveIteratorIterator<DC>>,
            ActualExecutionWindow,
            ActualServerExecution,
        >,
    >,
}

impl<AC, DC> ActualServerExecutionIterator<AC, DC> {
    /// Create a new `ActualExecutionIterator`
    /// for the server and its `available_execution` as well as its `constrained_demand`
    ///
    /// Takes a reference to a Server, the Servers constrained execution curve and the Servers constrained demand curve
    ///
    pub fn new(
        server_properties: ServerProperties,
        available_execution: AC,
        constrained_demand: DC,
    ) -> Self
    where
        AC: CurveIterator<CurveKind = UnconstrainedServerExecution>,
        DC: CurveIterator<CurveKind = ConstrainedServerDemand>,
    {
        let inner = InternalActualExecutionIterator::new(
            server_properties,
            available_execution,
            constrained_demand.into_iterator(),
        );
        let outer = unsafe {
            // Safety:
            // `InternalActualExecutionIterator` guarantees that the windows are in order and
            // either non-overlapping or adjacent
            JoinAdjacentIterator::new(inner)
        };
        ActualServerExecutionIterator {
            iter: Box::new(outer),
        }
    }
}

impl<AC, DC> CurveIterator for ActualServerExecutionIterator<AC, DC>
where
    AC: CurveIterator<CurveKind = UnconstrainedServerExecution>,
    DC: CurveIterator<CurveKind = ConstrainedServerDemand>,
{
    type CurveKind = ActualServerExecution;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.iter.next_window()
    }
}

/// type alias for the type used in `InternalActualExecutionIterator`
/// for easier naming
type FlattenedSplitAvailableSupply<AC> = FlatMap<
    CurveSplitIterator<<UnconstrainedServerExecution as CurveType>::WindowKind, AC>,
    Curve<UnconstrainedServerExecution>,
    fn((UnitNumber, Curve<UnconstrainedServerExecution>)) -> Curve<UnconstrainedServerExecution>,
>;

/// `CurveIterator` for calculating the actual execution of a Server
///
/// The resulting windows are in order and either adjacent or non-overlapping
///
#[derive(Debug)]
pub struct InternalActualExecutionIterator<AC, CDC> {
    /// the server for which to calculate the actual execution
    server_properties: ServerProperties,
    /// the remaining available execution
    available_execution:
        Box<CurveSplitIterator<<UnconstrainedServerExecution as CurveType>::WindowKind, AC>>,
    /// the peek of the remaining available execution that is not yet consumed
    execution_peek: Vec<Window<<UnconstrainedServerExecution as CurveType>::WindowKind>>,
    /// the group spend_budget is referring to
    current_group: UnitNumber,
    /// the spend budget of the current group
    // remembering one group is enough as we go through them in order
    spend_budget: TimeUnit,
    /// remaining constrained demand
    constrained_demand: Peeker<CurveIteratorIterator<CDC>, Window<Demand>>,
}

impl<'a, AC: Clone, CDC: Clone> Clone for InternalActualExecutionIterator<AC, CDC> {
    fn clone(&self) -> Self {
        InternalActualExecutionIterator {
            server_properties: self.server_properties,
            available_execution: self.available_execution.clone(),
            execution_peek: self.execution_peek.clone(),
            current_group: self.current_group,
            spend_budget: self.spend_budget,
            constrained_demand: self.constrained_demand.clone(),
        }
    }
}

impl<AC, CDC> InternalActualExecutionIterator<AC, CDC> {
    /// Create a new `ActualExecutionIterator`
    /// Takes a reference to a Server, the Servers constrained execution curve and the Servers constrained demand curve
    #[must_use]
    pub fn new(
        server_properties: ServerProperties,
        available_execution: AC,
        constrained_demand: CDC,
    ) -> Self
    where
        AC: CurveIterator<CurveKind = UnconstrainedServerExecution>,
        CDC: CurveIterator,
        CDC::CurveKind: CurveType<WindowKind = Demand>,
    {
        // Algorithm 4. (1)
        let split_execution =
            CurveSplitIterator::new(available_execution, server_properties.interval);

        InternalActualExecutionIterator {
            server_properties,
            available_execution: Box::new(split_execution),
            execution_peek: Vec::new(),
            current_group: 0,
            spend_budget: TimeUnit::ZERO,
            constrained_demand: Peeker::new(constrained_demand.into_iterator()),
        }
    }
}

impl<AC, CDC> FusedIterator for InternalActualExecutionIterator<AC, CDC>
where
    Self: Iterator,
    FlattenedSplitAvailableSupply<AC>: FusedIterator,
    CDC: FusedIterator,
{
}

impl<AC, CDC> Iterator for InternalActualExecutionIterator<AC, CDC>
where
    AC: CurveIterator<CurveKind = UnconstrainedServerExecution>,
    CDC: CurveIterator,
    CDC::CurveKind: CurveType<WindowKind = Demand>,
{
    type Item = Window<<ActualServerExecution as CurveType>::WindowKind>;

    // Algorithm 4. (4)
    fn next(&mut self) -> Option<Self::Item> {
        // (c)
        let demand = self.constrained_demand.next();

        // as we typically deal with limited demand but endless supply
        // check demand first
        if let Some(demand_window) = demand {
            loop {
                let supply = self
                    .execution_peek
                    .pop()
                    .or_else(|| self.available_execution.next());

                if let Some(mut supply_window) = supply {
                    let window_group = supply_window.budget_group(self.server_properties.interval);

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
                    } else if self.spend_budget >= self.server_properties.capacity {
                        if supply_window.end == WindowEnd::Infinite {
                            // Infinite supply window advance to next group
                            self.spend_budget = TimeUnit::ZERO;
                            self.current_group += 1;
                            supply_window.start =
                                self.current_group * self.server_properties.interval;
                        } else {
                            // budget exhausted skip supply window
                            continue;
                        }
                    }

                    // (b)

                    let remaining_budget = self.server_properties.capacity - self.spend_budget;

                    let valid_demand_segment = if demand_window.length() > remaining_budget {
                        let valid_end = demand_window.start + remaining_budget;
                        let valid = Window::new(demand_window.start, valid_end);
                        let residual = Window::new(valid_end, demand_window.end);

                        self.constrained_demand.restore_peek(residual);
                        valid
                    } else {
                        demand_window
                    };

                    // (d)
                    let result = Window::delta(&supply_window, &valid_demand_segment);

                    // (e)
                    match result.overlap.length() {
                        WindowEnd::Finite(length) => {
                            self.spend_budget += length;
                        }
                        WindowEnd::Infinite => {
                            unreachable!(
                                "valid_demand_segment has a length \
                            less than or equal to remaining_budget and therefore is finite,\
                            as such the overlap cannot be infinite"
                            )
                        }
                    }

                    // TODO
                    // it should be possible to also use a peeker for execution_peek and available_execution
                    // as the remaining_supply_head should always be useless and as such returned for the next next call
                    // we would still need to store it in between the call

                    vec![result.remaining_supply_head, result.remaining_supply_tail]
                        .into_iter()
                        .filter(|window| !window.is_empty())
                        .rev()
                        .for_each(|window| self.execution_peek.push(window));

                    break Some(result.overlap);
                } else {
                    assert!(
                        self.constrained_demand.next().is_none(),
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