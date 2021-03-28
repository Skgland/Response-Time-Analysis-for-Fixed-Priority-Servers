//! Module defining some operations more closely to the paper
//! used by the more constrained versions of the main implementation

use crate::curve::{AggregateExt, Curve, PartitionResult, PrimitiveCurve};
use crate::seal::{CurveType, WindowType};
use crate::server::{
    AggregatedServerDemand, AvailableServerExecution, ConstrainedServerDemand,
    ConstrainedServerExecution, Server,
};
use crate::time::TimeUnit;
use crate::window::{Overlap, Window};
use std::collections::{HashMap, VecDeque};

/// Calculate the aggregation (⊕) of two windows as defined in Definition 4. of the paper
#[must_use]
pub fn aggregate_window<W: WindowType>(
    window_a: &Window<W>,
    window_b: &Window<W>,
) -> Option<Window<W>> {
    // only defined for overlapping windows, return None when not overlapping
    window_a.overlaps(window_b).then(|| {
        let start = TimeUnit::min(window_a.start, window_b.start);
        let end = start + window_a.length() + window_b.length();
        Window::new(start, end)
    })
}

/// Aggregate two (Demand) Curves as defined in Definition 5. of the paper
///
/// # Panics
///
/// May panic with `debug_assertions` enabled, when invariants are violated
///
#[must_use]
pub fn aggregate_curve<
    T: WindowType,
    C1: CurveType<WindowKind = T>,
    C2: CurveType<WindowKind = T>,
>(
    mut curve_a: Curve<C1>,
    curve_b: Curve<C2>,
) -> Curve<C1> {
    for mut window in curve_b.into_windows() {
        let mut index = 0;

        // iteratively aggregate window with overlapping windows in new
        // until no window overlaps
        while index < curve_a.as_windows().len() {
            if let Some(aggregate) = aggregate_window(&curve_a.as_windows()[index], &window) {
                // remove window that was aggregated
                curve_a.as_mut_windows().remove(index);
                // replace window to be inserted by aggregated window
                window = aggregate;
                // continue at current index as it will not be inserted earlier
                continue;
            } else if curve_a.as_windows()[index].start > window.end {
                // window can be inserted at index, no need to look for further overlaps as
                // earlier overlaps are already handled, later overlaps can't happen
                break;
            } else {
                // window did not overlap with new[index],
                // but can't be inserted at index, try next index
                index += 1;
                continue;
            }
        }

        // index now contains either new.len() or the first index where window.end < new[index].start
        // this is where window will be inserted
        // all overlaps have been resolved

        #[cfg(debug_assertions)]
        {
            // find index where to insert new window
            let verify = curve_a
                .as_windows()
                .iter()
                .enumerate()
                .find_map(|(index, nw)| (nw.start > window.end).then(|| index))
                .unwrap_or_else(|| curve_a.as_windows().len());
            debug_assert_eq!(index, verify);
        }

        // this insert needs to preserve the Curve invariants
        curve_a.as_mut_windows().insert(index, window);
    }

    #[cfg(debug_assertions)]
    {
        for new in curve_a.as_windows().windows(2) {
            match new {
                [prev, next] => {
                    // ordered
                    // assert is_sorted_by_key on .start once that is stable
                    debug_assert!(prev.start < next.start);
                    // non-overlapping
                    debug_assert!(!prev.overlaps(next));
                }
                _ => unreachable!(
                    "Iteration over slice windows of size 2, can't have other slice lengths10"
                ),
            }
        }
    }

    curve_a
}

/// Calculate the Servers constrained demand curve,
/// using the aggregated server demand curve
/// based on the Algorithm 1. from the paper and described in Section 5.1 of the paper
#[must_use]
pub fn constrained_server_demand(
    server: &Server,
    aggregated_curve: Curve<AggregatedServerDemand>,
) -> Curve<ConstrainedServerDemand> {
    // (1)
    let mut splits: HashMap<_, _> = aggregated_curve.split(server.interval);

    let mut key = if let Some(&key) = splits.keys().min() {
        key
    } else {
        // curve must be empty
        return Curve::empty();
    };

    // (2)
    while Some(&key) <= splits.keys().max() {
        if let Some(curve) = splits.remove(&key) {
            // index here is exclusive while the paper uses an inclusive index
            let PartitionResult { index, head, tail } = curve.partition(key, server);

            let mut windows = curve.into_windows();

            let keep = windows
                .drain(..index)
                .chain(std::iter::once(head).filter(|window| !window.is_empty()))
                .collect();

            let constrained = unsafe { Curve::from_windows_unchecked(keep) };

            // re-insert constrained split
            splits.insert(key, constrained);

            let delta_k = tail.length()
                + windows
                    .into_iter()
                    .skip(1) // skip window split into tail and head
                    .map(|window| window.length())
                    .sum::<TimeUnit>();

            if delta_k > TimeUnit::ZERO {
                let old = splits.remove(&(key + 1)).unwrap_or_else(Curve::empty);
                let transfer_start = (key + 1) * server.interval;
                let updated = old.aggregate::<PrimitiveCurve<_>>(Curve::new(Window::new(
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
        .aggregate::<Curve<_>>()
        .reclassify()
}

/// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
///
/// For the server with priority `server_index` calculate th actual execution
/// given the unconstrained execution and the constrained demand
///
/// # Panics
///
/// When `debug_assertions` are enabled and invariants are violated
#[must_use]
pub fn actual_server_execution(
    servers: &[Server],
    server_index: usize,
    unconstrained_execution: Curve<AvailableServerExecution>,
    constrained_demand: Curve<ConstrainedServerDemand>,
) -> Curve<ConstrainedServerExecution> {
    // Input

    let server = &servers[server_index];

    #[cfg(debug_assertions)]
    {
        constrained_demand.debug_validate();
    }

    // (1)
    let split_execution = {
        let mut split_execution: Vec<_> = unconstrained_execution
            .split(server.interval)
            .into_iter()
            .flat_map(|(_, curve)| {
                #[cfg(debug_assertions)]
                {
                    curve.debug_validate();
                }

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
    let mut constrained_execution: Vec<Window<Overlap<_, _>>> = Vec::new();

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
            constrained_execution.push(result.overlap);
        } else {
            break;
        }
    }

    Curve::overlap_from_windows(constrained_execution)
}
