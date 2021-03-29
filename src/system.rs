//! Module for the System type

use crate::curve::curve_types::PrimitiveCurve;
use crate::curve::{AggregateExt, Curve};
use crate::paper::check_assumption;
use crate::server::{
    AvailableServerExecution, ConstrainedServerExecution, HigherPriorityServerDemand, Server,
};
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
    pub fn aggregated_higher_priority_demand_curve(
        &self,
        index: usize,
        up_to: TimeUnit,
    ) -> Curve<HigherPriorityServerDemand> {
        self.servers[..index]
            .iter()
            .map(|server| server.constraint_demand_curve(up_to))
            .aggregate()
    }

    /// Calculate the system wide hyper periode
    /// accounting for all servers and tasks
    /// up to and including the server with priority `server_index`
    ///
    /// Section 7.1 §2 Sentence 3, allows to exclude lower priority servers from the swh periode calculation,
    /// when analysing tasks of a server
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
    pub fn available_server_execution_curve(
        &self,
        server_index: usize,
        up_to: TimeUnit,
    ) -> Curve<AvailableServerExecution> {
        let result = Curve::delta::<_, PrimitiveCurve<_>>(
            Curve::total(up_to),
            self.aggregated_higher_priority_demand_curve(server_index, up_to),
        );

        result.remaining_supply
    }

    /// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
    /// TODO more detail, what do the parameters mean
    ///
    /// # Panics
    /// When the assumption is violated that each server has it's capacity available
    /// each periode
    #[must_use]
    pub fn actual_execution_curve(
        &self,
        server_index: usize,
        up_to: TimeUnit,
    ) -> Curve<ConstrainedServerExecution> {
        // Input

        let unconstrained_execution = self.available_server_execution_curve(server_index, up_to);

        assert!(
            check_assumption(
                &self.as_servers()[server_index],
                unconstrained_execution.clone(),
                up_to
            ),
            "Up to: {:?}\nServer: {:#?}\nSupply: {:#?}",
            up_to,
            &self.as_servers()[server_index],
            unconstrained_execution
        );
        let constrained_demand = self.servers[server_index].constraint_demand_curve(up_to);

        crate::paper::actual_server_execution(
            self.servers,
            server_index,
            unconstrained_execution,
            constrained_demand,
        )
    }
}

#[cfg(test)]
mod tests;
