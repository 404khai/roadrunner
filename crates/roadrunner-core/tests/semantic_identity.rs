//! Semantic identity covers all supported immutable publication paths.
use roadrunner_core::geo::{CanonicalCoordinate, KilometersPerHour};
use roadrunner_core::graph::*;
use std::fmt::Debug;
fn ok<T, E: Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("{e:?}"))
}
fn graph(mode: u8, reverse: bool) -> FrozenGraph {
    let mut b = GraphBuilder::new(
        GraphSnapshotId::new(123),
        GraphMetadata::new(
            "motorcycle/v1",
            "fixture",
            GraphBuildIdentity::new(
                "identity-fixture",
                "compiler/v1",
                "normalization/v1",
                "fixture",
            ),
        ),
    );
    let points = [
        ok(CanonicalCoordinate::new(0, 0)),
        ok(CanonicalCoordinate::new(0, 10_000)),
        ok(CanonicalCoordinate::new(0, 20_000)),
    ];
    for i in if reverse {
        vec![2_u64, 1, 0]
    } else {
        vec![0, 1, 2]
    } {
        ok(b.add_node(
            BuilderNodeId::new(i),
            points[usize::try_from(i).unwrap_or_else(|e| panic!("{e}"))],
        ));
    }
    for i in if reverse { vec![1_u64, 0] } else { vec![0, 1] } {
        let idx = usize::try_from(i).unwrap_or_else(|e| panic!("{e}"));
        let p = EdgeProperties::new(
            ok(KilometersPerHour::new(if mode == 2 { 25.0 } else { 30.0 })),
            None,
            if mode == 3 {
                AccessClass::Delivery
            } else {
                AccessClass::General
            },
        );
        let mut shape = vec![points[idx], points[idx + 1]];
        if mode == 1 {
            shape.insert(
                1,
                ok(CanonicalCoordinate::new(
                    10,
                    i32::try_from(idx).unwrap_or_else(|e| panic!("{e}")) * 10_000 + 5_000,
                )),
            );
        }
        ok(b.add_segment(
            BuilderSegmentId::new(i),
            BuilderNodeId::new(i),
            BuilderNodeId::new(i + 1),
            shape,
            Some(p),
            if mode == 4 { None } else { Some(p) },
        ));
    }
    ok(b.finalize())
}
#[test]
fn canonical_input_order_and_roundtrip_have_one_verified_identity() {
    let a = graph(0, false);
    let b = graph(0, true);
    assert_eq!(
        a.metadata().snapshot_digest(),
        b.metadata().snapshot_digest()
    );
    assert_eq!(ok(encode_graph_artifact(&a)), ok(encode_graph_artifact(&b)));
    let bytes = ok(encode_graph_artifact(&a));
    let decoded = ok(decode_graph_artifact(&bytes));
    assert_eq!(
        decoded.metadata().snapshot_digest(),
        a.metadata().snapshot_digest()
    );
    assert!(ok(inspect_graph_identity(&bytes)).semantic_identity_verified);
}
#[test]
fn geometry_speed_access_direction_and_maneuvers_cannot_alias() {
    let base = graph(0, false);
    let original = base.metadata().snapshot_digest().to_owned();
    for mode in 1..=4 {
        assert_ne!(graph(mode, false).metadata().snapshot_digest(), original);
    }
    let changed = ok(base.with_forbidden_maneuvers(vec![(EdgeId::new(0), EdgeId::new(2))]));
    assert_ne!(changed.metadata().snapshot_digest(), original);
    assert!(!changed.is_maneuver_allowed(EdgeId::new(0), EdgeId::new(2)));
    assert_eq!(
        ok(decode_graph_artifact(&ok(encode_graph_artifact(&changed))))
            .metadata()
            .snapshot_digest(),
        changed.metadata().snapshot_digest()
    );
}
#[test]
fn legacy_claims_are_inspection_only_and_never_relabelled() {
    let bytes = ok(encode_graph_artifact(&graph(0, false)));
    let historical = ok(String::from_utf8(bytes))
        .replace("\"schema_version\":4", "\"schema_version\":3")
        .into_bytes();
    let inspection = ok(inspect_graph_identity(&historical));
    assert!(!inspection.semantic_identity_verified);
    assert!(matches!(
        decode_graph_artifact(&historical),
        Err(GraphArtifactError::UnsupportedSchema { version: 3 })
    ));
    assert_eq!(
        inspection.claimed_digest,
        graph(0, false).metadata().snapshot_digest()
    );
}
