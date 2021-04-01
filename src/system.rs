//! Module for the System type

use crate::curve::{AggregateExt, Curve};
use crate::iterators::curve::{AggregationIterator, CurveDeltaIterator, InverseCurveIterator};

use crate::server::{
    ActualServerExecution, HigherPriorityServerDemand, Server, UnconstrainedServerExecution,
};

use crate::curve::curve_types::CurveType;
use crate::iterators::server::actual_execution::ActualServerExecutionIterator;
use crate::iterators::CurveIterator;
use crate::time::TimeUnit;
use crate::window::Window;

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
    pub fn aggregated_higher_priority_demand_curve_iter(
        &self,
        server_index: usize,
        up_to: TimeUnit,
    ) -> impl CurveIterator<
        <HigherPriorityServerDemand as CurveType>::WindowKind,
        CurveKind = HigherPriorityServerDemand,
    > + Clone
           + '_ {
        self.servers[..server_index]
            .iter()
            .map(move |server| server.constraint_demand_curve_iter(up_to))
            .aggregate::<AggregationIterator<_>>()
            .reclassify()
    }

    /// Calculate the system wide hyper periode
    /// accounting for all servers and tasks
    /// up to and including the server with priority `server_index`
    ///
    /// Section 7.1 §2 Sentence 3, allows to exclude lower priority servers from the swh periode calculation,
    /// when analysing tasks of a server
    #[must_use]
    pub fn system_wide_hyper_periode(&self, server_index: usize) -> TimeUnit {
        self.servers[..=server_index]
            .iter()
            .map(|server| server.interval)
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
        up_to: TimeUnit,
    ) -> impl CurveIterator<
        <UnconstrainedServerExecution as CurveType>::WindowKind,
        CurveKind = UnconstrainedServerExecution,
    > + Clone
           + '_ {
        let ahpc = self.aggregated_higher_priority_demand_curve_iter(server_index, up_to);

        InverseCurveIterator::new(ahpc, up_to)
    }

    /// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
    /// TODO more detail, what do the parameters mean
    ///
    #[must_use]
    pub fn actual_execution_curve_iter(
        &self,
        server_index: usize,
        up_to: TimeUnit,
    ) -> impl CurveIterator<
        <ActualServerExecution as CurveType>::WindowKind,
        CurveKind = ActualServerExecution,
    > + Clone
           + '_ {
        let unconstrained_execution =
            self.unconstrained_server_execution_curve_iter(server_index, up_to);

        // TODO re-introduce check regarding guaranteed capacity each interval
        /*
        pub fn check_assumption(
            server: &Server,
            available: Curve<AvailableServerExecution>,
            up_to: TimeUnit,
        ) -> bool {
            let groups = available.split(server.interval);

            for interval_index in 0..=((up_to - TimeUnit::ONE) / server.interval) {
                if !groups
                    .get(&interval_index)
                    .map_or(false, |curve| curve.capacity() >= server.capacity)
                {
                    return false;
                }
            }

            true
        }
        */

        let constrained_demand = self.servers[server_index].constraint_demand_curve_iter(up_to);

        ActualServerExecutionIterator::new(
            &self.servers[server_index],
            unconstrained_execution,
            constrained_demand,
        )
    }
}
