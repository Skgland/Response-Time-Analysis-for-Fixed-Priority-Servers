//! Module for the implementation of the Curve delta operation using iterators

use crate::curve::curve_types::CurveType;
use crate::iterators::curve::IterCurveWrapper;
use crate::iterators::CurveIterator;

use crate::window::{Overlap, Window, WindowDeltaResult};
use std::fmt::Debug;

use crate::time::TimeUnit;
use crate::window::window_types::WindowType;
use std::iter::FusedIterator;
use std::marker::PhantomData;

/// Item type of the `CurveDeltaIterator`
#[derive(Debug)]
pub enum Delta<S, D, SI, DI> {
    /// Indicate a Window of remaining supply
    RemainingSupply(Window<S>),
    /// Remaining Supply once Demand ran out
    EndSupply(Box<SI>),
    /// Indicate a Window of overlapping supply and demand
    Overlap(Window<Overlap<S, D>>),
    /// Indicate a Window of remaining demand
    RemainingDemand(Window<D>),
    /// Remaining Demand once Supply ran out
    EndDemand(Box<DI>),
}

impl<S, D, SI, DI> Delta<S, D, SI, DI> {
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

impl<S: WindowType, D: WindowType, SI, DI> CurveIterator<S>
    for RemainingSupplyIterator<S, D, SI, DI>
where
    Self: Debug,
    SI: CurveIterator<S>,
    DI: CurveIterator<D>,
{
    type CurveKind = SI::CurveKind;
}

impl<S, D, SI, DI> FusedIterator for RemainingSupplyIterator<S, D, SI, DI> where Self: Iterator {}

impl<S, D, SI, DI> Iterator for RemainingSupplyIterator<S, D, SI, DI>
where
    CurveDeltaIterator<D, S, DI, SI>: Iterator<Item = Delta<S, D, SI, DI>>,
    SI: CurveIterator<S>,
    DI: CurveIterator<D>,
{
    type Item = Window<S>;

    fn next(&mut self) -> Option<Self::Item> {
        'outer: loop {
            if let Some(end_supply) = self.end_supply.as_mut() {
                if let Some(supply) = end_supply.next() {
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
pub struct InverseCurveIterator<I, C, W> {
    /// The iterator to invert
    iter: I,
    /// The point where total would end
    upper_bound: TimeUnit,
    /// The end of the last window
    previous_end: TimeUnit,
    /// The type of the Produced Curves and the corresponding window type
    curve_type: PhantomData<(W, C)>,
}

impl<I, C, W> InverseCurveIterator<I, C, W> {
    /// Create a new `InverseCurveIterator`
    #[must_use]
    pub const fn new(iter: I, upper_bound: TimeUnit) -> Self {
        InverseCurveIterator {
            iter,
            upper_bound,
            previous_end: TimeUnit::ZERO,
            curve_type: PhantomData,
        }
    }
}

impl<I: Clone, C, W> Clone for InverseCurveIterator<I, C, W> {
    fn clone(&self) -> Self {
        InverseCurveIterator {
            iter: self.iter.clone(),
            upper_bound: self.upper_bound,
            previous_end: self.previous_end,
            curve_type: PhantomData,
        }
    }
}

impl<I: CurveIterator<W>, W: Debug, C: CurveType> CurveIterator<C::WindowKind>
    for InverseCurveIterator<I, C, W>
{
    type CurveKind = C;
}

impl<I: FusedIterator, W, C> FusedIterator for InverseCurveIterator<I, W, C> where Self: Iterator {}

impl<W, I: Iterator<Item = Window<W>>, C: CurveType> Iterator for InverseCurveIterator<I, C, W> {
    type Item = Window<C::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.upper_bound <= self.previous_end {
            None
        } else {
            while let Some(window) = self.iter.next() {
                if self.previous_end < window.start {
                    let result = Window::new(self.previous_end, window.start);
                    self.previous_end = window.end;
                    return Some(result);
                } else if self.previous_end == TimeUnit::ZERO && window.start == TimeUnit::ZERO {
                    self.previous_end = window.end;
                } else {
                    panic!("Overlapping Windows in CurveIterator 'self.iter'")
                }
            }

            let result = Window::new(self.previous_end, self.upper_bound);
            self.previous_end = self.upper_bound;
            Some(result)
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
    demand: Option<Box<DI>>,
    /// remaining supply curve
    supply: Option<Box<SI>>,
    /// peek of the demand curve
    remaining_demand: Option<Window<DW>>,
    /// peek of the supply curve
    remaining_supply: Vec<Window<SW>>,
}

impl<DW, SW, DI: Clone, SI: Clone> Clone for CurveDeltaIterator<DW, SW, DI, SI> {
    fn clone(&self) -> Self {
        CurveDeltaIterator {
            demand: self.demand.clone(),
            supply: self.supply.clone(),
            remaining_demand: self.remaining_demand.clone(),
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
}

impl<DW: WindowType, SW: WindowType, DI: CurveIterator<DW>, SI: CurveIterator<SW>>
    CurveDeltaIterator<DW, SW, DI, SI>
{
    /// Create a new Iterator for computing the delta between the supply and demand curve
    pub fn new(supply: SI, demand: DI) -> Self {
        CurveDeltaIterator {
            demand: Some(Box::new(demand)),
            supply: Some(Box::new(supply)),
            remaining_demand: None,
            remaining_supply: Vec::new(),
        }
    }

    /// Turn the `CurveDeltaIterator` into a `CurveIterator` that returns only the Overlap Windows
    pub fn overlap<C: CurveType<WindowKind = Overlap<SW, DW>>>(
        self,
    ) -> impl CurveIterator<C::WindowKind, CurveKind = C> + Clone
    where
        Self: Clone,
    {
        let inner = self.filter_map(Delta::overlap);
        unsafe {
            // Safety
            // self is an iterator of three interleaved curves, but using filter_map
            // we filter only one out
            // so the remaining iterator is a valid curve
            IterCurveWrapper::new(inner)
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
    DI: CurveIterator<DW>,
    SI: CurveIterator<SW>,
{
    type Item = Delta<SW, DW, SI, DI>;

    fn next(&mut self) -> Option<Self::Item> {
        #![allow(clippy::option_if_let_else)] // false positive, both branches move a value

        if let (Some(supply_iter), Some(demand_iter)) = (self.supply.as_mut(), self.demand.as_mut())
        {
            let demand = self.remaining_demand.take().or_else(|| demand_iter.next());

            if let Some(demand_window) = demand {
                let supply = self.remaining_supply.pop().or_else(|| supply_iter.next());

                if let Some(supply_window) = supply {
                    if demand_window.start < supply_window.end {
                        let WindowDeltaResult {
                            remaining_supply_head,
                            remaining_supply_tail,
                            overlap,
                            remaining_demand,
                        } = Window::delta(&supply_window, &demand_window);

                        // remember remaining supply
                        vec![remaining_supply_head, remaining_supply_tail]
                            .into_iter()
                            .filter(|window| !window.is_empty())
                            .rev()
                            .for_each(|window| self.remaining_supply.push(window));

                        // remember remaining demand
                        self.remaining_demand =
                            Some(remaining_demand).filter(|window| !window.is_empty());

                        // return overlap
                        Some(Delta::Overlap(overlap))
                    } else {
                        // supply is not usable for the demand
                        // remember unused demand
                        self.remaining_demand = Some(demand_window);
                        // return unusable supply
                        Some(Delta::RemainingSupply(supply_window))
                    }
                } else {
                    // no supply left
                    // clear supply iter
                    self.supply = None;
                    Some(Delta::RemainingDemand(demand_window))
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

            let rd = &mut self.remaining_demand;
            let s = &mut self.supply;
            let d = &mut self.demand;

            let lazy_remaining_demand = || rd.take().map(Delta::RemainingDemand);
            let lazy_supply_iter = || s.take().map(Delta::EndSupply);
            let lazy_demand_iter = || d.take().map(Delta::EndDemand);

            remaining_supply
                .or_else(lazy_remaining_demand)
                .or_else(lazy_supply_iter)
                .or_else(lazy_demand_iter)
        }
    }
}
