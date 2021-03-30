//! Module defining the Window and its operations

use std::fmt::Debug;
use std::marker::PhantomData;

use crate::curve::curve_types::PrimitiveCurve;
use crate::curve::Curve;
use crate::time::TimeUnit;
use crate::window::window_types::WindowType;

pub mod window_types {
    //!  Module for the `WindowType` trait

    use crate::seal::Seal;
    use crate::window::{Demand, Overlap, Supply};
    use std::fmt::Debug;

    /// Marker Trait for Window Types
    pub trait WindowType: Seal + Clone + Debug + Eq {}

    impl WindowType for Supply {}

    impl WindowType for Demand {}

    impl<P: WindowType, Q: WindowType> WindowType for Overlap<P, Q> {}
}

/// Type representing a Window based on the papers Definition 1.
///
/// With an extra Type Parameter to indicate the Window type
// Not Copy to prevent accidental errors due to implicit copy
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Window<T> {
    /// The Start point of the Window
    pub start: TimeUnit,
    /// The End Point of the Window
    pub end: TimeUnit,
    /// The Kind of the Window
    window_type: PhantomData<T>,
}

impl<T> Clone for Window<T> {
    fn clone(&self) -> Self {
        Window {
            start: self.start,
            end: self.end,
            window_type: PhantomData,
        }
    }
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
        if self.start < self.end {
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
                            //    and the remaining supply is split into head which is before the overlap
                            //    and tail which is after the overlap
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
    ///
    /// See Section 6.2 §3
    #[must_use]
    pub fn budget_group(&self, interval: TimeUnit) -> usize {
        self.start / interval
    }

    /// Create a funktion that returns true for all Windows that
    /// end below or at the limit
    pub fn limit(limit: TimeUnit) -> impl Fn(&Self) -> bool + Clone {
        move |window| window.end <= limit
    }
}

impl Window<Demand> {
    /// Version of [`crate::paper::aggregate_window`] that is constrained to `Window<Demand>`
    #[must_use]
    pub fn aggregate(&self, other: &Self) -> Option<Self> {
        crate::paper::aggregate_window(self, other)
    }
}

/// The Return Type for the [`Window::delta`] calculation
#[derive(Debug, Eq, PartialEq)] // Eq for tests
pub struct WindowDeltaResult<T: WindowType, Q: WindowType> {
    /// The unused "supply"
    /// TODO spilt into two windows remaining_head_supply and remaining_tail_supply
    pub remaining_supply: Curve<PrimitiveCurve<T>>,
    /// The Windows Overlap
    pub overlap: Window<Overlap<T, Q>>,
    /// The unfulfilled "demand"
    pub remaining_demand: Window<Q>,
}

/// Marker Type for Window, indicating a Supply Window
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Supply;

/// Marker Type for Window, indicating Demand
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Demand;

/// Marker Type for Window,indicating an Overlap between Supply and Demand
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Overlap<P, Q>(PhantomData<(P, Q)>);
