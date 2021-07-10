//! Module for the implementation of the `JoinAdjacentIterator`

use core::fmt::Debug;
use core::iter::Fuse;
use core::marker::PhantomData;

use crate::curve::curve_types::CurveType;
use crate::iterators::peek::Peeker;
use crate::iterators::{CurveIterator, CurveIteratorIterator};
use crate::window::Window;

/// `CurveIterator` for turning an Iterator that returns ordered windows,
/// that may be adjacent but that don't overlap further into a `CurveIterator`
#[derive(Debug)]
pub struct JoinAdjacentIterator<I, W, C> {
    /// the Iterator to join into a `CurveIterator`
    /// forced to be fused as otherwise we might
    /// violate a `CurveIterator` invariants
    iter: Peeker<Fuse<I>, Window<W>>,
    /// The `CurveType` this produces
    curve_type: PhantomData<C>,
}

impl<I: Clone, W, C> Clone for JoinAdjacentIterator<I, W, C> {
    fn clone(&self) -> Self {
        JoinAdjacentIterator {
            iter: self.iter.clone(),
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
        I: Iterator<Item = Window<W>>,
    {
        JoinAdjacentIterator {
            iter: Peeker::new(iter.fuse()),
            curve_type: PhantomData,
        }
    }
}

impl<C: CurveIterator>
    JoinAdjacentIterator<
        CurveIteratorIterator<C>,
        <C::CurveKind as CurveType>::WindowKind,
        C::CurveKind,
    >
{
    pub fn new_from_curve(curve_iter: C) -> Self {
        // Safety: Our Invariants are a subset of that of the CurveIterator
        JoinAdjacentIterator {
            iter: Peeker::new(curve_iter.into_iterator().fuse()),
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
            let current = self.iter.next();
            let peek = self.iter.peek_ref();

            match (current, peek) {
                (current, None) => break current,
                (None, Some(_)) => {
                    unreachable!("next is filled first")
                }
                (Some(current), Some(mut peek_ref)) => {
                    let peek = &*peek_ref;

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
                        *peek_ref = overlap;
                    } else {
                        break Some(current);
                    }
                }
            }
        }
    }
}
