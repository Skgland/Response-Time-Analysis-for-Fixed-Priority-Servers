//! Module for the implementation of the Curve aggregate operation using iterators

use alloc::vec::Vec;
use core::iter::Fuse;

use crate::curve::curve_types::CurveType;
use crate::curve::Aggregate;
use crate::iterators::peek::Peeker;
use crate::iterators::{CurveIterator, CurveIteratorIterator, ReclassifyIterator};
use crate::server::{
    ActualServerExecution, AggregatedServerDemand, ConstrainedServerDemand,
    HigherPriorityServerDemand, HigherPriorityServerExecution,
};
use crate::task::curve_types::{HigherPriorityTaskDemand, TaskDemand};
use crate::window::Window;
use core::fmt::Debug;

/// Iterator for Aggregating two Curve Iterators
///
/// Aggregate multiple (Demand) Curves as defined in Definition 5. of the paper
///
#[derive(Debug, Clone)]
pub struct AggregationIterator<I, W> {
    /// The CurveIterators to aggregate
    curves: Vec<Peeker<Fuse<CurveIteratorIterator<I>>, Window<W>>>,
}

impl<I, W> AggregationIterator<I, W>
where
    I: CurveIterator,
    I::CurveKind: CurveType<WindowKind = W>,
{
    /// Create a new `AggregationIterator`
    #[must_use]
    pub fn new(curves: Vec<I>) -> Self {
        AggregationIterator {
            curves: curves
                .into_iter()
                .map(|curve| Peeker::new(curve.fuse_curve()))
                .collect(),
        }
    }
}

impl<I, W> CurveIterator for AggregationIterator<I, W>
where
    I: CurveIterator,
    I::CurveKind: CurveType<WindowKind = W>,
    W: Debug,
{
    type CurveKind = I::CurveKind;

    fn next_window(&mut self) -> Option<Window<W>> {
        // find curve with earliest peek
        let result = self
            .curves
            .iter_mut()
            .enumerate()
            .filter_map(|(index, element)| {
                element
                    .peek_ref()
                    .map(|some_ref| (index, some_ref.start, some_ref))
            })
            .min_by_key(|(_, start, _)| *start)
            .map(|(index, _, some_ref)| (index, some_ref.take()));

        // take peek
        if let Some((original_index, first_peek)) = result {
            let mut overlap: Window<_> = first_peek;

            // the index that was last aggregated into overlap
            // if we reach it again without aggregating more we are done
            let mut aggregate_index = original_index;

            'outer: loop {
                let (tail, head) = self.curves.split_at_mut(original_index + 1);

                // start after index and cycle through all elements
                // until we reach and process an index again without aggregating since our last visit
                let iter = head
                    .iter_mut()
                    .enumerate()
                    .map(move |(i, element)| (i + original_index + 1, element))
                    .chain(tail.iter_mut().enumerate());

                for (index, element) in iter {
                    if let Some(peek) = element.peek_ref() {
                        if let Some(overlap_window) = overlap
                            .aggregate(&*peek)
                            .filter(|_| !overlap.adjacent(&*peek))
                        {
                            // update last aggregated index
                            aggregate_index = index;
                            // replace overlap with new overlap_window
                            overlap = overlap_window;
                            // clear the peek as we have used it
                            peek.take();
                            continue;
                        }
                    }

                    if aggregate_index == index {
                        // reached this again without aggregating
                        break 'outer Some(overlap);
                    }
                }
            }
        } else {
            None
        }
    }
}

/// Trait to mark which curve types may be aggregated into which other curve types
pub trait AggregateInto<Result = Self>: CurveType {}

impl<T: CurveType> AggregateInto for T {}

impl AggregateInto<HigherPriorityServerExecution> for ActualServerExecution {}
impl AggregateInto<HigherPriorityServerDemand> for ConstrainedServerDemand {}

impl AggregateInto<AggregatedServerDemand> for TaskDemand {}
impl AggregateInto<HigherPriorityTaskDemand> for TaskDemand {}

impl<AI, O, W> Aggregate<AI> for ReclassifyIterator<AggregationIterator<AI, W>, O>
where
    <AI as CurveIterator>::CurveKind: AggregateInto<O>,
    AI: CurveIterator,
    AI::CurveKind: CurveType<WindowKind = W>,
    W: Debug,
{
    fn aggregate<I>(iter: I) -> Self
    where
        I: Iterator<Item = AI>,
    {
        AggregationIterator::new(iter.collect()).reclassify()
    }
}
