//! Module for the implementation of the Curve split operation using iterators

use alloc::boxed::Box;
use core::iter::FusedIterator;

use crate::curve::curve_types::CurveType;
use crate::iterators::CurveIterator;
use crate::time::TimeUnit;
use crate::window::window_types::WindowType;
use crate::window::Window;
use crate::window::WindowEnd;

/// Curve Iterator for splitting a Curve in fixed Intervals
///
/// Split the curve on every interval boundary as defined in Definition 8. of the paper
/// When the last window of the input `CurveIterator` is an infinite window
/// that window will be spilt at most once, and in that case the last window returned
/// will start on a group boundary and be infinite
///
/// Will yield the windows of the groups in order
///
/// Not a `CurveIterator` as it can produce adjacent windows
///
#[derive(Debug, Clone)]
pub struct CurveSplitIterator<W, CI> {
    /// The remaining Curve to be split
    iter: Box<CI>,
    /// The remaining tail from the head of the last split
    tail: Option<Window<W>>,
    /// The interval at which to perform the splits
    interval: TimeUnit,
}

impl<W: WindowType, CI> CurveSplitIterator<W, CI>
where
    CI: CurveIterator,
{
    /// Split the `CurveIterator` at every interval
    pub fn new(iter: CI, interval: TimeUnit) -> Self {
        CurveSplitIterator {
            iter: Box::new(iter),
            tail: None,
            interval,
        }
    }
}

impl<W, CI> FusedIterator for CurveSplitIterator<W, CI>
where
    Self: Iterator,
    CI: FusedIterator,
{
}

impl<W: WindowType, CI> Iterator for CurveSplitIterator<W, CI>
where
    CI: CurveIterator,
    CI::CurveKind: CurveType<WindowKind = W>,
{
    type Item = Window<<CI::CurveKind as CurveType>::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        let first = self.tail.take().or_else(|| self.iter.next_window());

        first.map(|first| {
            let k = first.start / self.interval;
            if first.end <= (k + 1) * self.interval
                || first.start == k * self.interval && first.end == WindowEnd::Infinite
            {
                // window belongs completely to a group
                // or window starts on a group boundary and is infinite return as is
                first
            } else {
                // window belongs only partially to this group
                let init = Window::new(first.start, (k + 1) * self.interval);
                let tail = Window::new((k + 1) * self.interval, first.end);

                // remember remaining tail for next group
                self.tail = Some(tail);

                init
            }
        })
    }
}
