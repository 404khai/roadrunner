use std::collections::BTreeMap;

use roadrunner_core::geo::CanonicalCoordinate;
use serde::{Deserialize, Serialize};

/// Stable provenance for one normalized source dataset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetProvenance {
    /// Caller-declared source identity; paths and timestamps are excluded.
    pub source_id: String,
    /// SHA-256 of the exact input PBF bytes.
    pub source_sha256: String,
    /// Exact input size in bytes.
    pub source_size_bytes: u64,
}

/// Source primitive and retention counts collected during extraction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceStatistics {
    /// OSM nodes encountered in the PBF.
    pub nodes_seen: u64,
    /// OSM ways encountered in the PBF.
    pub ways_seen: u64,
    /// Ways matching the versioned routing-source schema.
    pub candidate_ways: u64,
    /// OSM relations encountered in the PBF.
    pub relations_seen: u64,
    /// Restriction relations encountered and preserved.
    pub restriction_relations_seen: u64,
    /// Unique node identities requested by retained topology.
    pub referenced_nodes_requested: u64,
    /// Requested nodes resolved from the PBF.
    pub referenced_nodes_resolved: u64,
    /// Requested node identities missing from the PBF.
    pub referenced_nodes_missing: u64,
}

/// A required OSM node and its exact E7 coordinate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedNode {
    /// Original OSM node identifier.
    pub osm_id: i64,
    /// Canonical WGS 84 E7 coordinate.
    pub coordinate: CanonicalCoordinate,
    /// Canonically ordered supported routing-source node tags.
    pub tags: BTreeMap<String, String>,
    /// Relevant explicit node values not interpreted by the source schema.
    pub unsupported_tags: BTreeMap<String, String>,
}

/// A potentially routing-relevant OSM way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedWay {
    /// Original OSM way identifier.
    pub osm_id: i64,
    /// Ordered original OSM node references.
    pub node_refs: Vec<i64>,
    /// Canonically ordered supported source tags.
    pub tags: BTreeMap<String, String>,
    /// Canonically ordered routing-relevant values not understood by v1 policy.
    pub unsupported_tags: BTreeMap<String, String>,
}

/// Type of an OSM relation member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsmElementKind {
    /// OSM node.
    Node,
    /// OSM way.
    Way,
    /// OSM relation.
    Relation,
}

/// One ordered member of a retained relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedRelationMember {
    /// Member element kind.
    pub kind: OsmElementKind,
    /// Original OSM element identifier.
    pub osm_id: i64,
    /// Original member role.
    pub role: String,
}

/// Restriction value classification retained for Phase 7.5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestrictionKind {
    /// A `no_*` restriction.
    No,
    /// An `only_*` restriction.
    Only,
    /// A restriction value outside the initial Phase 7.5 subset.
    Unsupported,
}

/// One independently preserved generic or qualified restriction value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedRestriction {
    /// Original canonical OSM tag key, such as `restriction:motorcycle`.
    pub tag: String,
    /// Original restriction value.
    pub value: String,
    /// Value-family classification independent of vehicle applicability.
    pub kind: RestrictionKind,
    /// Whether the tag describes conditional semantics.
    pub conditional: bool,
}

/// A routing-relevant OSM restriction relation retained but not enforced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedRelation {
    /// Original OSM relation identifier.
    pub osm_id: i64,
    /// Independently preserved generic and vehicle-qualified values.
    pub restrictions: Vec<NormalizedRestriction>,
    /// Optional exception list as provided by OSM.
    pub except: Option<String>,
    /// Ordered source members.
    pub members: Vec<NormalizedRelationMember>,
    /// Relevant values not interpreted in Phase 7.
    pub unsupported_tags: BTreeMap<String, String>,
}

/// Why an OSM node must remain a graph split point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitPoint {
    /// First or last node of a retained way.
    WayEndpoint,
    /// Node referenced by more than one retained way or repeated within one way.
    Junction,
    /// Node-via member of a retained restriction.
    RestrictionVia,
    /// Node carrying a barrier source semantic.
    Barrier,
    /// Node carrying an access boundary semantic.
    AccessBoundary,
    /// Node explicitly marked as a ford.
    Ford,
    /// Node-level highway semantic retained by the routing-source schema.
    HighwayNode,
}

/// Stable split-point record used during profile compilation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedSplitPoint {
    /// Original OSM node identifier.
    pub osm_node_id: i64,
    /// All applicable reasons in enum order.
    pub reasons: Vec<SplitPoint>,
}

/// Deterministic, profile-independent routing-source artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedOsmDataset {
    /// Normalization rules identifier.
    pub normalization_version: String,
    /// Source provenance.
    pub provenance: DatasetProvenance,
    /// Source primitive and retention statistics.
    pub source_statistics: SourceStatistics,
    /// Required nodes sorted by OSM identifier.
    pub nodes: Vec<NormalizedNode>,
    /// Candidate ways sorted by OSM identifier.
    pub ways: Vec<NormalizedWay>,
    /// Restriction relations sorted by OSM identifier.
    pub relations: Vec<NormalizedRelation>,
    /// Preserved topology split points sorted by node identifier.
    pub split_points: Vec<NormalizedSplitPoint>,
}
