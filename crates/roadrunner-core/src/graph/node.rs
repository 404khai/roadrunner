use serde::{Deserialize, Serialize};

use crate::geo::Coordinate;

use super::NodeId;

/// A vertex in Roadrunner's internal road-network graph.
///
/// Coordinates use Roadrunner's validated geographic domain type rather than raw
/// latitude and longitude values.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Node {
    id: NodeId,
    coordinate: Coordinate,
}

impl Node {
    /// Creates a graph node with a stable identity and coordinate.
    #[must_use]
    pub const fn new(id: NodeId, coordinate: Coordinate) -> Self {
        Self { id, coordinate }
    }

    /// Returns the node's identity.
    #[must_use]
    pub const fn id(self) -> NodeId {
        self.id
    }

    /// Returns the node's geographic coordinate.
    #[must_use]
    pub const fn coordinate(self) -> Coordinate {
        self.coordinate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_its_validated_coordinate() {
        let coordinate = Coordinate::new(6.5244, 3.3792);
        let Ok(coordinate) = coordinate else {
            panic!("expected the Lagos coordinate to be valid");
        };
        let node = Node::new(NodeId::new(1), coordinate);

        assert_eq!(node.id(), NodeId::new(1));
        assert_eq!(node.coordinate(), coordinate);
    }
}
