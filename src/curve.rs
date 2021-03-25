//! Module that defined Curve
//!
//! and all associated functions

use std::collections::{HashMap, VecDeque};

use crate::seal::{CurveType, WindowType};
use crate::server::{Server, ServerType};
use crate::time::TimeUnit;
use crate::window::{Demand, Overlap, Supply, Window};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct PrimitiveCurve<W: WindowType>(W);

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct OverlapCurve<P: CurveType, Q: CurveType>(P, Q);

/// A Curve is an ordered Set of non-overlapping Windows
///
/// The windows are ordered by their start
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Curve<C: CurveType> {
    /// windows contains an ordered Set of non-overlapping non-empty windows
    windows: Vec<Window<C::WindowKind>>,
}

/// Return Type for [`Curve::delta`](Curve::delta)
#[derive(Debug)]
pub struct CurveDeltaResult<
    P: CurveType,
    Q: CurveType,
    R: CurveType<WindowKind = Overlap<P::WindowKind, Q::WindowKind>>,
> {
    /// The remaining supply, can be 0-2 Windows
    pub remaining_supply: Curve<P>,
    /// The (used) Overlap between Supply and Demand
    pub overlap: Curve<R>,
    /// The remaining Demand that could not be fulfilled by the Supply
    pub remaining_demand: Curve<Q>,
}

impl<T: CurveType> Curve<T> {
    /// Create a new Curve from the provided window
    ///
    /// May return a Curve with no Windows when the provided Window is empty
    #[must_use]
    pub fn new(window: Window<T::WindowKind>) -> Self {
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
    pub fn as_windows(&self) -> &[Window<T::WindowKind>] {
        self.windows.as_slice()
    }

    /// Consumes self and returns the contained Windows
    #[must_use]
    pub fn into_windows(self) -> Vec<Window<T::WindowKind>> {
        self.windows
    }

    /// Create a new empty Curve
    #[must_use]
    pub fn empty() -> Self {
        Self { windows: vec![] }
    }

    /// Create a new Total Curve for the given limit
    #[must_use]
    pub fn total(up_to: TimeUnit) -> Self {
        Self::new(Window::new(TimeUnit::ZERO, up_to))
    }

    /// Create a new Curve from the given Vector of Windows
    /// without checking or guaranteeing that the Curve invariants are met
    /// by the list of windows.
    ///
    /// # Safety:
    /// Windows need to be non-overlapping and
    /// ordered based on start, to fulfill invariants of curve
    pub(crate) unsafe fn from_windows_unchecked(windows: Vec<Window<T::WindowKind>>) -> Self {
        Self { windows }
    }

    /// Return the Curves Capacity as defined by Definition 3. in the paper
    #[must_use]
    pub fn capacity(&self) -> TimeUnit {
        self.windows.iter().map(Window::length).sum()
    }

    /// Return true if the Capacity of the Curve is 0
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.windows
            .iter()
            .map(Window::length)
            .all(|c| c == TimeUnit::ZERO)
    }

    /// Calculate Delta between the Supply and the Demand based on Definition 7. from the paper
    #[must_use]
    pub fn delta<Q: CurveType, R: CurveType<WindowKind = Overlap<T::WindowKind, Q::WindowKind>>>(
        supply: Self,
        demand: Curve<Q>,
    ) -> CurveDeltaResult<T, Q, R> {
        let mut demand: VecDeque<_> = demand.windows.into_iter().collect();
        let mut supply: VecDeque<_> = supply.windows.into_iter().collect();

        let mut overlap: Curve<_> = Curve::empty();

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
            overlap.insert(Window::new(result.overlap.start, result.overlap.end));

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
            overlap,
            remaining_demand: Curve {
                windows: demand.into(),
            },
        }
    }

    /// Split the curve on every interval boundary as defined in Definition 8. of the paper
    #[must_use]
    pub fn split(self, interval: TimeUnit) -> HashMap<usize, Self> {
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

    /// Change the `CurveType` of the Curve
    /// requires that the `WindowType` of both [`CurveTypes`](CurveType) is the same
    #[must_use]
    pub fn reclassify<C: CurveType<WindowKind = T::WindowKind>>(self) -> Curve<C> {
        Curve {
            windows: self.windows,
        }
    }

    /// Insert window into the Curve
    ///
    /// # Panics
    /// If window overlaps by more than just the bounds
    pub fn insert(&mut self, window: Window<T::WindowKind>) {
        if window.is_empty() {
            // Curves don't contain empty windows
        } else if self.windows.is_empty()
            || self
                .windows
                .last()
                .filter(|last| last.end < window.start)
                .is_some()
        {
            self.windows.push(window);
        } else if let Some(prev) = self.windows.last().filter(|last| last.end == window.start) {
            let start = prev.start;
            self.windows.pop();
            self.windows.push(Window::new(start, window.end));
        } else if self
            .windows
            .first()
            .filter(|first| window.end < first.start)
            .is_some()
        {
            self.windows.insert(0, window);
        } else if let Some(next) = self
            .windows
            .first()
            .filter(|first| window.end < first.start)
        {
            let end = next.end;
            self.windows.remove(0);
            self.windows.push(Window::new(window.start, end));
        } else {
            let index = if let Some(index) = self
                .windows
                .as_slice()
                .windows(2)
                .enumerate()
                .find_map(|(index, windows)| match windows {
                    [prev, next] => {
                        if prev.end <= window.start && window.end <= next.start {
                            Some(index)
                        } else {
                            None
                        }
                    }
                    _ => unreachable!("Windows size 2 hanled above!"),
                }) {
                index
            } else {
                panic!("Can't insert Window {:?} into Curve {:?} !", window, self);
            };

            let prev = self.windows.remove(index);
            let next = self.windows.remove(index);

            // insert window between prev and next

            let (start, reinsert_prev) = if prev.end < window.start {
                (window.start, true)
            } else {
                (prev.start, false)
            };

            let (end, reinsert_next) = if window.end < next.start {
                (window.end, true)
            } else {
                (next.end, false)
            };

            if reinsert_next {
                self.windows.insert(index, next);
            }

            self.windows.insert(index, Window::new(start, end));

            if reinsert_prev {
                self.windows.insert(index, prev);
            }
        }
    }
}

impl<T: CurveType<WindowKind = Supply>> Curve<T> {
    /// Create a Curve of all provided Windows
    pub fn supply_from_windows<I: IntoIterator<Item = Window<T::WindowKind>>>(windows: I) -> Self {
        windows.into_iter().fold(Self::empty(), |mut acc, window| {
            acc.insert(window);
            acc
        })
    }
}

impl<P: WindowType, Q: WindowType, T: CurveType<WindowKind = Overlap<P, Q>>> Curve<T> {
    /// Create a Curve of all provided Windows
    pub fn overlap_from_windows<I: IntoIterator<Item = Window<T::WindowKind>>>(windows: I) -> Self {
        windows.into_iter().fold(Self::empty(), |mut acc, window| {
            acc.insert(window);
            acc
        })
    }
}

impl<T: CurveType<WindowKind = Demand>> Curve<T> {
    /// Create an aggregated Curve of all provided Windows
    pub fn demand_from_windows<I: IntoIterator<Item = Window<T::WindowKind>>>(windows: I) -> Self {
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
    pub fn aggregate<R: CurveType<WindowKind = T::WindowKind>>(mut self, other: Curve<R>) -> Self {
        for mut window in other.windows {
            let mut index = 0;

            // iteratively aggregate window with overlapping windows in new
            // until no window overlaps
            while index < self.windows.len() {
                if let Some(aggregate) = self.windows[index].aggregate(&window) {
                    // remove window that was aggregated
                    self.windows.remove(index);
                    // replace window to be inserted by aggregated window
                    window = aggregate;
                    // continue at current index as it will not be inserted earlier
                    continue;
                } else if self.windows[index].start > window.end {
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
                let verify = self
                    .windows
                    .iter()
                    .enumerate()
                    .find_map(|(index, nw)| (nw.start > window.end).then(|| index))
                    .unwrap_or_else(|| self.windows.len());
                debug_assert_eq!(index, verify);
            }

            // this insert needs to preserve the Curve invariants
            self.windows.insert(index, window);
        }

        #[cfg(debug_assertions)]
        {
            for new in self.as_windows().windows(2) {
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

        self
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
                    .scan(TimeUnit::ZERO, |acc, window| {
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
                        .sum::<TimeUnit>();
                let (head, tail) =
                    if remaining_capacity != TimeUnit::ZERO && index < self.windows.len() {
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
}

/// Return Type for [`Curve::partition`](Curve::partition)
#[derive(Debug)]
pub struct PartitionResult {
    /// The exclusive index up to which all demand fits into the current partition
    ///
    /// Note: the paper uses an inclusive index
    pub index: usize,

    /// If there is a window on the partitioning boundary
    /// this contains the split before the boundary, otherwise this contains an empty window
    pub head: Window<Demand>,

    /// If there is a window on the partitioning boundary
    /// this contains the split after the boundary,
    /// otherwise if there is no window on the boundary this contains the first window
    /// after the boundary or an empty window if there is no window after the boundary
    pub tail: Window<Demand>,
}

#[cfg(test)]
mod tests;
