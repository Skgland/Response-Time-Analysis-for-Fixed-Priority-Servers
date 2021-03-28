//! Module for Server definition
//!
//! and functions to be used with one or multiple Servers

use crate::curve::curve_types::PrimitiveCurve;
use crate::curve::{AggregateExt, Curve};
use crate::task::Task;
use crate::time::TimeUnit;

/// Marker Type for aggregated server demand curve
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct AggregatedServerDemand;

/// Marker Type for constrained server demand curve
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct ConstrainedServerDemand;

/// Marker Type for aggregated higher server server demand curve
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct HigherPriorityServerDemand;

/// Marker Type for unconstrained server demand curve
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct AvailableServerExecution;

/// Marker Type for constrained server execution curve
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct ConstrainedServerExecution;

/// Type Representing a Server
/// with a given set of tasks,
/// a capacity for fulfilling demand,
/// a replenishment interval for how
/// often the capacity is restored
/// ,and a server type determining if the
/// capacity is available only at the beginning of the interval
/// or until it is used up
#[derive(Debug)]
pub struct Server {
    /// The Tasks that produce Demand for this Server
    /// Sorted by priority with lower index equalling higher priority
    pub tasks: Vec<Task>,
    /// The capacity for fulfilling Demand
    pub capacity: TimeUnit,
    /// How often the capacity is available
    pub interval: TimeUnit,
    /// How the available capacity behaves
    pub server_type: ServerKind,
}

/// The Type of a Server
#[derive(Debug)]
pub enum ServerKind {
    /// Indicated that the Server is a Deferrable Server
    /// as described/defined in Section 5.2 Paragraph 2 of the paper
    Deferrable,
    /// Indicates that the Server is a Periodic Server
    /// as described/defined in Section 5.2 Paragraph 4 of the paper
    Periodic,
}

impl Server {
    /// Get a a reference to a slice of the Servers contained Tasks
    #[must_use]
    pub fn as_tasks(&self) -> &[Task] {
        self.tasks.as_slice()
    }

    /// Calculate the aggregated demand Curve of a given Server up to a specified limit
    /// As defined in Definition 11. in the paper
    #[must_use]
    pub fn aggregated_demand_curve(&self, up_to: TimeUnit) -> Curve<AggregatedServerDemand> {
        self.tasks
            .iter()
            .map(|task| task.demand_curve(up_to))
            .aggregate()
    }

    /// Calculate the constrained demand curve
    #[must_use]
    pub fn constrain_demand_curve(&self, up_to: TimeUnit) -> Curve<ConstrainedServerDemand> {
        let aggregated_curve = self.aggregated_demand_curve(up_to);
        crate::paper::constrained_server_demand(self, aggregated_curve)
    }

    /// Calculate the aggregated higher priority demand curve
    /// by aggregating the aggregated demand curves of all Servers with higher priority
    /// (lower value) than `index`.
    ///
    /// The index in the `servers` slice corresponds to the priority of the Server
    /// a lower index equals higher priority
    ///
    /// Based on the papers Definition 12.
    #[must_use]
    pub fn aggregated_higher_priority_demand_curve(
        servers: &[Server],
        index: usize,
        up_to: TimeUnit,
    ) -> Curve<HigherPriorityServerDemand> {
        servers[..index]
            .iter()
            .map(|server| server.constrain_demand_curve(up_to))
            .aggregate()
    }

    /// Calculate the unconstrained execution curve
    /// for the server with priority `index`.
    ///
    /// The Priority of a server is its index in the `servers` slice,
    /// a lower index entails a higher priority.
    ///
    /// See Definition 14. (2) of the paper for reference
    #[must_use]
    pub fn available_server_execution_curve(
        servers: &[Server],
        index: usize,
        up_to: TimeUnit,
    ) -> Curve<AvailableServerExecution> {
        let result = Curve::delta::<_, PrimitiveCurve<_>>(
            Curve::total(up_to),
            Server::aggregated_higher_priority_demand_curve(servers, index, up_to),
        );
        result.remaining_supply
    }

    /// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
    /// TODO more detail, what do the parameters mean
    #[must_use]
    pub fn actual_execution_curve(
        servers: &[Server],
        index: usize,
        up_to: TimeUnit,
    ) -> Curve<ConstrainedServerExecution> {
        // Input

        let unconstrained_execution =
            Server::available_server_execution_curve(servers, index, up_to);
        let constrained_demand = servers[index].constrain_demand_curve(up_to);

        crate::paper::actual_server_execution(
            servers,
            index,
            unconstrained_execution,
            constrained_demand,
        )
    }

    /// Calculate the system wide hyper periode
    /// accounting for all servers and tasks
    /// up to and including the server with priority `server_index`
    ///
    /// Section 7.1 §2 Sentence 3, allows to exclude lower priority servers from the swh periode calculation,
    /// when analysing tasks of a server
    pub fn system_wide_hyper_periode(servers: &[Server], server_index: usize) -> TimeUnit {
        servers[..=server_index]
            .iter()
            .map(|server| server.interval)
            .chain(
                servers
                    .iter()
                    .flat_map(|server| server.as_tasks().iter().map(|task| task.interval)),
            )
            .fold(TimeUnit::ONE, TimeUnit::lcm)
    }
}

#[cfg(test)]
mod tests;
