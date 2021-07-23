//! Module for the System type

use crate::curve::AggregateExt;
use crate::iterators::curve::{AggregationIterator, CapacityCheckIterator, InverseCurveIterator};

use crate::server::{
    ActualServerExecution, ConstrainedServerDemand, HigherPriorityServerDemand, Server,
    UnconstrainedServerExecution,
};

use crate::curve::curve_types::CurveType;
use crate::iterators::server::actual_execution::ActualServerExecutionIterator;
use crate::iterators::{ClonableCurveIterator, CurveIterator, ReclassifyIterator};
use crate::time::TimeUnit;
use alloc::boxed::Box;
use alloc::vec::Vec;

/// Type representing a System of Servers
#[derive(Debug)]
pub struct System<'a> {
    /// The Servers of the System
    servers: &'a [Server<'a>],
}

impl<'a> System<'a> {
    /// Create a new System from a slice of Servers,
    /// indexed by their priority,
    /// lowest index being the highest priority
    #[must_use]
    pub const fn new(servers: &'a [Server<'a>]) -> System<'a> {
        System { servers }
    }

    /// Get a slice reference to the systems servers
    #[must_use]
    pub const fn as_servers(&self) -> &'a [Server<'a>] {
        self.servers
    }

    /// Calculate the aggregated higher priority demand curve
    /// by aggregating the aggregated demand curves of all Servers with higher priority
    /// (lower value) than `index`.
    ///
    /// Based on the papers Definition 12.
    #[must_use]
    pub fn aggregated_higher_priority_demand_curve_iter<'b, CSDCI>(
        constrained_demand_curves: CSDCI,
    ) -> impl CurveIterator<CurveKind = HigherPriorityServerDemand> + Clone + 'b
    where
        CSDCI::Item: CurveIterator<CurveKind = ConstrainedServerDemand> + Clone + 'b,
        CSDCI: IntoIterator,
    {
        constrained_demand_curves
            .into_iter()
            .aggregate::<ReclassifyIterator<_, _>>()
    }

    pub fn aggregated_higher_priority_actual_execution_curve_iter<'b>(
        &'b self,
        server_index: usize,
    ) -> AggregationIterator<
        Box<dyn ClonableCurveIterator<'b, CurveKind = ActualServerExecution> + 'b>,
        <ActualServerExecution as CurveType>::WindowKind,
    > {
        let mut curves: Vec<Box<dyn ClonableCurveIterator<CurveKind = ActualServerExecution>>> =
            alloc::vec::Vec::with_capacity(2);

        if server_index > 0 {
            curves.push(Box::new(
                self.fixed_actual_execution_curve_iter(server_index - 1),
            ));
            if server_index > 1 {
                let curve = self
                    .aggregated_higher_priority_actual_execution_curve_iter(server_index - 1)
                    .reclassify();
                curves.push(Box::new(curve));
            }
        }

        AggregationIterator::new(curves)
    }

    /// Calculate the system wide hyper period
    /// accounting for all servers and tasks
    /// up to and including the server with priority `server_index`
    ///
    /// Section 7.1 §2 Sentence 3, allows to exclude lower priority servers from the swh period calculation,
    /// when analysing tasks of a server
    #[must_use]
    pub fn system_wide_hyper_period(&self, server_index: usize) -> TimeUnit {
        self.servers[..=server_index]
            .iter()
            .map(|server| server.properties.interval)
            .chain(
                self.servers
                    .iter()
                    .flat_map(|server| server.as_tasks().iter().map(|task| task.interval)),
            )
            .fold(TimeUnit::ONE, TimeUnit::lcm)
    }

    /// Calculate the unconstrained execution curve
    /// for the server with priority `index`.
    ///
    /// See Definition 13. of the paper for reference
    #[must_use]
    pub fn original_unconstrained_server_execution_curve_iter(
        &self,
        server_index: usize,
    ) -> impl CurveIterator<CurveKind = UnconstrainedServerExecution> + Clone + '_ {
        #![allow(clippy::redundant_closure_for_method_calls)]

        let csdi = self.servers[..server_index]
            .iter()
            .map(|server| server.constraint_demand_curve_iter());

        let ahpc = System::aggregated_higher_priority_demand_curve_iter(csdi);

        InverseCurveIterator::new(ahpc)
    }

    #[must_use]
    pub fn fixed_unconstrained_server_execution_curve_iter(
        &self,
        server_index: usize,
    ) -> impl CurveIterator<CurveKind = UnconstrainedServerExecution> + Clone + '_ {
        let ahpc = self.aggregated_higher_priority_actual_execution_curve_iter(server_index);

        InverseCurveIterator::new(ahpc)
    }

    /// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
    /// TODO more detail, what do the parameters mean
    /// # Panics
    ///
    /// When a server is not guaranteed its capacity every interval
    ///
    #[must_use]
    pub fn original_actual_execution_curve_iter(
        &self,
        server_index: usize,
    ) -> impl CurveIterator<CurveKind = ActualServerExecution> + Clone + '_ {
        let unchecked_unconstrained_execution =
            self.original_unconstrained_server_execution_curve_iter(server_index);

        let props = self.servers[server_index].properties;

        // split unconstrained execution curve into groups every server.interval
        // and check that each group has at least server.capacity of capacity
        let checked_unconstrained_execution = CapacityCheckIterator::new(
            unchecked_unconstrained_execution,
            props.capacity,
            props.interval,
        );

        let constrained_demand = self.servers[server_index].constraint_demand_curve_iter();

        ActualServerExecutionIterator::new(
            self.servers[server_index].properties,
            checked_unconstrained_execution,
            constrained_demand,
        )
    }

    #[must_use]
    pub fn fixed_actual_execution_curve_iter(
        &self,
        server_index: usize,
    ) -> impl CurveIterator<CurveKind = ActualServerExecution> + Clone + '_ {
        let unchecked_unconstrained_execution =
            self.fixed_unconstrained_server_execution_curve_iter(server_index);

        let props = self.servers[server_index].properties;

        // split unconstrained execution curve into groups every server.interval
        // and check that each group has at least server.capacity of capacity
        let checked_unconstrained_execution = CapacityCheckIterator::new(
            unchecked_unconstrained_execution,
            props.capacity,
            props.interval,
        );

        let constrained_demand = self.servers[server_index].constraint_demand_curve_iter();

        ActualServerExecutionIterator::new(
            self.servers[server_index].properties,
            checked_unconstrained_execution,
            constrained_demand,
        )
    }
}
