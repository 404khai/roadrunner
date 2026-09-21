use std::collections::{BTreeMap, BTreeSet};

use roadrunner_core::geo::{Meters, haversine_distance};
use roadrunner_core::graph::{
    BuilderNodeId, BuilderSegmentId, FrozenGraph, GraphBuildIdentity, GraphBuilder, GraphMetadata,
    GraphSnapshotId, encode_graph_artifact,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::info;

use crate::artifact::DecodedDataset;
use crate::error::{OsmError, invalid};
use crate::model::{NormalizedWay, OsmElementKind};
use crate::policy::{
    DELIVERY_MOTORCYCLE_PROFILE, NG_JURISDICTION_POLICY, TraversalPolicyDecision,
    WayPolicyDecision, apply_node_semantics, compile_directions,
};
use crate::provenance::{
    GraphProvenance, RestrictionProvenance, SourceNodeMapping, SourceSegmentMapping,
    SourceWayMapping, TraversalPolicyProvenance, encode_provenance_artifact,
    provenance_artifact_sha256,
};

const BUILD_CONFIGURATION: &str = "contraction=semantic_split_points_v2;retain_all_components=true;restrictions=preserved_not_enforced";
const COMPILER_SEMANTIC_VERSION: &str = "osm_graph_compiler_v2";

/// Deterministic weak-connectivity diagnostics for a compiled graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentDiagnostics {
    /// Number of weakly connected components.
    pub weak_component_count: usize,
    /// Node counts in descending order.
    pub weak_component_node_counts: Vec<usize>,
    /// Number of isolated routing nodes.
    pub isolated_node_count: usize,
    /// Number of strongly connected components.
    pub strong_component_count: usize,
    /// Strong-component node counts in descending order.
    pub strong_component_node_counts: Vec<usize>,
    /// Deterministic per-weak-component evidence.
    pub components: Vec<ComponentRecord>,
}

/// Explainable topology evidence for one weak component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentRecord {
    /// Stable audit ordinal after canonical component sorting.
    pub component_id: usize,
    /// Dense node count.
    pub node_count: usize,
    /// Physical segment count.
    pub segment_count: usize,
    /// Directed-edge count.
    pub directed_edge_count: usize,
    /// Source ways contributing compiled segments.
    pub source_way_count: usize,
    /// Strong components contained by this weak component.
    pub strong_component_count: usize,
    /// Minimum latitude in E7.
    pub min_latitude_e7: i32,
    /// Maximum latitude in E7.
    pub max_latitude_e7: i32,
    /// Minimum longitude in E7.
    pub min_longitude_e7: i32,
    /// Maximum longitude in E7.
    pub max_longitude_e7: i32,
    /// Evidence-based current fragmentation classification.
    pub cause: String,
    /// Source way identities for deterministic audit tracing.
    pub source_way_ids: Vec<i64>,
    /// Source ways that explain a profile-induced component boundary.
    pub fragmentation_evidence_way_ids: Vec<i64>,
}

/// Deterministic compiler and policy decision distributions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompilationDiagnostics {
    /// Source nodes retained only as contracted geometry shape points.
    pub contracted_shape_node_count: usize,
    /// Canonical geometry points stored by the graph.
    pub geometry_point_count: usize,
    /// Routing-node split reasons; a node may contribute to multiple counts.
    pub split_reason_counts: BTreeMap<String, usize>,
    /// Directed traversal counts by effective access class.
    pub access_traversal_counts: BTreeMap<String, usize>,
    /// Source way counts by effective directionality decision.
    pub directionality_way_counts: BTreeMap<String, usize>,
    /// Traversal speed-source decision counts.
    pub speed_source_counts: BTreeMap<String, usize>,
    /// Physical-suitability decision counts.
    pub physical_status_counts: BTreeMap<String, usize>,
    /// Consecutive equal coordinate pairs retained in candidate geometry.
    pub repeated_consecutive_geometry_points: usize,
    /// Candidate segments whose source endpoint identity is the same.
    pub self_loop_candidates: usize,
    /// Candidate segments with distinct source endpoints at equal coordinates.
    pub coincident_distinct_endpoint_candidates: usize,
    /// Node-via restriction relation count.
    pub node_via_restrictions: usize,
    /// Way-via restriction relation count.
    pub way_via_restrictions: usize,
    /// Preserved `no_*` restriction value count.
    pub no_restriction_values: usize,
    /// Preserved `only_*` restriction value count.
    pub only_restriction_values: usize,
    /// Preserved generic restriction value count.
    pub generic_restriction_values: usize,
    /// Preserved motorcycle-qualified restriction value count.
    pub motorcycle_restriction_values: usize,
    /// Preserved conditional restriction value count.
    pub conditional_restriction_values: usize,
    /// Relations with source members ready for Phase 7.5 resolution.
    pub source_resolvable_restrictions: usize,
    /// Relations with ambiguous member shapes.
    pub ambiguous_restrictions: usize,
    /// Relations whose required source members are absent from this graph.
    pub unresolved_restrictions: usize,
    /// Preserved forms outside the initial node-via subset.
    pub unsupported_restrictions: usize,
}

/// Reproducible manifest for one graph compilation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildManifest {
    /// Compiler crate version.
    pub compiler_version: String,
    /// Version of semantics that can change canonical graph output.
    pub compiler_semantic_version: String,
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
    /// Source primitive and retention counts from extraction.
    pub source_statistics: crate::model::SourceStatistics,
    /// Integrity of the normalized artifact payload.
    pub normalized_dataset_sha256: String,
    /// Integrity of the encoded graph artifact.
    pub graph_artifact_sha256: String,
    /// Integrity of the canonical source-to-graph provenance artifact.
    pub provenance_artifact_sha256: String,
    /// Deterministically derived snapshot identity.
    pub graph_snapshot_id: u64,
    /// Authoritative full semantic graph snapshot digest.
    pub graph_snapshot_digest: String,
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
    /// Zero-distance physical segment candidates quarantined from routing.
    pub zero_distance_candidate_count: usize,
    /// Frozen routing nodes.
    pub graph_node_count: usize,
    /// Frozen physical segments.
    pub graph_segment_count: usize,
    /// Frozen directional edges.
    pub graph_edge_count: usize,
    /// Weak-connectivity diagnostics.
    pub components: ComponentDiagnostics,
    /// Explainable compiler and source-policy distributions.
    pub diagnostics: CompilationDiagnostics,
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
    /// Validated source-to-graph correspondence for this snapshot.
    pub provenance: GraphProvenance,
}

#[derive(Debug, Serialize)]
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
    let mut policy_decisions = BTreeMap::<i64, WayPolicyDecision>::new();
    for way in &dataset.ways {
        let policy = compile_directions(way)?;
        let forward = policy.forward.properties;
        let reverse = policy.reverse.properties;
        if forward.is_none() && reverse.is_none() {
            policy_decisions.insert(way.osm_id, policy);
            continue;
        }
        split_way(way, &split_points, forward, reverse, &mut drafts)?;
        policy_decisions.insert(way.osm_id, policy);
    }
    drafts.sort_by_key(|draft| (draft.way_id, draft.ordinal));
    let normalized_nodes: BTreeMap<_, _> = dataset
        .nodes
        .iter()
        .map(|node| (node.osm_id, node))
        .collect();
    for draft in &mut drafts {
        let from = normalized_nodes
            .get(&draft.node_refs[0])
            .ok_or_else(|| invalid("segment source node semantics are missing"))?;
        let to = normalized_nodes
            .get(&draft.node_refs[draft.node_refs.len() - 1])
            .ok_or_else(|| invalid("segment target node semantics are missing"))?;
        draft.forward = apply_node_semantics(draft.forward, [from, to]);
        draft.reverse = apply_node_semantics(draft.reverse, [to, from]);
    }
    drafts.retain(|draft| draft.forward.is_some() || draft.reverse.is_some());
    let compiled_way_count = drafts
        .iter()
        .map(|draft| draft.way_id)
        .collect::<BTreeSet<_>>()
        .len();
    let mut zero_distance_candidate_count = 0_usize;
    drafts.retain(|draft| {
        let mut distance = Meters::ZERO;
        for pair in draft.node_refs.windows(2) {
            let Some(from) = coordinates.get(&pair[0]) else {
                return true;
            };
            let Some(to) = coordinates.get(&pair[1]) else {
                return true;
            };
            let Ok(total) =
                distance.checked_add(haversine_distance(from.to_coordinate(), to.to_coordinate()))
            else {
                return true;
            };
            distance = total;
        }
        if distance == Meters::ZERO {
            zero_distance_candidate_count += 1;
            false
        } else {
            true
        }
    });

    let routing_node_ids: BTreeSet<_> = drafts
        .iter()
        .flat_map(|draft| {
            [
                draft.node_refs[0],
                draft.node_refs[draft.node_refs.len() - 1],
            ]
        })
        .collect();
    let (snapshot_id, snapshot_digest) = derive_snapshot_identity(decoded, &drafts)?;
    let metadata = GraphMetadata::with_snapshot_digest(
        DELIVERY_MOTORCYCLE_PROFILE,
        NG_JURISDICTION_POLICY,
        GraphBuildIdentity::new(
            &dataset.provenance.source_id,
            &decoded.payload_sha256,
            &dataset.normalization_version,
            BUILD_CONFIGURATION,
        ),
        &snapshot_digest,
    );
    let mut builder = GraphBuilder::new(GraphSnapshotId::new(snapshot_id), metadata);
    for osm_id in &routing_node_ids {
        let coordinate = coordinates
            .get(osm_id)
            .copied()
            .ok_or_else(|| invalid(format!("missing routing-node coordinate {osm_id}")))?;
        builder.add_node(builder_node_id(*osm_id)?, coordinate)?;
    }
    for (index, draft) in drafts.iter().enumerate() {
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
    let provenance = build_provenance(
        decoded,
        &graph,
        &routing_node_ids,
        &drafts,
        &policy_decisions,
    )?;
    let components = component_diagnostics(&graph, dataset, &provenance);
    let diagnostics =
        compilation_diagnostics(dataset, &graph, &drafts, &policy_decisions, &provenance);
    let graph_bytes = encode_graph_artifact(&graph)
        .map_err(|error| invalid(format!("graph artifact encoding failed: {error}")))?;
    let provenance_bytes = encode_provenance_artifact(&provenance, &graph)?;
    let manifest = BuildManifest {
        compiler_version: env!("CARGO_PKG_VERSION").to_owned(),
        compiler_semantic_version: COMPILER_SEMANTIC_VERSION.to_owned(),
        normalization_version: dataset.normalization_version.clone(),
        routing_profile: DELIVERY_MOTORCYCLE_PROFILE.to_owned(),
        jurisdiction_policy: NG_JURISDICTION_POLICY.to_owned(),
        build_configuration: BUILD_CONFIGURATION.to_owned(),
        source_id: dataset.provenance.source_id.clone(),
        source_pbf_sha256: dataset.provenance.source_sha256.clone(),
        source_statistics: dataset.source_statistics.clone(),
        normalized_dataset_sha256: decoded.payload_sha256.clone(),
        graph_artifact_sha256: format!("{:x}", Sha256::digest(&graph_bytes)),
        provenance_artifact_sha256: provenance_artifact_sha256(&provenance_bytes),
        graph_snapshot_id: snapshot_id,
        graph_snapshot_digest: snapshot_digest,
        normalized_node_count: dataset.nodes.len(),
        normalized_way_count: dataset.ways.len(),
        preserved_restriction_count: dataset.relations.len(),
        compiled_way_count,
        excluded_way_count: dataset.ways.len() - compiled_way_count,
        zero_distance_candidate_count,
        graph_node_count: graph.node_count(),
        graph_segment_count: graph.segment_count(),
        graph_edge_count: graph.edge_count(),
        components,
        diagnostics,
        turn_restrictions_enforced: false,
    };
    info!(
        nodes = graph.node_count(),
        segments = graph.segment_count(),
        edges = graph.edge_count(),
        "compiled OSM motorcycle graph"
    );
    Ok(CompiledGraph {
        graph,
        manifest,
        provenance,
    })
}

#[allow(clippy::too_many_lines)]
fn compilation_diagnostics(
    dataset: &crate::model::NormalizedOsmDataset,
    graph: &FrozenGraph,
    drafts: &[SegmentDraft],
    decisions: &BTreeMap<i64, WayPolicyDecision>,
    provenance: &GraphProvenance,
) -> CompilationDiagnostics {
    let mut split_reason_counts = BTreeMap::new();
    for split in &dataset.split_points {
        for reason in &split.reasons {
            *split_reason_counts
                .entry(format!("{reason:?}").to_ascii_lowercase())
                .or_default() += 1;
        }
    }
    let mut access_traversal_counts = BTreeMap::new();
    for edge in graph.edges() {
        *access_traversal_counts
            .entry(format!("{:?}", edge.properties().access()).to_ascii_lowercase())
            .or_default() += 1;
    }
    let mut directionality_way_counts = BTreeMap::new();
    let mut speed_source_counts = BTreeMap::new();
    let mut physical_status_counts = BTreeMap::new();
    for decision in decisions.values() {
        *directionality_way_counts
            .entry(format!("{:?}", decision.directionality).to_ascii_lowercase())
            .or_default() += 1;
        for traversal in [&decision.forward, &decision.reverse] {
            if let Some(speed) = &traversal.speed {
                *speed_source_counts
                    .entry(speed.source_status.clone())
                    .or_default() += 1;
                *physical_status_counts
                    .entry(speed.physical_status.clone())
                    .or_default() += 1;
            }
        }
    }
    let coordinates: BTreeMap<_, _> = dataset
        .nodes
        .iter()
        .map(|node| (node.osm_id, node.coordinate))
        .collect();
    let repeated_consecutive_geometry_points = drafts
        .iter()
        .map(|draft| {
            draft
                .node_refs
                .windows(2)
                .filter(|pair| coordinates.get(&pair[0]) == coordinates.get(&pair[1]))
                .count()
        })
        .sum();
    let self_loop_candidates = drafts
        .iter()
        .filter(|draft| draft.node_refs.first() == draft.node_refs.last())
        .count();
    let coincident_distinct_endpoint_candidates = drafts
        .iter()
        .filter(|draft| {
            let first = draft.node_refs[0];
            let last = draft.node_refs[draft.node_refs.len() - 1];
            first != last && coordinates.get(&first) == coordinates.get(&last)
        })
        .count();
    let node_via_restrictions = provenance
        .restrictions
        .iter()
        .filter(|restriction| restriction.via_node_count > 0)
        .count();
    let way_via_restrictions = provenance
        .restrictions
        .iter()
        .filter(|restriction| restriction.via_way_count > 0)
        .count();
    let values = dataset
        .relations
        .iter()
        .flat_map(|relation| &relation.restrictions);
    let values: Vec<_> = values.collect();
    CompilationDiagnostics {
        contracted_shape_node_count: dataset.nodes.len().saturating_sub(graph.node_count()),
        geometry_point_count: graph.geometry_point_count(),
        split_reason_counts,
        access_traversal_counts,
        directionality_way_counts,
        speed_source_counts,
        physical_status_counts,
        repeated_consecutive_geometry_points,
        self_loop_candidates,
        coincident_distinct_endpoint_candidates,
        node_via_restrictions,
        way_via_restrictions,
        no_restriction_values: values
            .iter()
            .filter(|value| value.kind == crate::model::RestrictionKind::No)
            .count(),
        only_restriction_values: values
            .iter()
            .filter(|value| value.kind == crate::model::RestrictionKind::Only)
            .count(),
        generic_restriction_values: values
            .iter()
            .filter(|value| value.tag == "restriction" || value.tag == "restriction:conditional")
            .count(),
        motorcycle_restriction_values: values
            .iter()
            .filter(|value| value.tag.starts_with("restriction:motorcycle"))
            .count(),
        conditional_restriction_values: values.iter().filter(|value| value.conditional).count(),
        source_resolvable_restrictions: provenance
            .restrictions
            .iter()
            .filter(|restriction| restriction.status == "source_members_resolvable")
            .count(),
        ambiguous_restrictions: provenance
            .restrictions
            .iter()
            .filter(|restriction| restriction.status == "ambiguous_member_shape")
            .count(),
        unresolved_restrictions: provenance
            .restrictions
            .iter()
            .filter(|restriction| restriction.status == "unresolved_source_member")
            .count(),
        unsupported_restrictions: provenance
            .restrictions
            .iter()
            .filter(|restriction| restriction.status == "preserved_unsupported_way_via")
            .count()
            + values
                .iter()
                .filter(|value| value.kind == crate::model::RestrictionKind::Unsupported)
                .count(),
    }
}

#[allow(clippy::too_many_lines)]
fn build_provenance(
    decoded: &DecodedDataset,
    graph: &FrozenGraph,
    routing_node_ids: &BTreeSet<i64>,
    drafts: &[SegmentDraft],
    policy_decisions: &BTreeMap<i64, WayPolicyDecision>,
) -> Result<GraphProvenance, OsmError> {
    let nodes = routing_node_ids
        .iter()
        .enumerate()
        .map(|(index, osm_node_id)| {
            Ok(SourceNodeMapping {
                osm_node_id: *osm_node_id,
                node_id: u32::try_from(index)
                    .map_err(|_| invalid("routing-node provenance identity overflow"))?,
            })
        })
        .collect::<Result<Vec<_>, OsmError>>()?;
    let mut ways = Vec::with_capacity(decoded.dataset.ways.len());
    for way in &decoded.dataset.ways {
        let decision = policy_decisions
            .get(&way.osm_id)
            .ok_or_else(|| invalid(format!("missing policy decision for way {}", way.osm_id)))?;
        let mut segments = Vec::new();
        for (segment_index, draft) in drafts.iter().enumerate() {
            if draft.way_id != way.osm_id {
                continue;
            }
            let segment_id = u32::try_from(segment_index)
                .map_err(|_| invalid("segment provenance identity overflow"))?;
            let (
                forward_edge_id,
                reverse_edge_id,
                forward_access,
                reverse_access,
                forward_speed,
                reverse_speed,
            ) = graph
                .edges()
                .iter()
                .filter(|edge| edge.segment().value() == segment_id)
                .fold((None, None, None, None, None, None), |mut ids, edge| {
                    match edge.orientation() {
                        roadrunner_core::graph::Orientation::Forward => {
                            ids.0 = Some(edge.id().value());
                            ids.2 = Some(edge.properties().access());
                            ids.4 = Some(
                                edge.properties()
                                    .effective_free_flow_speed()
                                    .value()
                                    .to_bits(),
                            );
                        }
                        roadrunner_core::graph::Orientation::Reverse => {
                            ids.1 = Some(edge.id().value());
                            ids.3 = Some(edge.properties().access());
                            ids.5 = Some(
                                edge.properties()
                                    .effective_free_flow_speed()
                                    .value()
                                    .to_bits(),
                            );
                        }
                    }
                    ids
                });
            segments.push(SourceSegmentMapping {
                ordinal: draft.ordinal,
                segment_id,
                source_from_node: draft.node_refs[0],
                source_to_node: draft.node_refs[draft.node_refs.len() - 1],
                forward_edge_id,
                reverse_edge_id,
                forward_access,
                reverse_access,
                forward_effective_kph_bits: forward_speed,
                reverse_effective_kph_bits: reverse_speed,
            });
        }
        ways.push(SourceWayMapping {
            osm_way_id: way.osm_id,
            directionality: format!("{:?}", decision.directionality).to_ascii_lowercase(),
            directionality_reason: decision.directionality_reason.clone(),
            forward: traversal_provenance(&decision.forward),
            reverse: traversal_provenance(&decision.reverse),
            segments,
            diagnostics: decision.diagnostics.clone(),
        });
    }
    let mapped_way_ids: BTreeSet<_> = ways
        .iter()
        .filter(|way| !way.segments.is_empty())
        .map(|way| way.osm_way_id)
        .collect();
    let mapped_node_ids: BTreeSet<_> = nodes.iter().map(|node| node.osm_node_id).collect();
    let restrictions = decoded
        .dataset
        .relations
        .iter()
        .map(|relation| {
            let from_ways: Vec<_> = relation
                .members
                .iter()
                .filter(|member| member.kind == OsmElementKind::Way && member.role == "from")
                .collect();
            let via_nodes: Vec<_> = relation
                .members
                .iter()
                .filter(|member| member.kind == OsmElementKind::Node && member.role == "via")
                .collect();
            let via_ways: Vec<_> = relation
                .members
                .iter()
                .filter(|member| member.kind == OsmElementKind::Way && member.role == "via")
                .collect();
            let to_ways: Vec<_> = relation
                .members
                .iter()
                .filter(|member| member.kind == OsmElementKind::Way && member.role == "to")
                .collect();
            let source_members_mapped = from_ways
                .iter()
                .chain(&to_ways)
                .all(|member| mapped_way_ids.contains(&member.osm_id))
                && via_nodes
                    .iter()
                    .all(|member| mapped_node_ids.contains(&member.osm_id));
            let status = if !via_ways.is_empty() {
                "preserved_unsupported_way_via"
            } else if from_ways.len() == 1
                && via_nodes.len() == 1
                && to_ways.len() == 1
                && source_members_mapped
            {
                "source_members_resolvable"
            } else if source_members_mapped {
                "ambiguous_member_shape"
            } else {
                "unresolved_source_member"
            };
            RestrictionProvenance {
                osm_relation_id: relation.osm_id,
                from_way_count: from_ways.len(),
                via_node_count: via_nodes.len(),
                via_way_count: via_ways.len(),
                to_way_count: to_ways.len(),
                source_members_mapped,
                status: status.to_owned(),
            }
        })
        .collect();
    Ok(GraphProvenance {
        provenance_version: "osm_graph_provenance_v1".to_owned(),
        graph_snapshot_digest: graph.metadata().snapshot_digest().to_owned(),
        normalized_dataset_sha256: decoded.payload_sha256.clone(),
        nodes,
        ways,
        restrictions,
    })
}

fn traversal_provenance(decision: &TraversalPolicyDecision) -> TraversalPolicyProvenance {
    TraversalPolicyProvenance {
        access: decision.access,
        class_default_kph_bits: decision
            .speed
            .as_ref()
            .map(|speed| speed.class_default_kph.to_bits()),
        legal_limit_kph_bits: decision
            .speed
            .as_ref()
            .and_then(|speed| speed.legal_limit_kph)
            .map(f64::to_bits),
        physical_cap_kph_bits: decision
            .speed
            .as_ref()
            .and_then(|speed| speed.physical_cap_kph)
            .map(f64::to_bits),
        effective_kph_bits: decision
            .speed
            .as_ref()
            .map(|speed| speed.effective_kph.to_bits()),
        speed_source: decision
            .speed
            .as_ref()
            .map(|speed| speed.source_status.clone()),
        physical_status: decision
            .speed
            .as_ref()
            .map(|speed| speed.physical_status.clone()),
    }
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

fn derive_snapshot_identity(
    decoded: &DecodedDataset,
    drafts: &[SegmentDraft],
) -> Result<(u64, String), OsmError> {
    #[derive(Serialize)]
    struct SemanticPreimage<'a> {
        domain: &'static str,
        normalized_payload_sha256: &'a str,
        normalization_version: &'a str,
        routing_profile: &'static str,
        jurisdiction_policy: &'static str,
        compiler_semantic_version: &'static str,
        graph_schema_version: u32,
        provenance_schema_version: u32,
        build_configuration: &'static str,
        turn_restrictions_enforced: bool,
        drafts: &'a [SegmentDraft],
    }
    let preimage = serde_json::to_vec(&SemanticPreimage {
        domain: "roadrunner.graph-snapshot-digest.v1",
        normalized_payload_sha256: &decoded.payload_sha256,
        normalization_version: &decoded.dataset.normalization_version,
        routing_profile: DELIVERY_MOTORCYCLE_PROFILE,
        jurisdiction_policy: NG_JURISDICTION_POLICY,
        compiler_semantic_version: COMPILER_SEMANTIC_VERSION,
        graph_schema_version: 2,
        provenance_schema_version: 1,
        build_configuration: BUILD_CONFIGURATION,
        turn_restrictions_enforced: false,
        drafts,
    })?;
    let digest = Sha256::digest(preimage);
    let mut prefix = [0_u8; 8];
    prefix.copy_from_slice(&digest[..8]);
    Ok((u64::from_be_bytes(prefix), format!("{digest:x}")))
}

fn builder_node_id(osm_id: i64) -> Result<BuilderNodeId, OsmError> {
    u64::try_from(osm_id)
        .map(BuilderNodeId::new)
        .map_err(|_| invalid(format!("OSM node ID {osm_id} cannot be a builder identity")))
}

#[allow(clippy::too_many_lines)]
fn component_diagnostics(
    graph: &FrozenGraph,
    dataset: &crate::model::NormalizedOsmDataset,
    provenance: &GraphProvenance,
) -> ComponentDiagnostics {
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
    let mut members = BTreeMap::<usize, Vec<usize>>::new();
    for node in 0..parent.len() {
        let root = find(&mut parent, node);
        members.entry(root).or_default().push(node);
    }
    let isolated_node_count = has_edge.into_iter().filter(|value| !value).count();
    let mut weak_members: Vec<_> = members.into_values().collect();
    weak_members.sort_by(|left, right| {
        right
            .len()
            .cmp(&left.len())
            .then_with(|| left[0].cmp(&right[0]))
    });
    let mut weak_component_node_counts: Vec<_> = weak_members.iter().map(Vec::len).collect();
    weak_component_node_counts.sort_unstable_by(|left, right| right.cmp(left));
    let (strong_component_node_counts, strong_labels) = strong_components(graph);
    let source_roots = source_component_roots(dataset);
    let dense_to_source: BTreeMap<usize, i64> = provenance
        .nodes
        .iter()
        .map(|mapping| (mapping.node_id as usize, mapping.osm_node_id))
        .collect();
    let component_source_roots: Vec<BTreeSet<usize>> = weak_members
        .iter()
        .map(|nodes| {
            nodes
                .iter()
                .filter_map(|node| dense_to_source.get(node))
                .filter_map(|osm_id| source_roots.get(osm_id))
                .copied()
                .collect()
        })
        .collect();
    let dense_to_component: BTreeMap<usize, usize> = weak_members
        .iter()
        .enumerate()
        .flat_map(|(component, nodes)| nodes.iter().map(move |node| (*node, component)))
        .collect();
    let source_to_component: BTreeMap<i64, usize> = dense_to_source
        .iter()
        .filter_map(|(dense, source)| {
            dense_to_component
                .get(dense)
                .map(|component| (*source, *component))
        })
        .collect();
    let tagged_nodes: BTreeMap<_, _> = dataset
        .nodes
        .iter()
        .map(|node| (node.osm_id, &node.tags))
        .collect();
    let mut barrier_evidence = BTreeMap::<usize, BTreeSet<i64>>::new();
    for way in &dataset.ways {
        let touched: BTreeSet<_> = way
            .node_refs
            .iter()
            .filter_map(|node| source_to_component.get(node))
            .copied()
            .collect();
        let has_barrier = way.node_refs.iter().any(|node| {
            tagged_nodes
                .get(node)
                .is_some_and(|tags| tags.contains_key("barrier"))
        });
        if touched.len() > 1 && has_barrier {
            for component in touched {
                barrier_evidence
                    .entry(component)
                    .or_default()
                    .insert(way.osm_id);
            }
        }
    }
    let mut root_use_counts = BTreeMap::<usize, usize>::new();
    for roots in &component_source_roots {
        for root in roots {
            *root_use_counts.entry(*root).or_default() += 1;
        }
    }
    let mut records = Vec::with_capacity(weak_members.len());
    for (component_id, nodes) in weak_members.iter().enumerate() {
        let node_set: BTreeSet<_> = nodes.iter().copied().collect();
        let segment_ids: BTreeSet<_> = graph
            .segments()
            .iter()
            .filter(|segment| node_set.contains(&(segment.canonical_from().value() as usize)))
            .map(|segment| segment.id().value())
            .collect();
        let source_way_ids: Vec<_> = provenance
            .ways
            .iter()
            .filter(|way| {
                way.segments
                    .iter()
                    .any(|segment| segment_ids.contains(&segment.segment_id))
            })
            .map(|way| way.osm_way_id)
            .collect();
        let coordinates: Vec<_> = nodes
            .iter()
            .filter_map(|node| u32::try_from(*node).ok())
            .filter_map(|node| graph.node(roadrunner_core::graph::NodeId::new(node)))
            .map(|node| node.canonical_coordinate())
            .collect();
        let strong_count = nodes
            .iter()
            .filter_map(|node| strong_labels.get(*node))
            .copied()
            .collect::<BTreeSet<_>>()
            .len();
        let roots = &component_source_roots[component_id];
        let cause = if barrier_evidence.contains_key(&component_id) {
            "barrier_node_profile_exclusion"
        } else if roots.len() > 1 {
            "compiled_connection_across_source_components"
        } else if roots
            .iter()
            .any(|root| root_use_counts.get(root).copied().unwrap_or(0) > 1)
        {
            "profile_or_compiler_fragmentation"
        } else {
            "disconnected_in_retained_source_topology"
        };
        records.push(ComponentRecord {
            component_id,
            node_count: nodes.len(),
            segment_count: segment_ids.len(),
            directed_edge_count: graph
                .edges()
                .iter()
                .filter(|edge| segment_ids.contains(&edge.segment().value()))
                .count(),
            source_way_count: source_way_ids.len(),
            strong_component_count: strong_count,
            min_latitude_e7: coordinates
                .iter()
                .map(|coordinate| coordinate.latitude_e7())
                .min()
                .unwrap_or(0),
            max_latitude_e7: coordinates
                .iter()
                .map(|coordinate| coordinate.latitude_e7())
                .max()
                .unwrap_or(0),
            min_longitude_e7: coordinates
                .iter()
                .map(|coordinate| coordinate.longitude_e7())
                .min()
                .unwrap_or(0),
            max_longitude_e7: coordinates
                .iter()
                .map(|coordinate| coordinate.longitude_e7())
                .max()
                .unwrap_or(0),
            cause: cause.to_owned(),
            source_way_ids,
            fragmentation_evidence_way_ids: barrier_evidence
                .get(&component_id)
                .map_or_else(Vec::new, |ways| ways.iter().copied().collect()),
        });
    }
    ComponentDiagnostics {
        weak_component_count: records.len(),
        weak_component_node_counts,
        isolated_node_count,
        strong_component_count: strong_component_node_counts.len(),
        strong_component_node_counts,
        components: records,
    }
}

fn source_component_roots(dataset: &crate::model::NormalizedOsmDataset) -> BTreeMap<i64, usize> {
    let ids: BTreeMap<_, _> = dataset
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.osm_id, index))
        .collect();
    let mut parent: Vec<_> = (0..dataset.nodes.len()).collect();
    for way in &dataset.ways {
        for pair in way.node_refs.windows(2) {
            if let (Some(left), Some(right)) = (ids.get(&pair[0]), ids.get(&pair[1])) {
                union(&mut parent, *left, *right);
            }
        }
    }
    ids.into_iter()
        .map(|(osm_id, index)| (osm_id, find(&mut parent, index)))
        .collect()
}

fn strong_components(graph: &FrozenGraph) -> (Vec<usize>, Vec<usize>) {
    fn visit(node: usize, adjacency: &[Vec<usize>], seen: &mut [bool], order: &mut Vec<usize>) {
        if seen[node] {
            return;
        }
        seen[node] = true;
        for target in &adjacency[node] {
            visit(*target, adjacency, seen, order);
        }
        order.push(node);
    }
    fn assign(
        node: usize,
        reverse: &[Vec<usize>],
        component: usize,
        labels: &mut [usize],
        size: &mut usize,
    ) {
        if labels[node] != usize::MAX {
            return;
        }
        labels[node] = component;
        *size += 1;
        for target in &reverse[node] {
            assign(*target, reverse, component, labels, size);
        }
    }
    let mut adjacency = vec![Vec::new(); graph.node_count()];
    let mut reverse = vec![Vec::new(); graph.node_count()];
    for edge in graph.edges() {
        adjacency[edge.from().value() as usize].push(edge.to().value() as usize);
        reverse[edge.to().value() as usize].push(edge.from().value() as usize);
    }
    let mut seen = vec![false; graph.node_count()];
    let mut order = Vec::with_capacity(graph.node_count());
    for node in 0..graph.node_count() {
        visit(node, &adjacency, &mut seen, &mut order);
    }
    let mut labels = vec![usize::MAX; graph.node_count()];
    let mut sizes = Vec::new();
    for node in order.into_iter().rev() {
        if labels[node] == usize::MAX {
            let mut size = 0;
            assign(node, &reverse, sizes.len(), &mut labels, &mut size);
            sizes.push(size);
        }
    }
    sizes.sort_unstable_by(|left, right| right.cmp(left));
    (sizes, labels)
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
