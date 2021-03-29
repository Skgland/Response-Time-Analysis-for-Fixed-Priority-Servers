//! Module for Server definition
//!
//! and functions to be used with one or multiple Servers

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
}

#[cfg(test)]
mod tests;
