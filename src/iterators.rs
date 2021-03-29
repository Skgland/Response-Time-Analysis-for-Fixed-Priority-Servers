//! Module for the Iterator based implementation

use std::fmt::Debug;
use std::iter::{Empty, FusedIterator, TakeWhile};

use crate::curve::curve_types::CurveType;
use crate::window::Window;

/// Trait representing an Iterator that has the guarantees of a curve:
/// 1. Windows ordered by start
/// 2. Windows non-overlapping
/// 3. Windows non-empty
pub trait CurveIterator<'a, C: CurveType>:
    Iterator<Item = Window<C::WindowKind>> + FusedIterator + Debug + 'a
{
}

impl<'a, C: CurveType + 'a> CurveIterator<'a, C> for Empty<Window<C::WindowKind>> {}

pub mod curve;
pub mod server;
pub mod task;

impl<'a, C: CurveType + 'a> CurveIterator<'a, C> for Box<dyn CurveIterator<'a, C>> {}

impl<'a, C: CurveType, P: for<'r> FnMut(&'r I::Item) -> bool + 'a, I: CurveIterator<'a, C>>
    CurveIterator<'a, C> for TakeWhile<I, P>
{
}
