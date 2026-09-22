use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use roadrunner_core::graph::{AccessClass, FrozenGraph, Orientation};

use crate::error::{OsmError, invalid};

const MAGIC: &str = "ROADRUNNER_GRAPH_PROVENANCE";
const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    magic: String,
    schema_version: u32,
    payload_sha256: String,
    payload_json: String,
}

/// Source node mapped to its snapshot-local routing node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceNodeMapping {
    /// Original OSM node identity.
    pub osm_node_id: i64,
    /// Dense routing node identity.
    pub node_id: u32,
}

/// One source-way-relative compiled physical segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSegmentMapping {
    /// Ordinal along the ordered source way.
    pub ordinal: usize,
    /// Dense physical segment identity.
    pub segment_id: u32,
    /// First source node in the compiled segment geometry.
    pub source_from_node: i64,
    /// Last source node in the compiled segment geometry.
    pub source_to_node: i64,
    /// Forward directed edge when emitted.
    pub forward_edge_id: Option<u32>,
    /// Reverse directed edge when emitted.
    pub reverse_edge_id: Option<u32>,
    /// Access class actually compiled onto the forward traversal.
    pub forward_access: Option<AccessClass>,
    /// Access class actually compiled onto the reverse traversal.
    pub reverse_access: Option<AccessClass>,
    /// Effective forward speed actually compiled onto the traversal.
    pub forward_effective_kph_bits: Option<u64>,
    /// Effective reverse speed actually compiled onto the traversal.
    pub reverse_effective_kph_bits: Option<u64>,
}

/// Explainable policy outcome for one source-way traversal direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraversalPolicyProvenance {
    /// Effective reason-specific access class when the direction passed access policy.
    pub access: Option<AccessClass>,
    /// Class default before constraints.
    pub class_default_kph_bits: Option<u64>,
    /// Parsed legal limit when supported.
    pub legal_limit_kph_bits: Option<u64>,
    /// Physical suitability cap when applied.
    pub physical_cap_kph_bits: Option<u64>,
    /// Final deterministic free-flow speed when emitted.
    pub effective_kph_bits: Option<u64>,
    /// Speed-source diagnostic.
    pub speed_source: Option<String>,
    /// Physical-suitability diagnostic.
    pub physical_status: Option<String>,
}

/// Ordered compiled correspondence and policy decision for one source way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceWayMapping {
    /// Original OSM way identity.
    pub osm_way_id: i64,
    /// Effective directionality classification.
    pub directionality: String,
    /// Source/profile rule producing that classification.
    pub directionality_reason: String,
    /// Forward traversal policy.
    pub forward: TraversalPolicyProvenance,
    /// Reverse traversal policy.
    pub reverse: TraversalPolicyProvenance,
    /// Ordered compiled segments; empty when the way is excluded.
    pub segments: Vec<SourceSegmentMapping>,
    /// Deterministic policy diagnostics.
    pub diagnostics: Vec<String>,
}

/// Restriction-relation preservation and graph-resolution readiness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestrictionProvenance {
    /// Original OSM relation identity.
    pub osm_relation_id: i64,
    /// Number of `from` way members.
    pub from_way_count: usize,
    /// Number of node-via members.
    pub via_node_count: usize,
    /// Number of way-via members.
    pub via_way_count: usize,
    /// Number of `to` way members.
    pub to_way_count: usize,
    /// Whether all required source members can be mapped to this graph snapshot.
    pub source_members_mapped: bool,
    /// Restriction tag selected for this routing profile.
    pub applied_tag: Option<String>,
    /// Restriction value selected for this routing profile.
    pub applied_value: Option<String>,
    /// Resolved incoming traversal, when supported.
    pub incoming_edge_id: Option<u32>,
    /// Resolved declared `to` traversal, when supported.
    pub outgoing_edge_id: Option<u32>,
    /// Canonical forbidden transitions compiled from this relation.
    pub forbidden_maneuvers: Vec<CompiledManeuverProvenance>,
    /// Deterministic enforcement or unsupported-form classification.
    pub status: String,
}

/// One graph-level forbidden transition produced by a source restriction.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CompiledManeuverProvenance {
    /// Incoming directed edge.
    pub incoming_edge_id: u32,
    /// Forbidden outgoing directed edge.
    pub outgoing_edge_id: u32,
}

/// Canonical source-to-graph mapping bound to one semantic graph snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphProvenance {
    /// Provenance semantics identifier.
    pub provenance_version: String,
    /// Authoritative full semantic graph snapshot digest.
    pub graph_snapshot_digest: String,
    /// Canonical normalized-source payload hash.
    pub normalized_dataset_sha256: String,
    /// Source routing-node mappings in OSM identity order.
    pub nodes: Vec<SourceNodeMapping>,
    /// Source way mappings in OSM identity order.
    pub ways: Vec<SourceWayMapping>,
    /// Preserved restriction diagnostics in relation identity order.
    pub restrictions: Vec<RestrictionProvenance>,
}

/// Encodes canonical provenance bytes and validates the mapping first.
///
/// # Errors
///
/// Returns an error when provenance is inconsistent or serialization fails.
pub fn encode_provenance_artifact(
    provenance: &GraphProvenance,
    graph: &FrozenGraph,
) -> Result<Vec<u8>, OsmError> {
    validate_provenance(provenance, graph)?;
    let payload = serde_json::to_vec(provenance)?;
    let payload_sha256 = format!("{:x}", Sha256::digest(&payload));
    let payload_json = String::from_utf8(payload)
        .map_err(|error| invalid(format!("canonical provenance is not UTF-8: {error}")))?;
    Ok(serde_json::to_vec(&Envelope {
        magic: MAGIC.to_owned(),
        schema_version: SCHEMA_VERSION,
        payload_sha256,
        payload_json,
    })?)
}

/// Decodes canonical provenance and validates it against the exact graph.
///
/// # Errors
///
/// Returns an error for malformed, noncanonical, corrupt, or mismatched input.
pub fn decode_provenance_artifact(
    bytes: &[u8],
    graph: &FrozenGraph,
) -> Result<GraphProvenance, OsmError> {
    let envelope: Envelope = serde_json::from_slice(bytes)?;
    if envelope.magic != MAGIC {
        return Err(invalid("invalid graph provenance magic"));
    }
    if envelope.schema_version != SCHEMA_VERSION {
        return Err(OsmError::UnsupportedSchema {
            version: envelope.schema_version,
        });
    }
    if serde_json::to_vec(&envelope)? != bytes {
        return Err(invalid("graph provenance framing is not canonical"));
    }
    let actual = format!("{:x}", Sha256::digest(envelope.payload_json.as_bytes()));
    if actual != envelope.payload_sha256 {
        return Err(OsmError::IntegrityMismatch);
    }
    let provenance: GraphProvenance = serde_json::from_str(&envelope.payload_json)?;
    if serde_json::to_string(&provenance)? != envelope.payload_json {
        return Err(invalid("graph provenance payload is not canonical"));
    }
    validate_provenance(&provenance, graph)?;
    Ok(provenance)
}

/// Returns the SHA-256 hash of exact canonical provenance artifact bytes.
#[must_use]
pub fn provenance_artifact_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[allow(clippy::too_many_lines)]
fn validate_provenance(provenance: &GraphProvenance, graph: &FrozenGraph) -> Result<(), OsmError> {
    if provenance.provenance_version != "osm_graph_provenance_v2"
        || provenance.graph_snapshot_digest != graph.metadata().snapshot_digest()
        || !is_sha256(&provenance.graph_snapshot_digest)
        || !is_sha256(&provenance.normalized_dataset_sha256)
    {
        return Err(invalid("provenance snapshot identity is invalid"));
    }
    if provenance
        .nodes
        .windows(2)
        .any(|pair| pair[0].osm_node_id >= pair[1].osm_node_id)
        || provenance
            .ways
            .windows(2)
            .any(|pair| pair[0].osm_way_id >= pair[1].osm_way_id)
        || provenance
            .restrictions
            .windows(2)
            .any(|pair| pair[0].osm_relation_id >= pair[1].osm_relation_id)
    {
        return Err(invalid("provenance source identities are not canonical"));
    }
    for node in &provenance.nodes {
        if graph
            .node(roadrunner_core::graph::NodeId::new(node.node_id))
            .is_none()
        {
            return Err(invalid("provenance references an absent routing node"));
        }
    }
    let mut compiled_maneuvers = std::collections::BTreeSet::new();
    for restriction in &provenance.restrictions {
        match (restriction.incoming_edge_id, restriction.outgoing_edge_id) {
            (Some(incoming), Some(outgoing)) => {
                let incoming = graph
                    .edge(roadrunner_core::graph::EdgeId::new(incoming))
                    .ok_or_else(|| invalid("restriction incoming edge is absent"))?;
                let outgoing = graph
                    .edge(roadrunner_core::graph::EdgeId::new(outgoing))
                    .ok_or_else(|| invalid("restriction outgoing edge is absent"))?;
                if incoming.to() != outgoing.from() {
                    return Err(invalid("restriction resolved edges are not connected"));
                }
            }
            (None, None) => {}
            _ => return Err(invalid("restriction edge resolution is incomplete")),
        }
        if restriction
            .forbidden_maneuvers
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(invalid("restriction maneuver provenance is not canonical"));
        }
        for maneuver in &restriction.forbidden_maneuvers {
            let pair = (
                roadrunner_core::graph::EdgeId::new(maneuver.incoming_edge_id),
                roadrunner_core::graph::EdgeId::new(maneuver.outgoing_edge_id),
            );
            if !compiled_maneuvers.insert(pair) {
                continue;
            }
            if graph.is_maneuver_allowed(pair.0, pair.1) {
                return Err(invalid("restriction provenance maneuver is not enforced"));
            }
        }
    }
    if compiled_maneuvers.len() != graph.forbidden_maneuvers().len() {
        return Err(invalid(
            "provenance does not cover every forbidden maneuver",
        ));
    }
    let mut mapped_segments = std::collections::BTreeSet::new();
    for way in &provenance.ways {
        if way
            .segments
            .windows(2)
            .any(|pair| pair[0].ordinal >= pair[1].ordinal)
        {
            return Err(invalid("source-way segment ordering is not canonical"));
        }
        for mapping in &way.segments {
            let segment_id = roadrunner_core::graph::RoadSegmentId::new(mapping.segment_id);
            if graph.segment(segment_id).is_none() || !mapped_segments.insert(mapping.segment_id) {
                return Err(invalid(
                    "provenance segment mapping is invalid or duplicated",
                ));
            }
            validate_edge_mapping(
                graph,
                segment_id,
                Orientation::Forward,
                mapping.forward_edge_id,
            )?;
            validate_edge_mapping(
                graph,
                segment_id,
                Orientation::Reverse,
                mapping.reverse_edge_id,
            )?;
            validate_compiled_traversal(graph, segment_id, mapping)?;
        }
    }
    if mapped_segments.len() != graph.segment_count() {
        return Err(invalid("provenance does not cover every graph segment"));
    }
    Ok(())
}

fn validate_compiled_traversal(
    graph: &FrozenGraph,
    segment_id: roadrunner_core::graph::RoadSegmentId,
    mapping: &SourceSegmentMapping,
) -> Result<(), OsmError> {
    for (orientation, access, speed_bits) in [
        (
            Orientation::Forward,
            mapping.forward_access,
            mapping.forward_effective_kph_bits,
        ),
        (
            Orientation::Reverse,
            mapping.reverse_access,
            mapping.reverse_effective_kph_bits,
        ),
    ] {
        let actual = graph
            .edges()
            .iter()
            .find(|edge| edge.segment() == segment_id && edge.orientation() == orientation)
            .map(|edge| {
                (
                    edge.properties().access(),
                    edge.properties()
                        .effective_free_flow_speed()
                        .value()
                        .to_bits(),
                )
            });
        if actual != access.zip(speed_bits) {
            return Err(invalid(
                "provenance traversal policy does not match the compiled edge",
            ));
        }
    }
    Ok(())
}

fn validate_edge_mapping(
    graph: &FrozenGraph,
    segment_id: roadrunner_core::graph::RoadSegmentId,
    orientation: Orientation,
    edge_id: Option<u32>,
) -> Result<(), OsmError> {
    let actual = graph
        .edges()
        .iter()
        .find(|edge| edge.segment() == segment_id && edge.orientation() == orientation)
        .map(|edge| edge.id().value());
    if actual != edge_id {
        return Err(invalid(
            "provenance edge orientation mapping disagrees with graph",
        ));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
