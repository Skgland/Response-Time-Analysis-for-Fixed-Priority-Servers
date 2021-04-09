//! Module for the System type

use crate::curve::AggregateExt;
use crate::iterators::curve::{CurveSplitIterator, InverseCurveIterator};

use crate::server::{
    ActualServerExecution, ConstrainedServerDemand, HigherPriorityServerDemand, Server,
    UnconstrainedServerExecution,
};

use crate::curve::curve_types::CurveType;
use crate::iterators::server::actual_execution::ActualServerExecutionIterator;
use crate::iterators::{CurveIterator, JoinAdjacentIterator, ReclassifyIterator};
use crate::time::TimeUnit;

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
    ) -> impl CurveIterator<
        <HigherPriorityServerDemand as CurveType>::WindowKind,
        CurveKind = HigherPriorityServerDemand,
    > + Clone
           + 'b
    where
        CSDCI::Item: CurveIterator<
                <ConstrainedServerDemand as CurveType>::WindowKind,
                CurveKind = ConstrainedServerDemand,
            > + Clone
            + 'b,
        CSDCI: IntoIterator,
    {
        constrained_demand_curves
            .into_iter()
            .aggregate::<ReclassifyIterator<_, _, _>>()
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
    pub fn unconstrained_server_execution_curve_iter(
        &self,
        server_index: usize,
    ) -> impl CurveIterator<
        <UnconstrainedServerExecution as CurveType>::WindowKind,
        CurveKind = UnconstrainedServerExecution,
    > + Clone
           + '_ {
        let csdi = self.servers[..server_index]
            .iter()
            .map(move |server| server.constraint_demand_curve_iter());

        let ahpc = System::aggregated_higher_priority_demand_curve_iter(csdi);

        InverseCurveIterator::new(ahpc)
    }

    /// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
    /// TODO more detail, what do the parameters mean
    /// # Panics
    ///
    /// When a server is not guaranteed its capacity every interval
    ///
    #[must_use]
    pub fn actual_execution_curve_iter(
        &self,
        server_index: usize,
    ) -> impl CurveIterator<
        <ActualServerExecution as CurveType>::WindowKind,
        CurveKind = ActualServerExecution,
    > + Clone
           + '_ {
        let unchecked_unconstrained_execution =
            self.unconstrained_server_execution_curve_iter(server_index);

        let min_capacity = self.servers[server_index].properties.capacity;

        // split unconstrained execution curve into groups every server.interval
        // and check that each group has at least server.capacity of capacity
        let checked = CurveSplitIterator::new(
            unchecked_unconstrained_execution,
            self.servers[server_index].properties.interval,
        )
        .inspect(move |(_, group)| assert!(group.capacity() >= min_capacity))
        .flat_map(|(_, group)| group.into_iter());

        let checked_unconstrained_execution = unsafe { JoinAdjacentIterator::new(checked) };

        let constrained_demand = self.servers[server_index].constraint_demand_curve_iter();

        ActualServerExecutionIterator::new(
            self.servers[server_index].properties,
            checked_unconstrained_execution,
            constrained_demand,
        )
    }
}
