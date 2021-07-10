//! Iterators for basic Curve Operations
//!
//! such as `IntoIter`, split, aggregate, delta
//!
//! also `FromIterator` implementation for `Curve`
//!

use core::fmt::Debug;
use core::iter::{FromIterator, FusedIterator};
use core::marker::PhantomData;

use alloc::vec::Vec;

pub use aggregate::AggregationIterator;
pub use delta::{
    CurveDeltaIterator,
    Delta::{self, *},
    InverseCurveIterator,
};
pub use split::CurveSplitIterator;

use crate::curve::curve_types::CurveType;
use crate::curve::Curve;
use crate::iterators::join::JoinAdjacentIterator;
use crate::iterators::CurveIterator;
use crate::time::{TimeUnit, UnitNumber};
use crate::window::window_types::WindowType;
use crate::window::Window;
use crate::window::WindowEnd;

mod aggregate;
mod delta;
mod split;

/// Trait to construct a value of a type from a `CurveIterator`
/// Mirroring [`std::iter::FromIterator`]
pub trait FromCurveIterator<C: CurveType> {
    /// Construct a value from iter
    fn from_curve_iter<CI: CurveIterator<CurveKind = C>>(iter: CI) -> Self;
}

impl<C: CurveType, T: FromIterator<Window<C::WindowKind>>> FromCurveIterator<C> for T {
    fn from_curve_iter<CI: CurveIterator<CurveKind = C>>(iter: CI) -> Self {
        iter.into_iterator().collect()
    }
}

impl<C: CurveType> FromCurveIterator<C> for Curve<C> {
    fn from_curve_iter<CI: CurveIterator<CurveKind = C>>(iter: CI) -> Self {
        let windows = iter.normalize().into_iterator().collect();
        unsafe {
            // Safety:
            // windows collected from `CurveIterator`
            // which invariants guarantee that this is safe
            Curve::from_windows_unchecked(windows)
        }
    }
}

/// `CurveIterator` for iterating a [`Curve`]
#[derive(Debug)]
pub struct CurveIter<C: CurveType> {
    /// The remaining windows of the Curve
    curve: Vec<Window<C::WindowKind>>,
}

impl<C: CurveType> Clone for CurveIter<C> {
    fn clone(&self) -> Self {
        CurveIter {
            curve: self.curve.clone(),
        }
    }
}

impl<C: CurveType> IntoIterator for Curve<C> {
    type Item = Window<C::WindowKind>;
    type IntoIter = CurveIter<C>;

    fn into_iter(self) -> Self::IntoIter {
        let mut windows = self.into_windows();
        windows.reverse();
        CurveIter { curve: windows }
    }
}

impl<C: CurveType> CurveIterator for CurveIter<C> {
    type CurveKind = C;

    fn next_window(&mut self) -> Option<Window<C::WindowKind>> {
        self.curve.pop()
    }
}

impl<C: CurveType> FusedIterator for CurveIter<C> {}

impl<C: CurveType> Iterator for CurveIter<C> {
    type Item = Window<C::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_window()
    }
}

/// Wrapper for wrapping an Iterator into a `CurveIterator`
#[derive(Debug)]
pub struct IterCurveWrapper<I, C> {
    /// the wrapped iterator
    iter: I,

    /// the curve type of this `CurveIterator`
    curve_kind: PhantomData<C>,
}

impl<I: Clone, C> Clone for IterCurveWrapper<I, C> {
    fn clone(&self) -> Self {
        IterCurveWrapper {
            iter: self.iter.clone(),
            curve_kind: PhantomData,
        }
    }
}

impl<I, C> IterCurveWrapper<I, C> {
    /// Wrap an Iterator into a `CurveIterator`
    ///
    /// # Safety
    /// The invariants of a `CurveIterator` need to be upheld
    ///
    pub const unsafe fn new(iter: I) -> Self {
        IterCurveWrapper {
            iter,
            curve_kind: PhantomData,
        }
    }
}

impl<C, I> CurveIterator for IterCurveWrapper<I, C>
where
    Self: Debug,
    C: CurveType,
    I: Iterator<Item = Window<C::WindowKind>>,
{
    type CurveKind = C;

    fn next_window(&mut self) -> Option<Window<C::WindowKind>> {
        self.iter.next()
    }
}

/// Checks that each interval contains a minimum amount of capacity
/// # Panics
/// during usage if this is not the case
#[derive(Debug, Clone)]
pub struct CapacityCheckIterator<W, I, C> {
    /// the inner iterator doing all the work
    iter: JoinAdjacentIterator<InnerCapacityCheckIterator<W, I>, W, C>,
}

impl<W, I, C> CapacityCheckIterator<W, I, C>
where
    W: WindowType,
    I: CurveIterator,
    I::CurveKind: CurveType<WindowKind = W>,
{
    /// Create a new `CapacityCheckIterator`
    ///
    /// That checks that ever `interval` of the curve `to_be_checked` contains at least
    /// `capacity` of capacity
    ///
    /// The returned Iterator panics when not enough capacity was available in a processed group.
    /// The panics occurs when the first window of the next group is requested
    pub fn new(to_be_checked: I, capacity: TimeUnit, interval: TimeUnit) -> Self {
        let inner = InnerCapacityCheckIterator {
            iter: CurveSplitIterator::new(to_be_checked, interval),
            capacity,
            interval,
            current_group: 0,
            accounted: WindowEnd::Finite(TimeUnit::ZERO),
        };

        let outer = unsafe { JoinAdjacentIterator::new(inner) };

        CapacityCheckIterator { iter: outer }
    }
}

impl<W, I, C> CurveIterator for CapacityCheckIterator<W, I, C>
where
    I: CurveIterator<CurveKind = C>,
    C: CurveType<WindowKind = W> + Debug,
    W: WindowType,
{
    type CurveKind = C;

    fn next_window(&mut self) -> Option<Window<C::WindowKind>> {
        self.iter.next_window()
    }
}

/// Inner Iterator for the `CapacityCheckIterator`
#[derive(Debug, Clone)]
struct InnerCapacityCheckIterator<W, I> {
    /// wrapped curve split iterator
    iter: CurveSplitIterator<W, I>,
    /// the capacity each interval should have at least
    capacity: TimeUnit,
    /// the interval in which to check for sufficient capacity
    interval: TimeUnit,
    /// the current group being accounted
    current_group: UnitNumber,
    /// the capacity currently witnessed up to now in the current group
    accounted: WindowEnd,
}

impl<W, I> Iterator for InnerCapacityCheckIterator<W, I>
where
    W: WindowType,
    I: CurveIterator,
    I::CurveKind: CurveType<WindowKind = W>,
{
    type Item = Window<W>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.iter.next() {
            let next_group = next.budget_group(self.interval);

            if next_group == self.current_group {
                self.accounted += next.length();
            } else if next_group == self.current_group + 1 {
                if self.accounted < self.capacity {
                    panic!(
                        "Not enough capacity in group {}, expected at least {:?} capacity , got {:?}, next group {:?}!",
                        self.current_group, self.capacity, self.accounted, next_group
                    );
                }
                self.current_group = next_group;
                self.accounted = next.length();
            } else {
                panic!(
                    "No capacity for group {}, expected {:?} capacity, next group {}!",
                    self.current_group + 1,
                    self.capacity,
                    next_group
                );
            };

            Some(next)
        } else {
            None
        }
    }
}
