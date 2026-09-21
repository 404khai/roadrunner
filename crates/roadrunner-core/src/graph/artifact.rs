use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::geo::{CanonicalCoordinate, KilometersPerHour, Meters, Seconds};

use super::{
    AccessClass, DISTANCE_INVARIANT_TOLERANCE_METERS, DirectedEdge, EdgeId, EdgeProperties,
    FrozenGraph, GeometryRange, GraphBuildIdentity, GraphMetadata, GraphSnapshotId, Node, NodeId,
    Orientation, RoadSegment, RoadSegmentId,
};

const MAGIC: &str = "ROADRUNNER_GRAPH";
const SCHEMA_VERSION: u32 = 2;
static TEMPORARY_ARTIFACT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    magic: String,
    schema_version: u32,
    payload_sha256: String,
    payload_json: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    snapshot_id: u64,
    snapshot_digest: String,
    routing_profile: String,
    jurisdiction_policy: String,
    source_dataset: String,
    source_integrity: String,
    compiler_version: String,
    normalization_version: String,
    build_configuration: String,
    turn_restrictions_enforced: bool,
    nodes: Vec<CoordinateDto>,
    segments: Vec<SegmentDto>,
    edges: Vec<EdgeDto>,
    adjacency_offsets: Vec<u64>,
    geometry: Vec<CoordinateDto>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SegmentDto {
    from: u32,
    to: u32,
    geometry_start: u32,
    geometry_len: u32,
    distance_meters_bits: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EdgeDto {
    segment: u32,
    orientation: Orientation,
    from: u32,
    to: u32,
    effective_speed_kph_bits: u64,
    legal_speed_limit_kph_bits: Option<u64>,
    access: AccessClass,
    free_flow_travel_time_seconds_bits: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinateDto {
    latitude_e7: i32,
    longitude_e7: i32,
}

impl From<CanonicalCoordinate> for CoordinateDto {
    fn from(value: CanonicalCoordinate) -> Self {
        Self {
            latitude_e7: value.latitude_e7(),
            longitude_e7: value.longitude_e7(),
        }
    }
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
        snapshot_digest: graph.metadata().snapshot_digest().to_owned(),
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
            .map(|node| node.canonical_coordinate().into())
            .collect(),
        segments: graph
            .segments()
            .iter()
            .map(|segment| SegmentDto {
                from: segment.canonical_from().value(),
                to: segment.canonical_to().value(),
                geometry_start: segment.geometry().start(),
                geometry_len: segment.geometry().len(),
                distance_meters_bits: segment.distance().value().to_bits(),
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
                effective_speed_kph_bits: edge
                    .properties()
                    .effective_free_flow_speed()
                    .value()
                    .to_bits(),
                legal_speed_limit_kph_bits: edge
                    .properties()
                    .legal_speed_limit()
                    .map(KilometersPerHour::value)
                    .map(f64::to_bits),
                access: edge.properties().access(),
                free_flow_travel_time_seconds_bits: edge.free_flow_travel_time().value().to_bits(),
            })
            .collect(),
        adjacency_offsets: graph
            .adjacency_offsets()
            .iter()
            .map(|value| *value as u64)
            .collect(),
        geometry: graph
            .geometry_points()
            .iter()
            .copied()
            .map(Into::into)
            .collect(),
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
    let canonical_envelope = serde_json::to_vec(&envelope)
        .map_err(|source| GraphArtifactError::Serialization { source })?;
    if canonical_envelope != bytes {
        return Err(invalid("graph artifact framing is not canonical"));
    }
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
    let payload: Payload = serde_json::from_str(&envelope.payload_json)
        .map_err(|source| GraphArtifactError::Serialization { source })?;
    let canonical_payload = serde_json::to_string(&payload)
        .map_err(|source| GraphArtifactError::Serialization { source })?;
    if canonical_payload != envelope.payload_json {
        let position = canonical_payload
            .bytes()
            .zip(envelope.payload_json.bytes())
            .position(|(left, right)| left != right)
            .unwrap_or(canonical_payload.len().min(envelope.payload_json.len()));
        return Err(invalid(format!(
            "graph payload is not canonical at byte {position}"
        )));
    }
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
    if !is_sha256(&payload.snapshot_digest) {
        return Err(invalid(
            "graph snapshot digest is not a canonical SHA-256 value",
        ));
    }
    if payload.turn_restrictions_enforced {
        return Err(invalid(
            "schema version 1 cannot contain enforced turn restrictions",
        ));
    }
    let node_coordinates: Vec<CanonicalCoordinate> = payload
        .nodes
        .into_iter()
        .map(|coordinate| {
            CanonicalCoordinate::new(coordinate.latitude_e7, coordinate.longitude_e7)
                .map_err(|error| invalid(error.to_string()))
        })
        .collect::<Result<_, _>>()?;
    let geometry: Vec<CanonicalCoordinate> = payload
        .geometry
        .into_iter()
        .map(|coordinate| {
            CanonicalCoordinate::new(coordinate.latitude_e7, coordinate.longitude_e7)
                .map_err(|error| invalid(error.to_string()))
        })
        .collect::<Result<_, _>>()?;
    let nodes: Vec<Node> = node_coordinates
        .into_iter()
        .enumerate()
        .map(|(index, coordinate)| {
            u32::try_from(index)
                .map(|value| Node::new(NodeId::new(value), coordinate))
                .map_err(|_| invalid("node count exceeds dense ID width"))
        })
        .collect::<Result<_, _>>()?;

    let mut segments = Vec::with_capacity(payload.segments.len());
    let mut expected_geometry_start = 0_usize;
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
        if start != expected_geometry_start {
            return Err(invalid(
                "segment geometry ranges are not contiguous and canonical",
            ));
        }
        let len = dto.geometry_len as usize;
        let end = start
            .checked_add(len)
            .ok_or_else(|| invalid("geometry range overflows"))?;
        let points = geometry
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
        let distance = Meters::new(f64::from_bits(dto.distance_meters_bits))
            .map_err(|error| invalid(error.to_string()))?;
        if distance == Meters::ZERO {
            return Err(invalid("ordinary road segment distance must be positive"));
        }
        segments.push(RoadSegment::new(
            id,
            from,
            to,
            GeometryRange::new(dto.geometry_start, dto.geometry_len),
            distance,
        ));
        expected_geometry_start = end;
    }
    if expected_geometry_start != geometry.len() {
        return Err(invalid(
            "segment geometry ranges do not cover the geometry pool",
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
    let mut segment_orientations = vec![(false, false); segments.len()];
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
        if previous_key.is_some_and(|previous| previous >= key) {
            return Err(invalid("edges are not in canonical adjacency order"));
        }
        previous_key = Some(key);
        let orientation_slot = &mut segment_orientations[dto.segment as usize];
        let present = match dto.orientation {
            Orientation::Forward => &mut orientation_slot.0,
            Orientation::Reverse => &mut orientation_slot.1,
        };
        if *present {
            return Err(invalid("duplicate segment orientation traversal"));
        }
        *present = true;
        let speed = KilometersPerHour::new(f64::from_bits(dto.effective_speed_kph_bits))
            .map_err(|error| invalid(error.to_string()))?;
        if speed.value() == 0.0 {
            return Err(invalid("effective speed must be positive"));
        }
        let legal = dto
            .legal_speed_limit_kph_bits
            .map(f64::from_bits)
            .map(KilometersPerHour::new)
            .transpose()
            .map_err(|error| invalid(error.to_string()))?;
        let travel_time = Seconds::new(f64::from_bits(dto.free_flow_travel_time_seconds_bits))
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
    if segment_orientations
        .iter()
        .any(|(forward, reverse)| !forward && !reverse)
    {
        return Err(invalid("segment has no directed traversal"));
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
        GraphMetadata::with_snapshot_digest(
            payload.routing_profile,
            payload.jurisdiction_policy,
            GraphBuildIdentity::from_artifact(
                payload.source_dataset,
                payload.source_integrity,
                payload.compiler_version,
                payload.normalization_version,
                payload.build_configuration,
            ),
            payload.snapshot_digest,
        ),
        nodes,
        segments,
        edges,
        adjacency_offsets,
        geometry,
    ))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::graph::{BuilderNodeId, BuilderSegmentId, GraphBuilder};

    fn fixture() -> FrozenGraph {
        let from = CanonicalCoordinate::new(0, 0)
            .unwrap_or_else(|error| panic!("fixture coordinate: {error}"));
        let to = CanonicalCoordinate::new(0, 10_000)
            .unwrap_or_else(|error| panic!("fixture coordinate: {error}"));
        let speed =
            KilometersPerHour::new(30.0).unwrap_or_else(|error| panic!("fixture speed: {error}"));
        let mut builder = GraphBuilder::new(
            GraphSnapshotId::new(1),
            GraphMetadata::new(
                "test",
                "test",
                GraphBuildIdentity::new("fixture", "source", "v1", "default"),
            ),
        );
        assert!(builder.add_node(BuilderNodeId::new(1), from).is_ok());
        assert!(builder.add_node(BuilderNodeId::new(2), to).is_ok());
        assert!(
            builder
                .add_segment(
                    BuilderSegmentId::new(1),
                    BuilderNodeId::new(1),
                    BuilderNodeId::new(2),
                    vec![from, to],
                    Some(EdgeProperties::new(speed, None, AccessClass::General)),
                    None,
                )
                .is_ok()
        );
        builder
            .finalize()
            .unwrap_or_else(|error| panic!("fixture graph: {error}"))
    }

    fn mutate_payload(mutator: impl FnOnce(&mut Payload)) -> Vec<u8> {
        let encoded = encode_graph_artifact(&fixture())
            .unwrap_or_else(|error| panic!("fixture encoding: {error}"));
        let envelope: Envelope = serde_json::from_slice(&encoded)
            .unwrap_or_else(|error| panic!("fixture envelope: {error}"));
        let mut payload: Payload = serde_json::from_str(&envelope.payload_json)
            .unwrap_or_else(|error| panic!("fixture payload: {error}"));
        mutator(&mut payload);
        let payload_json = serde_json::to_string(&payload)
            .unwrap_or_else(|error| panic!("mutated payload: {error}"));
        let payload_sha256 = format!("{:x}", Sha256::digest(payload_json.as_bytes()));
        serde_json::to_vec(&Envelope {
            magic: MAGIC.to_owned(),
            schema_version: SCHEMA_VERSION,
            payload_sha256,
            payload_json,
        })
        .unwrap_or_else(|error| panic!("mutated envelope: {error}"))
    }

    #[test]
    fn rejects_invalid_coordinates_duplicate_edges_and_geometry_gaps() {
        let invalid_coordinate = mutate_payload(|payload| {
            payload.nodes[0].latitude_e7 = 900_000_001;
        });
        assert!(decode_graph_artifact(&invalid_coordinate).is_err());

        let duplicate_edge = mutate_payload(|payload| {
            let duplicate = EdgeDto {
                segment: payload.edges[0].segment,
                orientation: payload.edges[0].orientation,
                from: payload.edges[0].from,
                to: payload.edges[0].to,
                effective_speed_kph_bits: payload.edges[0].effective_speed_kph_bits,
                legal_speed_limit_kph_bits: payload.edges[0].legal_speed_limit_kph_bits,
                access: payload.edges[0].access,
                free_flow_travel_time_seconds_bits: payload.edges[0]
                    .free_flow_travel_time_seconds_bits,
            };
            payload.edges.push(duplicate);
            payload.adjacency_offsets[1] = 2;
            payload.adjacency_offsets[2] = 2;
        });
        assert!(decode_graph_artifact(&duplicate_edge).is_err());

        let geometry_gap = mutate_payload(|payload| {
            payload.geometry.insert(
                0,
                CoordinateDto {
                    latitude_e7: 0,
                    longitude_e7: 0,
                },
            );
            payload.segments[0].geometry_start = 1;
        });
        assert!(decode_graph_artifact(&geometry_gap).is_err());
    }

    #[test]
    fn deep_verification_rejects_rehashed_derived_distance_corruption() {
        let corrupted = mutate_payload(|payload| {
            let distance = f64::from_bits(payload.segments[0].distance_meters_bits);
            payload.segments[0].distance_meters_bits = (distance * 2.0).to_bits();
        });
        let graph = decode_graph_artifact(&corrupted)
            .unwrap_or_else(|error| panic!("structurally valid corruption: {error}"));
        assert!(verify_graph_deep(&graph).is_err());
    }
}

/// Independently recomputes expensive graph invariants.
///
/// # Errors
///
/// Returns an error when derived distance, traversal time, geometry, or
/// adjacency disagrees with the frozen graph.
pub fn verify_graph_deep(graph: &FrozenGraph) -> Result<(), GraphArtifactError> {
    let mut reconstructed_offsets = vec![0_usize; graph.node_count() + 1];
    for segment in graph.segments() {
        let points = graph
            .segment_geometry(segment.id())
            .map_err(|error| invalid(error.to_string()))?;
        let mut derived = Meters::ZERO;
        for pair in points.windows(2) {
            derived = derived
                .checked_add(crate::geo::haversine_distance(
                    pair[0].to_coordinate(),
                    pair[1].to_coordinate(),
                ))
                .map_err(|error| invalid(error.to_string()))?;
        }
        if (derived.value() - segment.distance().value()).abs()
            > DISTANCE_INVARIANT_TOLERANCE_METERS
        {
            return Err(invalid(
                "stored segment distance differs from canonical geometry",
            ));
        }
        let endpoint = crate::geo::haversine_distance(
            points[0].to_coordinate(),
            points[points.len() - 1].to_coordinate(),
        );
        if derived.value() + DISTANCE_INVARIANT_TOLERANCE_METERS < endpoint.value() {
            return Err(invalid("segment violates endpoint distance lower bound"));
        }
    }
    for edge in graph.edges() {
        let segment = graph
            .segment(edge.segment())
            .ok_or_else(|| invalid("edge references missing segment"))?;
        let expected = segment.distance().value()
            / (edge.properties().effective_free_flow_speed().value() / 3.6);
        if (expected - edge.free_flow_travel_time().value()).abs()
            > DISTANCE_INVARIANT_TOLERANCE_METERS
        {
            return Err(invalid("stored free-flow time differs from derived value"));
        }
        reconstructed_offsets[edge.from().value() as usize + 1] += 1;
    }
    for index in 1..reconstructed_offsets.len() {
        reconstructed_offsets[index] += reconstructed_offsets[index - 1];
    }
    if reconstructed_offsets != graph.adjacency_offsets() {
        return Err(invalid("serialized adjacency differs from directed edges"));
    }
    Ok(())
}
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
