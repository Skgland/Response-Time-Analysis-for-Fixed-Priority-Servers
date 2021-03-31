use std::iter::FusedIterator;

use crate::curve::curve_types::CurveType;
use crate::curve::AggregateExt;
use crate::iterators::curve::RecursiveAggregatedDemandIterator;
use crate::iterators::CurveIterator;
use crate::task::{HigherPriorityTaskDemand, Task, TaskDemand};
use crate::time::UnitNumber;
use crate::window::{Demand, Window};

/// `CurveIterator` for a Tasks Demand
#[derive(Debug, Clone)]
pub struct TaskDemandIterator<'a> {
    /// the Task this Iterator generates demand for
    task: &'a Task,
    /// The next Job index for which to generate Demand
    next_job: UnitNumber,
}

impl<'a> TaskDemandIterator<'a> {
    /// Create a `CurveIterator` for a Tasks Demand
    #[must_use]
    pub const fn new(task: &'a Task) -> Self {
        TaskDemandIterator { task, next_job: 0 }
    }
}

impl<'a> CurveIterator<Demand> for TaskDemandIterator<'a> {
    type CurveKind = TaskDemand;
}

impl FusedIterator for TaskDemandIterator<'_> {}

impl Iterator for TaskDemandIterator<'_> {
    type Item = Window<<TaskDemand as CurveType>::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_job == UnitNumber::MAX {
            // prevent overflow of self.next_job
            eprintln!("Task reached overflow! {:?}", self.task);
            None
        } else {
            // TODO this will overflow before self.next_job
            // unless interval is 1 and offset 0
            let start = self.task.offset + self.next_job * self.task.interval;
            let end = start + self.task.demand;
            self.next_job += 1;
            Some(Window::new(start, end))
        }
    }
}

/// `CurveIterator` for Higher Priority Task Demand
#[derive(Debug)]
pub struct HigherPriorityTaskDemandIterator<'a> {
    /// The wrapped curve iterator
    iterator: RecursiveAggregatedDemandIterator<'a, TaskDemand>,
}

impl<'a> Clone for HigherPriorityTaskDemandIterator<'a> {
    fn clone(&self) -> Self {
        HigherPriorityTaskDemandIterator {
            iterator: self.iterator.clone(),
        }
    }
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

impl<'a> CurveIterator<<HigherPriorityTaskDemand as CurveType>::WindowKind>
    for HigherPriorityTaskDemandIterator<'a>
{
    type CurveKind = HigherPriorityTaskDemand;
}

impl<'a> FusedIterator for HigherPriorityTaskDemandIterator<'a>
where
    Self: Iterator,
    RecursiveAggregatedDemandIterator<'a, TaskDemand>: FusedIterator,
{
}

impl<'a> Iterator for HigherPriorityTaskDemandIterator<'a> {
    type Item = Window<<HigherPriorityTaskDemand as CurveType>::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next()
    }
}
