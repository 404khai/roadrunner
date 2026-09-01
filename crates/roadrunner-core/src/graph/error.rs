use thiserror::Error;

use super::{EdgeId, NodeId};

/// Errors produced while constructing or querying a graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum GraphError {
    /// A node already uses the supplied identity.
    #[error("node {id} already exists")]
    DuplicateNode {
        /// The duplicated node identity.
        id: NodeId,
    },

    /// An edge already uses the supplied identity.
    #[error("edge {id} already exists")]
    DuplicateEdge {
        /// The duplicated edge identity.
        id: EdgeId,
    },

    /// A requested node does not exist.
    #[error("node {id} does not exist")]
    NodeNotFound {
        /// The missing node identity.
        id: NodeId,
    },

    /// A requested edge does not exist.
    #[error("edge {id} does not exist")]
    EdgeNotFound {
        /// The missing edge identity.
        id: EdgeId,
    },

    /// A new edge references a source node that is absent.
    #[error("edge {edge_id} references missing source node {node_id}")]
    MissingSourceNode {
        /// The edge being inserted.
        edge_id: EdgeId,
        /// The missing source node.
        node_id: NodeId,
    },

    /// A new edge references a destination node that is absent.
    #[error("edge {edge_id} references missing destination node {node_id}")]
    MissingDestinationNode {
        /// The edge being inserted.
        edge_id: EdgeId,
        /// The missing destination node.
        node_id: NodeId,
    },

    /// Internal node and adjacency storage disagree.
    #[error("node {node_id} has no adjacency list")]
    MissingAdjacency {
        /// The node whose adjacency list is absent.
        node_id: NodeId,
    },

    /// Internal edge identity and adjacency storage disagree.
    #[error("edge {edge_id} is not stored under source node {source_node}")]
    MissingStoredEdge {
        /// The edge missing from its expected adjacency list.
        edge_id: EdgeId,
        /// The expected source node.
        source_node: NodeId,
    },
}
