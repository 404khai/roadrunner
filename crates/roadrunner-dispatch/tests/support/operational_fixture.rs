//! Reusable three-node Phase 17 evaluator fixture for runtime boundary tests.
#![allow(clippy::float_cmp)]
use std::fmt::Debug;

use roadrunner_core::geo::{CanonicalCoordinate, KilometersPerHour, Seconds, haversine_distance};
use roadrunner_core::graph::{
    AccessClass, BuilderNodeId, BuilderSegmentId, EdgeProperties, FrozenGraph, GraphBuildIdentity,
    GraphBuilder, GraphMetadata, GraphSnapshotId, NodeId,
};
use roadrunner_dispatch::*;

pub(crate) fn ok<T, E: Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("fixture: {e:?}"))
}
pub(crate) fn secs(v: f64) -> Seconds {
    ok(Seconds::new(v))
}
pub(crate) fn at(v: f64) -> DispatchInstant {
    ok(DispatchInstant::new(v))
}
pub(crate) fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
pub(crate) fn policy() -> OrderPolicy {
    OrderPolicy {
        id: "test-protections/v1".into(),
        version: 1,
        deadline: AdmissionDeadline::SoftObserved,
        max_completion_delay: None,
        pickup_service: secs(2.0),
        dropoff_service: secs(3.0),
        readiness: ReadinessRule::ValidForecastV1,
    }
}

pub(crate) struct Fixture {
    pub(crate) graph: FrozenGraph,
    pub(crate) data: WorldData,
    pub(crate) anchors: RoutingAnchors,
}
impl Fixture {
    pub(crate) fn new() -> Self {
        let mut b = GraphBuilder::new(
            GraphSnapshotId::new(17),
            GraphMetadata::new(
                "test/v1",
                "synthetic",
                GraphBuildIdentity::new("pooling-test", "v1", "v1", "synthetic"),
            ),
        );
        let points: Vec<_> = [0, 10_000, 20_000]
            .into_iter()
            .map(|x| ok(CanonicalCoordinate::new(0, x)))
            .collect();
        for (id, c) in (0_u64..).zip(&points) {
            ok(b.add_node(BuilderNodeId::new(id), *c));
        }
        for (id, to, time) in [(0, 1_usize, 10.0), (1, 2, 20.0)] {
            let d = haversine_distance(points[0].to_coordinate(), points[to].to_coordinate());
            let prop = EdgeProperties::new(
                ok(KilometersPerHour::new(d.value() / time * 3.6)),
                None,
                AccessClass::General,
            );
            ok(b.add_segment(
                BuilderSegmentId::new(id),
                BuilderNodeId::new(0),
                BuilderNodeId::new(ok(u64::try_from(to))),
                vec![points[0], points[to]],
                Some(prop),
                Some(prop),
            ));
        }
        let graph = ok(b.finalize());
        let anchor = |n: u32| RoutingAnchor {
            coordinate: ok(graph.node(NodeId::new(n)).ok_or("node")).coordinate(),
            graph_digest: graph.metadata().snapshot_digest().into(),
            node: NodeId::new(n),
        };
        let mut data = WorldData::default();
        let mut anchors = RoutingAnchors::default();
        for id in 1..=3 {
            let o = OrderId::new(id);
            let pickup = anchor(0);
            let dropoff = anchor(if id == 1 { 2 } else { 1 });
            data.orders.insert(
                o,
                Order {
                    id: o,
                    pickup: pickup.coordinate,
                    dropoff: dropoff.coordinate,
                    created_at: at(0.0),
                    deadline: None,
                    demand: CapacityUnits::new(1),
                },
            );
            data.readiness.insert(
                o,
                OrderReadiness {
                    expected_at: Some(at(15.0)),
                    observed_at: None,
                },
            );
            data.fulfillment.insert(o, FulfillmentState::AwaitingPickup);
            anchors.pickups.insert(o, pickup);
            anchors.dropoffs.insert(o, dropoff);
        }
        for id in 1..=2 {
            let r = RiderId::new(id);
            let origin = anchor(u32::from(id != 1));
            data.profiles.insert(
                r,
                RiderProfile {
                    id: r,
                    routing_profile: "test/v1".into(),
                    max_capacity: CapacityUnits::new(3),
                },
            );
            data.riders.insert(
                r,
                RiderState {
                    coordinate: origin.coordinate,
                    availability: Availability::Available,
                },
            );
            data.plans.insert(r, RiderPlan::default());
            anchors.riders.insert(r, origin);
        }
        Self {
            graph,
            data,
            anchors,
        }
    }
    pub(crate) fn inputs(&self, world: &World, now: DispatchInstant) -> PoolingInputs {
        let provider = ok(CoreRouteProvider::new(
            &self.graph,
            TrafficContext::FreeFlow,
        ));
        PoolingInputs {
            identity: PredictionIdentity {
                prediction: "fixture/v1".into(),
                service: "fixture/v1".into(),
                optimizer: "exhaustive-insertion/v1".into(),
                routing: provider.provenance(),
            },
            policies: world.data().orders.keys().map(|o| (*o, policy())).collect(),
            forecasts: world
                .data()
                .orders
                .keys()
                .map(|o| {
                    (
                        *o,
                        ReadinessForecast {
                            id: format!("forecast-{}", o.value()),
                            generated_at: at(0.0),
                            valid_until: at(1000.0),
                            expected_at: ok(world.data().readiness[o]
                                .expected_at
                                .ok_or("expected")),
                        },
                    )
                })
                .collect(),
            projections: world
                .data()
                .profiles
                .keys()
                .map(|r| {
                    (
                        *r,
                        ok(project_execution(
                            world.data(),
                            *r,
                            now,
                            self.anchors.riders[r].clone(),
                            None,
                        )),
                    )
                })
                .collect(),
            work_budget: u64::MAX,
        }
    }
    pub(crate) fn decide(
        &self,
        world: &World,
        o: OrderId,
        inputs: PoolingInputs,
    ) -> Result<InsertionDecision, PoolingError> {
        let provider = ok(CoreRouteProvider::new(
            &self.graph,
            TrafficContext::FreeFlow,
        ));
        let snapshot = ok(DispatchSnapshot::new(
            world,
            at(0.0),
            RoutingEpoch(at(0.0)),
            &provider,
            &self.anchors,
        ));
        insert_order(&snapshot, o, inputs)
    }
}
