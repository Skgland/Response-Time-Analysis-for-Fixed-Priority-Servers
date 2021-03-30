//! Module for the System type

use crate::curve::{AggregateExt, Curve};
use crate::iterators::curve::{CurveDeltaIterator, RecursiveAggregatedDemandIterator};

use crate::server::{
    ActualServerExecution, AvailableServerExecution, HigherPriorityServerDemand, Server,
};

use crate::iterators::{CurveIterator, ReclassifyExt};
use crate::time::TimeUnit;

/// Type representing a System of Servers
#[derive(Debug)]
pub struct System<'a> {
    /// The Servers of the System
    servers: &'a [Server],
}

impl System<'_> {
    /// Create a new System from a slice of Servers,
    /// indexed by their priority,
    /// lowest index being the highest priority
    #[must_use]
    pub const fn new(servers: &[Server]) -> System {
        System { servers }
    }

    /// Get a slice reference to the systems servers
    #[must_use]
    pub const fn as_servers(&self) -> &[Server] {
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
    ) -> impl CurveIterator<HigherPriorityServerDemand> + Clone {
        self.servers[..server_index]
            .iter()
            .map(move |server| server.constraint_demand_curve_iter(up_to))
            .aggregate::<RecursiveAggregatedDemandIterator<_>>()
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
    /// See Definition 14. (2) of the paper for reference
    #[must_use]
    pub fn available_server_execution_curve_iter(
        &self,
        server_index: usize,
        up_to: TimeUnit,
    ) -> impl CurveIterator<AvailableServerExecution> + Clone {
        let total: Curve<AvailableServerExecution> = Curve::total(up_to);

        CurveDeltaIterator::new(
            total.into_iter(),
            self.aggregated_higher_priority_demand_curve_iter(server_index, up_to),
        )
        .remaining_supply::<AvailableServerExecution>()
    }

    /// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
    /// TODO more detail, what do the parameters mean
    ///
    #[must_use]
    pub fn actual_execution_curve_iter(
        &self,
        server_index: usize,
        up_to: TimeUnit,
    ) -> impl CurveIterator<ActualServerExecution> + Clone {
        let unconstrained_execution =
            self.available_server_execution_curve_iter(server_index, up_to);

        // TODO re-introduce check regarding guaranteed capacity each interval

        let constrained_demand = self.servers[server_index].constraint_demand_curve_iter(up_to);

        crate::paper::actual_server_execution_iter(
            self.servers,
            server_index,
            unconstrained_execution,
            constrained_demand,
        )
    }
}

#[cfg(test)]
mod tests;
