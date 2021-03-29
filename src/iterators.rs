//! Module for the Iterator based implementation

use std::fmt::Debug;
use std::iter::{Empty, FusedIterator, TakeWhile};

use crate::curve::curve_types::CurveType;
use crate::window::{Demand, Window};
use std::marker::PhantomData;

pub mod curve;
pub mod server;
pub mod task;

/// Extension trait for reclassifying a `CurveIterator`
/// to any compatible `CurveType`
pub trait ReclassifyExt<'a, O: CurveType> {
    /// reclassify a `CurveIterator`
    fn reclassify<C: CurveType<WindowKind = O::WindowKind>>(
        self,
    ) -> ReclassifyIterator<'a, O, Self, C>
    where
        Self: CurveIterator<'a, O> + Sized;
}

impl<'a, O: CurveType, T> ReclassifyExt<'a, O> for T
where
    T: CurveIterator<'a, O>,
{
    fn reclassify<C: CurveType<WindowKind = O::WindowKind>>(
        self,
    ) -> ReclassifyIterator<'a, O, Self, C>
    where
        Self: CurveIterator<'a, O> + Sized,
    {
        ReclassifyIterator {
            iter: self,
            phantom: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct ReclassifyIterator<
    'a,
    O: CurveType,
    I: CurveIterator<'a, O>,
    C: CurveType<WindowKind = O::WindowKind>,
> {
    iter: I,
    phantom: PhantomData<(&'a (), O, C)>,
}

impl<
        'a,
        O: CurveType + 'a,
        I: CurveIterator<'a, O>,
        C: CurveType<WindowKind = O::WindowKind> + 'a,
    > CurveIterator<'a, C> for ReclassifyIterator<'a, O, I, C>
{
}

impl<'a, O: CurveType, I: CurveIterator<'a, O>, C: CurveType<WindowKind = O::WindowKind>>
    FusedIterator for ReclassifyIterator<'a, O, I, C>
{
}

impl<'a, O: CurveType, I: CurveIterator<'a, O>, C: CurveType<WindowKind = O::WindowKind>> Iterator
    for ReclassifyIterator<'a, O, I, C>
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

/// Trait representing an Iterator that has the guarantees of a curve:
/// 1. Windows ordered by start
/// 2. Windows non-overlapping
/// 3. Windows non-empty
pub trait CurveIterator<'a, C: CurveType>:
    Iterator<Item = Window<C::WindowKind>> + FusedIterator + Debug + 'a
{
}

impl<'a, C: CurveType + 'a> CurveIterator<'a, C> for Empty<Window<C::WindowKind>> {}

impl<'a, C: CurveType + 'a> CurveIterator<'a, C> for Box<dyn CurveIterator<'a, C>> {}

impl<'a, C: CurveType, P: for<'r> FnMut(&'r I::Item) -> bool + 'a, I: CurveIterator<'a, C>>
    CurveIterator<'a, C> for TakeWhile<I, P>
{
}

impl<'t, 'a, C: CurveType, T> CurveIterator<'t, C> for &'t mut T where T: CurveIterator<'a, C> {}

/// `CurveIterator` for turning an Iterator that returns ordered windows,
/// that may be adjacent but that don't overlap further into a `CurveIterator`
#[derive(Debug)]
pub struct JoinAdjacentIterator<I, C>
where
    I: Iterator<Item = Window<C::WindowKind>> + FusedIterator,
    C: CurveType,
{
    iter: I,
    peek: Option<I::Item>,
    curve_type: PhantomData<C>,
}

impl<I, C> JoinAdjacentIterator<I, C>
where
    I: Iterator<Item = Window<C::WindowKind>> + FusedIterator,
    C: CurveType,
{
    /// Create a new `JoinAdjacentIterator`
    /// # Safety
    ///
    /// The Iterator I must return Windows in order that are either don't overlap or at most adjacent
    pub unsafe fn new(iter: I) -> Self {
        JoinAdjacentIterator {
            iter,
            peek: None,
            curve_type: PhantomData,
        }
    }
}

impl<'a, C, I> CurveIterator<'a, C> for JoinAdjacentIterator<I, C>
where
    I: Iterator<Item = Window<C::WindowKind>> + FusedIterator + Debug + 'a,
    C: CurveType<WindowKind = Demand> + 'a,
{
}

impl<'a, C, I> FusedIterator for JoinAdjacentIterator<I, C>
where
    I: Iterator<Item = Window<C::WindowKind>> + FusedIterator,
    C: CurveType<WindowKind = Demand> + 'a,
{
}

impl<'a, C, I> Iterator for JoinAdjacentIterator<I, C>
where
    I: Iterator<Item = Window<C::WindowKind>> + FusedIterator,
    C: CurveType<WindowKind = Demand> + 'a,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let current = self.peek.take().or_else(|| self.iter.next());
            self.peek = self.iter.next();

            match (current, self.peek.as_ref()) {
                (current, None) => break current,
                (None, Some(_)) => {
                    unreachable!("next is filled first")
                }
                (Some(current), Some(peek)) => {
                    if let Some(overlap) = current.aggregate(peek) {
                        assert!(overlap.start == current.start);
                        assert!(overlap.end == peek.end);
                        self.peek = Some(overlap);
                    } else {
                        break Some(current);
                    }
                }
            }
        }
    }
}
