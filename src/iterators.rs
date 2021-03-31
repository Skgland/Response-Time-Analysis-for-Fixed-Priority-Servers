//! Module for the Iterator based implementation

use std::fmt::Debug;
use std::iter::{Empty, Fuse, FusedIterator, TakeWhile};

use crate::curve::curve_types::CurveType;
use crate::window::window_types::WindowType;
use crate::window::Window;
use std::marker::PhantomData;

pub mod curve;
pub mod server;
pub mod task;

/// Extension trait for reclassifying a `CurveIterator`
/// to any compatible `CurveType`
pub trait ReclassifyExt<'a, C>: CurveIterator<'a, C>
where
    C: CurveType,
{
    /// reclassify a `CurveIterator`
    fn reclassify(self) -> ReclassifyIterator<'a, Self, C>
    where
        Self: Sized;
}

impl<'a, C, T> ReclassifyExt<'a, C> for T
where
    T: CurveIterator<'a, C>,
    C: CurveType,
{
    fn reclassify(self) -> ReclassifyIterator<'a, Self, C> {
        ReclassifyIterator {
            iter: self,
            phantom: PhantomData,
        }
    }
}

/// `CurveIterator` wrapper to change the Curve type to any compatibly `CurveType`
#[derive(Debug)]
pub struct ReclassifyIterator<'a, I, C> {
    /// the wrapped CurveIterator
    iter: I,
    /// The original lifetime and `CurveType`
    phantom: PhantomData<(&'a (), C)>,
}

impl<'a, I: Clone, C> Clone for ReclassifyIterator<'a, I, C> {
    fn clone(&self) -> Self {
        ReclassifyIterator {
            iter: self.iter.clone(),
            phantom: PhantomData,
        }
    }
}

impl<'a, I, O, C> CurveIterator<'a, O> for ReclassifyIterator<'a, I, C>
where
    I: CurveIterator<'a, C> + 'a,
    O: CurveType + 'a,
    C: CurveType<WindowKind = O::WindowKind> + 'a,
{
}

impl<'a, I, C> FusedIterator for ReclassifyIterator<'a, I, C> where I: FusedIterator {}

impl<'a, I, C> Iterator for ReclassifyIterator<'a, I, C>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

/// A trait to allow the Cloning of Boxed dyn `CurveIterator`s
pub trait DynBoxCurveClone<'a, C: CurveType>: CurveIterator<'a, C> {
    fn box_clone(&self) -> Box<dyn CurveIterator<'a, C>>;
}

impl<'a, C: CurveType, T: CurveIterator<'a, C>> DynBoxCurveClone<'a, C> for T
where
    T: Clone,
{
    fn box_clone(&self) -> Box<dyn CurveIterator<'a, C>> {
        Box::new(self.clone())
    }
}

impl<'a, C: CurveType + 'a> Clone for Box<dyn CurveIterator<'a, C>> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// Trait representing an Iterator that has the guarantees of a curve:
/// 1. Windows ordered by start
/// 2. Windows non-overlapping
/// 3. Windows non-empty
///
/// Or in other words all finite prefixes of the Iterator are a valid Curves
///
pub trait CurveIterator<'a, C: CurveType>:
    Iterator<Item = Window<C::WindowKind>> + Debug + 'a
{
}

impl<'a, C> CurveIterator<'a, C> for Empty<Window<C::WindowKind>> where C: CurveType + 'a {}

impl<'a, C> CurveIterator<'a, C> for Box<dyn CurveIterator<'a, C>> where C: CurveType + 'a {}

impl<'a, C: CurveType, CI> CurveIterator<'a, C> for Fuse<CI> where CI: CurveIterator<'a, C> {}

impl<'a, C, P, I> CurveIterator<'a, C> for TakeWhile<I, P>
where
    C: CurveType,
    P: for<'r> FnMut(&'r I::Item) -> bool + 'a,
    I: CurveIterator<'a, C>,
{
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

impl<'a, W, C, I> CurveIterator<'a, C> for JoinAdjacentIterator<I, W, C>
where
    Self: Debug,
    W: WindowType + 'a,
    I: Iterator<Item = Window<W>> + 'a,
    C: CurveType<WindowKind = W> + 'a,
{
}

impl<'a, W, C, I> FusedIterator for JoinAdjacentIterator<I, W, C> where Self: Iterator {}

impl<'a, W, C, I> Iterator for JoinAdjacentIterator<I, W, C>
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
                    assert!(current.start <= peek.start);
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
