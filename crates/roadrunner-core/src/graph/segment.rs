use serde::Serialize;

use crate::geo::Meters;

use super::{NodeId, RoadSegmentId};

/// A contiguous range in the canonical geometry point pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GeometryRange {
    start: u32,
    len: u32,
}

impl GeometryRange {
    pub(super) const fn new(start: u32, len: u32) -> Self {
        Self { start, len }
    }
    /// Returns the first point index.
    #[must_use]
    pub const fn start(self) -> u32 {
        self.start
    }
    /// Returns the number of points.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.len
    }
    /// Returns whether the range is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }
}

/// A physical normalized road corridor with canonical geometry stored once.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RoadSegment {
    id: RoadSegmentId,
    canonical_from: NodeId,
    canonical_to: NodeId,
    geometry: GeometryRange,
    distance: Meters,
}

impl RoadSegment {
    pub(super) const fn new(
        id: RoadSegmentId,
        canonical_from: NodeId,
        canonical_to: NodeId,
        geometry: GeometryRange,
        distance: Meters,
    ) -> Self {
        Self {
            id,
            canonical_from,
            canonical_to,
            geometry,
            distance,
        }
    }
    /// Returns the snapshot-local segment identity.
    #[must_use]
    pub const fn id(self) -> RoadSegmentId {
        self.id
    }
    /// Returns the first endpoint in canonical geometry order.
    #[must_use]
    pub const fn canonical_from(self) -> NodeId {
        self.canonical_from
    }
    /// Returns the last endpoint in canonical geometry order.
    #[must_use]
    pub const fn canonical_to(self) -> NodeId {
        self.canonical_to
    }
    /// Returns the canonical geometry range.
    #[must_use]
    pub const fn geometry(self) -> GeometryRange {
        self.geometry
    }
    /// Returns compiler-derived physical length.
    #[must_use]
    pub const fn distance(self) -> Meters {
        self.distance
    }
}
