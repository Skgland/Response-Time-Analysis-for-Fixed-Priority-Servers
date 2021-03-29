use crate::curve::curve_types::CurveType;
use crate::curve::Curve;
use crate::iterators::curve::{CurveSplitIterator, RecursiveAggregatedDemandIterator};
use crate::server::{AggregatedServerDemand, Server};
use crate::window::Window;

/// `CurveIterator` for the Constrained Server Demand of a Server
#[derive(Debug)]
struct ConstrainedServerDemandIterator<'a> {
    /// The Server for which to calculate the constrained demand
    server: &'a Server,
    /// The remaining aggregated Demand of the Server
    groups: CurveSplitIterator<
        'a,
        AggregatedServerDemand,
        RecursiveAggregatedDemandIterator<'a, AggregatedServerDemand>,
    >,
    /// The next group
    group_peek: Option<(usize, Curve<AggregatedServerDemand>)>,
    /// The spill from the previous group
    spill: Option<Window<<AggregatedServerDemand as CurveType>::WindowKind>>,
}

// TODO impl CurveIterator for ConstrainedServerDemandIterator
