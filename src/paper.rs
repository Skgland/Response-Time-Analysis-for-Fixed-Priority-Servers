//! Module defining some operations more closely to the paper
//! used by the more constrained versions of the main implementation

use crate::curve::curve_types::CurveType;
use crate::curve::Curve;
use crate::iterators::curve::CollectCurveExt;
use crate::iterators::server::ActualExecutionIterator;
use crate::iterators::CurveIterator;
use crate::server::{
    ActualServerExecution, AvailableServerExecution, ConstrainedServerDemand, Server,
};
use crate::time::TimeUnit;
use crate::window::window_types::WindowType;
use crate::window::Window;

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
                    debug_assert!(
                        prev.start < next.start,
                        "Curve windows should be sorted but {:#?} and {:#?} are out of order!",
                        prev,
                        next
                    );
                    // non-overlapping
                    debug_assert!(
                        !prev.overlaps(next),
                        "Curve windows should not overlap but {:#?} and {:#?} do!",
                        prev,
                        next
                    );
                }
                _ => unreachable!(
                    "Iteration over slice windows of size 2, can't have other slice lengths10"
                ),
            }
        }
    }

    curve_a
}

/// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
///
/// For the server with priority `server_index` calculate th actual execution
/// given the unconstrained execution and the constrained demand
///
/// # Panics
///
/// When `debug_assertions` are enabled and invariants are violated
pub fn actual_server_execution_iter<'a>(
    servers: &'a [Server],
    server_index: usize,
    available_execution: impl CurveIterator<'a, AvailableServerExecution> + Clone,
    constrained_demand: impl CurveIterator<'a, ConstrainedServerDemand> + Clone,
) -> impl CurveIterator<'a, ActualServerExecution> + Clone {
    ActualExecutionIterator::new(
        servers,
        server_index,
        available_execution,
        constrained_demand,
    )
}

/// Check if the assumption holds that every server has it's full capacity available
#[must_use]
pub fn check_assumption(
    server: &Server,
    available: Curve<AvailableServerExecution>,
    up_to: TimeUnit,
) -> bool {
    let groups = available.split(server.interval);

    for interval_index in 0..=((up_to - TimeUnit::ONE) / server.interval) {
        if !groups
            .get(&interval_index)
            .map_or(false, |curve| curve.capacity() >= server.capacity)
        {
            return false;
        }
    }

    true
}
