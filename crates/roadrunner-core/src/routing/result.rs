use serde::Serialize;

use crate::cost::RouteCost;
use crate::geo::{Meters, Seconds};
use crate::graph::{EdgeId, GraphSnapshotId, NodeId};

/// Algorithm used to compute a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingAlgorithm {
    /// Dijkstra's algorithm.
    Dijkstra,
    /// A* search.
    AStar,
}

/// A validated route tied to the graph snapshot that produced it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteResult {
    graph_snapshot_id: GraphSnapshotId,
    algorithm: RoutingAlgorithm,
    path: Vec<NodeId>,
    edges: Vec<EdgeId>,
    total_distance: Meters,
    total_cost: RouteCost,
    elapsed_travel_time: Seconds,
    expanded_states: usize,
}

#[derive(Clone, Copy)]
pub(super) struct RouteMetrics {
    pub(super) total_distance: Meters,
    pub(super) total_cost: RouteCost,
    pub(super) elapsed_travel_time: Seconds,
    pub(super) expanded_states: usize,
}

impl RouteResult {
    pub(super) fn new(
        graph_snapshot_id: GraphSnapshotId,
        algorithm: RoutingAlgorithm,
        path: Vec<NodeId>,
        edges: Vec<EdgeId>,
        metrics: RouteMetrics,
    ) -> Self {
        Self {
            graph_snapshot_id,
            algorithm,
            path,
            edges,
            total_distance: metrics.total_distance,
            total_cost: metrics.total_cost,
            elapsed_travel_time: metrics.elapsed_travel_time,
            expanded_states: metrics.expanded_states,
        }
    }
    /// Returns the graph snapshot identity.
    #[must_use]
    pub const fn graph_snapshot_id(&self) -> GraphSnapshotId {
        self.graph_snapshot_id
    }
    /// Returns the algorithm.
    #[must_use]
    pub const fn algorithm(&self) -> RoutingAlgorithm {
        self.algorithm
    }
    /// Returns ordered nodes.
    #[must_use]
    pub fn path(&self) -> &[NodeId] {
        &self.path
    }
    /// Returns ordered directed edges.
    #[must_use]
    pub fn edges(&self) -> &[EdgeId] {
        &self.edges
    }
    /// Returns physical route distance.
    #[must_use]
    pub const fn total_distance(&self) -> Meters {
        self.total_distance
    }
    /// Returns accumulated objective cost.
    #[must_use]
    pub const fn total_cost(&self) -> RouteCost {
        self.total_cost
    }
    /// Returns elapsed traversal time.
    #[must_use]
    pub const fn elapsed_travel_time(&self) -> Seconds {
        self.elapsed_travel_time
    }
    /// Returns the number of search-state expansion events.
    #[must_use]
    pub const fn expanded_states(&self) -> usize {
        self.expanded_states
    }
}
