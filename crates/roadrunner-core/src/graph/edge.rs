use serde::{Deserialize, Serialize};

use crate::geo::{Meters, Seconds};

use super::{EdgeId, NodeId};

/// A directed connection from one graph node to another.
///
/// A one-way road is one edge. A bidirectional road is represented by two edges
/// with opposite endpoints. Distance and base travel time remain independent of
/// the cost model selected by a routing request.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    id: EdgeId,
    from: NodeId,
    to: NodeId,
    distance: Meters,
    base_travel_time: Seconds,
}

impl Edge {
    /// Creates a directed edge with validated distance and base travel time.
    #[must_use]
    pub const fn new(
        id: EdgeId,
        from: NodeId,
        to: NodeId,
        distance: Meters,
        base_travel_time: Seconds,
    ) -> Self {
        Self {
            id,
            from,
            to,
            distance,
            base_travel_time,
        }
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

    /// Returns the edge distance.
    #[must_use]
    pub const fn distance(self) -> Meters {
        self.distance
    }

    /// Returns the edge's travel time before dynamic adjustments.
    #[must_use]
    pub const fn base_travel_time(self) -> Seconds {
        self.base_travel_time
    }
}
