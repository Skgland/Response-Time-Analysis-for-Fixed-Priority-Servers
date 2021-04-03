//! Module defining a Unit of Time

use std::fmt::{Debug, Formatter};
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, Mul, Sub};

mod util {
    //! Utility Module for the time module

    /// Calculate the least common multiple
    pub(crate) const fn lcm(a: super::UnitNumber, b: super::UnitNumber) -> super::UnitNumber {
        if a == b {
            a
        } else {
            a * b / gcd(a, b)
        }
    }

    /// Calculate the greatest common divisor
    const fn gcd(mut a: super::UnitNumber, mut b: super::UnitNumber) -> super::UnitNumber {
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

/// The type that represents a unit-less unsigned number
pub type UnitNumber = usize;

/// The Type representing some Units of Time
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct TimeUnit(UnitNumber);

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

    /// Get the Numeric Value of the `TimeUnit` as a `UnitNumber`
    #[must_use]
    pub const fn as_unit(self) -> UnitNumber {
        self.0
    }
}

impl From<UnitNumber> for TimeUnit {
    fn from(time: UnitNumber) -> Self {
        TimeUnit(time)
    }
}

impl Div for TimeUnit {
    type Output = UnitNumber;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Mul<TimeUnit> for UnitNumber {
    type Output = TimeUnit;

    fn mul(self, rhs: TimeUnit) -> Self::Output {
        TimeUnit(self * rhs.0)
    }
}

impl Mul<UnitNumber> for TimeUnit {
    type Output = TimeUnit;

    fn mul(self, rhs: UnitNumber) -> Self::Output {
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
