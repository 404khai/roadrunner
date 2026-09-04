use serde::Serialize;

use crate::cost::RouteCost;
use crate::geo::Meters;
use crate::graph::{EdgeId, NodeId};

/// The algorithm used to compute a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingAlgorithm {
    /// Dijkstra's shortest-path algorithm.
    Dijkstra,
    /// A* shortest-path search with a scaled Haversine heuristic.
    AStar,
}

/// A validated shortest-path result and its search diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteResult {
    algorithm: RoutingAlgorithm,
    path: Vec<NodeId>,
    edges: Vec<EdgeId>,
    total_distance: Meters,
    total_cost: RouteCost,
    visited_nodes: usize,
}

impl RouteResult {
    pub(super) fn new(
        algorithm: RoutingAlgorithm,
        path: Vec<NodeId>,
        edges: Vec<EdgeId>,
        total_distance: Meters,
        total_cost: RouteCost,
        visited_nodes: usize,
    ) -> Self {
        Self {
            algorithm,
            path,
            edges,
            total_distance,
            total_cost,
            visited_nodes,
        }
    }

    /// Returns the algorithm used for this result.
    #[must_use]
    pub const fn algorithm(&self) -> RoutingAlgorithm {
        self.algorithm
    }

    /// Returns the ordered node path from source through destination.
    #[must_use]
    pub fn path(&self) -> &[NodeId] {
        &self.path
    }

    /// Returns the ordered directed edges connecting [`Self::path`].
    #[must_use]
    pub fn edges(&self) -> &[EdgeId] {
        &self.edges
    }

    /// Returns total physical distance independently of the selected cost model.
    #[must_use]
    pub const fn total_distance(&self) -> Meters {
        self.total_distance
    }

    /// Returns the accumulated value selected by the cost model.
    #[must_use]
    pub const fn total_cost(&self) -> RouteCost {
        self.total_cost
    }

    /// Returns the number of nodes finalized by the search.
    #[must_use]
    pub const fn visited_nodes(&self) -> usize {
        self.visited_nodes
    }
}
