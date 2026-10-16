//! Checked, renderer-neutral canonical route geometry. No graph substitution or snapping.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    geo::{CanonicalCoordinate, Meters},
    graph::{EdgeId, FrozenGraph, NodeId, Orientation, RoadSegmentId},
};

/// Independent export admission limits, unrelated to routing search budgets.
#[derive(Debug, Clone, Copy)]
pub struct GeometryLimits {
    /// Maximum traversal occurrences.
    pub occurrences: usize,
    /// Maximum embedded E7 vertices, including duplicated occurrence endpoints.
    pub points: usize,
}
impl Default for GeometryLimits {
    fn default() -> Self {
        Self {
            occurrences: 20_000,
            points: 200_000,
        }
    }
}

/// Explicit checked-export failure; failure never returns repaired geometry.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GeometryError {
    /// Original graph dependency was not supplied.
    #[error("original graph unavailable")]
    Unavailable,
    /// Full semantic digest differs, checked before resolving local IDs.
    #[error("graph digest mismatch")]
    GraphMismatch,
    /// Unsupported anchor/encoding semantics.
    #[error("unsupported geometry semantics")]
    Unsupported,
    /// Invalid IDs, topology, endpoints, or degenerate ordinary segment.
    #[error("invalid route geometry")]
    Invalid,
    /// Independent export budget exceeded; authoritative route is unchanged.
    #[error("geometry export limit exceeded")]
    LimitExceeded,
}

/// An occurrence, identified by path identity plus ordinal, including its own endpoints.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TraversalOccurrence {
    /// Zero-based ordinal within this immutable spatial path.
    pub ordinal: usize,
    /// Graph-local directed traversal.
    pub edge: EdgeId,
    /// Graph-local canonical segment.
    pub segment: RoadSegmentId,
    /// Direction relative to the segment pool.
    pub orientation: Orientation,
    /// Directed source.
    pub from: NodeId,
    /// Directed target.
    pub to: NodeId,
    /// Inclusive first vertex in `points_e7`.
    pub point_start: usize,
    /// Exclusive vertex boundary; no boundary vertices are deduplicated.
    pub point_end: usize,
    /// Canonical segment meters, not rendered-line distance.
    pub distance_meters: Meters,
}

/// Exact canonical geometry; a no-travel path has one point and no occurrences.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CheckedRouteGeometry {
    /// Geometry contract version.
    pub schema_version: u32,
    /// WGS84 integer E7; fields name axes explicitly. No altitude.
    pub encoding: &'static str,
    /// Full authoritative graph digest.
    pub graph_digest: String,
    /// Versioned SHA-256 over graph digest, ordered nodes and directed edges.
    pub path_id: String,
    /// Ordered graph-local node anchors, including the no-travel anchor.
    pub nodes: Vec<NodeId>,
    /// Exact original directed vertices, including multiplicity and joint duplicates.
    pub points_e7: Vec<CanonicalCoordinate>,
    /// Ordered occurrences with explicit boundaries.
    pub occurrences: Vec<TraversalOccurrence>,
    /// True only for one node, no traversals.
    pub no_travel: bool,
}

/// Resolves a detached node-anchored path against its original graph.
///
/// # Errors
/// Returns a typed failure before allocating geometry when limits, graph identity,
/// IDs, directed topology, endpoint association or positive segment length fail.
#[allow(clippy::too_many_lines)]
pub fn checked_route_geometry(
    graph: Option<&FrozenGraph>,
    graph_digest: &str,
    nodes: &[NodeId],
    edges: &[EdgeId],
    limits: GeometryLimits,
) -> Result<CheckedRouteGeometry, GeometryError> {
    let graph = graph.ok_or(GeometryError::Unavailable)?;
    if graph.metadata().snapshot_digest() != graph_digest {
        return Err(GeometryError::GraphMismatch);
    }
    if edges.len() > limits.occurrences {
        return Err(GeometryError::LimitExceeded);
    }
    if nodes.is_empty() || nodes.len().checked_sub(1) != Some(edges.len()) {
        return Err(GeometryError::Invalid);
    }
    let mut point_count = usize::from(edges.is_empty());
    for (index, id) in edges.iter().enumerate() {
        let edge = graph.edge(*id).ok_or(GeometryError::Invalid)?;
        if edge.from() != nodes[index] || edge.to() != nodes[index + 1] {
            return Err(GeometryError::Invalid);
        }
        let segment = graph
            .segment(edge.segment())
            .ok_or(GeometryError::Invalid)?;
        if segment.distance().value() <= 0.0 {
            return Err(GeometryError::Invalid);
        }
        let shape = graph
            .segment_geometry(edge.segment())
            .map_err(|_| GeometryError::Invalid)?;
        if shape.len() < 2
            || shape
                .iter()
                .any(|p| CanonicalCoordinate::new(p.latitude_e7(), p.longitude_e7()).is_err())
        {
            return Err(GeometryError::Invalid);
        }
        let from = graph
            .node(edge.from())
            .ok_or(GeometryError::Invalid)?
            .canonical_coordinate();
        let to = graph
            .node(edge.to())
            .ok_or(GeometryError::Invalid)?
            .canonical_coordinate();
        let (first, last) = match edge.orientation() {
            Orientation::Forward => (shape.first(), shape.last()),
            Orientation::Reverse => (shape.last(), shape.first()),
        };
        if first != Some(&from) || last != Some(&to) {
            return Err(GeometryError::Invalid);
        }
        point_count = point_count
            .checked_add(shape.len())
            .ok_or(GeometryError::LimitExceeded)?;
        if point_count > limits.points {
            return Err(GeometryError::LimitExceeded);
        }
    }
    if point_count > limits.points {
        return Err(GeometryError::LimitExceeded);
    }
    let anchor = graph
        .node(nodes[0])
        .ok_or(GeometryError::Invalid)?
        .canonical_coordinate();
    CanonicalCoordinate::new(anchor.latitude_e7(), anchor.longitude_e7())
        .map_err(|_| GeometryError::Invalid)?;
    let mut points = Vec::with_capacity(point_count);
    let mut occurrences = Vec::with_capacity(edges.len());
    for (ordinal, id) in edges.iter().enumerate() {
        let edge = graph.edge(*id).ok_or(GeometryError::Invalid)?;
        let shape = graph
            .segment_geometry(edge.segment())
            .map_err(|_| GeometryError::Invalid)?;
        let point_start = points.len();
        match edge.orientation() {
            Orientation::Forward => points.extend_from_slice(shape),
            Orientation::Reverse => points.extend(shape.iter().rev().copied()),
        }
        occurrences.push(TraversalOccurrence {
            ordinal,
            edge: *id,
            segment: edge.segment(),
            orientation: edge.orientation(),
            from: edge.from(),
            to: edge.to(),
            point_start,
            point_end: points.len(),
            distance_meters: graph
                .segment(edge.segment())
                .ok_or(GeometryError::Invalid)?
                .distance(),
        });
    }
    if edges.is_empty() {
        points.push(anchor);
    }
    let mut hash = Sha256::new();
    hash.update(b"roadrunner/spatial-path/v1\0");
    hash.update(graph_digest.as_bytes());
    hash.update((nodes.len() as u64).to_le_bytes());
    for node in nodes {
        hash.update(node.value().to_le_bytes());
    }
    hash.update((edges.len() as u64).to_le_bytes());
    for edge in edges {
        hash.update(edge.value().to_le_bytes());
    }
    Ok(CheckedRouteGeometry {
        schema_version: 1,
        encoding: "wgs84_e7",
        graph_digest: graph_digest.to_owned(),
        path_id: format!("path-v1:{:x}", hash.finalize()),
        nodes: nodes.to_vec(),
        points_e7: points,
        occurrences,
        no_travel: edges.is_empty(),
    })
}
