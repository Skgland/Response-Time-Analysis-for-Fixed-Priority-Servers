//! Module for implementing `CurveIterator`s relate to Tasks

use crate::curve::curve_types::CurveType;
use crate::iterators::CurveIterator;
use crate::task::curve_types::TaskDemand;
use crate::task::Task;
use crate::time::{TimeUnit, UnitNumber};
use crate::window::{Demand, Window};

/// `CurveIterator` for a Tasks Demand
#[derive(Debug, Clone)]
pub struct TaskDemandIterator {
    /// the Task this Iterator generates demand for
    task: Task,
    /// The next Job index for which to generate Demand
    next_job: UnitNumber,
}

impl TaskDemandIterator {
    /// Create a `CurveIterator` for a Tasks Demand
    #[must_use]
    pub const fn new(task: Task) -> Self {
        TaskDemandIterator { task, next_job: 0 }
    }
}

impl CurveIterator for TaskDemandIterator {
    type CurveKind = TaskDemand;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        // using checked arithmetic to stop on overflow
        let start = self
            .task
            .offset
            .as_unit()
            .checked_add(self.next_job.checked_mul(self.task.interval.as_unit())?)?;
        let end = UnitNumber::checked_add(start, self.task.demand.as_unit())?;
        self.next_job = self.next_job.checked_add(1)?;
        Some(Window::new(TimeUnit::from(start), TimeUnit::from(end)))
    }
}

impl Iterator for TaskDemandIterator {
    type Item = Window<Demand>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_window()
    }
}
