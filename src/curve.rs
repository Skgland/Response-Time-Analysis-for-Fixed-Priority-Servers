//! Module that defines the finite Curve type
//!
//! and all associated functions

use std::fmt::Debug;

use curve_types::CurveType;

use crate::iterators::curve::{CurveDeltaIterator, Delta};
use crate::server::{ServerKind, ServerProperties};

use crate::iterators::CurveIterator;
use crate::time::{TimeUnit, UnitNumber};
use crate::window::window_types::WindowType;
use crate::window::{Demand, Overlap, Window};

pub mod curve_types;

/// A Curve is an ordered Set of non-overlapping Windows
///
/// The windows are ordered by their start
#[derive(Debug, Eq)]
pub struct Curve<C: CurveType> {
    /// windows contains an ordered Set of non-overlapping non-empty windows
    windows: Vec<Window<C::WindowKind>>,
}

impl<C: CurveType> PartialEq for Curve<C> {
    fn eq(&self, other: &Self) -> bool {
        self.windows.eq(&other.windows)
    }
}

impl<C: CurveType> Clone for Curve<C> {
    fn clone(&self) -> Self {
        Curve {
            windows: self.windows.clone(),
        }
    }
}

impl<T: CurveType> Curve<T> {
    /// Create a new Curve from the provided window
    ///
    /// May return a Curve with no Windows when the provided Window is empty
    #[must_use]
    pub fn new(window: Window<T::WindowKind>) -> Self {
        let windows = if window.is_empty() {
            // Empty windows can be ignored
            vec![]
        } else {
            // A Curve with only a single has
            // the windows always ordered and non-overlapping
            vec![window]
        };

        Self { windows }
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

    /// compare the curve to a curve iterator
    /// consuming the iterator in the process
    pub fn eq_curve_iterator<CI: CurveIterator<T::WindowKind, CurveKind = T>>(
        &self,
        mut other: CI,
    ) -> bool {
        let mut windows = self.as_windows().iter();

        loop {
            match (windows.next(), other.next()) {
                (None, None) => break true,
                (Some(_), None) | (None, Some(_)) => break false,
                (Some(left), Some(right)) => {
                    if left.ne(&right) {
                        break false;
                    }
                }
            }
        }
    }
}

impl<T: CurveType<WindowKind = Demand>> Curve<T> {
    /// Partition the Curve as Defined by Algorithms 2. and 3. of the paper
    ///
    /// The implementation here deviates from the paper by returning an exclusive index while the paper uses an inclusive index
    #[must_use]
    pub fn partition(
        &self,
        group_index: UnitNumber,
        server_properties: ServerProperties,
    ) -> PartitionResult {
        match server_properties.server_type {
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
                        (*acc <= server_properties.capacity).then(|| index + 1)
                    })
                    .last()
                    .unwrap_or(0);

                // (2)
                let remaining_capacity = server_properties.capacity
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

                let limit = group_index * server_properties.interval + server_properties.capacity;

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

/// Return Type for [`CurveDeltaIterator::collect_delta`]
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

impl<DW: WindowType, SW: WindowType, DI, SI> CurveDeltaIterator<DW, SW, DI, SI>
where
    DI: CurveIterator<DW>,
    SI: CurveIterator<SW>,
{
    /// collect the complete `CurveDeltaIterator`
    ///
    /// # Warning
    ///
    /// Won't terminate if `CurveDelaIterator` is infinite as it will try to consume the complete iterator
    ///
    pub fn collect_delta<R: CurveType<WindowKind = Overlap<SW, DW>>>(
        self,
    ) -> CurveDeltaResult<SI::CurveKind, DI::CurveKind, R>
    where
        Self: Iterator<Item = Delta<SW, DW, SI, DI>>,
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
                Delta::EndSupply(supply) => {
                    supply.for_each(|window| result.remaining_supply.windows.push(window))
                }
                Delta::EndDemand(demand) => {
                    demand.for_each(|window| result.remaining_demand.windows.push(window))
                }
            }
        }

        result
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

/// Extension trait to allow calling aggregate on an iterator
pub trait AggregateExt: Iterator + Sized {
    /// aggregate all iterator elements
    /// acts similar to [`std::iter::Iterator::sum`]
    fn aggregate<A: Aggregate<Self::Item>>(self) -> A {
        A::aggregate(self)
    }
}

impl<I: Iterator> AggregateExt for I {}

/// Trait used by the `AggregateExt` Extension trait
pub trait Aggregate<A = Self> {
    /// aggregate all elements of `iter` into a new Self
    /// pendant to [`std::iter::Sum`]
    fn aggregate<I>(iter: I) -> Self
    where
        I: Iterator<Item = A>;
}
