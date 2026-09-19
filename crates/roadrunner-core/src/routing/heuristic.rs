use thiserror::Error;

use crate::cost::{CostError, CostKind, HeuristicPolicy, RouteCost, SearchCapability};
use crate::geo::{KilometersPerHour, haversine_distance};
use crate::graph::{FrozenGraph, GraphSnapshotId, NodeId};

const NUMERICAL_LOWER_BOUND_FACTOR: f64 = 1.0 - 1.0e-9;

fn conservative_lower_bound(value: f64) -> f64 {
    if value == 0.0 {
        return 0.0;
    }
    value * NUMERICAL_LOWER_BOUND_FACTOR
}

/// Lower-bound policy used by A*.
pub trait Heuristic: Send + Sync {
    /// Returns the objective kind estimated by this heuristic.
    fn kind(&self) -> CostKind;
    /// Returns the named policy whose proof contract this implementation uses.
    fn policy(&self) -> HeuristicPolicy;
    /// Returns the graph snapshot to which prevalidated parameters are bound.
    fn graph_snapshot_id(&self) -> Option<GraphSnapshotId> {
        None
    }
    /// Returns whether the heuristic is proved for a search capability.
    fn supports(&self, capability: SearchCapability) -> bool;
    /// Estimates remaining objective cost.
    ///
    /// # Errors
    ///
    /// Returns an error when the estimate cannot be represented.
    fn estimate(
        &self,
        graph: &FrozenGraph,
        node: NodeId,
        destination: NodeId,
    ) -> Result<RouteCost, CostError>;
}

/// Conservative heuristic that always returns zero.
#[derive(Debug, Clone, Copy)]
pub struct ZeroHeuristic {
    kind: CostKind,
}

impl ZeroHeuristic {
    /// Creates a zero heuristic for an objective kind.
    #[must_use]
    pub const fn new(kind: CostKind) -> Self {
        Self { kind }
    }
}

impl Heuristic for ZeroHeuristic {
    fn kind(&self) -> CostKind {
        self.kind
    }
    fn policy(&self) -> HeuristicPolicy {
        HeuristicPolicy::Zero
    }
    fn supports(&self, capability: SearchCapability) -> bool {
        matches!(
            capability,
            SearchCapability::StaticNonNegative | SearchCapability::FifoEarliestArrival
        )
    }
    fn estimate(
        &self,
        _graph: &FrozenGraph,
        _node: NodeId,
        _destination: NodeId,
    ) -> Result<RouteCost, CostError> {
        Ok(RouteCost::zero(self.kind))
    }
}

/// Haversine lower bound for physical-distance routing.
#[derive(Debug, Default, Clone, Copy)]
pub struct DistanceHaversine;

impl Heuristic for DistanceHaversine {
    fn kind(&self) -> CostKind {
        CostKind::Distance
    }
    fn policy(&self) -> HeuristicPolicy {
        HeuristicPolicy::DistanceHaversine
    }
    fn supports(&self, capability: SearchCapability) -> bool {
        capability == SearchCapability::StaticNonNegative
    }
    fn estimate(
        &self,
        graph: &FrozenGraph,
        node: NodeId,
        destination: NodeId,
    ) -> Result<RouteCost, CostError> {
        let from = graph
            .node(node)
            .map_or(crate::geo::Coordinate::ORIGIN, |value| value.coordinate());
        let to = graph
            .node(destination)
            .map_or(crate::geo::Coordinate::ORIGIN, |value| value.coordinate());
        RouteCost::new(
            CostKind::Distance,
            conservative_lower_bound(haversine_distance(from, to).value()),
        )
    }
}

/// Haversine travel-time lower bound using a proven maximum profile speed.
#[derive(Debug, Clone, Copy)]
pub struct TravelTimeHaversine {
    maximum_speed: KilometersPerHour,
    graph_snapshot_id: GraphSnapshotId,
}

impl TravelTimeHaversine {
    /// Creates a travel-time heuristic from a positive maximum possible speed.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero speed.
    pub fn for_graph(
        graph: &FrozenGraph,
        maximum_speed: KilometersPerHour,
    ) -> Result<Self, HeuristicConfigError> {
        if maximum_speed.value() == 0.0 {
            return Err(HeuristicConfigError::ZeroMaximumSpeed);
        }
        let observed_maximum = graph
            .edges()
            .iter()
            .map(|edge| edge.properties().effective_free_flow_speed().value())
            .fold(0.0_f64, f64::max);
        if maximum_speed.value() < observed_maximum {
            return Err(HeuristicConfigError::SpeedBoundTooLow {
                configured_kph: maximum_speed.value(),
                observed_kph: observed_maximum,
            });
        }
        Ok(Self {
            maximum_speed,
            graph_snapshot_id: graph.snapshot_id(),
        })
    }
}

impl Heuristic for TravelTimeHaversine {
    fn kind(&self) -> CostKind {
        CostKind::TravelTime
    }
    fn policy(&self) -> HeuristicPolicy {
        HeuristicPolicy::TravelTimeHaversine
    }
    fn graph_snapshot_id(&self) -> Option<GraphSnapshotId> {
        Some(self.graph_snapshot_id)
    }
    fn supports(&self, capability: SearchCapability) -> bool {
        matches!(
            capability,
            SearchCapability::StaticNonNegative | SearchCapability::FifoEarliestArrival
        )
    }
    fn estimate(
        &self,
        graph: &FrozenGraph,
        node: NodeId,
        destination: NodeId,
    ) -> Result<RouteCost, CostError> {
        let from = graph
            .node(node)
            .map_or(crate::geo::Coordinate::ORIGIN, |value| value.coordinate());
        let to = graph
            .node(destination)
            .map_or(crate::geo::Coordinate::ORIGIN, |value| value.coordinate());
        let seconds = haversine_distance(from, to).value() / (self.maximum_speed.value() / 3.6);
        RouteCost::new(CostKind::TravelTime, conservative_lower_bound(seconds))
    }
}

/// Invalid immutable heuristic configuration.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum HeuristicConfigError {
    /// Maximum possible speed must be positive.
    #[error("maximum possible speed must be greater than zero")]
    ZeroMaximumSpeed,
    /// Configured bound is lower than an edge's compiled free-flow speed.
    #[error(
        "maximum speed {configured_kph} km/h is below observed graph speed {observed_kph} km/h"
    )]
    SpeedBoundTooLow {
        /// Rejected configured upper bound.
        configured_kph: f64,
        /// Largest compiled edge speed.
        observed_kph: f64,
    },
}
