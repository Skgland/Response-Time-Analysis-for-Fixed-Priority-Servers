//! Module for the Implementation of the `WindowEnd` type (to be renamed)

use std::cmp::Ordering;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Sub};

use crate::time::TimeUnit;

// TODO find better name
/// Type for the
/// 1. End of a window
/// 2. Length of a window
/// 3. Capacity of a Curve
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum WindowEnd {
    /// A Finite Window end, Window length, Curve length
    Finite(TimeUnit),
    /// An Infinite Window end, Window length, Curve length
    Infinite,
}

impl WindowEnd {
    /// return the minimal value
    /// Finite values are always smaller than Infinite
    #[must_use]
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
        *self = *self + rhs;
    }
}

impl Add for WindowEnd {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        #![allow(clippy::op_ref)]
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
        #![allow(clippy::op_ref)]
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
        time.map_or(Self::Infinite, |time| Self::Finite(time.into()))
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
