//! Iterators for basic Curve Operations
//!
//! such as `IntoIter`, split, aggregate, delta
//!
//! also `FromIterator` implementation for `Curve`
//!
use std::collections::VecDeque;
use std::iter::FusedIterator;

pub use aggregate::{AggregatedDemandIterator, RecursiveAggregatedDemandIterator};
pub use delta::{
    CurveDeltaIterator,
    Delta::{self, *},
};

pub use split::CurveSplitIterator;

use crate::curve::curve_types::CurveType;
use crate::curve::Curve;
use crate::iterators::CurveIterator;
use crate::window::Window;
use std::fmt::Debug;

mod aggregate;
mod delta;
mod split;

/// Trait to construct a value of a type from a `CurveIterator`
/// Mirroring [`std::iter::FromIterator`]
pub trait FromCurveIterator<C: CurveType> {
    /// Construct a value from iter
    fn from_curve_iter<CI: IntoIterator>(iter: CI) -> Self
    where
        CI::IntoIter: CurveIterator<C>;
}

impl<IC: CurveType, C: CurveType<WindowKind = IC::WindowKind>> FromCurveIterator<IC> for Curve<C> {
    fn from_curve_iter<CI: IntoIterator>(iter: CI) -> Self
    where
        CI::IntoIter: CurveIterator<IC>,
    {
        let windows = iter.into_iter().collect();
        unsafe {
            // windows collected from `CurveIterator`
            // which invariants guarantee that this is safe
            Curve::from_windows_unchecked(windows)
        }
    }
}

/// Extension trait mirroring [`std::iter::Iterator::collect`]
pub trait CollectCurveExt<C: CurveType>: CurveIterator<C> + Sized {
    /// collect the iterator
    fn collect_curve<R: FromCurveIterator<C>>(self) -> R {
        R::from_curve_iter(self)
    }
}

impl<C: CurveType, CI: CurveIterator<C>> CollectCurveExt<C> for CI {}

/// `CurveIterator` for iterating a [`Curve`]
#[derive(Debug)]
pub struct CurveIter<C: CurveType> {
    /// The remaining windows of the Curve
    curve: VecDeque<Window<C::WindowKind>>,
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
        CurveIter {
            curve: self.into_windows().into(),
        }
    }
}

impl<C: CurveType> CurveIterator<C> for CurveIter<C> {}

impl<C: CurveType> FusedIterator for CurveIter<C> {}

impl<C: CurveType> Iterator for CurveIter<C> {
    type Item = Window<C::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        self.curve.pop_front()
    }
}

/// Wrapper for wrapping an Iterator into a `CurveIterator`
#[derive(Debug, Clone)]
struct IterCurveWrapper<I> {
    /// the wrapped iterator
    iter: I,
}

impl<I> IterCurveWrapper<I> {
    /// Wrap an Iterator into a `CurveIterator`
    ///
    /// # Safety
    /// The invariants of a `CurveIterator` need to be upheld
    ///
    pub const unsafe fn new(iter: I) -> Self {
        IterCurveWrapper { iter }
    }
}

impl<C, I> CurveIterator<C> for IterCurveWrapper<I>
where
    Self: Debug,
    C: CurveType,
    I: Iterator<Item = Window<C::WindowKind>>,
{
}

impl<I> FusedIterator for IterCurveWrapper<I> where I: FusedIterator {}

impl<I: Iterator> Iterator for IterCurveWrapper<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}
