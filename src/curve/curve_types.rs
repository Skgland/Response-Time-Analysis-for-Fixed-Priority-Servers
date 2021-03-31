//! Definition and implementations of `CurveType` trait and

use std::fmt::Debug;

use crate::seal::Seal;
use crate::server::{
    ActualServerExecution, AggregatedServerDemand, AvailableServerExecution,
    ConstrainedServerDemand, HigherPriorityServerDemand,
};
use crate::task::{
    ActualTaskExecution, AvailableTaskExecution, HigherPriorityTaskDemand, TaskDemand,
};
use crate::window::window_types::WindowType;
use crate::window::{Demand, Overlap, Supply};
use std::marker::PhantomData;

/// Sealed Marker Trait for Curve Types
pub trait CurveType: Seal + Debug {
    /// The [`WindowKind`](CurveType::WindowKind) for the Windows of the Curve
    type WindowKind: WindowType;
}

impl<W: WindowType> CurveType for UnspecifiedCurve<W> {
    type WindowKind = W;
}

impl CurveType for AggregatedServerDemand {
    type WindowKind = <TaskDemand as CurveType>::WindowKind;
}

impl CurveType for ConstrainedServerDemand {
    type WindowKind = <AggregatedServerDemand as CurveType>::WindowKind;
}

impl CurveType for HigherPriorityServerDemand {
    type WindowKind = <ConstrainedServerDemand as CurveType>::WindowKind;
}

impl CurveType for AvailableServerExecution {
    type WindowKind = Overlap<Supply, Demand>;
}

impl CurveType for ActualServerExecution {
    type WindowKind = Overlap<Overlap<Supply, Demand>, Demand>;
}

impl CurveType for TaskDemand {
    type WindowKind = Demand;
}

impl CurveType for HigherPriorityTaskDemand {
    type WindowKind = <TaskDemand as CurveType>::WindowKind;
}

impl CurveType for AvailableTaskExecution {
    type WindowKind = <ActualServerExecution as CurveType>::WindowKind;
}

impl CurveType for ActualTaskExecution {
    type WindowKind = Overlap<
        <AvailableTaskExecution as CurveType>::WindowKind,
        <TaskDemand as CurveType>::WindowKind,
    >;
}

/// Marker Type for unspecified Curves
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct UnspecifiedCurve<W>(PhantomData<W>);
