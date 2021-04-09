//! Module defining the Window and its operations

use std::fmt::Debug;
use std::marker::PhantomData;

use crate::time::{TimeUnit, UnitNumber};
use crate::window::window_types::WindowType;
use std::cmp::Ordering;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Sub};

pub mod window_types {
    //!  Module for the `WindowType` trait

    use crate::seal::Seal;
    use crate::window::{Demand, Overlap, Supply};
    use std::fmt::Debug;

    /// Marker Trait for Window Types
    pub trait WindowType: Seal + Debug {}

    impl WindowType for Supply {}

    impl WindowType for Demand {}

    impl<P: WindowType, Q: WindowType> WindowType for Overlap<P, Q> {}
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum WindowEnd {
    Finite(TimeUnit),
    Infinite,
}

impl WindowEnd {
    pub fn min(self, other: Self) -> Self {
        if self < other {
            self
        } else {
            other
        }
    }
}

impl AddAssign for WindowEnd {
    fn add_assign(&mut self, rhs: Self) {
        *self = &*self + &rhs;
    }
}

impl Add for WindowEnd {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        &self + &rhs
    }
}

impl Add<TimeUnit> for WindowEnd {
    type Output = WindowEnd;

    fn add(self, rhs: TimeUnit) -> Self::Output {
        match self {
            WindowEnd::Finite(us) => Self::Finite(us + rhs),
            WindowEnd::Infinite => Self::Infinite,
        }
    }
}

impl Add<WindowEnd> for TimeUnit {
    type Output = WindowEnd;

    fn add(self, rhs: WindowEnd) -> Self::Output {
        match rhs {
            WindowEnd::Finite(them) => WindowEnd::Finite(them + self),
            WindowEnd::Infinite => WindowEnd::Infinite,
        }
    }
}

impl Add for &mut WindowEnd {
    type Output = WindowEnd;

    fn add(self, rhs: Self) -> Self::Output {
        &*self + &*rhs
    }
}

impl Add for &WindowEnd {
    type Output = WindowEnd;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (WindowEnd::Finite(a), WindowEnd::Finite(b)) => WindowEnd::Finite(*a + *b),
            (WindowEnd::Infinite, _) | (_, WindowEnd::Infinite) => WindowEnd::Infinite,
        }
    }
}

impl PartialOrd for WindowEnd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Infinite, Self::Infinite) => Some(Ordering::Equal),
            (Self::Infinite, Self::Finite(_)) => Some(Ordering::Greater),
            (Self::Finite(_), Self::Infinite) => Some(Ordering::Less),
            (Self::Finite(a), Self::Finite(b)) => a.partial_cmp(b),
        }
    }
}

impl PartialEq<TimeUnit> for WindowEnd {
    fn eq(&self, other: &TimeUnit) -> bool {
        match self {
            WindowEnd::Finite(us) => us.eq(other),
            WindowEnd::Infinite => false,
        }
    }
}

impl PartialEq<WindowEnd> for TimeUnit {
    fn eq(&self, other: &WindowEnd) -> bool {
        match other {
            WindowEnd::Finite(them) => them.eq(self),
            WindowEnd::Infinite => false,
        }
    }
}

impl PartialOrd<TimeUnit> for WindowEnd {
    fn partial_cmp(&self, other: &TimeUnit) -> Option<Ordering> {
        match self {
            WindowEnd::Finite(us) => us.partial_cmp(other),
            WindowEnd::Infinite => Some(Ordering::Greater),
        }
    }
}

impl PartialOrd<WindowEnd> for TimeUnit {
    fn partial_cmp(&self, other: &WindowEnd) -> Option<Ordering> {
        match other {
            WindowEnd::Finite(them) => self.partial_cmp(them),
            WindowEnd::Infinite => Some(Ordering::Less),
        }
    }
}

impl<I: Into<TimeUnit>> From<Option<I>> for WindowEnd {
    fn from(time: Option<I>) -> Self {
        if let Some(time) = time {
            Self::Finite(time.into())
        } else {
            Self::Infinite
        }
    }
}

impl<I: Into<TimeUnit>> From<I> for WindowEnd {
    fn from(time: I) -> Self {
        Self::Finite(time.into())
    }
}

impl Sum for WindowEnd {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(WindowEnd::Finite(TimeUnit::ZERO), |acc, next| {
            match (acc, next) {
                (Self::Finite(a), Self::Finite(b)) => Self::Finite(a + b),
                (WindowEnd::Infinite, _) | (_, WindowEnd::Infinite) => Self::Infinite,
            }
        })
    }
}

impl Sub<TimeUnit> for WindowEnd {
    type Output = WindowEnd;

    fn sub(self, rhs: TimeUnit) -> Self::Output {
        match self {
            WindowEnd::Finite(time) => Self::Finite(time - rhs),
            WindowEnd::Infinite => Self::Infinite,
        }
    }
}

/// Type representing a Window based on the papers Definition 1.
///
/// With an extra Type Parameter to indicate the Window type
// Not Copy to prevent accidental errors due to implicit copy
#[derive(Debug, Hash, Eq)]
pub struct Window<T> {
    /// The Start point of the Window
    pub start: TimeUnit,
    /// The End Point of the Window
    pub end: WindowEnd,
    /// The Kind of the Window
    window_type: PhantomData<T>,
}

impl<W> PartialEq for Window<W> {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
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
    pub fn new<I: Into<TimeUnit>, E: Into<WindowEnd>>(start: I, end: E) -> Self {
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
            end: WindowEnd::Finite(TimeUnit::ZERO),
            window_type: PhantomData,
        }
    }

    /// Calculate the window length as defined in Definition 1. of the paper
    #[must_use]
    pub fn length(&self) -> WindowEnd {
        match self.end {
            WindowEnd::Finite(end) => {
                let end = if self.start < end {
                    end - self.start
                } else {
                    TimeUnit::ZERO
                };
                WindowEnd::Finite(end)
            }
            WindowEnd::Infinite => WindowEnd::Infinite,
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
                remaining_supply_head: supply.clone(),
                remaining_supply_tail: Window::empty(),
                overlap: Window::empty(),
                remaining_demand: demand.clone(),
            }
        } else {
            let overlap_start = TimeUnit::max(supply.start, demand.start);
            let overlap_end: WindowEnd =
                overlap_start + WindowEnd::min(demand.length(), supply.end - overlap_start);
            let overlap = Window::new(overlap_start, overlap_end);

            let remaining_demand = match overlap.length() {
                WindowEnd::Finite(length) => Window::new(demand.start + length, demand.end),
                WindowEnd::Infinite => {
                    // Infinite supply satisfies infinite demand, no demand left

                    Window::empty()
                }
            };

            let remaining_supply_head = Window::new(supply.start, overlap.start);
            let remaining_supply_tail = match overlap.end {
                WindowEnd::Finite(overlap_end) => Window::new(overlap_end, supply.end),
                WindowEnd::Infinite => {
                    // Infinite supply satisfies infinite demand, no tail supply left
                    Window::empty()
                }
            };

            WindowDeltaResult {
                remaining_demand,
                remaining_supply_head,
                remaining_supply_tail,
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
    pub fn budget_group(&self, interval: TimeUnit) -> UnitNumber {
        self.start / interval
    }

    /// Create a function that returns true for all Windows that
    /// end below or at the limit
    pub fn limit(limit: TimeUnit) -> impl Fn(&Self) -> bool + Clone {
        move |window| window.end <= limit
    }
}

impl Window<Demand> {
    /// Calculate the aggregation (⊕) of two windows as defined in Definition 4. of the paper
    #[must_use]
    pub fn aggregate(&self, other: &Self) -> Option<Self> {
        // only defined for overlapping windows, return None when not overlapping
        self.overlaps(other).then(|| {
            let start = TimeUnit::min(self.start, other.start);
            let end = start + self.length() + other.length();
            Window::new(start, end)
        })
    }
}

/// The Return Type for the [`Window::delta`] calculation
#[derive(Debug, Eq, PartialEq)] // Eq for tests
pub struct WindowDeltaResult<P: WindowType, Q: WindowType> {
    /// The unused supply at the start of the original supply window
    pub remaining_supply_head: Window<P>,
    /// The unused supply at the end of the original supply window
    pub remaining_supply_tail: Window<P>,
    /// The Windows Overlap
    pub overlap: Window<Overlap<P, Q>>,
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
