use std::marker::PhantomData;

use crate::curve::curve_types::CurveType;
use crate::curve::Curve;
use crate::iterators::CurveIterator;
use crate::time::TimeUnit;
use crate::window::Window;

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
