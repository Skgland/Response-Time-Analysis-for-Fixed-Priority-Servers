//! Non pub Module for Sealing Traits

use crate::curve::{OverlapCurve, PrimitiveCurve};
use crate::server::{
    AggregatedServerDemand, ConstrainedServerDemand, ConstrainedServerExecution,
    HigherPriorityServerDemand, UnconstrainedServerExecution,
};
use crate::task::{
    ActualTaskExecution, AvailableTaskExecution, HigherPriorityTaskDemand, TaskDemand,
};
use crate::window::{Demand, Overlap, Supply};
use std::fmt::Debug;

/// Sealed Marker Trait for Window Types
pub trait WindowType: Clone + Debug + Eq {}

impl WindowType for Supply {}

impl WindowType for Demand {}

impl<P: WindowType, Q: WindowType> WindowType for Overlap<P, Q> {}

/// Sealed Marker Trait for Curve Types
pub trait CurveType: Debug + Eq {
    /// The [`WindowKind`] for the Windows of the Curve
    type WindowKind: WindowType;
}

impl<P: CurveType, Q: CurveType> CurveType for OverlapCurve<P, Q> {
    type WindowKind = Overlap<P::WindowKind, Q::WindowKind>;
}

impl<W: WindowType> CurveType for PrimitiveCurve<W> {
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

impl CurveType for UnconstrainedServerExecution {
    type WindowKind = Overlap<Supply, Demand>;
}

impl CurveType for ConstrainedServerExecution {
    type WindowKind = Overlap<Overlap<Supply, Demand>, Demand>;
}

impl CurveType for TaskDemand {
    type WindowKind = Demand;
}

impl CurveType for HigherPriorityTaskDemand {
    type WindowKind = <TaskDemand as CurveType>::WindowKind;
}

impl CurveType for AvailableTaskExecution {
    type WindowKind = <ConstrainedServerExecution as CurveType>::WindowKind;
}

impl CurveType for ActualTaskExecution {
    type WindowKind = Overlap<
        <AvailableTaskExecution as CurveType>::WindowKind,
        <TaskDemand as CurveType>::WindowKind,
    >;
}
