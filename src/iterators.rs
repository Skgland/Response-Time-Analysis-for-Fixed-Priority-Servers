//! Module for the Iterator based implementation

use std::fmt::Debug;
use std::iter::{Empty, Fuse, TakeWhile};
use std::marker::PhantomData;

use crate::curve::curve_types::{CurveType, UnspecifiedCurve};
use crate::iterators::curve::FromCurveIterator;
use crate::window::window_types::WindowType;
use crate::window::Window;

pub mod curve;
pub mod join;
pub mod server;
pub mod task;

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
    fn reclassify<O>(self) -> ReclassifyIterator<Self, O>
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

/// `CurveIterator` wrapper to change the Curve type to any compatibly `CurveType`
#[derive(Debug)]
pub struct ReclassifyIterator<I, O> {
    /// the wrapped CurveIterator
    iter: I,
    /// The output curve type and `CurveType`
    phantom: PhantomData<O>,
}

impl<I: Clone, O> Clone for ReclassifyIterator<I, O> {
    fn clone(&self) -> Self {
        ReclassifyIterator {
            iter: self.iter.clone(),
            phantom: PhantomData,
        }
    }
}

impl<I, O> CurveIterator for ReclassifyIterator<I, O>
where
    I: CurveIterator,
    O: CurveType<WindowKind = <I::CurveKind as CurveType>::WindowKind>,
{
    type CurveKind = O;

    fn next_window(&mut self) -> Option<Window<O::WindowKind>> {
        self.iter.next_window()
    }
}

/// Wrap a `CurveIterator` to be a `CurveIterator` and an `Iterator`
#[derive(Debug, Clone)]
pub struct CurveIteratorIterator<I> {
    /// the wrapped `CurveIterator`
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

/// A version of the standard libraries [`Peekable`](std::iter::Peekable) that lets one restore/replace/clear the peek element
#[derive(Debug, Clone)]
pub struct Peeker<I, IT> {
    iter: I,
    peek_window: Option<Option<IT>>,
}

impl<I, IT> Peeker<I, IT>
where
    I: Iterator<Item = IT>,
{
    /// Create a new `Peeker`
    pub fn new(inner: I) -> Self {
        Self {
            iter: inner,
            peek_window: None,
        }
    }

    /// Take a peek at the element that will be returned from the next next call
    pub fn peek(&mut self) -> Option<&IT> {
        self.peek_ref_mut().as_ref()
    }

    /// Take a mutable peek at the element that will be returned from the next next call
    /// Changing the value behing the reference will change the next element
    pub fn peek_mut(&mut self) -> Option<&mut IT> {
        self.peek_ref_mut().as_mut()
    }

    /// Make sure the peek slot is filled and return a mutable reference to the inner option
    fn peek_ref_mut(&mut self) -> &mut Option<IT> {
        let iter = &mut self.iter;
        self.peek_window.get_or_insert_with(|| iter.next())
    }

    /// Clear the window that was peeked at
    ///
    /// # Panics
    /// If there is no window held as peek
    fn clear_peek(&mut self) {
        assert!(self.peek_window.take().flatten().is_some())
    }

    /// Replace an existing peek window with a new one
    ///
    /// # Panics
    /// If there is no window held as peek
    pub fn replace_peek(&mut self, window: IT) -> IT {
        self.peek_window.replace(Some(window)).flatten().unwrap()
    }

    /// Set a peek window if there currently is none
    ///
    /// # Panics
    /// If there is a window held as peek
    pub fn restore_peek(&mut self, window: IT) {
        if let None = self.peek_window.take().flatten() {
            self.peek_window = Some(Some(window))
        } else {
            panic!("Restoring over existing peek window!")
        }
    }
}

impl<I, IT> Iterator for Peeker<I, IT>
where
    I: Iterator<Item = IT>,
{
    type Item = IT;

    fn next(&mut self) -> Option<Self::Item> {
        self.peek_window.take().unwrap_or_else(|| self.iter.next())
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
