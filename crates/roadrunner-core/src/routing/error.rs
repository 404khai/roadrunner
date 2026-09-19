use thiserror::Error;

use crate::cost::{CostError, CostKind, HeuristicPolicy, SearchCapability};
use crate::geo::UnitError;
use crate::graph::{EdgeId, GraphError, NodeId};

/// Identifies an invalid route endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteEndpoint {
    /// Route origin.
    Source,
    /// Route destination.
    Destination,
}

impl std::fmt::Display for RouteEndpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Source => "source",
            Self::Destination => "destination",
        })
    }
}

/// Errors produced while calculating or reconstructing a route.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum RoutingError {
    /// A requested endpoint is absent.
    #[error("{endpoint} node {node_id} does not exist")]
    NodeNotFound {
        /// Endpoint role.
        endpoint: RouteEndpoint,
        /// Missing node.
        node_id: NodeId,
    },
    /// No permitted path connects valid endpoints.
    #[error("no route from node {source_node} to node {destination}")]
    NoRoute {
        /// Valid origin.
        source_node: NodeId,
        /// Valid destination.
        destination: NodeId,
    },
    /// Graph lookup failed.
    #[error("graph query failed: {source}")]
    Graph {
        /// Underlying graph error.
        #[source]
        source: GraphError,
    },
    /// Traversal evaluation failed.
    #[error("traversal evaluation failed for edge {edge_id}: {source}")]
    TraversalEvaluation {
        /// Edge being evaluated.
        edge_id: EdgeId,
        /// Underlying evaluator error.
        #[source]
        source: CostError,
    },
    /// An evaluator produced the wrong objective kind.
    #[error("edge {edge_id} returned {actual} cost; expected {expected}")]
    CostKindMismatch {
        /// Edge producing the mismatch.
        edge_id: EdgeId,
        /// Evaluator's declared kind.
        expected: CostKind,
        /// Returned kind.
        actual: CostKind,
    },
    /// A node-state algorithm cannot serve the requested capability.
    #[error("routing algorithm does not support evaluator capability {capability:?}")]
    UnsupportedCapability {
        /// Unsupported requirement.
        capability: SearchCapability,
    },
    /// Heuristic and evaluator objective kinds differ.
    #[error("heuristic produces {heuristic}; evaluator produces {evaluator}")]
    HeuristicKindMismatch {
        /// Heuristic objective kind.
        heuristic: CostKind,
        /// Evaluator objective kind.
        evaluator: CostKind,
    },
    /// The heuristic does not support the evaluator capability.
    #[error("heuristic is incompatible with evaluator capability {capability:?}")]
    HeuristicCapabilityMismatch {
        /// Unsupported evaluator capability.
        capability: SearchCapability,
    },
    /// The evaluator has not proved compatibility with the selected heuristic policy.
    #[error("evaluator is incompatible with heuristic policy {policy:?}")]
    HeuristicPolicyMismatch {
        /// Rejected policy.
        policy: HeuristicPolicy,
    },
    /// Prevalidated heuristic parameters belong to another graph snapshot.
    #[error("heuristic parameters were validated for a different graph snapshot")]
    HeuristicGraphMismatch,
    /// FIFO earliest-arrival evaluation did not use travel time as objective.
    #[error("FIFO edge {edge_id} objective must equal its travel time")]
    FifoObjectiveMismatch {
        /// Invalid FIFO edge.
        edge_id: EdgeId,
    },
    /// Objective accumulation failed.
    #[error("objective accumulation failed at edge {edge_id}: {source}")]
    CostAccumulation {
        /// Edge whose objective overflowed.
        edge_id: EdgeId,
        /// Underlying cost error.
        #[source]
        source: CostError,
    },
    /// Elapsed-time accumulation failed.
    #[error("elapsed-time accumulation failed at edge {edge_id}: {source}")]
    TimeAccumulation {
        /// Edge whose elapsed time overflowed.
        edge_id: EdgeId,
        /// Underlying unit error.
        #[source]
        source: UnitError,
    },
    /// Heuristic evaluation failed.
    #[error("heuristic evaluation failed for node {node_id}: {source}")]
    HeuristicEvaluation {
        /// Node being estimated.
        node_id: NodeId,
        /// Underlying cost error.
        #[source]
        source: CostError,
    },
    /// Combining path and heuristic costs failed.
    #[error("estimated-total calculation failed for node {node_id}: {source}")]
    EstimatedTotal {
        /// Node whose total failed.
        node_id: NodeId,
        /// Underlying cost error.
        #[source]
        source: CostError,
    },
    /// Physical distance accumulation failed.
    #[error("distance accumulation failed at edge {edge_id}: {source}")]
    DistanceAccumulation {
        /// Edge whose distance overflowed.
        edge_id: EdgeId,
        /// Underlying unit error.
        #[source]
        source: UnitError,
    },
    /// A predecessor is absent.
    #[error("route predecessor is missing for node {node_id}")]
    MissingPredecessor {
        /// Node without a predecessor.
        node_id: NodeId,
    },
    /// A predecessor edge is absent.
    #[error("route predecessor edge {edge_id} is missing")]
    MissingRouteEdge {
        /// Missing edge.
        edge_id: EdgeId,
    },
    /// A predecessor does not lead to the expected node.
    #[error("predecessor edge {edge_id} leads to {actual_to}; expected {expected_to}")]
    InvalidPredecessorEdge {
        /// Invalid edge.
        edge_id: EdgeId,
        /// Expected edge target.
        expected_to: NodeId,
        /// Actual edge target.
        actual_to: NodeId,
    },
    /// The predecessor chain contains a cycle.
    #[error("route predecessor chain contains a cycle")]
    PredecessorCycle,
}
