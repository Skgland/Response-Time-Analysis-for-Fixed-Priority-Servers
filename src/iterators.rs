//! Module for the Iterator based implementation

use std::fmt::Debug;
use std::iter::{Empty, Fuse, TakeWhile};

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

impl<I, O, C> CurveIterator for ReclassifyIterator<I, C, O>
where
    I: CurveIterator<CurveKind = C>,
    O: CurveType<WindowKind = C::WindowKind>,
    C: CurveType,
{
    type CurveKind = O;

    fn next_window(&mut self) -> Option<Window<O::WindowKind>> {
        self.iter.next_window()
    }
}

/// Trait representing an Iterator that has the guarantees of a curve:
/// 1. Windows ordered by start
/// 2. Windows non-overlapping
/// 3. Windows non-empty
///
/// Or in other words all finite prefixes of the Iterator are a valid Curves
///
pub trait CurveIterator: Debug {
    /// The type of the curve being iterated
    type CurveKind: CurveType;

    /// calculate and returns the next window of the curve iterator
    /// advancing the iterator in the process
    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>>;

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

    fn take_while_curve<F>(self, fun: F) -> TakeWhile<CurveIteratorIterator<Self>, F>
    where
        Self: Sized,
        F: for<'a> FnMut(&'a Window<<Self::CurveKind as CurveType>::WindowKind>) -> bool,
    {
        self.into_iterator().take_while(fun)
    }

    fn fuse_curve(self) -> Fuse<CurveIteratorIterator<Self>>
    where
        Self: Sized,
    {
        self.into_iterator().fuse()
    }

    fn into_iterator(self) -> CurveIteratorIterator<Self>
    where
        Self: Sized,
    {
        CurveIteratorIterator { iter: self }
    }
}

#[derive(Debug, Clone)]
pub struct CurveIteratorIterator<I> {
    iter: I,
}

impl<I> CurveIterator for CurveIteratorIterator<I>
where
    I: CurveIterator,
{
    type CurveKind = I::CurveKind;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.iter.next_window()
    }
}

impl<I> Iterator for CurveIteratorIterator<I>
where
    I: CurveIterator,
{
    type Item = Window<<I::CurveKind as CurveType>::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next_window()
    }
}

impl<W> CurveIterator for Empty<Window<W>>
where
    W: WindowType,
{
    type CurveKind = UnspecifiedCurve<W>;

    fn next_window(&mut self) -> Option<Window<W>> {
        None
    }
}

impl<W: WindowType, CI> CurveIterator for Fuse<CI>
where
    CI: CurveIterator + Iterator<Item = Window<W>>,
    CI::CurveKind: CurveType<WindowKind = W>,
{
    type CurveKind = CI::CurveKind;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.next()
    }
}

impl<W, P, CI> CurveIterator for TakeWhile<CI, P>
where
    P: for<'r> FnMut(&'r Window<W>) -> bool,
    CI: CurveIterator + Iterator<Item = Window<W>>,
    CI::CurveKind: CurveType<WindowKind = W>,
{
    type CurveKind = CI::CurveKind;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.next()
    }
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

impl<C, I> CurveIterator for JoinAdjacentIterator<I, C::WindowKind, C>
where
    Self: Debug,
    C: CurveType,
    I: Iterator<Item = Window<C::WindowKind>>,
{
    type CurveKind = C;

    fn next_window(&mut self) -> Option<Window<C::WindowKind>> {
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
