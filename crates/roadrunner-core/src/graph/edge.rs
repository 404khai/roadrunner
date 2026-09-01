use serde::{Deserialize, Serialize};

use super::{EdgeId, NodeId};

/// A directed connection from one graph node to another.
///
/// A one-way road is one edge. A bidirectional road is represented by two edges
/// with opposite endpoints. Geographic and cost attributes arrive in later phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    id: EdgeId,
    from: NodeId,
    to: NodeId,
}

impl Edge {
    /// Creates a directed edge between two nodes.
    #[must_use]
    pub const fn new(id: EdgeId, from: NodeId, to: NodeId) -> Self {
        Self { id, from, to }
    }

    /// Returns the edge's identity.
    #[must_use]
    pub const fn id(self) -> EdgeId {
        self.id
    }

    /// Returns the source node.
    #[must_use]
    pub const fn from(self) -> NodeId {
        self.from
    }

    /// Returns the destination node.
    #[must_use]
    pub const fn to(self) -> NodeId {
        self.to
    }
}
