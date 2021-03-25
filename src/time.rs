//! Module defining a Unit of Time

use std::fmt::{Debug, Formatter};
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, Mul, Sub};

mod util {
    //! Utility Module for the time module

    /// Calculate the least common multiple
    pub(crate) const fn lcm(a: super::InternalTime, b: super::InternalTime) -> super::InternalTime {
        if a == b {
            a
        } else {
            a * b / gcd(a, b)
        }
    }

    /// Calculate the greatest common divisor
    const fn gcd(mut a: super::InternalTime, mut b: super::InternalTime) -> super::InternalTime {
        while a != b {
            if a > b {
                a -= b
            } else {
                b -= a
            }
        }
        a
    }
}

/// The Type that `TimeUnit` is represented by internally
type InternalTime = usize;

/// The Type representing some Units of Time
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct TimeUnit(InternalTime);

impl Debug for TimeUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl TimeUnit {
    /// Zero Units of Time
    pub const ZERO: TimeUnit = TimeUnit(0);

    /// One Unit of Time
    pub const ONE: TimeUnit = TimeUnit(1);

    /// Get the longer/maximal Unit of Time
    #[must_use]
    pub fn max(self, other: Self) -> Self {
        TimeUnit(self.0.max(other.0))
    }

    /// Calculate the least common multiple
    #[must_use]
    pub const fn lcm(self, other: Self) -> Self {
        TimeUnit(util::lcm(self.0, other.0))
    }
}

impl From<InternalTime> for TimeUnit {
    fn from(time: InternalTime) -> Self {
        TimeUnit(time)
    }
}

impl Div for TimeUnit {
    type Output = usize;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Mul<TimeUnit> for usize {
    type Output = TimeUnit;

    fn mul(self, rhs: TimeUnit) -> Self::Output {
        TimeUnit(self * rhs.0)
    }
}

impl Mul<usize> for TimeUnit {
    type Output = TimeUnit;

    fn mul(self, rhs: usize) -> Self::Output {
        TimeUnit(self.0 * rhs)
    }
}

impl Add for TimeUnit {
    type Output = TimeUnit;

    fn add(self, rhs: Self) -> Self::Output {
        TimeUnit(self.0 + rhs.0)
    }
}

impl AddAssign for TimeUnit {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl AsRef<TimeUnit> for TimeUnit {
    fn as_ref(&self) -> &TimeUnit {
        self
    }
}

impl<T: AsRef<TimeUnit>> Sub<T> for TimeUnit {
    type Output = TimeUnit;

    fn sub(self, rhs: T) -> Self::Output {
        TimeUnit(self.0 - rhs.as_ref().0)
    }
}

impl Sum for TimeUnit {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(TimeUnit::from(0), Self::add)
    }
}
