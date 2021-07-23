//! Non pub Module for Sealing Traits

use crate::curve::curve_types::UnspecifiedCurve;
use crate::server::{
    ActualServerExecution, AggregatedServerDemand, ConstrainedServerDemand,
    HigherPriorityServerDemand, HigherPriorityServerExecution, UnconstrainedServerExecution,
};
use crate::task::curve_types::{
    ActualTaskExecution, AvailableTaskExecution, HigherPriorityTaskDemand, TaskDemand,
};
use crate::window::window_types::WindowType;
use crate::window::{Demand, Overlap, Supply};

/// Trait used as Sub-Trait for Sealing other Traits
pub trait Seal {}

// WindowKind

impl<P: WindowType, Q: WindowType> Seal for Overlap<P, Q> {}
impl Seal for Supply {}
impl Seal for Demand {}

// CurveKind
impl<W: WindowType> Seal for UnspecifiedCurve<W> {}

// Serve Curves
impl Seal for AggregatedServerDemand {}
impl Seal for ConstrainedServerDemand {}
impl Seal for HigherPriorityServerDemand {}
impl Seal for HigherPriorityServerExecution {}
impl Seal for UnconstrainedServerExecution {}
impl Seal for ActualServerExecution {}

// Task Curves
impl Seal for TaskDemand {}
impl Seal for HigherPriorityTaskDemand {}
impl Seal for AvailableTaskExecution {}
impl Seal for ActualTaskExecution {}
