//! Deterministic OpenStreetMap source extraction and graph compilation.
//!
//! This crate owns the Phase 7 boundary between raw `.osm.pbf` input and
//! Roadrunner's source-independent [`roadrunner_core::graph::FrozenGraph`].
//! Supported node-via turn restrictions are compiled into maneuver-aware graph
//! transitions; unsupported forms remain available as diagnostics.

mod artifact;
mod compile;
mod error;
mod extract;
mod model;
mod policy;
mod provenance;
mod snapshot;

pub use artifact::{
    DecodedDataset, decode_dataset_artifact, encode_dataset_artifact, write_dataset_artifact_atomic,
};
pub use compile::{
    BuildManifest, CompilationDiagnostics, CompiledGraph, ComponentDiagnostics, ComponentRecord,
    compile_motorcycle_graph,
};
pub use error::OsmError;
pub use extract::extract_pbf;
pub use model::{
    DatasetProvenance, NormalizedNode, NormalizedOsmDataset, NormalizedRelation,
    NormalizedRelationMember, NormalizedRestriction, NormalizedSplitPoint, NormalizedWay,
    OsmElementKind, RestrictionKind, SourceStatistics, SplitPoint,
};
pub use policy::{DELIVERY_MOTORCYCLE_PROFILE, NG_JURISDICTION_POLICY};
pub use provenance::{
    CompiledManeuverProvenance, GraphProvenance, RestrictionProvenance, SourceNodeMapping,
    SourceSegmentMapping, SourceWayMapping, TraversalPolicyProvenance, decode_provenance_artifact,
    encode_provenance_artifact, provenance_artifact_sha256,
};
pub use snapshot::{LoadedGraphSnapshot, load_snapshot_bundle, write_snapshot_bundle_atomic};

/// Normalization contract used by the Phase 7 source artifact.
pub const NORMALIZATION_VERSION: &str = "osm_normalization_v2";
