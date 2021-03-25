//! Module for the Task definition

use crate::curve::Curve;
use crate::server::Server;
use crate::time::TimeUnit;
use crate::window::{Demand, Overlap, Window};

/// The Task type based on the Modeling described in the second paragraph
/// of Chapter 3. in the paper
#[derive(Debug)]
pub struct Task {
    /// The offset of the tasks, O index i in the paper
    pub offset: TimeUnit,
    /// The demand induced by the task
    /// called the worst-case execution time (WCET) C index i in the paper
    pub demand: TimeUnit,
    /// The interval of the task, called Periode P index i in the paper
    pub interval: TimeUnit,
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

    /// Generate the Demand Curve for the Task up to he specified limit
    ///
    /// Only complete Cycles will be considered
    ///
    /// Based on Definition 10. of the paper
    #[must_use]
    pub fn demand_curve(&self, up_to: TimeUnit) -> Curve<Demand> {
        let mut start = self.offset;
        let mut curve = Curve::empty();

        while start <= (up_to - self.demand) {
            curve = curve.aggregate(Curve::new(Window::new(start, start + self.demand)));
            start += self.interval;
        }

        curve
    }

    /// calculate the Higher Priority task Demand for the task with priority `index` as defined in Definition 14. (1) in the paper,
    /// for a set of tasks indexed by their priority (lower index <=> higher priority) and up to the specified limit
    pub fn higher_priority_task_demand(
        tasks: &[Self],
        index: usize,
        up_to: TimeUnit,
    ) -> Curve<Demand> {
        tasks[..index]
            .iter()
            .map(|task| task.demand_curve(up_to))
            .fold(Curve::empty(), Curve::aggregate)
    }

    /// Calculate the available execution Curve for the task with priority `task_index` of the server with priority `server_index`
    /// up to the specified limit.
    ///
    /// Based on Definition 14. (2) of the paper
    #[must_use]
    pub fn available_execution_curve(
        servers: &[Server],
        server_index: usize,
        task_index: usize,
        up_to: TimeUnit,
    ) -> Curve<Overlap> {
        let constrained_server_execution_curve =
            Server::constrained_execution_curve(servers, server_index, up_to);
        let higher_priority_task_demand =
            Task::higher_priority_task_demand(servers[server_index].as_tasks(), task_index, up_to);

        let result = Curve::delta(
            constrained_server_execution_curve,
            higher_priority_task_demand,
        );

        result.remaining_supply
    }

    /// Calculate the actual execution Curve for the Task with priority `task_index` of the Server with priority `server_index`
    /// up to the specified limit.
    ///
    /// Based on Definition 14. (3) of the paper
    #[must_use]
    pub fn actual_execution_curve(
        servers: &[Server],
        server_index: usize,
        task_index: usize,
        up_to: TimeUnit,
    ) -> Curve<Overlap> {
        let available_execution_curve =
            Task::available_execution_curve(servers, server_index, task_index, up_to);
        let task_demand_curve = servers[server_index].as_tasks()[task_index].demand_curve(up_to);

        let result = Curve::delta(available_execution_curve, task_demand_curve);
        result.overlap
    }

    /// Calculate the WCRT for the task with priority `task_index` for the Server with priority `server_index`
    #[must_use]
    pub fn worst_case_response_time(
        servers: &[Server],
        server_index: usize,
        task_index: usize,
    ) -> TimeUnit {
        let swh = Server::system_wide_hyper_periode(servers);

        let actual_execution_time =
            Task::actual_execution_curve(servers, server_index, task_index, swh);

        let mut worst_case = TimeUnit::ZERO;
        let mut j = 1;

        let task = &servers[server_index].as_tasks()[task_index];

        loop {
            let arrival = task.job_arrival(j - 1);

            if arrival >= swh {
                break;
            }

            let t = j * task.demand;

            let r = Task::time_to_provide(&actual_execution_time, t) - arrival;

            worst_case = TimeUnit::max(worst_case, r);

            j += 1;
        }

        worst_case
    }

    /// Calculate the time till the execution curve has served t Units of Demand
    /// Implementing Algorithm 5. form the paper
    #[must_use]
    pub(crate) fn time_to_provide(actual_execution_time: &Curve<Overlap>, t: TimeUnit) -> TimeUnit {
        // Note: paper lists wants to find largest index k with the sum of the windows 0..=k < t
        // but when calculating k the sum skips 0
        // finding the largest index k with the sum of the windows 1..=k < t

        // (1)
        // index here is exclusive aka. k+1
        let (index, sum) = actual_execution_time
            .as_windows()
            .iter()
            .enumerate()
            .scan(TimeUnit::ZERO, |acc, (index, window)| {
                *acc += window.length();
                (*acc < t).then(|| (index + 1, *acc))
            })
            .last()
            .unwrap_or((0, TimeUnit::ZERO));

        // (2)
        let b = t - sum;

        // this should hold as sum is the largest sum of head window lengths less than t
        debug_assert!(b > TimeUnit::ZERO);

        // TODO what to do when index+1 is out of bounds?
        let ttp = actual_execution_time.as_windows()[index].start + b;

        ttp
    }

    /// Calculate the arrival for the job_index+1-th job
    ///
    /// Note: The paper uses 1-index for jobs while this uses 0-index
    #[must_use]
    pub fn job_arrival(&self, job_index: usize) -> TimeUnit {
        self.offset + job_index * self.interval
    }
}

#[cfg(test)]
mod tests;
