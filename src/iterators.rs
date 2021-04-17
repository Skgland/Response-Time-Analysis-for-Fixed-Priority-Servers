//! Module for the Iterator based implementation

use std::fmt::Debug;
use std::iter::{Empty, Fuse, FusedIterator, TakeWhile};

use crate::curve::curve_types::{CurveType, UnspecifiedCurve};
use crate::iterators::curve::FromCurveIterator;
use crate::window::window_types::WindowType;
use crate::window::Window;
use std::marker::PhantomData;

pub mod curve;
pub mod server;
pub mod task;

/// `CurveIterator` wrapper to change the Curve type to any compatibly `CurveType`
#[derive(Debug)]
pub struct ReclassifyIterator<I, C, O> {
    /// the wrapped CurveIterator
    iter: I,
    /// The original and output curve type and `CurveType`
    phantom: PhantomData<(C, O)>,
}

impl<I: Clone, C, O> Clone for ReclassifyIterator<I, C, O> {
    fn clone(&self) -> Self {
        ReclassifyIterator {
            iter: self.iter.clone(),
            phantom: PhantomData,
        }
    }
}

impl<I, O, C> CurveIterator<O::WindowKind> for ReclassifyIterator<I, C, O>
where
    I: CurveIterator<C::WindowKind, CurveKind = C>,
    O: CurveType<WindowKind = C::WindowKind>,
    C: CurveType,
{
    type CurveKind = O;
}

impl<I, C, O> FusedIterator for ReclassifyIterator<I, C, O> where I: FusedIterator {}

impl<I, C, O> Iterator for ReclassifyIterator<I, C, O>
where
    I: Iterator,
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
///
/// Or in other words all finite prefixes of the Iterator are a valid Curves
///
pub trait CurveIterator<W>
where
    Self: Iterator<Item = Window<W>> + Debug,
{
    /// The type of the curve being iterated
    type CurveKind: CurveType<WindowKind = W>;

    /// calculate and returns the next window of the curve iterator
    /// advancing the iterator in the process
    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.next()
    }

    /// collect the iterator mirroring [`std::iter::Iterator::collect`]
    #[must_use]
    fn collect_curve<R: FromCurveIterator<Self::CurveKind>>(self) -> R
    where
        Self: Sized,
    {
        R::from_curve_iter(self)
    }

    /// reclassify a `CurveIterator`
    #[must_use]
    fn reclassify<O>(self) -> ReclassifyIterator<Self, Self::CurveKind, O>
    where
        Self: Sized,
    {
        ReclassifyIterator {
            iter: self,
            phantom: PhantomData,
        }
    }
}

impl<W> CurveIterator<W> for Empty<Window<W>>
where
    W: WindowType,
{
    type CurveKind = UnspecifiedCurve<W>;
}

impl<W: WindowType, CI> CurveIterator<W> for Fuse<CI>
where
    CI: CurveIterator<W>,
{
    type CurveKind = CI::CurveKind;
}

impl<W, P, I> CurveIterator<W> for TakeWhile<I, P>
where
    P: for<'r> FnMut(&'r I::Item) -> bool,
    I: CurveIterator<W>,
{
    type CurveKind = I::CurveKind;
}

/// `CurveIterator` for turning an Iterator that returns ordered windows,
/// that may be adjacent but that don't overlap further into a `CurveIterator`
#[derive(Debug)]
pub struct JoinAdjacentIterator<I, W, C> {
    /// the Iterator to join into a `CurveIterator`
    /// forced to be fused as otherwise we might
    /// violate a `CurveIterator` invariants
    iter: Fuse<I>,
    /// the peek of the wrapped iterator
    peek: Option<Window<W>>,
    /// The `CurveType` this produces
    curve_type: PhantomData<C>,
}

impl<I: Clone, W, C> Clone for JoinAdjacentIterator<I, W, C> {
    fn clone(&self) -> Self {
        JoinAdjacentIterator {
            iter: self.iter.clone(),
            peek: self.peek.clone(),
            curve_type: PhantomData,
        }
    }
}

impl<I, W, C> JoinAdjacentIterator<I, W, C> {
    /// Create a new `JoinAdjacentIterator`
    /// # Safety
    ///
    /// The Iterator I must return Windows in order that are either don't overlap or at most adjacent
    pub unsafe fn new(iter: I) -> Self
    where
        I: Iterator,
    {
        JoinAdjacentIterator {
            iter: iter.fuse(),
            peek: None,
            curve_type: PhantomData,
        }
    }
}

impl<C, I> CurveIterator<C::WindowKind> for JoinAdjacentIterator<I, C::WindowKind, C>
where
    Self: Debug,
    I: Iterator<Item = Window<C::WindowKind>>,
    C: CurveType,
{
    type CurveKind = C;
}

impl<W, C, I> FusedIterator for JoinAdjacentIterator<I, W, C> where Self: Iterator {}

impl<W, C, I> Iterator for JoinAdjacentIterator<I, W, C>
where
    W: WindowType,
    I: Iterator<Item = Window<W>>,
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
                    // assert correct order
                    assert!(
                        current.start <= peek.start,
                        "The wrapped Iterator violated its invariant of windows being ordered!"
                    );

                    if current.overlaps(peek) {
                        let overlap = Window::new(current.start, peek.end);
                        // assert that windows where adjacent and didn't overlap further as this
                        // as that is assumed by `JoinAdjacentIterator`
                        assert_eq!(overlap.length(), current.length() + peek.length());
                        self.peek = Some(overlap);
                    } else {
                        break Some(current);
                    }
                }
            }
        }
    }
}
