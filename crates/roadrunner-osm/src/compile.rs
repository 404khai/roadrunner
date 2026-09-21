use std::collections::{BTreeMap, BTreeSet};

use roadrunner_core::graph::{
    BuilderNodeId, BuilderSegmentId, FrozenGraph, GraphBuildIdentity, GraphBuilder, GraphMetadata,
    GraphSnapshotId, encode_graph_artifact,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::info;

use crate::artifact::DecodedDataset;
use crate::error::{OsmError, invalid};
use crate::model::NormalizedWay;
use crate::policy::{DELIVERY_MOTORCYCLE_PROFILE, NG_JURISDICTION_POLICY, compile_directions};

const BUILD_CONFIGURATION: &str =
    "contraction=split_points_v1;retain_all_components=true;restrictions=preserved_not_enforced";

/// Deterministic weak-connectivity diagnostics for a compiled graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentDiagnostics {
    /// Number of weakly connected components.
    pub weak_component_count: usize,
    /// Node counts in descending order.
    pub weak_component_node_counts: Vec<usize>,
    /// Number of isolated routing nodes.
    pub isolated_node_count: usize,
}

/// Reproducible manifest for one graph compilation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildManifest {
    /// Compiler crate version.
    pub compiler_version: String,
    /// Normalization rules version.
    pub normalization_version: String,
    /// Routing profile identifier.
    pub routing_profile: String,
    /// Jurisdiction policy identifier.
    pub jurisdiction_policy: String,
    /// Canonical build-configuration declaration.
    pub build_configuration: String,
    /// Declared source dataset identity.
    pub source_id: String,
    /// Integrity of the exact source PBF.
    pub source_pbf_sha256: String,
    /// Integrity of the normalized artifact payload.
    pub normalized_dataset_sha256: String,
    /// Integrity of the encoded graph artifact.
    pub graph_artifact_sha256: String,
    /// Deterministically derived snapshot identity.
    pub graph_snapshot_id: u64,
    /// Retained normalized nodes.
    pub normalized_node_count: usize,
    /// Retained candidate ways.
    pub normalized_way_count: usize,
    /// Preserved restriction relations.
    pub preserved_restriction_count: usize,
    /// Ways accepted by the profile.
    pub compiled_way_count: usize,
    /// Ways excluded by the profile.
    pub excluded_way_count: usize,
    /// Frozen routing nodes.
    pub graph_node_count: usize,
    /// Frozen physical segments.
    pub graph_segment_count: usize,
    /// Frozen directional edges.
    pub graph_edge_count: usize,
    /// Weak-connectivity diagnostics.
    pub components: ComponentDiagnostics,
    /// Phase boundary declaration.
    pub turn_restrictions_enforced: bool,
}

/// Graph and manifest produced by the initial motorcycle compiler.
#[derive(Debug, Clone)]
pub struct CompiledGraph {
    /// Immutable routing graph.
    pub graph: FrozenGraph,
    /// Reproducible compilation manifest.
    pub manifest: BuildManifest,
}

#[derive(Debug)]
struct SegmentDraft {
    way_id: i64,
    ordinal: usize,
    node_refs: Vec<i64>,
    forward: Option<roadrunner_core::graph::EdgeProperties>,
    reverse: Option<roadrunner_core::graph::EdgeProperties>,
}

/// Compiles `delivery_motorcycle_v1` with `ng_v1` defaults.
///
/// Static directionality becomes edge existence and contextual access remains an
/// edge property. Restriction relations affect split points but are not enforced.
///
/// # Errors
///
/// Returns an error if normalized references are inconsistent or core graph
/// invariants reject the compiled topology.
#[allow(clippy::too_many_lines)]
pub fn compile_motorcycle_graph(decoded: &DecodedDataset) -> Result<CompiledGraph, OsmError> {
    let dataset = &decoded.dataset;
    let coordinates: BTreeMap<_, _> = dataset
        .nodes
        .iter()
        .map(|node| (node.osm_id, node.coordinate))
        .collect();
    let split_points: BTreeSet<_> = dataset
        .split_points
        .iter()
        .map(|point| point.osm_node_id)
        .collect();

    let mut drafts = Vec::new();
    let mut compiled_way_count = 0_usize;
    for way in &dataset.ways {
        let (forward, reverse) = compile_directions(way)?;
        if forward.is_none() && reverse.is_none() {
            continue;
        }
        compiled_way_count += 1;
        split_way(way, &split_points, forward, reverse, &mut drafts)?;
    }
    drafts.sort_by_key(|draft| (draft.way_id, draft.ordinal));

    let routing_node_ids: BTreeSet<_> = drafts
        .iter()
        .flat_map(|draft| {
            [
                draft.node_refs[0],
                draft.node_refs[draft.node_refs.len() - 1],
            ]
        })
        .collect();
    let snapshot_id = derive_snapshot_id(decoded);
    let metadata = GraphMetadata::new(
        DELIVERY_MOTORCYCLE_PROFILE,
        NG_JURISDICTION_POLICY,
        GraphBuildIdentity::new(
            &dataset.provenance.source_id,
            &decoded.payload_sha256,
            &dataset.normalization_version,
            BUILD_CONFIGURATION,
        ),
    );
    let mut builder = GraphBuilder::new(GraphSnapshotId::new(snapshot_id), metadata);
    for osm_id in routing_node_ids {
        let coordinate = coordinates
            .get(&osm_id)
            .copied()
            .ok_or_else(|| invalid(format!("missing routing-node coordinate {osm_id}")))?;
        builder.add_node(builder_node_id(osm_id)?, coordinate)?;
    }
    for (index, draft) in drafts.into_iter().enumerate() {
        let geometry = draft
            .node_refs
            .iter()
            .map(|id| {
                coordinates
                    .get(id)
                    .copied()
                    .ok_or_else(|| invalid(format!("missing geometry coordinate {id}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let segment_id =
            u64::try_from(index).map_err(|_| invalid("compiled segment identity overflow"))?;
        let from = builder_node_id(draft.node_refs[0])?;
        let to = builder_node_id(draft.node_refs[draft.node_refs.len() - 1])?;
        builder.add_segment(
            BuilderSegmentId::new(segment_id),
            from,
            to,
            geometry,
            draft.forward,
            draft.reverse,
        )?;
    }
    let graph = builder.finalize()?;
    let components = component_diagnostics(&graph);
    let graph_bytes = encode_graph_artifact(&graph)
        .map_err(|error| invalid(format!("graph artifact encoding failed: {error}")))?;
    let manifest = BuildManifest {
        compiler_version: env!("CARGO_PKG_VERSION").to_owned(),
        normalization_version: dataset.normalization_version.clone(),
        routing_profile: DELIVERY_MOTORCYCLE_PROFILE.to_owned(),
        jurisdiction_policy: NG_JURISDICTION_POLICY.to_owned(),
        build_configuration: BUILD_CONFIGURATION.to_owned(),
        source_id: dataset.provenance.source_id.clone(),
        source_pbf_sha256: dataset.provenance.source_sha256.clone(),
        normalized_dataset_sha256: decoded.payload_sha256.clone(),
        graph_artifact_sha256: format!("{:x}", Sha256::digest(&graph_bytes)),
        graph_snapshot_id: snapshot_id,
        normalized_node_count: dataset.nodes.len(),
        normalized_way_count: dataset.ways.len(),
        preserved_restriction_count: dataset.relations.len(),
        compiled_way_count,
        excluded_way_count: dataset.ways.len() - compiled_way_count,
        graph_node_count: graph.node_count(),
        graph_segment_count: graph.segment_count(),
        graph_edge_count: graph.edge_count(),
        components,
        turn_restrictions_enforced: false,
    };
    info!(
        nodes = graph.node_count(),
        segments = graph.segment_count(),
        edges = graph.edge_count(),
        "compiled OSM motorcycle graph"
    );
    Ok(CompiledGraph { graph, manifest })
}

fn split_way(
    way: &NormalizedWay,
    split_points: &BTreeSet<i64>,
    forward: Option<roadrunner_core::graph::EdgeProperties>,
    reverse: Option<roadrunner_core::graph::EdgeProperties>,
    output: &mut Vec<SegmentDraft>,
) -> Result<(), OsmError> {
    let mut start = 0_usize;
    let mut ordinal = 0_usize;
    for index in 1..way.node_refs.len() {
        if index == way.node_refs.len() - 1 || split_points.contains(&way.node_refs[index]) {
            let node_refs = way.node_refs[start..=index].to_vec();
            if node_refs.len() < 2 {
                return Err(invalid(format!(
                    "way {} produced malformed segment",
                    way.osm_id
                )));
            }
            output.push(SegmentDraft {
                way_id: way.osm_id,
                ordinal,
                node_refs,
                forward,
                reverse,
            });
            ordinal += 1;
            start = index;
        }
    }
    Ok(())
}

fn derive_snapshot_id(decoded: &DecodedDataset) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(decoded.payload_sha256.as_bytes());
    hasher.update(DELIVERY_MOTORCYCLE_PROFILE.as_bytes());
    hasher.update(NG_JURISDICTION_POLICY.as_bytes());
    hasher.update(BUILD_CONFIGURATION.as_bytes());
    let digest = hasher.finalize();
    let mut prefix = [0_u8; 8];
    prefix.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(prefix)
}

fn builder_node_id(osm_id: i64) -> Result<BuilderNodeId, OsmError> {
    u64::try_from(osm_id)
        .map(BuilderNodeId::new)
        .map_err(|_| invalid(format!("OSM node ID {osm_id} cannot be a builder identity")))
}

fn component_diagnostics(graph: &FrozenGraph) -> ComponentDiagnostics {
    let mut parent: Vec<_> = (0..graph.node_count()).collect();
    let mut has_edge = vec![false; graph.node_count()];
    for edge in graph.edges() {
        has_edge[edge.from().value() as usize] = true;
        has_edge[edge.to().value() as usize] = true;
        union(
            &mut parent,
            edge.from().value() as usize,
            edge.to().value() as usize,
        );
    }
    let mut counts = BTreeMap::<usize, usize>::new();
    for node in 0..parent.len() {
        let root = find(&mut parent, node);
        *counts.entry(root).or_default() += 1;
    }
    let isolated_node_count = has_edge.into_iter().filter(|value| !value).count();
    let mut weak_component_node_counts: Vec<_> = counts.into_values().collect();
    weak_component_node_counts.sort_unstable_by(|left, right| right.cmp(left));
    ComponentDiagnostics {
        weak_component_count: weak_component_node_counts.len(),
        weak_component_node_counts,
        isolated_node_count,
    }
}

fn find(parent: &mut [usize], mut node: usize) -> usize {
    while parent[node] != node {
        parent[node] = parent[parent[node]];
        node = parent[node];
    }
    node
}

fn union(parent: &mut [usize], left: usize, right: usize) {
    let left_root = find(parent, left);
    let right_root = find(parent, right);
    if left_root != right_root {
        let (smaller, larger) = if left_root < right_root {
            (left_root, right_root)
        } else {
            (right_root, left_root)
        };
        parent[larger] = smaller;
    }
}
