use serde::{Deserialize, Serialize};

use crate::geo::{KilometersPerHour, Seconds};

use super::{EdgeId, NodeId, RoadSegmentId};

/// Orientation of a traversal relative to canonical segment geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    /// Canonical geometry order.
    Forward,
    /// Reverse canonical geometry order.
    Reverse,
}

/// Normalized static access attached to a potentially traversable edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessClass {
    /// Unconditionally available under static profile rules.
    General,
    /// Requires request-time authorization or endpoint semantics.
    Contextual,
}

/// Compiled directional attributes supplied to graph construction.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EdgeProperties {
    effective_free_flow_speed: KilometersPerHour,
    legal_speed_limit: Option<KilometersPerHour>,
    access: AccessClass,
}

impl EdgeProperties {
    /// Creates directional routing attributes.
    #[must_use]
    pub const fn new(
        effective_free_flow_speed: KilometersPerHour,
        legal_speed_limit: Option<KilometersPerHour>,
        access: AccessClass,
    ) -> Self {
        Self {
            effective_free_flow_speed,
            legal_speed_limit,
            access,
        }
    }
    /// Returns the compiled effective free-flow speed.
    #[must_use]
    pub const fn effective_free_flow_speed(self) -> KilometersPerHour {
        self.effective_free_flow_speed
    }
    /// Returns the parsed legal speed limit when present.
    #[must_use]
    pub const fn legal_speed_limit(self) -> Option<KilometersPerHour> {
        self.legal_speed_limit
    }
    /// Returns normalized static/contextual access.
    #[must_use]
    pub const fn access(self) -> AccessClass {
        self.access
    }
}

/// A directed, potentially permitted traversal of a physical road segment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DirectedEdge {
    id: EdgeId,
    segment: RoadSegmentId,
    orientation: Orientation,
    from: NodeId,
    to: NodeId,
    properties: EdgeProperties,
    free_flow_travel_time: Seconds,
}

impl DirectedEdge {
    pub(super) const fn new(
        id: EdgeId,
        segment: RoadSegmentId,
        orientation: Orientation,
        from: NodeId,
        to: NodeId,
        properties: EdgeProperties,
        free_flow_travel_time: Seconds,
    ) -> Self {
        Self {
            id,
            segment,
            orientation,
            from,
            to,
            properties,
            free_flow_travel_time,
        }
    }
    /// Returns the snapshot-local edge identity.
    #[must_use]
    pub const fn id(self) -> EdgeId {
        self.id
    }
    /// Returns the physical segment identity.
    #[must_use]
    pub const fn segment(self) -> RoadSegmentId {
        self.segment
    }
    /// Returns traversal orientation.
    #[must_use]
    pub const fn orientation(self) -> Orientation {
        self.orientation
    }
    /// Returns the source node.
    #[must_use]
    pub const fn from(self) -> NodeId {
        self.from
    }
    /// Returns the target node.
    #[must_use]
    pub const fn to(self) -> NodeId {
        self.to
    }
    /// Returns directional attributes.
    #[must_use]
    pub const fn properties(self) -> EdgeProperties {
        self.properties
    }
    /// Returns deterministic uncongested traversal time.
    #[must_use]
    pub const fn free_flow_travel_time(self) -> Seconds {
        self.free_flow_travel_time
    }
}
