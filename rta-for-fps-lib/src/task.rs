//! Module for the Task definition

use crate::curve::curve_types::CurveType;
use crate::curve::{AggregateExt, Curve};
use crate::iterators::curve::{
    AggregationIterator, CurveDeltaIterator, OverlapIterator, RemainingSupplyIterator,
};
use crate::iterators::task::TaskDemandIterator;
use crate::iterators::{CurveIterator, ReclassifyIterator};
use crate::server::ActualServerExecution;
use crate::system::{FixedActualExecution, OriginalActualServerExecution, System};
use crate::task::curve_types::{
    ActualTaskExecution, AvailableTaskExecution, HigherPriorityTaskDemand,
};
use crate::time::{TimeUnit, UnitNumber};
use crate::window::WindowEnd;
use crate::window::{Demand, Window};
use core::fmt::Debug;

pub mod curve_types {
    //! Module for `CurveType`s of a Task

    /// Marker Type for Curves representing a Tasks Demand
    #[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
    pub struct TaskDemand;

    /// Mark Type for Curves representing aggregated higher priority task demand
    #[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
    pub struct HigherPriorityTaskDemand;

    /// Marker type for Curves representing the available execution for a task
    #[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
    pub struct AvailableTaskExecution;

    /// Marker type for Curves representing the actual execution for a task
    #[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
    pub struct ActualTaskExecution;
}

/// The Task type based on the Modeling described in the second paragraph
/// of Chapter 3. in the paper
#[derive(Debug, Clone, Copy)]
pub struct Task {
    /// The offset of the tasks, O index i in the paper
    pub offset: TimeUnit,
    /// The demand induced by the task
    /// called the worst-case execution time (WCET) C index i in the paper
    pub demand: TimeUnit,
    /// The interval of the task, called Period P index i in the paper
    pub interval: TimeUnit,
}

/**
    A `CurveIterator` over a tasks available execution curve
*/
#[derive(Clone, Debug)]
pub struct AvailableExecution<HPTD, ASEC>(
    ReclassifyIterator<
        RemainingSupplyIterator<
            <ActualServerExecution as CurveType>::WindowKind,
            Demand,
            ASEC,
            HPTD,
        >,
        AvailableTaskExecution,
    >,
);

impl<HPTD, ASEC> CurveIterator for AvailableExecution<HPTD, ASEC>
where
    Self: Debug,
    HPTD: CurveIterator,
    HPTD::CurveKind: CurveType<WindowKind = Demand>,
    ASEC: CurveIterator,
    ASEC::CurveKind: CurveType<WindowKind = <ActualServerExecution as CurveType>::WindowKind>,
{
    type CurveKind = AvailableTaskExecution;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.0.next_window()
    }
}

/**
A `CurveIterator` over the actual task execution of a task using the original algorithm
 */
#[derive(Clone, Debug)]
pub struct OriginalActualTaskExecution(
    OverlapIterator<
        TaskDemandIterator,
        AvailableExecution<AggregatedHPTaskDemand, OriginalActualServerExecution>,
        Demand,
        <AvailableTaskExecution as CurveType>::WindowKind,
        ActualTaskExecution,
    >,
);

impl CurveIterator for OriginalActualTaskExecution {
    type CurveKind = ActualTaskExecution;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.0.next_window()
    }
}

/**
   A `CurveIterator` over the actual task execution of a task using the fixed algorithm
*/
#[derive(Clone, Debug)]
pub struct FixedActualTaskExecution(
    OverlapIterator<
        TaskDemandIterator,
        AvailableExecution<AggregatedHPTaskDemand, FixedActualExecution>,
        Demand,
        <AvailableTaskExecution as CurveType>::WindowKind,
        ActualTaskExecution,
    >,
);

impl CurveIterator for FixedActualTaskExecution {
    type CurveKind = ActualTaskExecution;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.0.next_window()
    }
}

/**
 * A `CurveIterator` over a tasks aggregated higher priority task demand
 */
#[derive(Clone, Debug)]
pub struct AggregatedHPTaskDemand(
    ReclassifyIterator<AggregationIterator<TaskDemandIterator, Demand>, HigherPriorityTaskDemand>,
);

impl CurveIterator for AggregatedHPTaskDemand {
    type CurveKind = HigherPriorityTaskDemand;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.0.next_window()
    }
}

impl Task {
    /// Create a new Task with the corresponding parameters
    ///
    /// # Panics
    /// If the interval is shorter than the demand
    #[must_use]
    pub fn new<I: Into<TimeUnit>>(demand: I, interval: I, offset: I) -> Self {
        let demand = demand.into();
        let interval = interval.into();

        if interval < demand {
            panic!("Task can't have an interval shorter than its demand!")
        }

        Task {
            offset: offset.into(),
            demand,
            interval,
        }
    }

    /// calculate the Higher Priority task Demand for the task with priority `index` as defined in Definition 14. (1) in the paper,
    /// for a set of tasks indexed by their priority (lower index <=> higher priority) and up to the specified limit
    #[must_use]
    pub fn higher_priority_task_demand_iter(
        tasks: &[Self],
        index: usize,
    ) -> AggregatedHPTaskDemand {
        AggregatedHPTaskDemand(
            tasks[..index]
                .iter()
                .map(|task| task.into_iter())
                .aggregate::<ReclassifyIterator<_, _>>(),
        )
    }

    /// Calculate the available execution Curve for the task with priority `task_index` of the server with priority `server_index`
    /// up to the specified limit.
    ///
    /// Based on Definition 14. (2) of the paper
    #[must_use]
    pub fn available_execution_curve_impl<'a, HPTD, ASEC>(
        constrained_server_execution_curve: ASEC,
        higher_priority_task_demand: HPTD,
    ) -> AvailableExecution<HPTD, ASEC>
    where
        HPTD: CurveIterator<CurveKind = HigherPriorityTaskDemand> + Clone + 'a,
        ASEC: CurveIterator<CurveKind = ActualServerExecution> + Clone + 'a,
    {
        let delta = CurveDeltaIterator::new(
            constrained_server_execution_curve,
            higher_priority_task_demand,
        );

        AvailableExecution(
            delta
                .remaining_supply()
                .reclassify::<AvailableTaskExecution>(),
        )
    }

    /// Calculate the actual execution Curve for the Task with priority `task_index` of the Server with priority `server_index`
    /// up to the specified limit.
    ///
    /// Based on Definition 14. (3) of the paper
    #[must_use]
    pub fn original_actual_execution_curve_iter(
        system: &System,
        server_index: usize,
        task_index: usize,
    ) -> OriginalActualTaskExecution {
        let asec = system.original_actual_execution_curve_iter(server_index);
        let hptd = Task::higher_priority_task_demand_iter(
            system.as_servers()[server_index].as_tasks(),
            task_index,
        );

        let available_execution_curve = Task::available_execution_curve_impl(asec, hptd);

        let task_demand_curve =
            system.as_servers()[server_index].as_tasks()[task_index].into_iter();

        OriginalActualTaskExecution(
            CurveDeltaIterator::new(available_execution_curve, task_demand_curve)
                .overlap::<ActualTaskExecution>(),
        )
    }

    /**
       A fixed version of the papers original actual execution, using aggregated higher priority server execution
       instead of aggregated higher priority server constrained demand for the computation
    */
    #[must_use]
    pub fn fixed_actual_execution_curve_iter(
        system: &System,
        server_index: usize,
        task_index: usize,
    ) -> FixedActualTaskExecution {
        let asec = system.fixed_actual_execution_curve_iter(server_index);
        let hptd = Task::higher_priority_task_demand_iter(
            system.as_servers()[server_index].as_tasks(),
            task_index,
        );

        let available_execution_curve = Task::available_execution_curve_impl(asec, hptd);

        let task_demand_curve =
            system.as_servers()[server_index].as_tasks()[task_index].into_iter();

        FixedActualTaskExecution(
            CurveDeltaIterator::new(available_execution_curve, task_demand_curve)
                .overlap::<ActualTaskExecution>(),
        )
    }

    /// Calculate the WCRT for the task with priority `task_index` for the Server with priority `server_index`
    ///
    /// See definition 15. of the paper for reference
    ///
    /// Takes the system of servers that the task which worst case execution time shall be calculated is part of
    /// the priority/index of the server the Task belongs to
    /// and the tasks priority/index in that server
    /// as well as the time till which jobs that arrive prior shall be considered for the analysis
    ///
    /// # Panics
    /// When sanity checks fail
    #[must_use]
    pub fn original_worst_case_response_time(
        system: &System,
        server_index: usize,
        task_index: usize,
        arrival_before: TimeUnit,
    ) -> TimeUnit {
        let swh = arrival_before;

        let actual_execution_time_iter =
            Task::original_actual_execution_curve_iter(system, server_index, task_index);

        let task = &system.as_servers()[server_index].as_tasks()[task_index];

        // arrival of the last job that starts before the swh
        let last_job = (swh - task.offset - TimeUnit::ONE) / task.interval;

        let total_execution = (last_job + 1) * task.demand;
        let mut provided = WindowEnd::Finite(TimeUnit::ZERO);

        let actual_execution_time: Curve<_> = actual_execution_time_iter
            .take_while_curve(|task| {
                let take = provided < total_execution;
                provided += task.length();
                take
            })
            .collect_curve();

        // sanity check that last_job arrival is before swh
        assert!(
            task.job_arrival(last_job) < swh,
            "Last job should arrive before the system wide hyper period"
        );

        // sanity check that job after last_job is not before swh
        assert!(
            swh <= task.job_arrival(last_job + 1),
            "The job after the last job would arrive after or at the system wide hyper period"
        );

        assert!(
            WindowEnd::Finite((last_job + 1) * task.demand) <= actual_execution_time.capacity(),
            "There should be enough capacity for the last job"
        );

        (0..=last_job)
            .into_iter()
            .map(|job| {
                let arrival = task.job_arrival(job);
                let t = (job + 1) * task.demand;

                Task::time_to_provide(&actual_execution_time, t) - arrival
            })
            .max()
            .unwrap_or(TimeUnit::ZERO)
    }

    /// Same as `original_worst_case_response_time` in most aspects, but uses the `fixed_actual_execution_curve_iter` instead of the `original_actual_execution_curve_iter`
    /// # Panics
    /// When sanity checks fail
    #[must_use]
    pub fn fixed_worst_case_response_time(
        system: &System,
        server_index: usize,
        task_index: usize,
        arrival_before: TimeUnit,
    ) -> TimeUnit {
        let swh = arrival_before;

        let actual_execution_time_iter =
            Task::fixed_actual_execution_curve_iter(system, server_index, task_index);

        let task = &system.as_servers()[server_index].as_tasks()[task_index];

        // arrival of the last job that starts before the swh
        let last_job = (swh - task.offset - TimeUnit::ONE) / task.interval;

        let total_execution = (last_job + 1) * task.demand;
        let mut provided = WindowEnd::Finite(TimeUnit::ZERO);

        let actual_execution_time: Curve<_> = actual_execution_time_iter
            .take_while_curve(|task| {
                let take = provided < total_execution;
                provided += task.length();
                take
            })
            .collect_curve();

        // sanity check that last_job arrival is before swh
        assert!(
            task.job_arrival(last_job) < swh,
            "Last job should arrive before the system wide hyper period"
        );

        // sanity check that job after last_job is not before swh
        assert!(
            swh <= task.job_arrival(last_job + 1),
            "The job after the last job would arrive after or at the system wide hyper period"
        );

        assert!(
            WindowEnd::Finite((last_job + 1) * task.demand) <= actual_execution_time.capacity(),
            "There should be enough capacity for the last job"
        );

        (0..=last_job)
            .into_iter()
            .map(|job| {
                let arrival = task.job_arrival(job);
                let t = (job + 1) * task.demand;

                Task::time_to_provide(&actual_execution_time, t) - arrival
            })
            .max()
            .unwrap_or(TimeUnit::ZERO)
    }

    /// Calculate the time till the execution curve has served t Units of Demand
    /// Implementing Algorithm 5. form the paper
    ///
    /// # Panics
    /// When the capacity of the curve is less than t
    /// or t is [`TimeUnit::ZERO`]
    #[must_use]
    pub fn time_to_provide(
        actual_execution_time: &Curve<ActualTaskExecution>,
        t: TimeUnit,
    ) -> TimeUnit {
        // Note: paper lists wants to find largest index k with the sum of the windows 0..=k < t
        // but when calculating k the sum skips 0
        // finding the largest index k with the sum of the windows 1..=k < t
        // this appears to be a mix-up between 0-based and 1-based indexing and
        // is therefore not replicated in this implementation

        // (1)
        // index here is exclusive aka. k+1 as appose to inclusive as in the paper
        let (index, sum) = actual_execution_time
            .as_windows()
            .iter()
            .enumerate()
            .scan(TimeUnit::ZERO, |acc, (index, window)| {
                match window.length() {
                    WindowEnd::Finite(length) => {
                        *acc += length;
                        (*acc < t).then(|| (index + 1, *acc))
                    }
                    WindowEnd::Infinite => None,
                }
            })
            .last()
            .unwrap_or((0, TimeUnit::ZERO));

        // (2)
        let b = t - sum;

        // this should hold as sum is the largest sum of head window lengths less than t
        debug_assert!(
            b > TimeUnit::ZERO,
            "There should be remaining demand, but b = {:?}",
            b
        );

        actual_execution_time.as_windows()[index].start + b
    }

    /// Calculate the arrival for the job_index+1-th job
    ///
    /// Note: The paper uses 1-index for jobs while this uses 0-index
    #[must_use]
    pub fn job_arrival(&self, job_index: UnitNumber) -> TimeUnit {
        self.offset + job_index * self.interval
    }
}

impl IntoIterator for Task {
    type Item = Window<Demand>;
    type IntoIter = TaskDemandIterator;

    /// Generate the Demand Curve for the Task
    ///
    /// Based on Definition 9. and 10. of the paper
    fn into_iter(self) -> Self::IntoIter {
        TaskDemandIterator::new(self)
    }
}
