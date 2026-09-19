use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::geo::{CanonicalCoordinate, KilometersPerHour, Meters, Seconds};

use super::{
    AccessClass, DirectedEdge, EdgeId, EdgeProperties, FrozenGraph, GeometryRange,
    GraphBuildIdentity, GraphMetadata, GraphSnapshotId, Node, NodeId, Orientation, RoadSegment,
    RoadSegmentId,
};

const MAGIC: &str = "ROADRUNNER_GRAPH";
const SCHEMA_VERSION: u32 = 1;
static TEMPORARY_ARTIFACT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize)]
struct Envelope {
    magic: String,
    schema_version: u32,
    payload_sha256: String,
    payload_json: String,
}

#[derive(Serialize, Deserialize)]
struct Payload {
    snapshot_id: u64,
    routing_profile: String,
    jurisdiction_policy: String,
    source_dataset: String,
    source_integrity: String,
    compiler_version: String,
    normalization_version: String,
    build_configuration: String,
    turn_restrictions_enforced: bool,
    nodes: Vec<CanonicalCoordinate>,
    segments: Vec<SegmentDto>,
    edges: Vec<EdgeDto>,
    adjacency_offsets: Vec<u64>,
    geometry: Vec<CanonicalCoordinate>,
}

#[derive(Serialize, Deserialize)]
struct SegmentDto {
    from: u32,
    to: u32,
    geometry_start: u32,
    geometry_len: u32,
    distance_meters: f64,
}

#[derive(Serialize, Deserialize)]
struct EdgeDto {
    segment: u32,
    orientation: Orientation,
    from: u32,
    to: u32,
    effective_speed_kph: f64,
    legal_speed_limit_kph: Option<f64>,
    access: AccessClass,
    free_flow_travel_time_seconds: f64,
}

/// Graph artifact encoding or validation failure.
#[derive(Debug, Error)]
pub enum GraphArtifactError {
    /// Filesystem publication failed.
    #[error("graph artifact {operation} failed: {source}")]
    Io {
        /// Publication operation.
        operation: &'static str,
        /// Underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// JSON encoding or decoding failed.
    #[error("graph artifact serialization failed: {source}")]
    Serialization {
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },
    /// Artifact framing is not recognized.
    #[error("invalid graph artifact magic")]
    InvalidMagic,
    /// Schema is not supported by this loader.
    #[error("unsupported graph artifact schema {version}")]
    UnsupportedSchema {
        /// Rejected schema version.
        version: u32,
    },
    /// Payload integrity verification failed.
    #[error("graph artifact integrity hash does not match payload")]
    IntegrityMismatch,
    /// A structural or semantic invariant failed.
    #[error("invalid graph artifact: {message}")]
    Invalid {
        /// Explainable validation failure.
        message: String,
    },
}

/// Encodes and atomically publishes a graph artifact in the target directory.
///
/// # Errors
///
/// Returns an error if encoding, temporary-file creation, durable writing, or
/// atomic rename fails.
pub fn write_graph_artifact_atomic(
    path: &Path,
    graph: &FrozenGraph,
) -> Result<(), GraphArtifactError> {
    let bytes = encode_graph_artifact(graph)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| invalid("artifact path requires a file name"))?;
    let sequence = TEMPORARY_ARTIFACT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".{}.tmp-{}-{sequence}",
        file_name.to_string_lossy(),
        std::process::id()
    ));
    let publication = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|source| GraphArtifactError::Io {
                operation: "temporary-file creation",
                source,
            })?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|source| GraphArtifactError::Io {
                operation: "temporary-file write",
                source,
            })?;
        std::fs::rename(&temporary, path).map_err(|source| GraphArtifactError::Io {
            operation: "atomic rename",
            source,
        })?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| GraphArtifactError::Io {
                operation: "directory synchronization",
                source,
            })
    })();
    if publication.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    publication
}

/// Encodes a frozen graph as a canonical validated artifact.
///
/// # Errors
///
/// Returns an error if deterministic JSON encoding fails.
pub fn encode_graph_artifact(graph: &FrozenGraph) -> Result<Vec<u8>, GraphArtifactError> {
    let payload = Payload {
        snapshot_id: graph.snapshot_id().value(),
        routing_profile: graph.metadata().routing_profile().to_owned(),
        jurisdiction_policy: graph.metadata().jurisdiction_policy().to_owned(),
        source_dataset: graph
            .metadata()
            .build_identity()
            .source_dataset()
            .to_owned(),
        source_integrity: graph
            .metadata()
            .build_identity()
            .source_integrity()
            .to_owned(),
        compiler_version: graph
            .metadata()
            .build_identity()
            .compiler_version()
            .to_owned(),
        normalization_version: graph
            .metadata()
            .build_identity()
            .normalization_version()
            .to_owned(),
        build_configuration: graph
            .metadata()
            .build_identity()
            .build_configuration()
            .to_owned(),
        turn_restrictions_enforced: graph.metadata().turn_restrictions_enforced(),
        nodes: graph
            .nodes()
            .iter()
            .map(|node| node.canonical_coordinate())
            .collect(),
        segments: graph
            .segments()
            .iter()
            .map(|segment| SegmentDto {
                from: segment.canonical_from().value(),
                to: segment.canonical_to().value(),
                geometry_start: segment.geometry().start(),
                geometry_len: segment.geometry().len(),
                distance_meters: segment.distance().value(),
            })
            .collect(),
        edges: graph
            .edges()
            .iter()
            .map(|edge| EdgeDto {
                segment: edge.segment().value(),
                orientation: edge.orientation(),
                from: edge.from().value(),
                to: edge.to().value(),
                effective_speed_kph: edge.properties().effective_free_flow_speed().value(),
                legal_speed_limit_kph: edge
                    .properties()
                    .legal_speed_limit()
                    .map(KilometersPerHour::value),
                access: edge.properties().access(),
                free_flow_travel_time_seconds: edge.free_flow_travel_time().value(),
            })
            .collect(),
        adjacency_offsets: graph
            .adjacency_offsets()
            .iter()
            .map(|value| *value as u64)
            .collect(),
        geometry: graph.geometry_points().to_vec(),
    };
    let payload_bytes = serde_json::to_vec(&payload)
        .map_err(|source| GraphArtifactError::Serialization { source })?;
    let payload_sha256 = format!("{:x}", Sha256::digest(&payload_bytes));
    let payload_json = String::from_utf8(payload_bytes)
        .map_err(|error| invalid(format!("canonical payload is not UTF-8: {error}")))?;
    serde_json::to_vec(&Envelope {
        magic: MAGIC.to_owned(),
        schema_version: SCHEMA_VERSION,
        payload_sha256,
        payload_json,
    })
    .map_err(|source| GraphArtifactError::Serialization { source })
}

/// Decodes and validates artifact bytes before returning a trusted graph.
///
/// # Errors
///
/// Returns an error for malformed, corrupt, incompatible, or noncanonical input.
pub fn decode_graph_artifact(bytes: &[u8]) -> Result<FrozenGraph, GraphArtifactError> {
    let envelope: Envelope = serde_json::from_slice(bytes)
        .map_err(|source| GraphArtifactError::Serialization { source })?;
    if envelope.magic != MAGIC {
        return Err(GraphArtifactError::InvalidMagic);
    }
    if envelope.schema_version != SCHEMA_VERSION {
        return Err(GraphArtifactError::UnsupportedSchema {
            version: envelope.schema_version,
        });
    }
    let actual_hash = format!("{:x}", Sha256::digest(envelope.payload_json.as_bytes()));
    if actual_hash != envelope.payload_sha256 {
        return Err(GraphArtifactError::IntegrityMismatch);
    }
    let payload = serde_json::from_str(&envelope.payload_json)
        .map_err(|source| GraphArtifactError::Serialization { source })?;
    validate_payload(payload)
}

fn invalid(message: impl Into<String>) -> GraphArtifactError {
    GraphArtifactError::Invalid {
        message: message.into(),
    }
}

#[allow(clippy::too_many_lines)]
fn validate_payload(payload: Payload) -> Result<FrozenGraph, GraphArtifactError> {
    if payload.routing_profile.is_empty()
        || payload.jurisdiction_policy.is_empty()
        || payload.source_dataset.is_empty()
        || payload.source_integrity.is_empty()
        || payload.compiler_version.is_empty()
        || payload.normalization_version.is_empty()
        || payload.build_configuration.is_empty()
    {
        return Err(invalid("graph build metadata fields must be non-empty"));
    }
    if payload.turn_restrictions_enforced {
        return Err(invalid(
            "schema version 1 cannot contain enforced turn restrictions",
        ));
    }
    let nodes: Vec<Node> = payload
        .nodes
        .into_iter()
        .enumerate()
        .map(|(index, coordinate)| {
            u32::try_from(index)
                .map(|value| Node::new(NodeId::new(value), coordinate))
                .map_err(|_| invalid("node count exceeds dense ID width"))
        })
        .collect::<Result<_, _>>()?;

    let mut segments = Vec::with_capacity(payload.segments.len());
    for (index, dto) in payload.segments.into_iter().enumerate() {
        let id = RoadSegmentId::new(
            u32::try_from(index).map_err(|_| invalid("segment count exceeds dense ID width"))?,
        );
        let from = NodeId::new(dto.from);
        let to = NodeId::new(dto.to);
        let from_node = nodes
            .get(dto.from as usize)
            .ok_or_else(|| invalid("segment source is out of range"))?;
        let to_node = nodes
            .get(dto.to as usize)
            .ok_or_else(|| invalid("segment target is out of range"))?;
        let start = dto.geometry_start as usize;
        let len = dto.geometry_len as usize;
        let end = start
            .checked_add(len)
            .ok_or_else(|| invalid("geometry range overflows"))?;
        let points = payload
            .geometry
            .get(start..end)
            .ok_or_else(|| invalid("segment geometry is out of range"))?;
        if points.len() < 2 {
            return Err(invalid("segment geometry requires at least two points"));
        }
        if points.first() != Some(&from_node.canonical_coordinate())
            || points.last() != Some(&to_node.canonical_coordinate())
        {
            return Err(invalid("segment geometry endpoints do not match nodes"));
        }
        let distance =
            Meters::new(dto.distance_meters).map_err(|error| invalid(error.to_string()))?;
        segments.push(RoadSegment::new(
            id,
            from,
            to,
            GeometryRange::new(dto.geometry_start, dto.geometry_len),
            distance,
        ));
    }

    if payload.adjacency_offsets.len() != nodes.len() + 1 {
        return Err(invalid("adjacency offset length is invalid"));
    }
    let mut adjacency_offsets = Vec::with_capacity(payload.adjacency_offsets.len());
    let mut previous = 0_usize;
    for value in payload.adjacency_offsets {
        let converted = usize::try_from(value)
            .map_err(|_| invalid("adjacency offset exceeds platform width"))?;
        if converted < previous {
            return Err(invalid("adjacency offsets are not monotonic"));
        }
        adjacency_offsets.push(converted);
        previous = converted;
    }
    if adjacency_offsets.first() != Some(&0)
        || adjacency_offsets.last() != Some(&payload.edges.len())
    {
        return Err(invalid("adjacency offsets do not span the edge array"));
    }

    let mut edges = Vec::with_capacity(payload.edges.len());
    let mut previous_key = None;
    for (index, dto) in payload.edges.into_iter().enumerate() {
        let id = EdgeId::new(
            u32::try_from(index).map_err(|_| invalid("edge count exceeds dense ID width"))?,
        );
        let segment_id = RoadSegmentId::new(dto.segment);
        let segment = segments
            .get(dto.segment as usize)
            .ok_or_else(|| invalid("edge segment is out of range"))?;
        let expected = match dto.orientation {
            Orientation::Forward => (segment.canonical_from(), segment.canonical_to()),
            Orientation::Reverse => (segment.canonical_to(), segment.canonical_from()),
        };
        if (dto.from, dto.to) != (expected.0.value(), expected.1.value()) {
            return Err(invalid("edge orientation and endpoints disagree"));
        }
        let key = (dto.from, dto.to, dto.segment, dto.orientation);
        if previous_key.is_some_and(|previous| previous > key) {
            return Err(invalid("edges are not in canonical adjacency order"));
        }
        previous_key = Some(key);
        let speed = KilometersPerHour::new(dto.effective_speed_kph)
            .map_err(|error| invalid(error.to_string()))?;
        if speed.value() == 0.0 {
            return Err(invalid("effective speed must be positive"));
        }
        let legal = dto
            .legal_speed_limit_kph
            .map(KilometersPerHour::new)
            .transpose()
            .map_err(|error| invalid(error.to_string()))?;
        let travel_time = Seconds::new(dto.free_flow_travel_time_seconds)
            .map_err(|error| invalid(error.to_string()))?;
        edges.push(DirectedEdge::new(
            id,
            segment_id,
            dto.orientation,
            expected.0,
            expected.1,
            EdgeProperties::new(speed, legal, dto.access),
            travel_time,
        ));
    }
    for node_index in 0..nodes.len() {
        if edges[adjacency_offsets[node_index]..adjacency_offsets[node_index + 1]]
            .iter()
            .any(|edge| edge.from().value() as usize != node_index)
        {
            return Err(invalid("edge is stored under the wrong adjacency source"));
        }
    }

    Ok(FrozenGraph::from_validated_parts(
        GraphSnapshotId::new(payload.snapshot_id),
        GraphMetadata::new(
            payload.routing_profile,
            payload.jurisdiction_policy,
            GraphBuildIdentity::from_artifact(
                payload.source_dataset,
                payload.source_integrity,
                payload.compiler_version,
                payload.normalization_version,
                payload.build_configuration,
            ),
        ),
        nodes,
        segments,
        edges,
        adjacency_offsets,
        payload.geometry,
    ))
}
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
