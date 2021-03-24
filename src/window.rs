//! Module defining the Window and its operations

use crate::curve::{Curve, Demand, Overlap, Supply};
use crate::seal::WindowType;
use std::marker::PhantomData;

/// Type representing a Window based on the papers Definition 1.
///
/// With an extra Type Parameter to indicate the Window type
// Not Copy to prevent accidental errors due to implicit copy
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Window<T: WindowType> {
    /// The Start point of the Window
    pub start: usize,
    /// The End Point of the Window
    pub end: usize,
    /// The Kind of the Window
    window_type: PhantomData<T>,
}

impl<T: WindowType> Window<T> {
    /// Create a new Window
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        Window {
            start,
            end,
            window_type: PhantomData,
        }
    }

    /// Create a new empty Window
    #[must_use]
    pub fn empty() -> Self {
        Window {
            start: 0,
            end: 0,
            window_type: PhantomData,
        }
    }

    // Window for definition 13. Definition
    #[must_use]
    pub fn up_to(total: usize) -> Self {
        Window::new(0, total)
    }

    // Ω Definition 2.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        !(self.end < other.start || other.end < self.start)
    }

    // ⊕ Definition 4.
    #[must_use]
    pub fn aggregate(&self, other: &Self) -> Option<Self> {
        self.overlaps(other).then(|| {
            let start = usize::min(self.start, other.start);
            let end = start + self.length() + other.length();
            Window::new(start, end)
        })
    }

    // Definition 1.
    #[must_use]
    pub fn length(&self) -> usize {
        if self.end > self.start {
            self.end - self.start
        } else {
            0
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length() == 0
    }

    #[must_use]
    pub fn budget_group(&self, interval: usize) -> usize {
        self.start / interval
    }

    #[must_use]
    pub fn to_other<I: WindowType>(&self) -> Window<I> {
        Window::new(self.start, self.end)
    }
}

impl Window<Supply> {
    // Definition 6.
    #[must_use]
    pub fn delta(supply: &Window<Supply>, demand: &Window<Demand>) -> WindowDeltaResult {
        if supply.end < demand.start {
            WindowDeltaResult {
                remaining_supply: Curve::new(supply.clone()),
                overlap: Window::empty(),
                remaining_demand: demand.clone(),
            }
        } else {
            let overlap_start = usize::max(supply.start, demand.start);
            let overlap_end =
                overlap_start + usize::min(demand.length(), supply.end - overlap_start);
            let overlap = Window::new(overlap_start, overlap_end);

            let remaining_demand = Window::new(demand.start + overlap.length(), demand.end);

            let remaining_head_demand = Window::new(supply.start, overlap.start);
            let remaining_tail_demand = Window::new(overlap.end, supply.end);

            WindowDeltaResult {
                remaining_demand,
                remaining_supply: {
                    let mut curve = unsafe {
                        let mut windows = vec![remaining_head_demand, remaining_tail_demand];

                        windows.retain(|window| !window.is_empty());

                        // Safety: Invariants fulfilled by construction,
                        // 1. Order: head always before tail
                        // 2. Non-Overlapping, as supply and demand overlap
                        Curve::from_windows_unchecked(windows)
                    };
                    curve
                },
                overlap,
            }
        }
    }
}

// deriving Eq for testing
#[derive(Debug, Eq, PartialEq)]
pub struct WindowDeltaResult {
    pub remaining_supply: Curve<Supply>,
    pub overlap: Window<Overlap>,
    pub remaining_demand: Window<Demand>,
}

#[cfg(test)]
mod tests;
