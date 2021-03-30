//! Non pub Module for Sealing Traits

use crate::curve::curve_types::{CurveType, OverlapCurve, PrimitiveCurve};
use crate::server::{
    ActualServerExecution, AggregatedServerDemand, AvailableServerExecution,
    ConstrainedServerDemand, HigherPriorityServerDemand,
};
use crate::task::{
    ActualTaskExecution, AvailableTaskExecution, HigherPriorityTaskDemand, TaskDemand,
};
use crate::window::window_types::WindowType;
use crate::window::{Demand, Overlap, Supply};

/// Trait used as Sub-Trait for Sealing Traits
pub trait Seal {}

// WindowKind

impl<P: WindowType, Q: WindowType> Seal for Overlap<P, Q> {}
impl Seal for Supply {}
impl Seal for Demand {}

// CurveKind

impl<P: CurveType, Q: CurveType> Seal for OverlapCurve<P, Q> {}
impl<W: WindowType> Seal for PrimitiveCurve<W> {}

// Serve Curves
impl Seal for AggregatedServerDemand {}
impl Seal for ConstrainedServerDemand {}
impl Seal for HigherPriorityServerDemand {}
impl Seal for AvailableServerExecution {}
impl Seal for ActualServerExecution {}

// Task Curves
impl Seal for TaskDemand {}
impl Seal for HigherPriorityTaskDemand {}
impl Seal for AvailableTaskExecution {}
impl Seal for ActualTaskExecution {}
