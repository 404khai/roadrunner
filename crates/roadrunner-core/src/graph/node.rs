use serde::Serialize;

use crate::geo::{CanonicalCoordinate, Coordinate};

use super::NodeId;

/// A routing decision vertex in one frozen graph snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Node {
    id: NodeId,
    coordinate: CanonicalCoordinate,
}

impl Node {
    pub(super) const fn new(id: NodeId, coordinate: CanonicalCoordinate) -> Self {
        Self { id, coordinate }
    }

    /// Returns the snapshot-local dense identity.
    #[must_use]
    pub const fn id(self) -> NodeId {
        self.id
    }

    /// Returns canonical fixed-point geometry.
    #[must_use]
    pub const fn canonical_coordinate(self) -> CanonicalCoordinate {
        self.coordinate
    }

    /// Returns the coordinate in decimal degrees for calculations and APIs.
    #[must_use]
    pub fn coordinate(self) -> Coordinate {
        self.coordinate.to_coordinate()
    }
}
