use std::iter::{Empty, Fuse, FusedIterator};

use crate::curve::curve_types::CurveType;
use crate::curve::Aggregate;
use crate::iterators::CurveIterator;
use crate::window::{Demand, Window};

/// Iterator for Aggregating two Curve Iterators
///
/// Aggregate two (Demand) Curves as defined in Definition 5. of the paper
///
#[derive(Debug)]
pub struct AggregatedDemandIterator<C: CurveType<WindowKind = Demand>, I1, I2> {
    /// The first CurveIterator to aggregate
    curve1: Fuse<I1>,
    /// the peek of the first CurveIterator or
    /// if only one iterator is remaining the peek of the remaining iterator
    peek1: Option<Window<C::WindowKind>>,
    /// The second CurveIterator to aggregate
    curve2: Fuse<I2>,
    /// the peek of the second CurveIterator,
    /// unless only one iterator is remaining
    peek2: Option<Window<C::WindowKind>>,
    /// The peek overlap of both iterators
    overlap: Option<Window<C::WindowKind>>,
}

impl<C: CurveType<WindowKind = Demand>, I1: Clone, I2: Clone> Clone
    for AggregatedDemandIterator<C, I1, I2>
{
    fn clone(&self) -> Self {
        AggregatedDemandIterator {
            curve1: self.curve1.clone(),
            peek1: self.peek1.clone(),
            curve2: self.curve2.clone(),
            peek2: self.peek2.clone(),
            overlap: self.overlap.clone(),
        }
    }
}

impl<I1, I2> AggregatedDemandIterator<I1::CurveKind, I1, I2>
where
    I1: CurveIterator<Demand>,
    I2: CurveIterator<Demand, CurveKind = I1::CurveKind>,
{
    /// Create aggregated `CurveIterator` for two `CurveIterator`s
    #[must_use]
    pub fn new(curve1: I1, curve2: I2) -> AggregatedDemandIterator<I1::CurveKind, I1, I2> {
        AggregatedDemandIterator {
            curve1: curve1.fuse(),
            curve2: curve2.fuse(),
            peek1: None,
            peek2: None,
            overlap: None,
        }
    }
}

impl<I1, I2> CurveIterator<Demand> for AggregatedDemandIterator<I1::CurveKind, I1, I2>
where
    I1: CurveIterator<Demand>,
    I2: CurveIterator<Demand, CurveKind = I1::CurveKind>,
{
    type CurveKind = I1::CurveKind;
}

impl<I1, I2> Iterator for AggregatedDemandIterator<I1::CurveKind, I1, I2>
where
    I1: CurveIterator<Demand>,
    I2: CurveIterator<Demand, CurveKind = I1::CurveKind>,
{
    type Item = Window<<I1::CurveKind as CurveType>::WindowKind>;

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

impl<C, I1, I2> FusedIterator for AggregatedDemandIterator<C, I1, I2>
where
    Self: Iterator,
    C: CurveType<WindowKind = Demand>,
    I1: FusedIterator,
    I2: FusedIterator,
{
}

/// Type alias to make it easier to refer to the Self type of the below
/// impl of Aggregate
pub type RecursiveAggregatedDemandIterator<'a, C> = AggregatedDemandIterator<
    C,
    Box<dyn CurveIterator<Demand, CurveKind = C> + 'a>,
    Box<dyn CurveIterator<Demand, CurveKind = C> + 'a>,
>;

impl<'a, C, CI> Aggregate<CI> for RecursiveAggregatedDemandIterator<'a, C>
where
    C: CurveType<WindowKind = Demand> + 'a,
    CI: CurveIterator<Demand, CurveKind = C> + 'a,
{
    fn aggregate<I>(iter: I) -> Self
    where
        I: Iterator<Item = CI>,
    {
        iter.fold(
            AggregatedDemandIterator::new(
                Box::new(Empty::default().reclassify()),
                Box::new(Empty::default().reclassify()),
            ),
            |acc, window| AggregatedDemandIterator::new(Box::new(acc), Box::new(window)),
        )
    }
}
