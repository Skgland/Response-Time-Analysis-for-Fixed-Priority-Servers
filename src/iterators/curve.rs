//! Iterators for basic Curve Operations
//!
//! such as `IntoIter`, split, aggregate, delta
//!
//! also `FromIterator` implementation for `Curve`
//!

use std::iter::FusedIterator;

pub use delta::{
    CurveDeltaIterator,
    Delta::{self, *},
    InverseCurveIterator,
};

pub use aggregate::AggregationIterator;

pub use split::CurveSplitIterator;

use crate::curve::curve_types::CurveType;
use crate::curve::Curve;
use crate::iterators::{CurveIterator, JoinAdjacentIterator};
use crate::time::{TimeUnit, UnitNumber};
use crate::window::window_types::WindowType;
use crate::window::{Window, WindowEnd};
use std::fmt::Debug;
use std::marker::PhantomData;

mod aggregate;
mod delta;
mod split;

/// Trait to construct a value of a type from a `CurveIterator`
/// Mirroring [`std::iter::FromIterator`]
pub trait FromCurveIterator<C: CurveType> {
    /// Construct a value from iter
    fn from_curve_iter<CI: IntoIterator>(iter: CI) -> Self
    where
        CI::IntoIter: CurveIterator<C::WindowKind, CurveKind = C>;
}

impl<C: CurveType> FromCurveIterator<C> for Curve<C> {
    fn from_curve_iter<CI: IntoIterator>(iter: CI) -> Self
    where
        CI::IntoIter: CurveIterator<C::WindowKind, CurveKind = C>,
    {
        let windows = iter.into_iter().collect();
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

impl<C: CurveType> CurveIterator<C::WindowKind> for CurveIter<C> {
    type CurveKind = C;
}

impl<C: CurveType> FusedIterator for CurveIter<C> {}

impl<C: CurveType> Iterator for CurveIter<C> {
    type Item = Window<C::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        self.curve.pop()
    }
}

/// Wrapper for wrapping an Iterator into a `CurveIterator`
#[derive(Debug)]
struct IterCurveWrapper<I, C> {
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

impl<C, I> CurveIterator<C::WindowKind> for IterCurveWrapper<I, C>
where
    Self: Debug,
    C: CurveType,
    I: Iterator<Item = Window<C::WindowKind>>,
{
    type CurveKind = C;
}

impl<I, C> FusedIterator for IterCurveWrapper<I, C> where I: FusedIterator {}

impl<I: Iterator, C> Iterator for IterCurveWrapper<I, C> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[derive(Debug, Clone)]
pub struct CapacityCheckIterator<W, I, C> {
    iter: JoinAdjacentIterator<InnerCapacityCheckIterator<W, I>, W, C>,
}

impl<W, I, C> CapacityCheckIterator<W, I, C>
where
    W: WindowType,
    I: CurveIterator<W>,
{
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

impl<W, I, C> CurveIterator<W> for CapacityCheckIterator<W, I, C>
where
    I: CurveIterator<W, CurveKind = C>,
    C: CurveType<WindowKind = W> + Debug,
    W: WindowType,
{
    type CurveKind = C;
}

impl<W, I, C> FusedIterator for CapacityCheckIterator<W, I, C>
where
    Self: Iterator,
    I: FusedIterator,
{
}

impl<W, I, C> Iterator for CapacityCheckIterator<W, I, C>
where
    I: CurveIterator<W, Item = Window<W>>,
    W: WindowType,
{
    type Item = Window<W>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[derive(Debug, Clone)]
struct InnerCapacityCheckIterator<W, I> {
    iter: CurveSplitIterator<W, I>,
    capacity: TimeUnit,
    interval: TimeUnit,
    current_group: UnitNumber,
    accounted: WindowEnd,
}

impl<W, I> Iterator for InnerCapacityCheckIterator<W, I>
where
    W: WindowType,
    I: CurveIterator<W, Item = Window<W>>,
{
    type Item = Window<W>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.iter.next() {
            println!("Next Window: {:?}", next);
            println!("Accounted: {:?}", self.accounted);
            println!("Interval: {:?}", self.interval);

            let next_group = next.budget_group(self.interval);

            println!(
                "Current Group {}, Next Group {}",
                self.current_group, next_group
            );

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
