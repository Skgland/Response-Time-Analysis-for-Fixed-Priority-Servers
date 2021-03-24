//! Module for the Task definition

use crate::curve::{Curve, Demand};
use crate::window::Window;

/// The Task type based on the Modeling described in the second paragraph
/// of Chapter 3. in the paper  
pub struct Task {
    /// The offset of the tasks, O index i in the paper
    offset: usize,
    /// The demand induced by the task
    /// called the worst-case execution time (WCET) C index i in the paper
    demand: usize,
    /// The interval of the task, called Periode P index i in the paper
    interval: usize,
}

impl Task {
    /// Create a new Task with the corresponding parameters
    ///
    /// # Panics
    /// If the interval is shorter than the demand
    #[must_use]
    pub fn new(demand: usize, interval: usize, offset: usize) -> Self {
        if interval < demand {
            panic!("Task can't have an interval shorter than its demand!")
        }

        Task {
            offset,
            demand,
            interval,
        }
    }

    /// Generate the Demand Curve for the Task up to he specified limit
    ///
    /// Only complete Cycles will be considered
    ///
    /// Based on Definition 10. of the paper
    #[must_use]
    pub fn demand_curve(&self, up_to: usize) -> Curve<Demand> {
        let mut start = self.offset;
        let mut curve = Curve::empty();

        while start <= (up_to - self.demand) {
            curve = curve.aggregate(Curve::new(Window::new(start, start + self.demand)));
            start += self.interval;
        }

        curve
    }
}

#[cfg(test)]
mod tests;
