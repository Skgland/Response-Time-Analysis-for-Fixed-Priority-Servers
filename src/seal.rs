//! Non pub Module for Sealing Traits, currently only the [`WindowType`](WindowType) trait

use crate::window::{Demand, Overlap, Supply};

/// Sealed Marker Trait for Window Types
pub trait WindowType: Clone {}

impl WindowType for Supply {}

impl WindowType for Demand {}

impl WindowType for Overlap {}
