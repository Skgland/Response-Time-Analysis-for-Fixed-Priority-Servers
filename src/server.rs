//! Module for Server definition
//!
//! and functions to be used with one or multiple Servers

use std::collections::{HashMap, VecDeque};

use crate::curve::{Curve, PartitionResult};
use crate::task::Task;
use crate::time::TimeUnit;
use crate::window::{Demand, Overlap, Supply, Window};

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
    pub server_type: ServerType,
}

/// The Type of a Server
#[derive(Debug)]
pub enum ServerType {
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
    pub fn aggregated_demand_curve(&self, up_to: TimeUnit) -> Curve<Demand> {
        self.tasks
            .iter()
            .map(|task| task.demand_curve(up_to))
            .fold(Curve::empty(), Curve::aggregate)
    }

    /// Calculate the Servers constrained demand curve up to the specified limit,
    /// based on the Algorithm 1. from the paper
    pub fn constrain_demand_curve(&self, up_to: TimeUnit) -> Curve<Demand> {
        let aggregated_curve = self.aggregated_demand_curve(up_to);

        // (1)
        let mut splits: HashMap<_, _> = aggregated_curve.split(self.interval);

        let mut key = if let Some(&key) = splits.keys().min() {
            key
        } else {
            return splits
                .into_iter()
                .map(|(_, curve)| curve)
                .fold(Curve::empty(), Curve::aggregate);
        };

        // (2)
        while Some(&key) < splits.keys().max() {
            if let Some(curve) = splits.remove(&key) {
                // index here is exclusive while the paper uses an inclusive index
                let PartitionResult { index, head, tail } = curve.partition(key, self);

                let mut windows = curve.into_windows();
                let keep = windows
                    .drain(..index)
                    .chain(std::iter::once(head))
                    .collect();

                let constrained = unsafe { Curve::from_windows_unchecked(keep) };

                // re-insert constrained split
                splits.insert(key, constrained);

                let delta_k = tail.length()
                    + windows
                        .drain(..)
                        .skip(1) // skip window split into tail and head
                        .map(|window| window.length())
                        .sum::<TimeUnit>();

                if delta_k > TimeUnit::ZERO {
                    let old = splits.remove(&(key + 1)).unwrap_or_else(Curve::empty);
                    let transfer_start = (key + 1) * self.interval;
                    let updated = old.aggregate(Curve::new(Window::new(
                        transfer_start,
                        transfer_start + delta_k,
                    )));
                    splits.insert(key + 1, updated);
                }
            }
            key += 1;
        }

        splits
            .into_iter()
            .map(|(_, curve)| curve)
            .fold(Curve::empty(), Curve::aggregate)
    }

    /// Calculate the aggregated higher priority demand curve
    /// by aggregating the aggregated demand curves of all Servers with higher priority
    /// (lower value) than `index`.
    ///
    /// The index in the `servers` slice corresponds to the priority of the Server
    /// a lower index equals higher priority
    ///
    /// Based on the papers Definition 12.
    pub fn aggregated_higher_priority_demand_curve(
        servers: &[Server],
        index: usize,
        up_to: TimeUnit,
    ) -> Curve<Demand> {
        servers[..index]
            .iter()
            .map(|server| server.aggregated_demand_curve(up_to))
            .fold(Curve::empty(), Curve::aggregate)
    }

    /// Calculate the unconstrained execution curve
    /// for the server with priority `index`.
    ///
    /// The Priority of a server is its index in the `servers` slice,
    /// a lower index entails a higher priority.
    ///
    /// TODO reference paper
    #[must_use]
    pub fn unconstrained_execution_curve(
        servers: &[Server],
        index: usize,
        up_to: TimeUnit,
    ) -> Curve<Supply> {
        let result = Curve::delta(
            Curve::total(up_to),
            Server::aggregated_higher_priority_demand_curve(servers, index, up_to),
        );
        result.remaining_supply
    }

    /// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
    /// TODO more detail, what do the parameters mean
    #[must_use]
    pub fn constrained_execution_curve(
        servers: &[Server],
        index: usize,
        up_to: TimeUnit,
    ) -> Curve<Overlap> {
        // Input

        let unconstrained_execution = Server::unconstrained_execution_curve(servers, index, up_to);
        let server = &servers[index];
        let constrained_demand = server.constrain_demand_curve(up_to);

        constrained_demand.debug_validate();

        // (1)
        let split_execution = {
            let mut split_execution: Vec<_> = unconstrained_execution
                .split(server.interval)
                .into_iter()
                .flat_map(|(_, curve)| {
                    curve.debug_validate();
                    curve.into_windows().into_iter()
                })
                .collect();

            split_execution.sort_by_key(|window| window.start);
            split_execution
        };

        debug_assert!(split_execution.as_slice().windows(2).all(|windows| {
            if let [p, n] = windows {
                p.start < n.start
            } else {
                false
            }
        }));

        // (2)
        let mut current_supply = split_execution;
        let mut current_demand: VecDeque<_> = constrained_demand.into_windows().into();
        let mut constrained_execution = Vec::new();

        // (3) initialization will be done on demand
        let mut budgets = HashMap::new();

        // (4)
        // Note: Condition appears inverted in the paper as it is written it would be initially false
        // skipping the loop immediately
        //
        // C^e'_S(t) != {}
        while !current_supply.is_empty() {
            // C^d'_S(t) != {}
            if let Some(demand_window) = current_demand.pop_front() {
                // (a)
                let index = if let Some(index) =
                    current_supply
                        .iter()
                        .enumerate()
                        .find_map(|(index, window)| {
                            if window.end > demand_window.start
                                && *budgets
                                    .entry(window.budget_group(server.interval))
                                    .or_insert(TimeUnit::ZERO)
                                    < server.capacity
                            {
                                Some(index)
                            } else {
                                None
                            }
                        }) {
                    index
                } else {
                    // we still have supply and demand
                    // but the demand can't be fulfilled by the remaining supply
                    // TODO reflect this possibility in the return type and handle this case gracefully
                    // maybe return remaining supply, demand and already calculated overlap
                    // instead of just the calculated overlap
                    panic!()
                };

                let bg = current_supply[index].budget_group(server.interval);

                // (b)
                let remaining_budget = server.capacity
                    - budgets
                        .get(&bg)
                        .expect("Either already existed or initialized while finding the index!");

                let valid_demand_segment = if demand_window.length() > remaining_budget {
                    let valid =
                        Window::new(demand_window.start, demand_window.start + remaining_budget);
                    let residual = Window::new(valid.end, demand_window.end);

                    // (c)
                    current_demand.push_front(residual);

                    valid
                } else {
                    demand_window
                };

                // (d) , (f) removal of W_e,j
                let result = Window::delta(&current_supply.remove(index), &valid_demand_segment);

                // (e)
                debug_assert!(budgets.contains_key(&bg));
                budgets
                    .entry(bg)
                    .and_modify(|entry| *entry += result.overlap.length());

                // (f)
                result
                    .remaining_supply
                    .into_windows()
                    .into_iter()
                    .rev()
                    .for_each(|window| current_supply.insert(index, window));

                // (g)
                constrained_execution.push(result.overlap.to_other());
            } else {
                break;
            }
        }

        Curve::from_windows(constrained_execution).into_overlap()
    }

    /// Calculate the system wide hyper periode
    /// accounting for all servers and tasks
    ///
    /// Section 7.1
    pub fn system_wide_hyper_periode(servers: &[Server]) -> TimeUnit {
        servers
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
