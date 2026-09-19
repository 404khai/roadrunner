//! End-to-end correctness checks for the Phase 7 OSM pipeline.

use std::collections::BTreeMap;

use roadrunner_core::geo::CanonicalCoordinate;
use roadrunner_osm::{
    DatasetProvenance, NormalizedNode, NormalizedOsmDataset, NormalizedRelation,
    NormalizedRelationMember, NormalizedSplitPoint, NormalizedWay, OsmElementKind, RestrictionKind,
    SplitPoint, compile_motorcycle_graph, decode_dataset_artifact, encode_dataset_artifact,
    extract_pbf,
};

const SOURCE_ID: &str = "openstreetmap-api:map:3.3780,6.5230,3.3810,6.5260:2026-09-19";

fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures/phase-7/lagos-marina.osm.pbf")
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
        "018e45caa93284192c98e7d1b20ccf2a8f9923c2a28428311724240086fe4664"
    );
    let compiled = must(compile_motorcycle_graph(&decoded));
    assert_eq!(compiled.graph.node_count(), 57);
    assert_eq!(compiled.graph.segment_count(), 58);
    assert_eq!(compiled.graph.edge_count(), 112);
    assert_eq!(compiled.manifest.compiled_way_count, 36);
    assert_eq!(compiled.manifest.excluded_way_count, 1);
    assert_eq!(compiled.manifest.components.weak_component_count, 6);
    assert!(!compiled.graph.metadata().turn_restrictions_enforced());
    assert!(!compiled.manifest.turn_restrictions_enforced);
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
fn restriction_via_node_is_a_split_but_not_an_enforced_maneuver() {
    let dataset = synthetic_dataset();
    let encoded = must(encode_dataset_artifact(&dataset));
    let decoded = must(decode_dataset_artifact(&encoded));
    let compiled = must(compile_motorcycle_graph(&decoded));

    assert_eq!(compiled.graph.node_count(), 3);
    assert_eq!(compiled.graph.segment_count(), 2);
    assert_eq!(compiled.graph.edge_count(), 4);
    assert_eq!(compiled.manifest.preserved_restriction_count, 1);
    assert!(!compiled.manifest.turn_restrictions_enforced);
}

fn synthetic_dataset() -> NormalizedOsmDataset {
    NormalizedOsmDataset {
        normalization_version: "osm_normalization_v1".to_owned(),
        provenance: DatasetProvenance {
            source_id: "synthetic:restriction-boundary".to_owned(),
            source_sha256: "a".repeat(64),
            source_size_bytes: 1,
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
            kind: RestrictionKind::No,
            restriction: "no_u_turn".to_owned(),
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
    NormalizedNode { osm_id, coordinate }
}

fn must<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("unexpected error: {error:?}"),
    }
}
