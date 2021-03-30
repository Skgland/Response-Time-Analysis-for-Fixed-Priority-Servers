//! Module that defined Curve
//!
//! and all associated functions

use std::collections::HashMap;
use std::fmt::Debug;

use curve_types::CurveType;

use crate::iterators::curve::{CurveDeltaIterator, CurveSplitIterator, Delta};
use crate::server::{Server, ServerKind};

use crate::time::TimeUnit;
use crate::window::{Demand, Overlap, Window};

pub mod curve_types;

/// A Curve is an ordered Set of non-overlapping Windows
///
/// The windows are ordered by their start
#[derive(Debug, Eq, PartialEq)]
pub struct Curve<C: CurveType> {
    /// windows contains an ordered Set of non-overlapping non-empty windows
    windows: Vec<Window<C::WindowKind>>,
}

impl<C: CurveType> Clone for Curve<C> {
    fn clone(&self) -> Self {
        Curve {
            windows: self.windows.clone(),
        }
    }
}

/// Return Type for [`Curve::delta`](Curve::delta)
#[derive(Debug, PartialEq)]
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

impl<DC: CurveType, SC: CurveType, DI, SI> CurveDeltaIterator<'_, DC, SC, DI, SI> {
    /// collect the complete `CurveDeltaIterator`
    ///
    /// # Warning
    ///
    /// Won't terminate of `CurveDelaIterator` if infinite as it will try to consume the complete iterator
    ///
    pub fn collect<R: CurveType<WindowKind = Overlap<SC::WindowKind, DC::WindowKind>>>(
        self,
    ) -> CurveDeltaResult<SC, DC, R>
    where
        Self: Iterator<Item = Delta<SC::WindowKind, DC::WindowKind>>,
    {
        let mut result = CurveDeltaResult {
            remaining_supply: Curve::empty(),
            overlap: Curve::empty(),
            remaining_demand: Curve::empty(),
        };

        for delta in self {
            match delta {
                Delta::RemainingSupply(supply) => result.remaining_supply.windows.push(supply),
                Delta::Overlap(overlap) => result.overlap.windows.push(overlap),
                Delta::RemainingDemand(demand) => result.remaining_demand.windows.push(demand),
            }
        }

        result
    }
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
    /// TODO make unsafe as one can violate the curves invariants
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

    /// Change the `CurveType` of the Curve,
    /// requires that the `WindowType` of both [`CurveTypes`](trait@CurveType) is the same
    #[must_use]
    pub fn reclassify<C: CurveType<WindowKind = T::WindowKind>>(self) -> Curve<C> {
        Curve {
            windows: self.windows,
        }
    }
}

impl<T: CurveType<WindowKind = Demand>> Curve<T> {
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
                            // we have remaining capacity and a window to fill the remaining budget
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

impl<T: CurveType> Curve<T> {
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
