//! Independent E7 and topology oracles for detached geometry.
#![allow(clippy::float_cmp)]
use roadrunner_core::{
    geo::{CanonicalCoordinate, KilometersPerHour},
    graph::*,
    routing::*,
};
use std::fmt::Debug;
fn ok<T, E: Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}
fn fixture(zero: bool, offset: i32) -> FrozenGraph {
    let mut b = GraphBuilder::new(
        GraphSnapshotId::new(1),
        GraphMetadata::new(
            "motorcycle/v1",
            "fixture",
            GraphBuildIdentity::new("geometry", "1", "1", "1"),
        ),
    );
    let a = ok(CanonicalCoordinate::new(offset, 0));
    let c = ok(CanonicalCoordinate::new(
        offset,
        if zero { 0 } else { 20_000 },
    ));
    ok(b.add_node(BuilderNodeId::new(0), a));
    ok(b.add_node(BuilderNodeId::new(1), c));
    let p = EdgeProperties::new(ok(KilometersPerHour::new(30.0)), None, AccessClass::General);
    let middle = if zero {
        a
    } else {
        ok(CanonicalCoordinate::new(offset + 1000, 10_000))
    };
    ok(b.add_segment(
        BuilderSegmentId::new(0),
        BuilderNodeId::new(0),
        BuilderNodeId::new(1),
        vec![a, middle, middle, c],
        Some(p),
        Some(p),
    ));
    ok(b.finalize())
}
#[test]
fn actual_builder_artifact_zero_distance_discrepancy_is_contained() {
    let g = fixture(true, 0);
    assert_eq!(g.segments()[0].distance().value(), 0.0);
    let bytes = ok(encode_graph_artifact(&g));
    let error = decode_graph_artifact(&bytes)
        .err()
        .unwrap_or_else(|| panic!("decoder must reject ordinary zero-distance segment"));
    assert!(
        error
            .to_string()
            .contains("ordinary road segment distance must be positive")
    );
    assert_eq!(
        checked_route_geometry(
            Some(&g),
            g.metadata().snapshot_digest(),
            &[NodeId::new(0), NodeId::new(1)],
            &[EdgeId::new(0)],
            GeometryLimits::default()
        ),
        Err(GeometryError::Invalid)
    );
    let stationary = ok(checked_route_geometry(
        Some(&g),
        g.metadata().snapshot_digest(),
        &[NodeId::new(0)],
        &[],
        GeometryLimits::default(),
    ));
    assert!(stationary.no_travel);
    assert_eq!(stationary.points_e7.len(), 1);
}
#[test]
fn exact_reverse_repeated_occurrences_and_boundaries() {
    let g = fixture(false, 0);
    let nodes = [0, 1, 0, 1].map(NodeId::new);
    let edges = [0, 1, 0].map(EdgeId::new);
    let e = ok(checked_route_geometry(
        Some(&g),
        g.metadata().snapshot_digest(),
        &nodes,
        &edges,
        GeometryLimits::default(),
    ));
    let pairs: Vec<_> = e
        .points_e7
        .iter()
        .map(|p| (p.latitude_e7(), p.longitude_e7()))
        .collect();
    assert_eq!(
        pairs,
        vec![
            (0, 0),
            (1000, 10000),
            (1000, 10000),
            (0, 20000),
            (0, 20000),
            (1000, 10000),
            (1000, 10000),
            (0, 0),
            (0, 0),
            (1000, 10000),
            (1000, 10000),
            (0, 20000)
        ]
    );
    assert_eq!(
        e.occurrences
            .iter()
            .map(|o| (o.ordinal, o.point_start, o.point_end))
            .collect::<Vec<_>>(),
        vec![(0, 0, 4), (1, 4, 8), (2, 8, 12)]
    );
    assert_eq!(e.occurrences[1].orientation, Orientation::Reverse);
    assert_ne!(
        e.path_id,
        ok(checked_route_geometry(
            Some(&g),
            g.metadata().snapshot_digest(),
            &nodes[..2],
            &edges[..1],
            GeometryLimits::default()
        ))
        .path_id
    );
    let j = ok(serde_json::to_value(&e));
    assert_eq!(j["points_e7"][1]["longitude_e7"], 10000);
}
#[test]
fn digest_guard_precedes_local_id_resolution_and_limits_are_explicit() {
    let a = fixture(false, 0);
    let b = fixture(false, 999);
    let resolve = |g, d, n: &[NodeId], e: &[EdgeId], l| checked_route_geometry(g, d, n, e, l);
    assert_eq!(
        resolve(
            Some(&b),
            a.metadata().snapshot_digest(),
            &[NodeId::new(0), NodeId::new(1)],
            &[EdgeId::new(0)],
            GeometryLimits::default()
        ),
        Err(GeometryError::GraphMismatch)
    );
    assert_eq!(
        resolve(
            Some(&a),
            a.metadata().snapshot_digest(),
            &[],
            &[],
            GeometryLimits::default()
        ),
        Err(GeometryError::Invalid)
    );
    assert_eq!(
        resolve(
            Some(&a),
            a.metadata().snapshot_digest(),
            &[NodeId::new(999)],
            &[],
            GeometryLimits::default()
        ),
        Err(GeometryError::Invalid)
    );
    assert_eq!(
        resolve(
            Some(&b),
            a.metadata().snapshot_digest(),
            &[NodeId::new(999)],
            &[],
            GeometryLimits::default()
        ),
        Err(GeometryError::GraphMismatch)
    );
    assert_eq!(
        resolve(
            None,
            a.metadata().snapshot_digest(),
            &[],
            &[],
            GeometryLimits::default()
        ),
        Err(GeometryError::Unavailable)
    );
    assert_eq!(
        resolve(
            Some(&a),
            a.metadata().snapshot_digest(),
            &[NodeId::new(1), NodeId::new(0)],
            &[EdgeId::new(0)],
            GeometryLimits::default()
        ),
        Err(GeometryError::Invalid)
    );
    assert_eq!(
        resolve(
            Some(&a),
            a.metadata().snapshot_digest(),
            &[NodeId::new(0), NodeId::new(1)],
            &[EdgeId::new(0)],
            GeometryLimits {
                occurrences: 1,
                points: 3
            }
        ),
        Err(GeometryError::LimitExceeded)
    );
}

#[test]
fn required_semantics_fail_closed_optional_fields_degrade() {
    use serde_json::json;
    let base = json!({"schema_version":1,"geometry":"wgs84_e7","authority":"deterministic_prediction","future_optional":"ignored"});
    let semantics: RouteEvidenceSemantics = ok(serde_json::from_value(base.clone()));
    ok(semantics.check());
    for field in ["geometry", "authority"] {
        let mut unknown = base.clone();
        unknown[field] = json!("unknown_required");
        assert!(serde_json::from_value::<RouteEvidenceSemantics>(unknown).is_err());
    }
    let mut future = base;
    future["schema_version"] = json!(2);
    assert_eq!(
        ok(serde_json::from_value::<RouteEvidenceSemantics>(future)).check(),
        Err(GeometryError::Unsupported)
    );
}

#[test]
fn original_access_context_changes_evaluation_identity_without_changing_spatial_path() {
    use roadrunner_core::{
        cost::{DistanceCost, RoutingContext},
        geo::Seconds,
    };
    let g = fixture(false, 0);
    let export = |context: RoutingContext| {
        let route = ok(dijkstra(
            &g,
            NodeId::new(0),
            NodeId::new(1),
            &DistanceCost,
            &context,
        ));
        let provenance = EvaluationProvenance {
            evaluator: "distance/v1".into(),
            traffic_digest: None,
            departure_seconds: Seconds::ZERO,
            routing_context: context,
            time_domain: "fixture/v1".into(),
            heuristic: None,
        };
        ok(route_evidence(
            &g,
            &route,
            provenance,
            GeometryLimits::default(),
        ))
    };
    let a = export(RoutingContext::new());
    let b = export(RoutingContext::new().with_private_access());
    assert_eq!(a.geometry.path_id, b.geometry.path_id);
    assert_ne!(a.evaluation_id, b.evaluation_id);
    assert_eq!(a.distance_meters, b.distance_meters);
    assert_eq!(a.objective, b.objective);
}
