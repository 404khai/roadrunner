use serde::{Deserialize, Serialize};

use super::NodeId;

/// A vertex in Roadrunner's internal road-network graph.
///
/// Geographic coordinates are added in Phase 3. Keeping this Phase 2 type
/// topology-only prevents raw coordinate values from leaking into the graph API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    id: NodeId,
}

impl Node {
    /// Creates a graph node with a stable identity.
    #[must_use]
    pub const fn new(id: NodeId) -> Self {
        Self { id }
    }

    /// Returns the node's identity.
    #[must_use]
    pub const fn id(self) -> NodeId {
        self.id
    }
}
