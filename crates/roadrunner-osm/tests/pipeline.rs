//! End-to-end correctness checks for the Phase 7 OSM pipeline.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use roadrunner_core::cost::{DistanceCost, RoutingContext};
use roadrunner_core::geo::CanonicalCoordinate;
use roadrunner_core::graph::{FrozenGraph, NodeId};
use roadrunner_core::routing::{DistanceHaversine, RouteResult, RoutingError, astar, dijkstra};
use roadrunner_osm::{
    DatasetProvenance, NormalizedNode, NormalizedOsmDataset, NormalizedRelation,
    NormalizedRelationMember, NormalizedRestriction, NormalizedSplitPoint, NormalizedWay,
    OsmElementKind, RestrictionKind, SourceStatistics, SplitPoint, compile_motorcycle_graph,
    decode_dataset_artifact, encode_dataset_artifact, extract_pbf, load_snapshot_bundle,
    write_snapshot_bundle_atomic,
};
use serde::Deserialize;

const SOURCE_ID: &str = "openstreetmap-api:map:3.3780,6.5230,3.3810,6.5260:2026-09-19";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures/phase-7/lagos-marina.osm.pbf")
}

fn semantics_fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures/phase-7/source-semantics.osm.pbf")
}

#[test]
fn real_pbf_extracts_and_compiles_with_pinned_results() {
    let dataset = must(extract_pbf(fixture_path(), SOURCE_ID));
    assert_eq!(
        dataset.provenance.source_sha256,
        "f1f0f4e7abfcc9396c6b67bbd08b720a0481bf69a636d6c07c50980ea15b8687"
    );
    assert_eq!(dataset.nodes.len(), 218);
    assert_eq!(dataset.ways.len(), 37);
    assert_eq!(dataset.relations.len(), 0);
    assert!(
        dataset
            .nodes
            .windows(2)
            .all(|pair| pair[0].osm_id < pair[1].osm_id)
    );
    assert!(
        dataset
            .ways
            .windows(2)
            .all(|pair| pair[0].osm_id < pair[1].osm_id)
    );

    let first = must(encode_dataset_artifact(&dataset));
    let second_dataset = must(extract_pbf(fixture_path(), SOURCE_ID));
    let second = must(encode_dataset_artifact(&second_dataset));
    assert_eq!(first, second);

    let decoded = must(decode_dataset_artifact(&first));
    assert_eq!(
        decoded.payload_sha256(),
        "fbecddd9af9a77383a350bb806cf68072d6d4b506a5fcf43d63ddc076f1b3983"
    );
    let compiled = must(compile_motorcycle_graph(&decoded));
    assert_eq!(compiled.graph.node_count(), 58);
    assert_eq!(compiled.graph.segment_count(), 60);
    assert_eq!(compiled.graph.edge_count(), 114);
    assert_eq!(compiled.manifest.components.weak_component_count, 5);
    assert!(compiled.graph.metadata().turn_restrictions_enforced());
    assert!(compiled.manifest.turn_restrictions_enforced);
}

#[test]
fn source_schema_preserves_node_connectors_and_restriction_variants() {
    let dataset = must(extract_pbf(
        semantics_fixture_path(),
        "synthetic:phase7-source-semantics",
    ));
    assert_eq!(dataset.nodes.len(), 5);
    assert_eq!(dataset.ways.len(), 3);
    assert_eq!(dataset.relations.len(), 2);
    let barrier = dataset
        .nodes
        .iter()
        .find(|node| node.osm_id == 2)
        .unwrap_or_else(|| panic!("barrier node"));
    assert_eq!(
        barrier.tags.get("barrier").map(String::as_str),
        Some("gate")
    );
    assert_eq!(
        barrier.tags.get("access").map(String::as_str),
        Some("delivery")
    );
    assert!(dataset.ways.iter().any(|way| {
        way.osm_id == 300 && way.tags.get("route").map(String::as_str) == Some("ferry")
    }));
    let relation = dataset
        .relations
        .iter()
        .find(|relation| relation.osm_id == 400)
        .unwrap_or_else(|| panic!("qualified restriction"));
    assert_eq!(relation.restrictions.len(), 2);
    assert_eq!(relation.restrictions[0].tag, "restriction");
    assert_eq!(relation.restrictions[1].tag, "restriction:motorcycle");
    let via = dataset
        .split_points
        .iter()
        .find(|split| split.osm_node_id == 2)
        .unwrap_or_else(|| panic!("via barrier split"));
    assert!(via.reasons.contains(&SplitPoint::RestrictionVia));
    assert!(via.reasons.contains(&SplitPoint::Barrier));
    assert!(via.reasons.contains(&SplitPoint::AccessBoundary));

    let bytes = must(encode_dataset_artifact(&dataset));
    let decoded = must(decode_dataset_artifact(&bytes));
    let compiled = must(compile_motorcycle_graph(&decoded));
    assert_eq!(compiled.provenance.restrictions.len(), 2);
    assert_eq!(
        compiled.provenance.restrictions[0].status,
        "ambiguous_graph_traversal"
    );
    assert_eq!(
        compiled.provenance.restrictions[1].status,
        "unresolved_source_member"
    );
    assert!(
        compiled
            .provenance
            .ways
            .iter()
            .any(|way| way.osm_way_id == 300 && way.segments.is_empty())
    );
    assert!(compiled.graph.metadata().turn_restrictions_enforced());
    let provenance_bytes = must(roadrunner_osm::encode_provenance_artifact(
        &compiled.provenance,
        &compiled.graph,
    ));
    let round_trip = must(roadrunner_osm::decode_provenance_artifact(
        &provenance_bytes,
        &compiled.graph,
    ));
    let reencoded = must(roadrunner_osm::encode_provenance_artifact(
        &round_trip,
        &compiled.graph,
    ));
    assert_eq!(provenance_bytes, reencoded);
}

#[test]
fn semantic_graph_changes_change_authoritative_snapshot_digest() {
    let dataset = must(extract_pbf(fixture_path(), SOURCE_ID));
    let decoded = must(decode_dataset_artifact(&must(encode_dataset_artifact(
        &dataset,
    ))));
    let original = must(compile_motorcycle_graph(&decoded));
    let mut changed_dataset = dataset;
    let way = changed_dataset
        .ways
        .first_mut()
        .unwrap_or_else(|| panic!("fixture way"));
    way.unsupported_tags
        .insert("surface".to_owned(), "paved".to_owned());
    let changed = must(decode_dataset_artifact(&must(encode_dataset_artifact(
        &changed_dataset,
    ))));
    let changed = must(compile_motorcycle_graph(&changed));
    assert_ne!(
        original.manifest.graph_snapshot_digest,
        changed.manifest.graph_snapshot_digest
    );
    assert_ne!(
        original.manifest.graph_snapshot_id,
        changed.manifest.graph_snapshot_id
    );
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "roadrunner-phase7-snapshot-swap-{}-{sequence}",
        std::process::id()
    ));
    let original_path = root.join("original");
    let changed_path = root.join("changed");
    must(std::fs::create_dir(&root));
    must(write_snapshot_bundle_atomic(&original_path, &original));
    must(write_snapshot_bundle_atomic(&changed_path, &changed));
    must(std::fs::copy(
        changed_path.join("graph.rr-provenance"),
        original_path.join("graph.rr-provenance"),
    ));
    assert!(load_snapshot_bundle(&original_path, false).is_err());
    must(std::fs::remove_dir_all(&root));
}

#[derive(Deserialize)]
struct RouteCorpus {
    schema_version: u32,
    fixture: String,
    queries: Vec<RouteQuery>,
}

#[derive(Deserialize)]
struct RouteQuery {
    id: String,
    from_osm_node: i64,
    to_osm_node: i64,
    expected: String,
}

#[test]
fn real_snapshot_round_trip_preserves_validated_routes() {
    let dataset = must(extract_pbf(fixture_path(), SOURCE_ID));
    let normalized = must(encode_dataset_artifact(&dataset));
    let decoded = must(decode_dataset_artifact(&normalized));
    let compiled = must(compile_motorcycle_graph(&decoded));
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let snapshot_path = std::env::temp_dir().join(format!(
        "roadrunner-phase7-route-corpus-{}-{sequence}",
        std::process::id()
    ));
    must(write_snapshot_bundle_atomic(&snapshot_path, &compiled));
    let loaded = must(load_snapshot_bundle(&snapshot_path, true));
    let corpus_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures/phase-7/route-corpus.json");
    let corpus: RouteCorpus = must(
        std::fs::read(&corpus_path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|error| error.to_string())),
    );
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.fixture, "lagos-marina.osm.pbf");
    for query in corpus.queries {
        let source = source_node(&compiled, query.from_osm_node);
        let destination = source_node(&compiled, query.to_osm_node);
        let fresh_dijkstra = dijkstra(
            &compiled.graph,
            source,
            destination,
            &DistanceCost,
            &RoutingContext::new(),
        );
        let loaded_dijkstra = dijkstra(
            &loaded.graph,
            source,
            destination,
            &DistanceCost,
            &RoutingContext::new(),
        );
        match query.expected.as_str() {
            "reachable" => {
                let fresh = fresh_dijkstra.unwrap_or_else(|error| {
                    panic!("query {} should be reachable: {error}", query.id)
                });
                let loaded_route = loaded_dijkstra.unwrap_or_else(|error| {
                    panic!("loaded query {} should be reachable: {error}", query.id)
                });
                assert_eq!(fresh, loaded_route, "fresh/load mismatch for {}", query.id);
                validate_distance_route(&compiled.graph, &fresh, source, destination);
                let a_star = astar(
                    &loaded.graph,
                    source,
                    destination,
                    &DistanceCost,
                    &DistanceHaversine,
                    &RoutingContext::new(),
                )
                .unwrap_or_else(|error| panic!("A* query {} failed: {error}", query.id));
                validate_distance_route(&loaded.graph, &a_star, source, destination);
                assert_eq!(fresh.total_cost(), a_star.total_cost());
            }
            "unreachable" => {
                assert!(matches!(fresh_dijkstra, Err(RoutingError::NoRoute { .. })));
                assert!(matches!(loaded_dijkstra, Err(RoutingError::NoRoute { .. })));
            }
            other => panic!("query {} has unsupported expectation {other}", query.id),
        }
    }
    assert!(std::fs::remove_dir_all(&snapshot_path).is_ok());
}

fn source_node(compiled: &roadrunner_osm::CompiledGraph, osm_node_id: i64) -> NodeId {
    compiled
        .provenance
        .nodes
        .iter()
        .find(|mapping| mapping.osm_node_id == osm_node_id)
        .map_or_else(
            || panic!("missing source node {osm_node_id}"),
            |mapping| NodeId::new(mapping.node_id),
        )
}

fn validate_distance_route(
    graph: &FrozenGraph,
    route: &RouteResult,
    source: NodeId,
    destination: NodeId,
) {
    assert_eq!(route.path().first(), Some(&source));
    assert_eq!(route.path().last(), Some(&destination));
    assert_eq!(route.path().len(), route.edges().len() + 1);
    assert_eq!(route.graph_snapshot_id(), graph.snapshot_id());
    assert_eq!(
        route.graph_snapshot_digest(),
        graph.metadata().snapshot_digest()
    );
    let mut distance = 0.0;
    let mut elapsed = 0.0;
    for (index, edge_id) in route.edges().iter().enumerate() {
        let edge = graph
            .edge(*edge_id)
            .unwrap_or_else(|| panic!("missing route edge {edge_id}"));
        assert_eq!(edge.from(), route.path()[index]);
        assert_eq!(edge.to(), route.path()[index + 1]);
        let segment = graph
            .segment(edge.segment())
            .unwrap_or_else(|| panic!("missing route segment {}", edge.segment()));
        distance += segment.distance().value();
        elapsed += edge.free_flow_travel_time().value();
        let geometry = must(graph.segment_geometry(segment.id()));
        let expected_endpoints = match edge.orientation() {
            roadrunner_core::graph::Orientation::Forward => (geometry.first(), geometry.last()),
            roadrunner_core::graph::Orientation::Reverse => (geometry.last(), geometry.first()),
        };
        assert_eq!(
            expected_endpoints.0.copied(),
            graph
                .node(edge.from())
                .map(|node| node.canonical_coordinate())
        );
        assert_eq!(
            expected_endpoints.1.copied(),
            graph
                .node(edge.to())
                .map(|node| node.canonical_coordinate())
        );
    }
    assert!((distance - route.total_distance().value()).abs() < f64::EPSILON);
    assert!((distance - route.total_cost().value()).abs() < f64::EPSILON);
    assert!((elapsed - route.elapsed_travel_time().value()).abs() < f64::EPSILON);
}

#[test]
fn artifact_integrity_failure_is_rejected() {
    let dataset = synthetic_dataset();
    let mut encoded = must(encode_dataset_artifact(&dataset));
    let Some(position) = encoded.iter().position(|byte| *byte == b'3') else {
        panic!("fixture contains a digit");
    };
    encoded[position] = b'4';
    assert!(decode_dataset_artifact(&encoded).is_err());
}

#[test]
fn noncanonical_artifact_framing_is_rejected() {
    let dataset = synthetic_dataset();
    let mut encoded = must(encode_dataset_artifact(&dataset));
    encoded.insert(0, b' ');
    assert!(decode_dataset_artifact(&encoded).is_err());
}

#[test]
fn unsupported_restriction_shape_is_diagnosed_while_capability_is_enabled() {
    let dataset = synthetic_dataset();
    let encoded = must(encode_dataset_artifact(&dataset));
    let decoded = must(decode_dataset_artifact(&encoded));
    let compiled = must(compile_motorcycle_graph(&decoded));

    assert_eq!(compiled.graph.node_count(), 3);
    assert_eq!(compiled.graph.segment_count(), 2);
    assert_eq!(compiled.graph.edge_count(), 4);
    assert_eq!(compiled.manifest.preserved_restriction_count, 1);
    assert!(compiled.manifest.turn_restrictions_enforced);
    assert_eq!(
        compiled.provenance.restrictions[0].status,
        "ambiguous_member_shape"
    );
}

#[test]
fn no_turn_compiles_to_a_forbidden_pair_and_changes_the_route() {
    let compiled = compile_dataset(&maneuver_dataset(
        vec![restriction(
            "restriction",
            "no_right_turn",
            RestrictionKind::No,
        )],
        None,
        true,
    ));
    let provenance = &compiled.provenance.restrictions[0];
    assert_eq!(provenance.status, "enforced_no");
    assert_eq!(provenance.forbidden_maneuvers.len(), 1);
    let route = must(dijkstra(
        &compiled.graph,
        source_node(&compiled, 1),
        source_node(&compiled, 3),
        &DistanceCost,
        &RoutingContext::new(),
    ));
    let forbidden = &provenance.forbidden_maneuvers[0];
    assert!(!route.edges().windows(2).any(|pair| {
        pair[0].value() == forbidden.incoming_edge_id
            && pair[1].value() == forbidden.outgoing_edge_id
    }));
    assert!(route.path().contains(&source_node(&compiled, 4)));
}

#[test]
fn only_turn_forbids_every_other_exit_and_motorcycle_value_wins() {
    let compiled = compile_dataset(&maneuver_dataset(
        vec![
            restriction("restriction", "no_right_turn", RestrictionKind::No),
            restriction(
                "restriction:motorcycle",
                "only_straight_on",
                RestrictionKind::Only,
            ),
        ],
        None,
        false,
    ));
    let provenance = &compiled.provenance.restrictions[0];
    assert_eq!(provenance.status, "enforced_only");
    assert_eq!(
        provenance.applied_tag.as_deref(),
        Some("restriction:motorcycle")
    );
    assert_eq!(provenance.forbidden_maneuvers.len(), 2);
    assert!(
        provenance
            .forbidden_maneuvers
            .iter()
            .all(|maneuver| { Some(maneuver.outgoing_edge_id) != provenance.outgoing_edge_id })
    );
}

#[test]
fn generic_restriction_exception_for_motorcycles_is_diagnosed_not_enforced() {
    let compiled = compile_dataset(&maneuver_dataset(
        vec![restriction(
            "restriction",
            "no_left_turn",
            RestrictionKind::No,
        )],
        Some("motorcycle"),
        false,
    ));
    let provenance = &compiled.provenance.restrictions[0];
    assert_eq!(provenance.status, "not_applicable_to_motorcycle");
    assert!(provenance.forbidden_maneuvers.is_empty());
    assert!(compiled.graph.forbidden_maneuvers().is_empty());
}

#[test]
fn no_u_turn_on_the_same_way_resolves_opposite_traversals() {
    let mut dataset = maneuver_dataset(
        vec![restriction("restriction", "no_u_turn", RestrictionKind::No)],
        None,
        false,
    );
    dataset.relations[0].members[2].osm_id = 10;
    let compiled = compile_dataset(&dataset);
    let provenance = &compiled.provenance.restrictions[0];
    assert_eq!(provenance.status, "enforced_no");
    let maneuver = &provenance.forbidden_maneuvers[0];
    let incoming = compiled
        .graph
        .edge(roadrunner_core::graph::EdgeId::new(
            maneuver.incoming_edge_id,
        ))
        .unwrap_or_else(|| panic!("incoming edge"));
    let outgoing = compiled
        .graph
        .edge(roadrunner_core::graph::EdgeId::new(
            maneuver.outgoing_edge_id,
        ))
        .unwrap_or_else(|| panic!("outgoing edge"));
    assert_eq!(incoming.segment(), outgoing.segment());
    assert_ne!(incoming.orientation(), outgoing.orientation());
    assert!(
        !compiled
            .graph
            .is_maneuver_allowed(incoming.id(), outgoing.id())
    );
}

fn compile_dataset(dataset: &NormalizedOsmDataset) -> roadrunner_osm::CompiledGraph {
    let encoded = must(encode_dataset_artifact(dataset));
    let decoded = must(decode_dataset_artifact(&encoded));
    must(compile_motorcycle_graph(&decoded))
}

fn restriction(tag: &str, value: &str, kind: RestrictionKind) -> NormalizedRestriction {
    NormalizedRestriction {
        tag: tag.to_owned(),
        value: value.to_owned(),
        kind,
        conditional: false,
    }
}

fn maneuver_dataset(
    restrictions: Vec<NormalizedRestriction>,
    except: Option<&str>,
    one_way: bool,
) -> NormalizedOsmDataset {
    let mut from_tags = BTreeMap::from([("highway".to_owned(), "residential".to_owned())]);
    if one_way {
        from_tags.insert("oneway".to_owned(), "yes".to_owned());
    }
    let road_tags = BTreeMap::from([("highway".to_owned(), "residential".to_owned())]);
    NormalizedOsmDataset {
        normalization_version: "osm_normalization_v2".to_owned(),
        provenance: DatasetProvenance {
            source_id: "synthetic:maneuver-aware".to_owned(),
            source_sha256: "b".repeat(64),
            source_size_bytes: 1,
        },
        source_statistics: SourceStatistics {
            nodes_seen: 4,
            ways_seen: 4,
            candidate_ways: 4,
            relations_seen: 1,
            restriction_relations_seen: 1,
            referenced_nodes_requested: 4,
            referenced_nodes_resolved: 4,
            referenced_nodes_missing: 0,
        },
        nodes: vec![
            coordinate_node(1, 65_240_000, 33_780_000),
            coordinate_node(2, 65_240_000, 33_790_000),
            coordinate_node(3, 65_240_000, 33_800_000),
            coordinate_node(4, 65_250_000, 33_790_000),
        ],
        ways: vec![
            way(10, vec![1, 2], from_tags),
            way(20, vec![2, 3], road_tags.clone()),
            way(30, vec![2, 4], road_tags.clone()),
            way(40, vec![4, 3], road_tags),
        ],
        relations: vec![NormalizedRelation {
            osm_id: 100,
            restrictions,
            except: except.map(str::to_owned),
            members: vec![
                relation_member(OsmElementKind::Way, 10, "from"),
                relation_member(OsmElementKind::Node, 2, "via"),
                relation_member(OsmElementKind::Way, 20, "to"),
            ],
            unsupported_tags: BTreeMap::new(),
        }],
        split_points: vec![
            split(1, vec![SplitPoint::WayEndpoint]),
            split(
                2,
                vec![
                    SplitPoint::WayEndpoint,
                    SplitPoint::Junction,
                    SplitPoint::RestrictionVia,
                ],
            ),
            split(3, vec![SplitPoint::WayEndpoint, SplitPoint::Junction]),
            split(4, vec![SplitPoint::WayEndpoint, SplitPoint::Junction]),
        ],
    }
}

fn way(osm_id: i64, node_refs: Vec<i64>, tags: BTreeMap<String, String>) -> NormalizedWay {
    NormalizedWay {
        osm_id,
        node_refs,
        tags,
        unsupported_tags: BTreeMap::new(),
    }
}

fn relation_member(kind: OsmElementKind, osm_id: i64, role: &str) -> NormalizedRelationMember {
    NormalizedRelationMember {
        kind,
        osm_id,
        role: role.to_owned(),
    }
}

fn split(osm_node_id: i64, reasons: Vec<SplitPoint>) -> NormalizedSplitPoint {
    NormalizedSplitPoint {
        osm_node_id,
        reasons,
    }
}

fn coordinate_node(osm_id: i64, latitude_e7: i32, longitude_e7: i32) -> NormalizedNode {
    let coordinate = must(CanonicalCoordinate::new(latitude_e7, longitude_e7));
    NormalizedNode {
        osm_id,
        coordinate,
        tags: BTreeMap::new(),
        unsupported_tags: BTreeMap::new(),
    }
}

fn synthetic_dataset() -> NormalizedOsmDataset {
    NormalizedOsmDataset {
        normalization_version: "osm_normalization_v2".to_owned(),
        provenance: DatasetProvenance {
            source_id: "synthetic:restriction-boundary".to_owned(),
            source_sha256: "a".repeat(64),
            source_size_bytes: 1,
        },
        source_statistics: SourceStatistics {
            nodes_seen: 3,
            ways_seen: 1,
            candidate_ways: 1,
            relations_seen: 1,
            restriction_relations_seen: 1,
            referenced_nodes_requested: 3,
            referenced_nodes_resolved: 3,
            referenced_nodes_missing: 0,
        },
        nodes: vec![
            node(1, 65_240_000),
            node(2, 65_241_000),
            node(3, 65_242_000),
        ],
        ways: vec![NormalizedWay {
            osm_id: 10,
            node_refs: vec![1, 2, 3],
            tags: BTreeMap::from([("highway".to_owned(), "residential".to_owned())]),
            unsupported_tags: BTreeMap::new(),
        }],
        relations: vec![NormalizedRelation {
            osm_id: 20,
            restrictions: vec![NormalizedRestriction {
                tag: "restriction".to_owned(),
                value: "no_u_turn".to_owned(),
                kind: RestrictionKind::No,
                conditional: false,
            }],
            except: None,
            members: vec![NormalizedRelationMember {
                kind: OsmElementKind::Node,
                osm_id: 2,
                role: "via".to_owned(),
            }],
            unsupported_tags: BTreeMap::new(),
        }],
        split_points: vec![
            NormalizedSplitPoint {
                osm_node_id: 1,
                reasons: vec![SplitPoint::WayEndpoint],
            },
            NormalizedSplitPoint {
                osm_node_id: 2,
                reasons: vec![SplitPoint::RestrictionVia],
            },
            NormalizedSplitPoint {
                osm_node_id: 3,
                reasons: vec![SplitPoint::WayEndpoint],
            },
        ],
    }
}

fn node(osm_id: i64, latitude_e7: i32) -> NormalizedNode {
    let Ok(coordinate) = CanonicalCoordinate::new(latitude_e7, 33_790_000) else {
        panic!("test coordinate is valid");
    };
    NormalizedNode {
        osm_id,
        coordinate,
        tags: BTreeMap::new(),
        unsupported_tags: BTreeMap::new(),
    }
}

fn must<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("unexpected error: {error:?}"),
    }
}
