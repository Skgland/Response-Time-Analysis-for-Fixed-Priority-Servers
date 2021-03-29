use std::iter::FusedIterator;

use crate::curve::curve_types::CurveType;
use crate::curve::AggregateExt;
use crate::iterators::curve::AggregatedDemandIterator;
use crate::iterators::CurveIterator;
use crate::task::{HigherPriorityTaskDemand, Task, TaskDemand};
use crate::window::Window;

/// `CurveIterator` for a Tasks Demand
#[derive(Debug, Clone)]
pub struct TaskDemandIterator<'a> {
    /// the Task this Iterator generates demand for
    task: &'a Task,
    /// The next Job index for which to generate Demand
    next_job: usize,
}

impl<'a> TaskDemandIterator<'a> {
    /// Create a `CurveIterator` for a Tasks Demand
    #[must_use]
    pub const fn new(task: &'a Task) -> Self {
        TaskDemandIterator { task, next_job: 0 }
    }
}

impl<'a> CurveIterator<'a, TaskDemand> for TaskDemandIterator<'a> {}

impl FusedIterator for TaskDemandIterator<'_> {}

impl Iterator for TaskDemandIterator<'_> {
    type Item = Window<<TaskDemand as CurveType>::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO handle overflow, don't advance and return None, when overflow would occur
        let job = self.next_job;
        self.next_job += 1;
        let start = self.task.offset + job * self.task.interval;
        let end = start + self.task.demand;
        Some(Window::new(start, end))
    }
}

/// `CurveIterator` for Higher Priority Task Demand
#[derive(Debug)]
pub struct HigherPriorityTaskDemandIterator<'a> {
    /// The wrapped curve iterator
    iterator: AggregatedDemandIterator<
        'a,
        TaskDemand,
        Box<dyn CurveIterator<'a, TaskDemand>>,
        Box<dyn CurveIterator<'a, TaskDemand>>,
    >,
}

impl<'a> HigherPriorityTaskDemandIterator<'a> {
    /// Create a `CurveIterator` for the aggregated Demand of
    /// all task with higher priority than `task_index`
    #[must_use]
    pub fn new(tasks: &'a [Task], task_index: usize) -> Self {
        let aggregate = tasks[..task_index]
            .iter()
            .map(|task| task.into_iter())
            .aggregate();
        Self {
            iterator: aggregate,
        }
    }
}

impl<'a> CurveIterator<'a, HigherPriorityTaskDemand> for HigherPriorityTaskDemandIterator<'a> {}

impl<'a> FusedIterator for HigherPriorityTaskDemandIterator<'a> {}

impl<'a> Iterator for HigherPriorityTaskDemandIterator<'a> {
    type Item = Window<<HigherPriorityTaskDemand as CurveType>::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next()
    }
}
