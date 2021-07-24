//! Module for Server definition
//!
//! and functions to be used with one or multiple Servers

use crate::curve::AggregateExt;

use crate::iterators::curve::AggregationIterator;
use crate::iterators::server::constrained_demand::ConstrainedServerDemandIterator;
use crate::iterators::task::TaskDemandIterator;
use crate::iterators::ReclassifyIterator;
use crate::task::Task;
use crate::time::TimeUnit;
use crate::window::Demand;

/// Marker Type for aggregated server demand curve
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct AggregatedServerDemand;

/// Marker Type for constrained server demand curve
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct ConstrainedServerDemand;

/// Marker Type for aggregated higher priority server demand curve
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct HigherPriorityServerDemand;

/// Marker Type for aggregated higher priority server actual Execution curve
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct HigherPriorityServerExecution;

/// Marker Type for unconstrained server demand curve
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct UnconstrainedServerExecution;

/// Marker Type for constrained server execution curve
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct ActualServerExecution;

/// Type Representing a Server
/// with a given set of tasks,
/// a capacity for fulfilling demand,
/// a replenishment interval for how
/// often the capacity is restored
/// ,and a server type determining if the
/// capacity is available only at the beginning of the interval
/// or until it is used up
#[derive(Debug, Clone)]
pub struct Server<'a> {
    /// The Tasks that produce Demand for this Server
    /// Sorted by priority with lower index equalling higher priority
    pub tasks: &'a [Task],
    /// The properties of the Server
    pub properties: ServerProperties,
}

/// The Properties of a server
#[derive(Debug, Clone, Copy)]
pub struct ServerProperties {
    /// The capacity for fulfilling Demand
    pub capacity: TimeUnit,
    /// How often the capacity is available
    pub interval: TimeUnit,
    /// How the available capacity behaves
    pub server_type: ServerKind,
}

/// The Type of a Server
#[derive(Debug, Clone, Copy)]
pub enum ServerKind {
    /// Indicated that the Server is a Deferrable Server
    /// as described/defined in Section 5.2 Paragraph 2 of the paper
    Deferrable,
    /// Indicates that the Server is a Periodic Server
    /// as described/defined in Section 5.2 Paragraph 4 of the paper
    Periodic,
}

pub type AggregatedTaskDemand =
    ReclassifyIterator<AggregationIterator<TaskDemandIterator, Demand>, AggregatedServerDemand>;

pub type ConstrainedDemand = ConstrainedServerDemandIterator<AggregatedTaskDemand>;

impl<'a> Server<'a> {
    /// Create a new Server with the given Tasks and properties
    #[must_use]
    pub const fn new(
        tasks: &'a [Task],
        capacity: TimeUnit,
        interval: TimeUnit,
        server_type: ServerKind,
    ) -> Self {
        Server {
            tasks,
            properties: ServerProperties {
                capacity,
                interval,
                server_type,
            },
        }
    }

    /// Get a a reference to a slice of the Servers contained Tasks
    #[must_use]
    pub const fn as_tasks(&self) -> &'a [Task] {
        self.tasks
    }

    /// Calculate the aggregated demand Curve of a given Server up to a specified limit
    /// As defined in Definition 11. in the paper
    #[must_use]
    pub fn aggregated_demand_curve_iter(&self) -> AggregatedTaskDemand {
        self.tasks
            .iter()
            .map(|task| task.into_iter())
            .aggregate::<ReclassifyIterator<_, _>>()
    }

    /// Calculate the constrained demand curve
    #[must_use]
    pub fn constraint_demand_curve_iter(&self) -> ConstrainedDemand {
        ConstrainedServerDemandIterator::new(self.properties, self.aggregated_demand_curve_iter())
    }
}
