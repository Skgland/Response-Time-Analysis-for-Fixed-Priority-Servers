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
pub trait ReclassifyExt<C>: CurveIterator<C>
where
    C: CurveType,
{
    /// reclassify a `CurveIterator`
    fn reclassify(self) -> ReclassifyIterator<Self, C>
    where
        Self: Sized;
}

impl<C, T> ReclassifyExt<C> for T
where
    T: CurveIterator<C>,
    C: CurveType,
{
    fn reclassify(self) -> ReclassifyIterator<Self, C> {
        ReclassifyIterator {
            iter: self,
            phantom: PhantomData,
        }
    }
}

/// `CurveIterator` wrapper to change the Curve type to any compatibly `CurveType`
#[derive(Debug)]
pub struct ReclassifyIterator<I, C> {
    /// the wrapped CurveIterator
    iter: I,
    /// The original lifetime and `CurveType`
    phantom: PhantomData<C>,
}

impl<I: Clone, C> Clone for ReclassifyIterator<I, C> {
    fn clone(&self) -> Self {
        ReclassifyIterator {
            iter: self.iter.clone(),
            phantom: PhantomData,
        }
    }
}

impl<I, O, C> CurveIterator<O> for ReclassifyIterator<I, C>
where
    I: CurveIterator<C>,
    O: CurveType,
    C: CurveType<WindowKind = O::WindowKind>,
{
}

impl<I, C> FusedIterator for ReclassifyIterator<I, C> where I: FusedIterator {}

impl<I, C> Iterator for ReclassifyIterator<I, C>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

/// A trait to allow the Cloning of Boxed dyn `CurveIterator`s
pub trait DynBoxCurveClone<'a, C: CurveType>: CurveIterator<C> {
    /// get a clone inside a box
    fn box_clone(&self) -> Box<dyn CurveIterator<C> + 'a>;
}

impl<'a, C: CurveType, T: CurveIterator<C>> DynBoxCurveClone<'a, C> for T
where
    T: Clone + 'a,
{
    fn box_clone(&self) -> Box<dyn CurveIterator<C> + 'a> {
        Box::new(self.clone())
    }
}

impl<'a, C: CurveType + 'a> Clone for Box<dyn CurveIterator<C> + 'a> {
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
pub trait CurveIterator<C: CurveType>: Iterator<Item = Window<C::WindowKind>> + Debug {}

impl<C> CurveIterator<C> for Empty<Window<C::WindowKind>> where C: CurveType {}

impl<'a, C> CurveIterator<C> for Box<dyn CurveIterator<C> + 'a> where C: CurveType {}

impl<C: CurveType, CI> CurveIterator<C> for Fuse<CI> where CI: CurveIterator<C> {}

impl<C, P, I> CurveIterator<C> for TakeWhile<I, P>
where
    C: CurveType,
    P: for<'r> FnMut(&'r I::Item) -> bool,
    I: CurveIterator<C>,
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

impl<W, C, I> CurveIterator<C> for JoinAdjacentIterator<I, W, C>
where
    Self: Debug,
    W: WindowType,
    I: Iterator<Item = Window<W>>,
    C: CurveType<WindowKind = W>,
{
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
