//! Module defining the Window and its operations

use std::marker::PhantomData;

use crate::curve::Curve;
use crate::seal::WindowType;
use crate::time::TimeUnit;

/// Type representing a Window based on the papers Definition 1.
///
/// With an extra Type Parameter to indicate the Window type
// Not Copy to prevent accidental errors due to implicit copy
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Window<T: WindowType> {
    /// The Start point of the Window
    pub start: TimeUnit,
    /// The End Point of the Window
    pub end: TimeUnit,
    /// The Kind of the Window
    window_type: PhantomData<T>,
}

impl<T: WindowType> Window<T> {
    /// Create a new Window
    #[must_use]
    pub fn new<I: Into<TimeUnit>>(start: I, end: I) -> Self {
        Window {
            start: start.into(),
            end: end.into(),
            window_type: PhantomData,
        }
    }

    /// Create a new empty Window
    #[must_use]
    pub fn empty() -> Self {
        Window {
            start: TimeUnit::ZERO,
            end: TimeUnit::ZERO,
            window_type: PhantomData,
        }
    }

    /// Calculate the window length as defined in Definition 1. of the paper
    #[must_use]
    pub fn length(&self) -> TimeUnit {
        if self.end > self.start {
            self.end - self.start
        } else {
            TimeUnit::ZERO
        }
    }

    /// Calculate the overlap (Ω) of two windows as defined in Definition 2. of the paper
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        !(self.end < other.start || other.end < self.start)
    }
    /// Calculate the aggregation (⊕) of two windows as defined in Definition 4. of the paper
    #[must_use]
    pub fn aggregate(&self, other: &Self) -> Option<Self> {
        self.overlaps(other).then(|| {
            let start = TimeUnit::min(self.start, other.start);
            let end = start + self.length() + other.length();
            Window::new(start, end)
        })
    }

    /// Calculate the Window delta as defined in Definition 6. of the paper
    #[must_use]
    pub fn delta<Q: WindowType>(supply: &Self, demand: &Window<Q>) -> WindowDeltaResult<T, Q> {
        if supply.end < demand.start {
            WindowDeltaResult {
                remaining_supply: Curve::new(supply.clone()),
                overlap: Window::empty(),
                remaining_demand: demand.clone(),
            }
        } else {
            let overlap_start = TimeUnit::max(supply.start, demand.start);
            let overlap_end =
                overlap_start + TimeUnit::min(demand.length(), supply.end - overlap_start);
            let overlap = Window::new(overlap_start, overlap_end);

            let remaining_demand = Window::new(demand.start + overlap.length(), demand.end);

            let remaining_head_demand = Window::new(supply.start, overlap.start);
            let remaining_tail_demand = Window::new(overlap.end, supply.end);

            WindowDeltaResult {
                remaining_demand,
                remaining_supply: {
                    {
                        let mut windows = vec![remaining_head_demand, remaining_tail_demand];

                        windows.retain(|window| !window.is_empty());
                        unsafe {
                            // Safety: Invariants fulfilled by construction,
                            // 1. Order: head always before tail
                            // 2. Non-Overlapping, as supply and demand overlap
                            Curve::from_windows_unchecked(windows)
                        }
                    }
                },
                overlap,
            }
        }
    }

    /// Whether the window is empty/has a length of 0
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length() == TimeUnit::ZERO
    }

    /// Calculate the Budget Group that the window falls into
    /// given a splitting interval
    /// TODO reference paper
    #[must_use]
    pub fn budget_group(&self, interval: TimeUnit) -> usize {
        self.start / interval
    }

    /// Convert one Window type into another
    #[must_use]
    pub fn to_other<I: WindowType>(&self) -> Window<I> {
        Window::new(self.start, self.end)
    }
}

/// The Return Type for the [`Window::delta`] calculation
#[derive(Debug, Eq, PartialEq)] // Eq for tests
pub struct WindowDeltaResult<P: WindowType, Q: WindowType> {
    /// The unused "supply"
    pub remaining_supply: Curve<P>,
    /// The Windows Overlap
    pub overlap: Window<Overlap>,
    /// The unfulfilled "demand"
    pub remaining_demand: Window<Q>,
}

#[cfg(test)]
mod tests;

/// Marker Type for Window, indicating a Supply Window
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Supply;

/// Marker Type for Window, indicating Demand
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Demand;

/// Marker Type for Window,indicating an Overlap between Supply and Demand
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Overlap;
