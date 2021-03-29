//! Module that defined Curve
//!
//! and all associated functions

use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;

use curve_types::CurveType;

use crate::iterators::curve::CurveSplitIterator;
use crate::server::{
    AggregatedServerDemand, ConstrainedServerDemand, HigherPriorityServerDemand, Server, ServerKind,
};
use crate::task::{HigherPriorityTaskDemand, TaskDemand};
use crate::time::TimeUnit;
use crate::window::window_types::WindowType;
use crate::window::{Demand, Overlap, Supply, Window};

pub mod curve_types;

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

    /// Return a mutabel reference to the contained window container
    pub(crate) fn as_mut_windows(&mut self) -> &mut Vec<Window<T::WindowKind>> {
        &mut self.windows
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
    /// # Safety
    /// Windows need to be non-overlapping and
    /// ordered based on start, to fulfill invariants of curve
    #[must_use]
    pub unsafe fn from_windows_unchecked(windows: Vec<Window<T::WindowKind>>) -> Self {
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
    ///
    /// # Panics
    /// When the supply usable for the demand is less than the demand
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
        while let Some(demand_window) = demand.front() {
            // (1) find valid supply window
            let maybe_j = supply
                .iter()
                .enumerate()
                .find_map(|(i, supply)| (demand_window.start < supply.end).then(|| i));

            let j = if let Some(j) = maybe_j {
                j
            } else {
                // exhausted usable supply
                // either we are done as supply.len() == 0 <=> Condition C^'_p(t) = {}
                if !supply.is_empty() {
                    // or all remaining supply is before remaining demand
                    // The paper does not handle this case, as it should probably not occur
                    panic!("Not enough useful supply for delta calculation!");
                }
                break;
            };

            // (3) removed used supply that will be used
            let supply_window = supply
                .remove(j)
                .expect("sanity check: j should still be a valid index");

            // (2) calculate delta
            let result = Window::delta(&supply_window, demand_window);

            // (3) add remaining supply
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

    /// Validate using `debug_assert!` that the Types invariants are met
    ///
    /// # Panics
    ///
    /// Panics when the Curve contains overlapping or out of order windows
    pub fn debug_validate(&self) {
        debug_assert!(
            self.windows
                .as_slice()
                .windows(2) // use array_windows once stable
                .all(|windows| if let [p, n] = windows {
                    p.start < n.start && !p.overlaps(n)
                } else {
                    unreachable!("Branch can be eliminated once array_windows is stable")
                }),
            "Broken Curve Invariant! {:#?}",
            self
        )
    }

    /// Change the `CurveType` of the Curve,
    /// requires that the `WindowType` of both [`CurveTypes`](trait@CurveType) is the same
    #[must_use]
    pub fn reclassify<C: CurveType<WindowKind = T::WindowKind>>(self) -> Curve<C> {
        Curve {
            windows: self.windows,
        }
    }

    /// Insert window into the Curve
    ///
    /// # Panics
    /// When window overlaps a window in the existing Curve,
    /// being adjacent is not considered overlapping,
    /// though the windows will be aggregated in that case
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
        windows.into_iter().aggregate()
    }

    /// Limited version of the curve aggregation defined in the paper
    ///
    /// Only defined for Demand Curves as it doesn't rely make sense for Overlap or Supply curves
    /// As overlapping Supply may not be available later and Overlap may not Overlap later
    #[must_use]
    pub fn aggregate<R: CurveType<WindowKind = T::WindowKind>>(self, other: Curve<R>) -> Self {
        crate::paper::aggregate_curve(self, other)
    }

    /// Partition the Curve as Defined by Algorithms 2. and 3. of the paper
    ///
    /// The implementation here deviates from the paper by returning an exclusive index while the paper uses an inclusive index
    #[must_use]
    pub fn partition(&self, group_index: usize, server: &Server) -> PartitionResult {
        match server.server_type {
            ServerKind::Deferrable => {
                // Algorithm 2.

                // Note for Step (1):
                // The paper indexes the Windows 0-based but iterates starting at 1
                // this appears to be a mix-up between 0-based and 1-based indexing
                // which is mixed throughout the paper

                // Note index is i+1 rather than i
                // as 0 is used in case the first window is larges than the server capacity
                // meaning index is exclusive here rather than inclusive as in the paper

                // (1)
                let index = self
                    .windows
                    .iter()
                    .enumerate()
                    .scan(TimeUnit::ZERO, |acc, (index, window)| {
                        *acc += window.length();
                        (*acc <= server.capacity).then(|| index + 1)
                    })
                    .last()
                    .unwrap_or(0);

                // (2)
                let remaining_capacity = server.capacity
                    - self.windows[..index]
                        .iter()
                        .map(Window::length)
                        .sum::<TimeUnit>();

                let (head, tail) = self.windows.get(index).map_or_else(
                    || (Window::empty(), Window::empty()),
                    |window| {
                        if remaining_capacity > TimeUnit::ZERO {
                            // we have remaining capacity the window to fill the remaining budget
                            let head_start = window.start;
                            let tail_end = window.end;
                            let split = head_start + remaining_capacity;
                            let head = Window::new(head_start, split);
                            let tail = Window::new(split, tail_end);
                            (head, tail)
                        } else {
                            // no capacity left set window as tail
                            (Window::empty(), window.clone())
                        }
                    },
                );

                PartitionResult { index, head, tail }
            }
            ServerKind::Periodic => {
                // Algorithm 3.
                // (1)

                let limit = group_index * server.interval + server.capacity;

                // Note index is i+1 rather than i,
                // as 0 is used to indicate that the first window is already past the limit
                // index need therefore be treated as exclusive rather than inclusive as in the paper

                let index = self
                    .windows
                    .iter()
                    .enumerate()
                    .filter_map(|(index, window)| (window.end < limit).then(|| index + 1))
                    .last()
                    .unwrap_or(0);

                // (2)
                let (head, tail) = self.windows.get(index).map_or_else(
                    || (Window::empty(), Window::empty()),
                    |window| {
                        if window.start < limit && limit < window.end {
                            // window crosses the limit, split it at the limit
                            let head = Window::new(window.start, limit);
                            let tail = Window::new(limit, window.end);
                            (head, tail)
                        } else {
                            // Window won't be split as it does not contain the limit
                            // just set the window as the tail
                            (Window::empty(), window.clone())
                        }
                    },
                );

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

impl<T: CurveType + Clone> Curve<T> {
    /// Split the curve on every interval boundary as defined in Definition 8. of the paper
    #[must_use]
    pub fn split(self, interval: TimeUnit) -> HashMap<usize, Self> {
        CurveSplitIterator::new(self.into_iter(), interval).collect()
    }
}

/// Extension trait to allow calling aggregate on an iterator
pub trait AggregateExt: Iterator + Sized {
    /// aggregate all iterator elements
    /// acts similar to [`std::iter::Iterator::sum`]
    fn aggregate<'a, A: Aggregate<'a, Self::Item> + 'a>(self) -> A
    where
        Self: 'a,
        Self::Item: 'a,
    {
        A::aggregate(self)
    }
}

impl<I: Iterator> AggregateExt for I {}

/// Trait used by the `AggregateExt` Extension trait
pub trait Aggregate<'a, A = Self>
where
    A: 'a,
{
    /// aggregate all elements of `iter` into a new Self
    /// pendant to [`std::iter::Sum`]
    fn aggregate<I>(iter: I) -> Self
    where
        I: Iterator<Item = A>,
        I::Item: 'a;
}

impl<'a, 'b: 'a, C: CurveType<WindowKind = Demand>> Aggregate<'a, Window<Demand>> for Curve<C> {
    fn aggregate<I>(iter: I) -> Self
    where
        I: Iterator<Item = Window<Demand>>,
    {
        iter.map(Self::new).fold(Self::empty(), Self::aggregate)
    }
}

/// Marker for `CurveTypes` that can be aggregated into other `CurveTypes`
pub trait AggregatesTo<R: CurveType>: CurveType {}

impl<T: CurveType> AggregatesTo<T> for T {}

impl AggregatesTo<AggregatedServerDemand> for TaskDemand {}
impl AggregatesTo<HigherPriorityServerDemand> for ConstrainedServerDemand {}
impl AggregatesTo<HigherPriorityTaskDemand> for TaskDemand {}

impl<'a, N: CurveType<WindowKind = Demand> + 'a, O: CurveType<WindowKind = Demand>>
    Aggregate<'a, Curve<N>> for Curve<O>
where
    N: AggregatesTo<O>,
{
    fn aggregate<I>(iter: I) -> Self
    where
        I: Iterator<Item = Curve<N>>,
    {
        iter.fold(Self::empty(), Self::aggregate)
    }
}

#[cfg(test)]
mod tests;
