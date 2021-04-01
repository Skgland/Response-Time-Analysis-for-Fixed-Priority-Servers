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
use crate::iterators::CurveIterator;
use crate::window::Window;
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
