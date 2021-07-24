//! Module for the implementation of the Curve delta operation using iterators

use core::fmt::Debug;
use core::iter::{FilterMap, FusedIterator};
use core::marker::PhantomData;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::curve::curve_types::CurveType;
use crate::iterators::curve::IterCurveWrapper;
use crate::iterators::peek::Peeker;
use crate::iterators::{CurveIterator, CurveIteratorIterator};
use crate::time::TimeUnit;
use crate::window::window_types::WindowType;
use crate::window::WindowEnd;
use crate::window::{Overlap, Window, WindowDeltaResult};

/// Item type of the `CurveDeltaIterator`
#[derive(Debug)]
pub enum Delta<D, S, DI, SI> {
    /// Indicate a Window of remaining supply
    RemainingSupply(Window<S>),
    /// Remaining Supply once Demand ran out
    EndSupply(Box<SI>),
    /// Indicate a Window of overlapping supply and demand
    Overlap(Window<Overlap<S, D>>),
    /// Indicate a Window of remaining demand
    RemainingDemand(Window<D>),
    /// Remaining Demand once Supply ran out
    EndDemand(Peeker<CurveIteratorIterator<Box<DI>>, Window<D>>),
}

impl<D, S, DI, SI> Delta<D, S, DI, SI> {
    /// turn delta into some overlap or none otherwise
    #[must_use]
    pub fn overlap(self) -> Option<Window<Overlap<S, D>>> {
        #![allow(clippy::missing_const_for_fn)] // false positive
        match self {
            Delta::RemainingSupply(_)
            | Delta::EndSupply(_)
            | Delta::RemainingDemand(_)
            | Delta::EndDemand(_) => None,
            Delta::Overlap(overlap) => Some(overlap),
        }
    }
}

/// Iterator Adapter for filtering a `CurveDeltaIterator` into only the remaining supply
///
/// See [`CurveDeltaIterator::remaining_supply`]
#[derive(Debug)]
pub struct RemainingSupplyIterator<S, D, SI, DI> {
    /// The CurveDeltaIterator from which to collect the supply
    delta: Option<CurveDeltaIterator<D, S, DI, SI>>,
    /// The remaining end_supply to return
    end_supply: Option<Box<SI>>,
}

impl<S, D, SI: Clone, DI: Clone> Clone for RemainingSupplyIterator<S, D, SI, DI> {
    fn clone(&self) -> Self {
        RemainingSupplyIterator {
            delta: self.delta.clone(),
            end_supply: self.end_supply.clone(),
        }
    }
}

impl<SI, DI> CurveIterator
    for RemainingSupplyIterator<
        <SI::CurveKind as CurveType>::WindowKind,
        <DI::CurveKind as CurveType>::WindowKind,
        SI,
        DI,
    >
where
    Self: Debug,
    SI: CurveIterator,
    DI: CurveIterator,
{
    type CurveKind = SI::CurveKind;

    fn next_window(&mut self) -> Option<Window<<SI::CurveKind as CurveType>::WindowKind>> {
        'outer: loop {
            if let Some(end_supply) = self.end_supply.as_mut() {
                if let Some(supply) = end_supply.next_window() {
                    return Some(supply);
                } else {
                    self.end_supply = None;
                }
            }
            if let Some(delta_iter) = self.delta.as_mut() {
                loop {
                    if let Some(delta) = delta_iter.next() {
                        match delta {
                            Delta::Overlap(_) | Delta::EndDemand(_) | Delta::RemainingDemand(_) => {
                                continue
                            }
                            Delta::RemainingSupply(supply) => return Some(supply),
                            Delta::EndSupply(supply) => {
                                self.end_supply = Some(supply);
                                continue 'outer;
                            }
                        }
                    } else {
                        self.delta = None;
                        break;
                    }
                }
            }
            return None;
        }
    }
}

/// Calculate the Inverse of a Curve
/// directly rather than calculating the delta between total and the curve
#[derive(Debug)]
pub struct InverseCurveIterator<I, C> {
    /// The iterator to invert
    iter: I,
    /// The end of the last window
    previous_end: WindowEnd,
    /// The type of the Produced Curves
    curve_type: PhantomData<C>,
}

impl<I, C> InverseCurveIterator<I, C> {
    /// Create a new `InverseCurveIterator`
    #[must_use]
    pub const fn new(iter: I) -> Self {
        InverseCurveIterator {
            iter,
            previous_end: WindowEnd::Finite(TimeUnit::ZERO),
            curve_type: PhantomData,
        }
    }
}

impl<I: Clone, C> Clone for InverseCurveIterator<I, C> {
    fn clone(&self) -> Self {
        InverseCurveIterator {
            iter: self.iter.clone(),
            previous_end: self.previous_end,
            curve_type: PhantomData,
        }
    }
}

impl<I: CurveIterator, C: CurveType> CurveIterator for InverseCurveIterator<I, C> {
    type CurveKind = C;

    fn next_window(&mut self) -> Option<Window<C::WindowKind>> {
        if let WindowEnd::Finite(mut previous_end) = self.previous_end {
            while let Some(window) = self.iter.next_window() {
                if self.previous_end < window.start {
                    let result = Window::new(previous_end, window.start);
                    self.previous_end = window.end;
                    return Some(result);
                } else if self.previous_end == window.start {
                    self.previous_end = window.end;
                    match self.previous_end {
                        WindowEnd::Finite(end) => previous_end = end,
                        WindowEnd::Infinite => return None,
                    }
                } else {
                    panic!("Overlapping Windows in CurveIterator 'self.iter'")
                }
            }

            let result = Window::new(previous_end, WindowEnd::Infinite);
            self.previous_end = WindowEnd::Infinite;
            Some(result)
        } else {
            None
        }
    }
}

/// An Iterator for calculating the Delta between
/// the Supply and the Demand based on Definition 7. from the paper
///
/// Returns interleaved with no fixed pattern the remaining supply, remaining demand and the overlap
///
#[derive(Debug)]
pub struct CurveDeltaIterator<DW, SW, DI, SI> {
    /// remaining demand curve
    demand: Option<Peeker<CurveIteratorIterator<Box<DI>>, Window<DW>>>,
    /// remaining supply curve
    supply: Option<Box<SI>>,
    /// peek of the supply curve
    remaining_supply: Vec<Window<SW>>,
}

impl<DW, SW, DI: Clone, SI: Clone> Clone for CurveDeltaIterator<DW, SW, DI, SI> {
    fn clone(&self) -> Self {
        CurveDeltaIterator {
            demand: self.demand.clone(),
            supply: self.supply.clone(),
            remaining_supply: self.remaining_supply.clone(),
        }
    }
}

impl<S, D, SI, DI> CurveDeltaIterator<D, S, DI, SI> {
    /// Turn the `CurveDeltaIterator` into a `CurveIterator` that returns only the Remaining Supply Windows
    #[must_use]
    pub const fn remaining_supply(self) -> RemainingSupplyIterator<S, D, SI, DI> {
        RemainingSupplyIterator {
            delta: Some(self),
            end_supply: None,
        }
    }

    /// Turn the `CurveDeltaIterator` into a `CurveIterator` that returns only the Overlap Windows
    #[must_use]
    pub fn overlap<C>(self) -> OverlapIterator<DI, SI, D, S, C>
    where
        Self: Iterator<Item = Delta<D, S, DI, SI>> + Debug,
        C: CurveType<WindowKind = Overlap<S, D>>,
    {
        let fun: fn(_) -> _ = Delta::overlap;
        let inner = self.filter_map(fun);
        let wrapped = unsafe {
            // Safety
            // self is an iterator of three interleaved curves, but using filter_map
            // we filter only one out
            // so the remaining iterator is a valid curve
            IterCurveWrapper::new(inner)
        };
        OverlapIterator(wrapped)
    }
}

/// Iterator Adapter for filtering a `CurveDeltaIterator` into only the overlap
///
/// See [`CurveDeltaIterator::overlap`]
#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
pub struct OverlapIterator<DI, SI, DW, SW, C>(
    IterCurveWrapper<
        FilterMap<
            CurveDeltaIterator<DW, SW, DI, SI>,
            fn(Delta<DW, SW, DI, SI>) -> Option<Window<Overlap<SW, DW>>>,
        >,
        C,
    >,
);

impl<DI, SI, DW, SW, C> CurveIterator for OverlapIterator<DI, SI, DW, SW, C>
where
    C: CurveType<WindowKind = Overlap<SW, DW>>,
    CurveDeltaIterator<DW, SW, DI, SI>: Iterator<Item = Delta<DW, SW, DI, SI>> + Debug,
    Self: Debug,
{
    type CurveKind = C;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.0.next_window()
    }
}

impl<DI: CurveIterator, SI: CurveIterator>
    CurveDeltaIterator<
        <DI::CurveKind as CurveType>::WindowKind,
        <SI::CurveKind as CurveType>::WindowKind,
        DI,
        SI,
    >
{
    /// Create a new Iterator for computing the delta between the supply and demand curve
    #[must_use]
    pub fn new(supply: SI, demand: DI) -> Self {
        CurveDeltaIterator {
            demand: Some(Peeker::new(Box::new(demand).into_iterator())),
            supply: Some(Box::new(supply)),
            remaining_supply: Vec::with_capacity(2), // I think 2 is the maximum size that is ever used
        }
    }
}

impl<DC, SC, DI, SI> FusedIterator for CurveDeltaIterator<DC, SC, DI, SI>
where
    Self: Iterator,
    DI: FusedIterator,
    SI: FusedIterator,
{
}

impl<DW, SW, DI, SI> Iterator for CurveDeltaIterator<DW, SW, DI, SI>
where
    DW: WindowType,
    SW: WindowType,
    DI: CurveIterator,
    DI::CurveKind: CurveType<WindowKind = DW>,
    SI: CurveIterator,
    SI::CurveKind: CurveType<WindowKind = SW>,
{
    type Item = Delta<DW, SW, DI, SI>;

    fn next(&mut self) -> Option<Self::Item> {
        #![allow(clippy::option_if_let_else)] // false positive, both branches move a value

        if let (Some(supply_iter), Some(demand_iter)) = (self.supply.as_mut(), self.demand.as_mut())
        {
            let demand = demand_iter.peek_ref();

            if let Some(demand_window) = demand {
                let supply = self
                    .remaining_supply
                    .pop()
                    .or_else(|| supply_iter.next_window());

                if let Some(supply_window) = supply {
                    if demand_window.start < supply_window.end {
                        let WindowDeltaResult {
                            remaining_supply_head,
                            remaining_supply_tail,
                            overlap,
                            remaining_demand,
                        } = Window::delta(&supply_window, &demand_window.take());

                        let remaining_supply = &mut self.remaining_supply;

                        // remember remaining supply

                        // FIXME should this ever use rust edition 2021 once that is released
                        // currently can't just call .into_iter() on the array
                        // due to backwards compatibility in rust edition 2018
                        IntoIterator::into_iter([remaining_supply_head, remaining_supply_tail])
                            .filter(|window| !window.is_empty())
                            .rev()
                            .for_each(|window| remaining_supply.push(window));

                        // remember remaining demand
                        if !remaining_demand.is_empty() {
                            demand_iter.restore_peek(remaining_demand);
                        }

                        // return overlap
                        Some(Delta::Overlap(overlap))
                    } else {
                        // supply is not usable for the demand
                        // return unusable supply
                        Some(Delta::RemainingSupply(supply_window))
                    }
                } else {
                    // no supply left
                    // clear supply iter
                    self.supply = None;
                    Some(Delta::RemainingDemand(demand_window.take()))
                }
            } else {
                // no demand left
                // clear demand iter
                self.demand = None;

                // finish up supply
                let remaining_supply = self.remaining_supply.pop().map(Delta::RemainingSupply);
                let lazy_supply_iter = || self.supply.take().map(Delta::EndSupply);
                remaining_supply.or_else(lazy_supply_iter)
            }
        } else {
            // demand or supply or both are gone, finish up.
            // if demand is gone remaining_demand should be None
            // likewise for supply

            let remaining_supply = self.remaining_supply.pop().map(Delta::RemainingSupply);

            let s = &mut self.supply;
            let d = &mut self.demand;

            let lazy_supply_iter = || s.take().map(Delta::EndSupply);
            let lazy_demand_iter = || d.take().map(Delta::EndDemand);

            remaining_supply
                .or_else(lazy_supply_iter)
                .or_else(lazy_demand_iter)
        }
    }
}
