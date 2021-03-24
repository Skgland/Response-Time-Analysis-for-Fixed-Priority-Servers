//! Non pub Module for Sealing Traits, currently only the [`WindowType`](WindowType) trait

use crate::curve::{Demand, Overlap, Supply};

/// Sealed Marker Trait for Window Types
pub trait WindowType {}

impl WindowType for Supply {}

impl WindowType for Demand {}

impl WindowType for Overlap {}
