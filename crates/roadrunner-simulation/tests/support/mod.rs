use std::fmt::Debug;

use roadrunner_core::geo::{CanonicalCoordinate, KilometersPerHour, Seconds, haversine_distance};
use roadrunner_core::graph::{
    AccessClass, BuilderNodeId, BuilderSegmentId, EdgeProperties, FrozenGraph, GraphBuildIdentity,
    GraphBuilder, GraphMetadata, GraphSnapshotId,
};
use roadrunner_simulation::{
    ActualReadiness, DispatchPolicy, OrderInput, RiderInput, SimulationScenario,
};

pub fn ok<T, E: Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("fixture: {error:?}"))
}
pub fn seconds(value: f64) -> Seconds {
    ok(Seconds::new(value))
}
pub fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}
pub fn graph() -> FrozenGraph {
    let mut builder = GraphBuilder::new(
        GraphSnapshotId::new(15),
        GraphMetadata::new(
            "simulation-motorcycle/v1",
            "synthetic",
            GraphBuildIdentity::new("simulation-test", "v1", "v1", "synthetic"),
        ),
    );
    let points: Vec<_> = [0, 10_000, 20_000, 30_000, 40_000]
        .into_iter()
        .map(|longitude| ok(CanonicalCoordinate::new(0, longitude)))
        .collect();
    for (id, point) in (0_u64..).zip(points.iter().copied()) {
        ok(builder.add_node(BuilderNodeId::new(id), point));
    }
    for (id, to, time) in [(0, 1_usize, 10.0), (1, 2, 20.0), (2, 3, 30.0)] {
        let distance = haversine_distance(points[0].to_coordinate(), points[to].to_coordinate());
        let properties = EdgeProperties::new(
            ok(KilometersPerHour::new(distance.value() / time * 3.6)),
            None,
            AccessClass::General,
        );
        ok(builder.add_segment(
            BuilderSegmentId::new(id),
            BuilderNodeId::new(0),
            BuilderNodeId::new(ok(u64::try_from(to))),
            vec![points[0], points[to]],
            Some(properties),
            Some(properties),
        ));
    }
    ok(builder.finalize())
}
pub fn order(id: u64, created: f64, ready: f64) -> OrderInput {
    OrderInput {
        admission: None,
        id,
        pickup_node: 0,
        dropoff_node: 3,
        created_at_seconds: seconds(created),
        expected_ready_at_seconds: Some(seconds(ready)),
        actual_readiness: ActualReadiness::Fixed {
            at_seconds: seconds(ready),
        },
        deadline_seconds: None,
        demand: 1,
    }
}
pub fn scenario() -> SimulationScenario {
    SimulationScenario {
        scenario_id: None,
        schema_version: 1,
        seed: 15,
        start_seconds: Seconds::ZERO,
        end_seconds: seconds(100.0),
        routing_epoch_seconds: Seconds::ZERO,
        graph_snapshot_digest: None,
        dispatch: DispatchPolicy::PreparationAware {
            idle_penalty_weight: 1.0,
        },
        riders: vec![RiderInput {
            id: 1,
            node: 1,
            capacity: 2,
            available: true,
        }],
        orders: vec![order(1, 0.0, 25.0)],
        initial_traffic: vec![],
        traffic_changes: vec![],
    }
}
