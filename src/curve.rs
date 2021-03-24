//! Module that defined Curve
//!
//! and all associated functions

use std::collections::{HashMap, VecDeque};

use crate::seal::WindowType;
use crate::server::{Server, ServerType};
use crate::window::Window;

/// A Curve is an ordered Set of non-overlapping Windows
///
/// The windows are ordered by their start
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Curve<T: WindowType> {
    /// windows contains an ordered Set of non-overlapping windows
    windows: Vec<Window<T>>,
}

/// Return Type for [`Curve::delta`](Curve::delta)
pub struct CurveDeltaResult {
    /// The remaining supply, can be 0-2 Windows
    pub remaining_supply: Curve<Supply>,
    /// The (used) Overlap between Supply and Demand
    pub overlap: Curve<Overlap>,
    /// The remaining Demand that could not be fulfilled by the Supply
    pub remaining_demand: Curve<Demand>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Marker Type for Window, indicating a Supply Window
pub struct Supply;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Marker Type for Window, indicating Demand
pub struct Demand;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Marker Type for Window,indicating an Overlap between Supply and Demand
pub struct Overlap;

impl<T: WindowType> Curve<T> {
    /// Create a new Curve from the provided window
    ///
    /// May return a Curve with no Windows when the provided Window is empty
    #[must_use]
    pub fn new(window: Window<T>) -> Self {
        if window.is_empty() {
            // Empty windows can be ignored
            Self::empty()
        } else {
            // A Curve with only a single has
            // the windows always ordered and non-overlapping
            Self {
                windows: vec![window],
            }
        }
    }

    /// Returns a slice reference to the contained windows
    #[must_use]
    pub fn as_windows(&self) -> &[Window<T>] {
        self.windows.as_slice()
    }

    /// Consumes self and returns the contained Windows
    #[must_use]
    pub fn into_windows(self) -> Vec<Window<T>> {
        self.windows
    }

    /// Create a new empty Curve
    #[must_use]
    pub fn empty() -> Self {
        Self { windows: vec![] }
    }

    /// Create a new Total Curve for the given limit
    #[must_use]
    pub fn total(up_to: usize) -> Self {
        Self::new(Window::up_to(up_to))
    }

    /// Create a new Curve from the given Vector of Windows
    /// without checking or guaranteeing that the Curve invariants are met
    /// by the list of windows.
    ///
    /// # Safety:
    /// Windows need to be non-overlapping and
    /// ordered based on start, to fulfill invariants of curve
    pub(crate) unsafe fn from_windows_unchecked(windows: Vec<Window<T>>) -> Self {
        Self { windows }
    }

    /// Return the Curves Capacity as defined by Definition 3. in the paper
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.windows.iter().map(Window::length).sum()
    }

    /// Return true if the Capacity of the Curve is 0
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.windows.iter().map(Window::length).all(|c| c == 0)
    }

    /// Split the curve on every interval boundary as defined in Definition 8. of the paper
    #[must_use]
    pub fn split(self, interval: usize) -> HashMap<usize, Self> {
        let mut curves: HashMap<_, Self> = HashMap::new();

        for mut window in self.windows {
            loop {
                let k = window.start / interval; // integer division rounds down by default

                if window.end <= (k + 1) * interval {
                    curves
                        .entry(k)
                        .or_insert_with(Self::empty)
                        .windows
                        .push(window);
                    // process next window from input Curve
                    break;
                } else {
                    let init = Window::new(window.start, (k + 1) * interval);
                    curves
                        .entry(k)
                        .or_insert_with(Self::empty)
                        .windows
                        .push(init);
                    window = Window::new((k + 1) * interval, window.end);
                    // reprocess updated remaining window
                    continue;
                }
            }
        }

        curves
    }

    /// Validate using `debug_assert!` that the Types invariants are met
    pub fn debug_validate(&self) {
        debug_assert!(self
            .windows
            .as_slice()
            .windows(2)
            .all(|windows| if let [p, n] = windows {
                p.start < n.start && !p.overlaps(n)
            } else {
                false
            }))
    }
}

impl Curve<Demand> {
    /// Create an aggregated Curve of all provided Windows
    pub fn from_windows<I: IntoIterator<Item = Window<Demand>>>(windows: I) -> Self {
        windows
            .into_iter()
            .map(Self::new)
            .fold(Self::empty(), Self::aggregate)
    }

    /// Aggregate two (Demand) Curves as defined in Definition 5. of the paper
    ///
    /// Only defined for Demand Curves as it doesn't rely make sense for Overlap or Supply curves
    /// As overlapping Supply may not be available later and Overlap may not Overlap later
    #[must_use]
    pub fn aggregate(self, other: Self) -> Self {
        let mut new = self.windows;

        for mut window in other.windows {
            let mut index = 0;
            while index < new.len() {
                if let Some(aggregate) = new[index].aggregate(&window) {
                    // remove window that was aggregated
                    new.remove(index);
                    // replace window to be inserted by aggregated window
                    window = aggregate;
                } else {
                    index += 1;
                }
            }

            // find index where to insert new window
            let index = new
                .iter()
                .enumerate()
                .find(|(_, nw)| nw.start > window.end)
                .map_or_else(|| new.len(), |(index, _)| index);

            new.insert(index, window);
        }

        for new in new[..].windows(2) {
            // assert new.is_sorted_by_key(|window|window.start) once is_sorted_by_key is stable
            match new {
                [prev, next] => {
                    // ordered and non-overlapping
                    debug_assert!(prev.end < next.start);
                    debug_assert!(!prev.overlaps(next));
                }
                _ => unreachable!(
                    "Iteration over slice windows of size 2, can't have other slice lengths10"
                ),
            }
        }

        // TODO Safety Comment
        unsafe { Self::from_windows_unchecked(new) }
    }

    /// Partition the Curve as Defined by Algorithms 2. and 3. of the paper
    ///
    /// The implementation here deviates from the paper by returning an exclusive index while the paper uses an inclusive index
    #[must_use]
    pub fn partition(&self, offset: usize, server: &Server) -> PartitionResult {
        match server.server_type {
            ServerType::Deferrable => {
                // Algorithm 2.
                // (1)
                let index = self
                    .windows
                    .iter()
                    .scan(0, |acc, window| {
                        *acc += window.length();
                        Some(*acc)
                    })
                    .enumerate()
                    .filter(|(_, used)| *used < server.capacity)
                    .last()
                    .map_or(0, |(index, _)| index + 1);

                // (2)
                let remaining_capacity = server.capacity
                    - self.windows[..index]
                        .iter()
                        .map(Window::length)
                        .sum::<usize>();
                let (head, tail) = if remaining_capacity != 0 && index < self.windows.len() {
                    let window = &self.windows[index];
                    let head_start = window.start;
                    let tail_end = window.end;
                    let split = head_start + remaining_capacity;
                    let head = Window::new(head_start, split);
                    let tail = Window::new(split, tail_end);
                    (head, tail)
                } else {
                    // Window won't be split as we don't have remaining capacity
                    // if there is a window set it as the tail, otherwise the tail is also empty
                    (
                        Window::empty(),
                        self.windows
                            .get(index)
                            .cloned()
                            .unwrap_or_else(Window::empty),
                    )
                };

                PartitionResult { index, head, tail }
            }
            ServerType::Periodic => {
                // Algorithm 3.
                // (1)
                let limit = offset * server.interval + server.capacity;
                let index = self
                    .windows
                    .iter()
                    .enumerate()
                    .filter_map(|(index, window)| (window.end < limit).then(|| index + 1))
                    .last()
                    .unwrap_or(0);

                // (2)
                let (head, tail) = if index <= self.windows.len()
                    && self.windows[index].start < limit
                    && limit < self.windows[index].end
                {
                    let window = &self.windows[index];
                    let head = Window::new(window.start, limit);
                    let tail = Window::new(limit, window.end);
                    (head, tail)
                } else {
                    // Window won't be split as it does not contain the limit
                    // if there is a window set it as the tail, otherwise the tail is also empty
                    (
                        Window::empty(),
                        self.windows
                            .get(index)
                            .cloned()
                            .unwrap_or_else(Window::empty),
                    )
                };

                PartitionResult { index, head, tail }
            }
        }
    }

    /// Convert a Demand Curve into an Overlap Curve
    #[must_use]
    pub fn into_overlap(self) -> Curve<Overlap> {
        Curve {
            windows: self
                .windows
                .into_iter()
                .map(|window| window.to_other())
                .collect(),
        }
    }
}

/// Return Type for [`Curve::partition`](Curve::partition)
pub struct PartitionResult {
    // exclusive index, reference paper uses inclusive index
    /// The exclusive index up to which all demand fits into the current partition
    pub index: usize,

    pub head: Window<Demand>,
    pub tail: Window<Demand>,
}

impl Curve<Supply> {
    /// Calculate Delta between the Supply and the Demand based on Definition 7. from the paper
    #[must_use]
    pub fn delta(supply: Self, demand: Curve<Demand>) -> CurveDeltaResult {
        let mut demand: VecDeque<_> = demand.windows.into_iter().collect();
        let mut supply: VecDeque<_> = supply.windows.into_iter().collect();

        let mut overlap: Curve<Demand> = Curve::empty();

        // get first demand window
        // if None we are done <=> Condition C^'_q(t) = {}
        'main: while let Some(demand_window) = demand.front() {
            // (1) find valid supply window
            let j = if let Some(i) = supply
                .iter()
                .enumerate()
                .find_map(|(i, supply)| (demand_window.start < supply.end).then(|| i))
            {
                i
            } else {
                // exhausted usable supply
                // either supply.len() == 0 <=> Condition C^'_p(t) = {}
                // or all remaining supply is before remaining demand,
                // this should not happen for schedulable scenarios
                break 'main;
            };

            let supply_window = supply
                .get(j)
                .expect("Index should have only be searched in bounds!");

            // (2) calculate delta
            let result = Window::delta(supply_window, demand_window);

            // (3) removed used supply and add remaining supply
            supply
                .remove(j)
                .expect("Index should have only be searched in bounds, and has yet to be removed!");

            result
                .remaining_supply
                .windows
                .into_iter()
                .rev()
                .filter(|w| !w.is_empty())
                .for_each(|window| supply.insert(j, window));

            // (4) add overlap windows
            overlap = overlap.aggregate(Curve::new(Window::new(
                result.overlap.start,
                result.overlap.end,
            )));

            // (5) remove completed demand and add remaining demand
            demand.pop_front();
            if !result.remaining_demand.is_empty() {
                demand.push_front(result.remaining_demand)
            }
        }

        CurveDeltaResult {
            remaining_supply: Self {
                windows: supply.into(),
            },
            overlap: overlap.into_overlap(),
            remaining_demand: Curve {
                windows: demand.into(),
            },
        }
    }
}

#[cfg(test)]
mod tests;
