use crate::curve::curve_types::CurveType;
use crate::iterators::curve::IterCurveWrapper;
use crate::iterators::CurveIterator;

use crate::window::{Overlap, Window, WindowDeltaResult};
use std::collections::VecDeque;
use std::fmt::Debug;

use std::marker::PhantomData;

/// Item type of the `CurveDeltaIterator`
#[derive(Debug)]
pub enum Delta<S, D> {
    /// Indicate a Window of remaining supply
    RemainingSupply(Window<S>),
    /// Indicate a Window of overlapping supply and demand
    Overlap(Window<Overlap<S, D>>),
    /// Indicate a Window of remaining demand
    RemainingDemand(Window<D>),
}

impl<S, D> Delta<S, D> {
    pub const fn overlap(self) -> Option<Window<Overlap<S, D>>> {
        match self {
            Delta::RemainingSupply(_) | Delta::RemainingDemand(_) => None,
            Delta::Overlap(overlap) => Some(overlap),
        }
    }
    pub const fn remaining_supply(self) -> Option<Window<S>> {
        match self {
            Delta::RemainingSupply(supply) => Some(supply),
            Delta::Overlap(_) | Delta::RemainingDemand(_) => None,
        }
    }
}

/// TODO description
#[derive(Debug)]
pub struct CurveDeltaIterator<'a, DC: CurveType, SC: CurveType, DI, SI> {
    /// remaining demand curve
    demand: DI,
    /// remaining supply curve
    supply: SI,
    /// peek of the demand curve
    remaining_demand: Option<Window<DC::WindowKind>>,
    /// peek of the supply curve
    remaining_supply: VecDeque<Window<SC::WindowKind>>,
    /// lifetime
    lifetime: PhantomData<&'a ()>,
}

impl<'a, DC: CurveType, SC: CurveType, DI: Clone, SI: Clone> Clone
    for CurveDeltaIterator<'a, DC, SC, DI, SI>
{
    fn clone(&self) -> Self {
        CurveDeltaIterator {
            demand: self.demand.clone(),
            supply: self.supply.clone(),
            remaining_demand: self.remaining_demand.clone(),
            remaining_supply: self.remaining_supply.clone(),
            lifetime: PhantomData,
        }
    }
}

impl<
        'a,
        DC: CurveType + 'a,
        SC: CurveType + 'a,
        DI: CurveIterator<'a, DC>,
        SI: CurveIterator<'a, SC>,
    > CurveDeltaIterator<'a, DC, SC, DI, SI>
{
    /// Create a new Iterator for computing the delta between the supply and demand curve
    pub fn new(supply: SI, demand: DI) -> Self {
        CurveDeltaIterator {
            demand,
            supply,
            remaining_demand: None,
            remaining_supply: VecDeque::default(),
            lifetime: PhantomData,
        }
    }

    /// Turn the `CurveDeltaIterator` into a `CurveIterator` that returns only the Overlap Windows
    pub fn overlap<C: CurveType<WindowKind = Overlap<SC::WindowKind, DC::WindowKind>> + 'a>(
        self,
    ) -> impl CurveIterator<'a, C> + Clone
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

    /// Turn the `CurveDeltaIterator` into a `CurveIterator` that returns only the Remaining Supply Windows
    pub fn remaining_supply<C>(self) -> impl CurveIterator<'a, C> + Clone
    where
        C: CurveType<WindowKind = SC::WindowKind> + 'a,
        Self: Clone,
    {
        let inner = self.filter_map(Delta::remaining_supply);

        unsafe {
            // Safety
            // self is an iterator of three interleaved curves, but using filter_map
            // we filter only one out
            // so the remaining iterator is a valid curve
            IterCurveWrapper::new(inner)
        }
    }
}

impl<'a, DC, SC, DI, SI> Iterator for CurveDeltaIterator<'a, DC, SC, DI, SI>
where
    DC: CurveType,
    SC: CurveType,
    DI: CurveIterator<'a, DC>,
    SI: CurveIterator<'a, SC>,
{
    type Item = Delta<SC::WindowKind, DC::WindowKind>;

    fn next(&mut self) -> Option<Self::Item> {
        #![allow(clippy::option_if_let_else)] // false positive, both branches move a value

        let demand = self.remaining_demand.take().or_else(|| self.demand.next());

        if let Some(demand_window) = demand {
            let supply = self
                .remaining_supply
                .pop_front()
                .or_else(|| self.supply.next());
            if let Some(supply_window) = supply {
                if demand_window.start < supply_window.end {
                    let WindowDeltaResult {
                        remaining_supply,
                        overlap,
                        remaining_demand,
                    } = Window::delta(&supply_window, &demand_window);

                    remaining_supply
                        .into_windows()
                        .into_iter()
                        .rev()
                        .for_each(|window| self.remaining_supply.push_front(window));

                    self.remaining_demand =
                        Some(remaining_demand).filter(|window| !window.is_empty());

                    Some(Delta::Overlap(overlap))
                } else {
                    // supply is not usable for the demand
                    self.remaining_demand = Some(demand_window);
                    Some(Delta::RemainingSupply(supply_window))
                }
            } else {
                // no supply left, return demand as remaining
                Some(Delta::RemainingDemand(demand_window))
            }
        } else {
            // no demand left, return supply as remaining
            let supply = self
                .remaining_supply
                .pop_front()
                .or_else(|| self.supply.next());
            supply.map(Delta::RemainingSupply)
        }
    }
}