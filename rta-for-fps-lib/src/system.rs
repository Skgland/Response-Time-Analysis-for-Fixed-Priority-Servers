//! Module for the System type

use crate::curve::AggregateExt;
use crate::iterators::curve::{AggregationIterator, CapacityCheckIterator, InverseCurveIterator};

use crate::server::{
    ActualServerExecution, ConstrainedDemand, ConstrainedServerDemand, HigherPriorityServerDemand,
    HigherPriorityServerExecution, Server, UnconstrainedServerExecution,
};

use crate::curve::curve_types::CurveType;
use crate::iterators::server::actual_execution::ActualServerExecutionIterator;
use crate::iterators::{CurveIterator, EitherCurveIterator, ReclassifyIterator};
use crate::time::TimeUnit;
use crate::window::Window;
use alloc::boxed::Box;
use alloc::vec::Vec;

/// Type representing a System of Servers
#[derive(Debug)]
pub struct System<'a> {
    /// The Servers of the System
    servers: &'a [Server<'a>],
}
/**
A `CurveIterator` over a servers aggregated higher priority demand
*/
#[derive(Clone, Debug)]
pub struct AggregatedHPServerDemand<CSD>(
    ReclassifyIterator<
        AggregationIterator<CSD, <ConstrainedServerDemand as CurveType>::WindowKind>,
        HigherPriorityServerDemand,
    >,
);

impl<CSD> CurveIterator for AggregatedHPServerDemand<CSD>
where
    CSD: CurveIterator<CurveKind = ConstrainedServerDemand>,
{
    type CurveKind = HigherPriorityServerDemand;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.0.next_window()
    }
}

/**
A `CurveIterator` over the Aggregated Higher Priority Execution of a Server
*/
#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
pub struct AggregatedHPExecution(
    ReclassifyIterator<
        AggregationIterator<
            EitherCurveIterator<
                FixedActualExecution,
                ReclassifyIterator<Box<AggregatedHPExecution>, ActualServerExecution>,
            >,
            <ActualServerExecution as CurveType>::WindowKind,
        >,
        HigherPriorityServerExecution,
    >,
);

impl CurveIterator for AggregatedHPExecution {
    type CurveKind = HigherPriorityServerExecution;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.0.next_window()
    }
}

/**
A `CurveIterator` over a servers unconstrained execution using the original algorithm
*/
#[derive(Clone, Debug)]
pub struct OriginalUnconstrainedExecution(
    InverseCurveIterator<AggregatedHPServerDemand<ConstrainedDemand>, UnconstrainedServerExecution>,
);

impl CurveIterator for OriginalUnconstrainedExecution {
    type CurveKind = UnconstrainedServerExecution;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.0.next_window()
    }
}

/**
A `CurveIterator` over the Unconstrained execution of a server
*/
#[derive(Clone, Debug)]
pub struct FixedUnconstrainedExecution(
    InverseCurveIterator<AggregatedHPExecution, UnconstrainedServerExecution>,
);

impl CurveIterator for FixedUnconstrainedExecution {
    type CurveKind = UnconstrainedServerExecution;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.0.next_window()
    }
}

/**
A `CurveIterator` over a Servers actual execution using the original algorithm
 */
#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
pub struct OriginalActualServerExecution(
    ActualServerExecutionIterator<
        CapacityCheckIterator<
            <<OriginalUnconstrainedExecution as CurveIterator>::CurveKind as CurveType>::WindowKind,
            OriginalUnconstrainedExecution,
            <OriginalUnconstrainedExecution as CurveIterator>::CurveKind,
        >,
        ConstrainedDemand,
    >,
);

impl CurveIterator for OriginalActualServerExecution {
    type CurveKind = ActualServerExecution;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.0.next_window()
    }
}

/**
A `CurveIterator` over a Servers actual execution using the fixed algorithm
*/
#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
pub struct FixedActualExecution(
    ActualServerExecutionIterator<
        CapacityCheckIterator<
            <<FixedUnconstrainedExecution as CurveIterator>::CurveKind as CurveType>::WindowKind,
            FixedUnconstrainedExecution,
            <FixedUnconstrainedExecution as CurveIterator>::CurveKind,
        >,
        ConstrainedDemand,
    >,
);

impl CurveIterator for FixedActualExecution {
    type CurveKind = ActualServerExecution;

    fn next_window(&mut self) -> Option<Window<<Self::CurveKind as CurveType>::WindowKind>> {
        self.0.next_window()
    }
}

impl<'a> System<'a> {
    /// Create a new System from a slice of Servers,
    /// indexed by their priority,
    /// lowest index being the highest priority
    #[must_use]
    pub const fn new(servers: &'a [Server<'a>]) -> System<'a> {
        System { servers }
    }

    /// Get a slice reference to the systems servers
    #[must_use]
    pub const fn as_servers(&self) -> &'a [Server<'a>] {
        self.servers
    }

    /// Calculate the aggregated higher priority demand curve
    /// by aggregating the aggregated demand curves of all Servers with higher priority
    /// (lower value) than `index`.
    ///
    /// Based on the papers Definition 12.
    #[must_use]
    pub fn aggregated_higher_priority_demand_curve_iter<'b, CSDCI>(
        constrained_demand_curves: CSDCI,
    ) -> AggregatedHPServerDemand<CSDCI::Item>
    where
        CSDCI::Item: CurveIterator<CurveKind = ConstrainedServerDemand> + Clone + 'b,
        CSDCI: IntoIterator,
    {
        let ahpd = constrained_demand_curves
            .into_iter()
            .aggregate::<ReclassifyIterator<_, _>>();
        AggregatedHPServerDemand(ahpd)
    }

    /**
    Calculate the aggregated higher priority actual execution of the server with index `server_index`
    */
    #[must_use]
    pub fn aggregated_higher_priority_actual_execution_curve_iter(
        &self,
        server_index: usize,
    ) -> AggregatedHPExecution {
        let mut curves: Vec<EitherCurveIterator<_, _>> = alloc::vec::Vec::with_capacity(2);

        if server_index > 0 {
            curves.push(EitherCurveIterator::Left(
                self.fixed_actual_execution_curve_iter(server_index - 1),
            ));
            if server_index > 1 {
                let curve = Box::new(
                    self.aggregated_higher_priority_actual_execution_curve_iter(server_index - 1),
                )
                .reclassify();
                curves.push(EitherCurveIterator::Right(curve));
            }
        }

        AggregatedHPExecution(curves.into_iter().aggregate())
    }

    /// Calculate the system wide hyper period
    /// accounting for all servers and tasks
    /// up to and including the server with priority `server_index`
    ///
    /// Section 7.1 §2 Sentence 3, allows to exclude lower priority servers from the swh period calculation,
    /// when analysing tasks of a server
    #[must_use]
    pub fn system_wide_hyper_period(&self, server_index: usize) -> TimeUnit {
        self.servers[..=server_index]
            .iter()
            .map(|server| server.properties.interval)
            .chain(
                self.servers
                    .iter()
                    .flat_map(|server| server.as_tasks().iter().map(|task| task.interval)),
            )
            .fold(TimeUnit::ONE, TimeUnit::lcm)
    }

    /**
    For the server with index `server_index` calculate up to which point in time we need to perform the analysis
    Replaces `system_wide_hyper_period` as that does not account for task offset
    */
    #[must_use]
    pub fn analysis_end(&self, server_index: usize) -> TimeUnit {
        let res = self.servers[..=server_index]
            .iter()
            .map(|server| (server.properties.interval, TimeUnit::ZERO))
            .chain(self.servers.iter().flat_map(|server| {
                server
                    .as_tasks()
                    .iter()
                    .map(|task| (task.interval, task.offset))
            }))
            .fold((TimeUnit::ONE, TimeUnit::ZERO), |acc, next| {
                (TimeUnit::lcm(acc.0, next.0), acc.1.max(next.1))
            });
        res.0 + res.1
    }

    /// Calculate the unconstrained execution curve
    /// for the server with priority `index`.
    ///
    /// See Definition 13. of the paper for reference
    #[must_use]
    pub fn original_unconstrained_server_execution_curve_iter(
        &self,
        server_index: usize,
    ) -> OriginalUnconstrainedExecution {
        #![allow(clippy::redundant_closure_for_method_calls)]

        let csdi = self.servers[..server_index]
            .iter()
            .map(|server| server.constraint_demand_curve_iter());

        let ahpc = System::aggregated_higher_priority_demand_curve_iter(csdi);

        OriginalUnconstrainedExecution(InverseCurveIterator::new(ahpc))
    }

    /**
    Calculate the unconstrained server execution using the aggregated hp actual execution rather than the aggregated hp constrained demand
    */
    #[must_use]
    pub fn fixed_unconstrained_server_execution_curve_iter(
        &self,
        server_index: usize,
    ) -> FixedUnconstrainedExecution {
        let ahpc = self.aggregated_higher_priority_actual_execution_curve_iter(server_index);

        FixedUnconstrainedExecution(InverseCurveIterator::new(ahpc))
    }

    /// Calculate the Constrained Execution Curve using Algorithm 4. from the paper
    /// TODO more detail, what do the parameters mean
    /// # Panics
    ///
    /// When a server is not guaranteed its capacity every interval
    ///
    #[must_use]
    pub fn original_actual_execution_curve_iter(
        &self,
        server_index: usize,
    ) -> OriginalActualServerExecution {
        let unchecked_unconstrained_execution =
            self.original_unconstrained_server_execution_curve_iter(server_index);

        let props = self.servers[server_index].properties;

        // split unconstrained execution curve into groups every server.interval
        // and check that each group has at least server.capacity of capacity
        let checked_unconstrained_execution = CapacityCheckIterator::new(
            unchecked_unconstrained_execution,
            props.capacity,
            props.interval,
        );

        let constrained_demand = self.servers[server_index].constraint_demand_curve_iter();

        OriginalActualServerExecution(ActualServerExecutionIterator::new(
            self.servers[server_index].properties,
            checked_unconstrained_execution,
            constrained_demand,
        ))
    }

    /**
    Calculate the actual execution with the fixed unconstrained server execution rather than the original unconstrained server execution
    */
    #[must_use]
    pub fn fixed_actual_execution_curve_iter(&self, server_index: usize) -> FixedActualExecution {
        let unchecked_unconstrained_execution =
            self.fixed_unconstrained_server_execution_curve_iter(server_index);

        let props = self.servers[server_index].properties;

        // split unconstrained execution curve into groups every server.interval
        // and check that each group has at least server.capacity of capacity
        let checked_unconstrained_execution = CapacityCheckIterator::new(
            unchecked_unconstrained_execution,
            props.capacity,
            props.interval,
        );

        let constrained_demand = self.servers[server_index].constraint_demand_curve_iter();

        FixedActualExecution(ActualServerExecutionIterator::new(
            self.servers[server_index].properties,
            checked_unconstrained_execution,
            constrained_demand,
        ))
    }
}
