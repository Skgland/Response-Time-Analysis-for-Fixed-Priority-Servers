//! Iterators for basic Curve Operations
//!
//! such as `IntoIter`, split, aggregate, TODO delta
use std::iter::{Empty, FusedIterator};
use std::marker::PhantomData;

use crate::curve::curve_types::CurveType;
use crate::curve::{Aggregate, Curve};
use crate::iterators::CurveIterator;
use crate::time::TimeUnit;
use crate::window::{Demand, Window};
use std::collections::VecDeque;

/// Iterator for Aggregating two Curve Iterators
#[derive(Debug)]
pub struct AggregatedDemandIterator<
    'a,
    C: CurveType<WindowKind = Demand>,
    I1: CurveIterator<'a, C>,
    I2: CurveIterator<'a, C>,
> {
    /// The first CurveIterator to aggregate
    curve1: I1,
    /// the peek of the first CurveIterator or
    /// if only one iterator is remaining the peek of the remaining iterator
    peek1: Option<Window<C::WindowKind>>,
    /// The second CurveIterator to aggregate
    curve2: I2,
    /// the peek of the second CurveIterator,
    /// unless only one iterator is remaining
    peek2: Option<Window<C::WindowKind>>,
    /// The peek overlap of both iterators
    overlap: Option<Window<C::WindowKind>>,

    /// The minimum lifetime of the contained CurveIterators
    /// and as such the maximum lifetime of Self
    lifetime: PhantomData<&'a ()>,
}

impl<
        'a,
        C: CurveType<WindowKind = Demand> + 'a,
        I1: CurveIterator<'a, C>,
        I2: CurveIterator<'a, C>,
    > AggregatedDemandIterator<'a, C, I1, I2>
{
    /// Create aggregated `CurveIterator` for two `CurveIterator`s
    #[must_use]
    pub fn new(curve1: I1, curve2: I2) -> AggregatedDemandIterator<'a, C, I1, I2> {
        AggregatedDemandIterator {
            curve1,
            curve2,
            peek1: None,
            peek2: None,
            overlap: None,
            lifetime: PhantomData,
        }
    }
}

impl<
        'a,
        C: CurveType<WindowKind = Demand> + 'a,
        I1: CurveIterator<'a, C>,
        I2: CurveIterator<'a, C>,
    > CurveIterator<'a, C> for AggregatedDemandIterator<'a, C, I1, I2>
{
}

impl<'a, C: CurveType<WindowKind = Demand>, I1: CurveIterator<'a, C>, I2: CurveIterator<'a, C>>
    Iterator for AggregatedDemandIterator<'a, C, I1, I2>
{
    type Item = Window<C::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let peek1 = self.peek1.take().or_else(|| self.curve1.next());
            let peek2 = self.peek2.take().or_else(|| self.curve2.next());

            if let Some(overlap_window) = self.overlap.take() {
                match (peek1, peek2) {
                    (None, None) => return Some(overlap_window),
                    (Some(peek1), Some(peek2)) => {
                        if let Some(overlap) = peek1.aggregate(&overlap_window) {
                            // aggregate overlap and peek1, remember peek2 and reiterate
                            self.peek2 = Some(peek2);
                            self.overlap = Some(overlap);
                        } else if let Some(overlap) = peek2.aggregate(&overlap_window) {
                            // aggregate overlap and peek2, remember peek1 and reiterate
                            self.peek1 = Some(peek1);
                            self.overlap = Some(overlap);
                        } else {
                            // neither peek1 nor peek2 overlaps with window
                            // remember peek1 and peek2 , return overlap
                            self.peek1 = Some(peek1);
                            self.peek2 = Some(peek2);
                            return Some(overlap_window);
                        }
                    }
                    (Some(peek), None) | (None, Some(peek)) => {
                        if let Some(overlap) = peek.aggregate(&overlap_window) {
                            // aggregate peek and overlap then reiterate
                            self.overlap = Some(overlap);
                        } else {
                            // peek and overlap don't overlap, remember peek and return overlap
                            // as only curve1 or curve2 remains it doesn't matter whether we
                            // remember peek as peek1 or peek2
                            self.peek1 = Some(peek);
                            return Some(overlap_window);
                        }
                    }
                }
            } else {
                match (peek1, peek2) {
                    (None, None) => return None,
                    (Some(peek1), Some(peek2)) => {
                        if let Some(overlap) = peek1.aggregate(&peek2) {
                            // need to reiterate as more overlap may exist now
                            self.overlap = Some(overlap)
                        } else {
                            // peek1 and peek2 don't overlap we can return the earlier
                            // and need to remember the later
                            if peek1.end < peek2.start {
                                self.peek2 = Some(peek2);
                                return Some(peek1);
                            } else if peek2.end < peek1.start {
                                self.peek1 = Some(peek1);
                                return Some(peek2);
                            } else {
                                unreachable!("Overlap already handled earlier")
                            }
                        }
                    }
                    (Some(peek), None) | (None, Some(peek)) => return Some(peek),
                }
            }
        }
    }
}

impl<'a, C: CurveType<WindowKind = Demand>, I1: CurveIterator<'a, C>, I2: CurveIterator<'a, C>>
    FusedIterator for AggregatedDemandIterator<'a, C, I1, I2>
{
}

pub type RecursiveAggregatedDemandIterator<'a, C> = AggregatedDemandIterator<
    'a,
    C,
    Box<dyn (CurveIterator<'a, C>)>,
    Box<dyn (CurveIterator<'a, C>)>,
>;

impl<'a, C: CurveType<WindowKind = Demand> + 'a, CI: CurveIterator<'a, C> + 'a> Aggregate<'a, CI>
    for RecursiveAggregatedDemandIterator<'a, C>
{
    fn aggregate<I>(iter: I) -> Self
    where
        I: Iterator<Item = CI>,
        I::Item: 'a,
    {
        iter.fold(
            AggregatedDemandIterator::new(Box::new(Empty::default()), Box::new(Empty::default())),
            |acc, window| AggregatedDemandIterator::new(Box::new(acc), Box::new(window)),
        )
    }
}

/// Curve Iterator for splitting a Curve in fixed Intervalls
///
/// Will yield the Groups in order
#[derive(Debug)]
pub struct CurveSplitIterator<'a, C: CurveType, CI: CurveIterator<'a, C>> {
    /// The remaining Curve to be split
    iter: CI,
    /// The remaining tail from the head of the last split
    tail: Option<Window<C::WindowKind>>,
    /// The interval at which to perform the splits
    interval: TimeUnit,
    /// The lifetime of the `CurveIterator`
    lifetime: PhantomData<&'a ()>,
}

impl<'a, C: CurveType, CI: CurveIterator<'a, C>> CurveSplitIterator<'a, C, CI> {
    /// Split the `CurveIterator` at every interval
    pub fn new(iter: CI, interval: TimeUnit) -> Self {
        CurveSplitIterator {
            iter,
            tail: None,
            interval,
            lifetime: PhantomData,
        }
    }
}

impl<'a, C: CurveType, CI: CurveIterator<'a, C>> Iterator for CurveSplitIterator<'a, C, CI> {
    type Item = (usize, Curve<C>);

    fn next(&mut self) -> Option<Self::Item> {
        let first = self.tail.take().or_else(|| self.iter.next());

        if let Some(first) = first {
            let mut windows = vec![];

            let k = first.start / self.interval;
            for window in std::iter::once(first).chain(&mut self.iter) {
                if k != window.start / self.interval {
                    // complete window does not belong to this group
                    self.tail = Some(window);
                    unsafe { return Some((k, Curve::from_windows_unchecked(windows))) }
                } else if window.end <= (k + 1) * self.interval {
                    // window belongs completely to the current group
                    windows.push(window);
                } else {
                    // window belongs only partially to this group
                    let init = Window::new(window.start, (k + 1) * self.interval);
                    let tail = Window::new((k + 1) * self.interval, window.end);

                    // add initial part belonging to current group to to current group
                    windows.push(init);
                    // remember remaining tail for next group
                    self.tail = Some(tail);

                    // group is full return group
                    unsafe {
                        return Some((k, Curve::from_windows_unchecked(windows)));
                    }
                }
            }
            Some((k, unsafe { Curve::from_windows_unchecked(windows) }))
        } else {
            None
        }
    }
}

/// `CurveIterator` for iterating a [`Curve`]
#[derive(Debug)]
pub struct CurveIter<C: CurveType> {
    /// The remaining windows of the Curve
    curve: VecDeque<Window<C::WindowKind>>,
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

impl<'a, C: CurveType + 'a> CurveIterator<'a, C> for CurveIter<C> {}

impl<C: CurveType> Iterator for CurveIter<C> {
    type Item = Window<C::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        self.curve.pop_front()
    }
}

impl<C: CurveType> FusedIterator for CurveIter<C> {}
