use std::fmt;

use thiserror::Error;

use crate::cost::{CostError, CostKind};
use crate::geo::UnitError;
use crate::graph::{EdgeId, GraphError, NodeId};

/// Identifies an endpoint when route validation fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteEndpoint {
    /// The requested route source.
    Source,
    /// The requested route destination.
    Destination,
}

impl fmt::Display for RouteEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source => formatter.write_str("source"),
            Self::Destination => formatter.write_str("destination"),
        }
    }
}

/// Errors produced while calculating or reconstructing a route.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum RoutingError {
    /// A requested endpoint is absent from the graph.
    #[error("{endpoint} node {node_id} does not exist")]
    NodeNotFound {
        /// Which route endpoint is absent.
        endpoint: RouteEndpoint,
        /// The missing node identity.
        node_id: NodeId,
    },

    /// No directed path connects two valid endpoint nodes.
    #[error("no route from node {source_node} to node {destination}")]
    NoRoute {
        /// The valid source node.
        source_node: NodeId,
        /// The valid destination node.
        destination: NodeId,
    },

    /// Reading graph adjacency failed.
    #[error("graph query failed: {source}")]
    Graph {
        /// The underlying graph error.
        #[source]
        source: GraphError,
    },

    /// A cost model rejected an edge.
    #[error("cost evaluation failed for edge {edge_id}: {source}")]
    CostEvaluation {
        /// The edge being evaluated.
        edge_id: EdgeId,
        /// The underlying cost error.
        #[source]
        source: CostError,
    },

    /// A model returned a cost kind different from the kind it advertises.
    #[error("edge {edge_id} returned {actual} cost; expected {expected}")]
    CostKindMismatch {
        /// The edge being evaluated.
        edge_id: EdgeId,
        /// The model's advertised cost kind.
        expected: CostKind,
        /// The returned cost kind.
        actual: CostKind,
    },

    /// An edge references a node that is missing during heuristic preparation.
    #[error("edge {edge_id} references missing node {node_id}")]
    MissingEdgeNode {
        /// The edge with the invalid endpoint.
        edge_id: EdgeId,
        /// The missing endpoint node.
        node_id: NodeId,
    },

    /// A node needed for heuristic evaluation is missing from the graph.
    #[error("heuristic node {node_id} is missing")]
    MissingHeuristicNode {
        /// The missing node identity.
        node_id: NodeId,
    },

    /// Constructing a node's heuristic cost failed.
    #[error("heuristic evaluation failed for node {node_id}: {source}")]
    HeuristicEvaluation {
        /// The node being estimated.
        node_id: NodeId,
        /// The underlying cost validation error.
        #[source]
        source: CostError,
    },

    /// Combining path and heuristic costs failed.
    #[error("estimated-total cost failed for node {node_id}: {source}")]
    EstimatedTotal {
        /// The node whose estimated total could not be represented.
        node_id: NodeId,
        /// The underlying cost validation error.
        #[source]
        source: CostError,
    },

    /// Accumulating route cost failed.
    #[error("cost accumulation failed at edge {edge_id}: {source}")]
    CostAccumulation {
        /// The edge whose cost could not be accumulated.
        edge_id: EdgeId,
        /// The underlying cost error.
        #[source]
        source: CostError,
    },

    /// Accumulating physical route distance failed.
    #[error("distance accumulation failed at edge {edge_id}: {source}")]
    DistanceAccumulation {
        /// The edge whose distance could not be accumulated.
        edge_id: EdgeId,
        /// The underlying unit error.
        #[source]
        source: UnitError,
    },

    /// The predecessor chain ended before reaching the source.
    #[error("route predecessor is missing for node {node_id}")]
    MissingPredecessor {
        /// The node without a predecessor edge.
        node_id: NodeId,
    },

    /// A predecessor references an edge no longer present in the graph.
    #[error("route predecessor edge {edge_id} is missing")]
    MissingRouteEdge {
        /// The absent edge identity.
        edge_id: EdgeId,
    },

    /// A predecessor edge does not lead to the expected node.
    #[error("predecessor edge {edge_id} leads to {actual_to}; expected {expected_to}")]
    InvalidPredecessorEdge {
        /// The invalid predecessor edge.
        edge_id: EdgeId,
        /// The node expected at the edge destination.
        expected_to: NodeId,
        /// The actual edge destination.
        actual_to: NodeId,
    },

    /// The predecessor chain contains more nodes than the graph.
    #[error("route predecessor chain contains a cycle")]
    PredecessorCycle,
}
