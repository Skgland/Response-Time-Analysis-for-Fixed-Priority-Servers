//! Module for the implementation of the Curve aggregate operation using iterators

use std::iter::Fuse;

use crate::curve::curve_types::CurveType;
use crate::curve::Aggregate;
use crate::iterators::{CurveIterator, CurveIteratorIterator, ReclassifyIterator};
use crate::server::{AggregatedServerDemand, ConstrainedServerDemand, HigherPriorityServerDemand};
use crate::task::curve_types::{HigherPriorityTaskDemand, TaskDemand};
use crate::window::{Demand, Window};

/// Elements for `AggregationIterator`
#[derive(Debug, Clone)]
pub struct Element<I> {
    /// The Iterator that is being iterated
    curve: Box<Fuse<I>>,
    /// If Some the element that would be the head of `curve`
    /// if we hadn't peeked
    peek: Option<Window<Demand>>,
}

/// Iterator for Aggregating two Curve Iterators
///
/// Aggregate multiple (Demand) Curves as defined in Definition 5. of the paper
///
#[derive(Debug, Clone)]
pub struct AggregationIterator<I> {
    /// The CurveIterators to aggregate
    curves: Vec<Element<CurveIteratorIterator<I>>>,
}

impl<I: CurveIterator> AggregationIterator<I> {
    /// Create a new `AggregationIterator`
    #[must_use]
    pub fn new(curves: Vec<I>) -> Self {
        AggregationIterator {
            curves: curves
                .into_iter()
                .map(|curve| Element {
                    curve: Box::new(curve.fuse_curve()),
                    peek: None,
                })
                .collect(),
        }
    }
}

impl<I> CurveIterator for AggregationIterator<I>
where
    I: CurveIterator,
    I::CurveKind: CurveType<WindowKind = Demand>,
{
    type CurveKind = I::CurveKind;

    fn next_window(&mut self) -> Option<Window<Demand>> {
        #![allow(clippy::option_if_let_else)] // false positive

        // fill all peek slots
        for element in &mut self.curves {
            element.peek = element.peek.take().or_else(|| element.curve.next_window());
        }

        // find curve with earliest peek
        let result = self
            .curves
            .iter_mut()
            .enumerate()
            .filter_map(|(index, element)| {
                element.peek = element.peek.take().or_else(|| element.curve.next_window());
                if let Some(peek) = element.peek.as_mut() {
                    Some((index, peek.start, &mut element.peek))
                } else {
                    None
                }
            })
            .min_by_key(|(_, start, _)| *start)
            .and_then(|(index, _, peek)| peek.take().map(|peek| (index, peek)));

        // take peek
        if let Some((original_index, first_peek)) = result {
            let mut overlap = first_peek;

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
                    if let Some(peek) = element.peek.take().or_else(|| element.curve.next_window())
                    {
                        if let Some(overlap_window) = overlap.aggregate(&peek) {
                            // update last aggregated index
                            aggregate_index = index;
                            // replace overlap with new overlap_window
                            overlap = overlap_window;
                            continue;
                        } else {
                            // restore peek
                            element.peek = Some(peek);
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

impl AggregateInto<HigherPriorityServerDemand> for ConstrainedServerDemand {}

impl AggregateInto<AggregatedServerDemand> for TaskDemand {}
impl AggregateInto<HigherPriorityTaskDemand> for TaskDemand {}

impl<AI, O> Aggregate<AI>
    for ReclassifyIterator<AggregationIterator<AI>, <AI as CurveIterator>::CurveKind, O>
where
    <AI as CurveIterator>::CurveKind: AggregateInto<O>,
    AI: CurveIterator,
    AI::CurveKind: CurveType<WindowKind = Demand>,
{
    fn aggregate<I>(iter: I) -> Self
    where
        I: Iterator<Item = AI>,
    {
        AggregationIterator::new(iter.collect()).reclassify()
    }
}
