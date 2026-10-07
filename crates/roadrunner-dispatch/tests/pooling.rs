//! Independent Phase 17 oracle, hand-worked evaluator and transition invariants.
#![allow(clippy::float_cmp)]
use std::fmt::Debug;

use roadrunner_core::geo::{CanonicalCoordinate, KilometersPerHour, Seconds, haversine_distance};
use roadrunner_core::graph::{
    AccessClass, BuilderNodeId, BuilderSegmentId, EdgeProperties, FrozenGraph, GraphBuildIdentity,
    GraphBuilder, GraphMetadata, GraphSnapshotId, NodeId,
};
use roadrunner_dispatch::*;

fn ok<T, E: Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("fixture: {e:?}"))
}
fn secs(v: f64) -> Seconds {
    ok(Seconds::new(v))
}
fn at(v: f64) -> DispatchInstant {
    ok(DispatchInstant::new(v))
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn policy() -> OrderPolicy {
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

struct Fixture {
    graph: FrozenGraph,
    data: WorldData,
    anchors: RoutingAnchors,
}
impl Fixture {
    fn new() -> Self {
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
    fn inputs(&self, world: &World, now: DispatchInstant) -> PoolingInputs {
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
    fn decide(
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
fn commit(world: &mut World, d: &InsertionDecision) {
    ok(world.commit_insertion(d, &d.evidence().context));
}
fn chosen(d: &InsertionDecision) -> &InsertionProposal {
    ok(d.proposal().ok_or("proposal"))
}

#[test]
fn hand_worked_all_intermediate_times_loads_and_colocated_independent_stops() {
    let fixture = Fixture::new();
    let world = ok(World::new(17, fixture.data.clone()));
    let mut inputs = fixture.inputs(&world, at(0.0));
    ok(inputs
        .policies
        .get_mut(&OrderId::new(2))
        .ok_or("fixture field"))
    .pickup_service = secs(4.0);
    let plan = RiderPlan {
        stops: vec![
            Stop::Pickup(OrderId::new(1)),
            Stop::Pickup(OrderId::new(2)),
            Stop::Dropoff(OrderId::new(2)),
            Stop::Dropoff(OrderId::new(1)),
        ],
    };
    // The independent evaluator accepts a provisional new assignment; existing A is committed input.
    let mut data = world.data().clone();
    data.assignments.insert(
        OrderId::new(1),
        CommittedAssignment {
            order: OrderId::new(1),
            rider: RiderId::new(1),
        },
    );
    data.plans.insert(
        RiderId::new(1),
        RiderPlan {
            stops: vec![
                Stop::Pickup(OrderId::new(1)),
                Stop::Dropoff(OrderId::new(1)),
            ],
        },
    );
    let world = ok(World::new(17, data));
    inputs.projections = fixture.inputs(&world, at(0.0)).projections;
    let provider = ok(CoreRouteProvider::new(
        &fixture.graph,
        TrafficContext::FreeFlow,
    ));
    let snapshot = ok(DispatchSnapshot::new(
        &world,
        at(0.0),
        RoutingEpoch(at(0.0)),
        &provider,
        &fixture.anchors,
    ));
    let e = ok(ok(evaluate_whole_plan(
        &snapshot,
        &inputs,
        RiderId::new(1),
        &plan,
        Some(OrderId::new(2)),
    )));
    for (s, (arrival, waiting, service, departure, load)) in e.stops.iter().zip([
        (0.0, 15.0, 2.0, 17.0, 1),
        (17.0, 0.0, 4.0, 21.0, 2),
        (31.0, 0.0, 3.0, 34.0, 1),
        (64.0, 0.0, 3.0, 67.0, 0),
    ]) {
        close(s.timeline.arrival.value(), arrival);
        close(s.timeline.waiting.value(), waiting);
        close(s.timeline.service.value(), service);
        close(s.timeline.departure.value(), departure);
        assert_eq!(s.load.value(), load);
    }
    close(e.travel.value(), 40.0);
    assert_eq!(e.completions[&OrderId::new(1)], at(67.0));
    assert_eq!(e.completions[&OrderId::new(2)], at(34.0));
}

// Oracle independently chooses pairs by selecting two NEW final-vector slots, filling
// all remaining slots from the existing suffix. It does not call production generation,
// pruning, or ranking; tuple comparison independently implements the audited objective.
fn oracle(
    fixture: &Fixture,
    world: &World,
    o: OrderId,
    inputs: &PoolingInputs,
) -> (RiderId, Vec<Stop>, f64, f64, u64) {
    let provider = ok(CoreRouteProvider::new(
        &fixture.graph,
        TrafficContext::FreeFlow,
    ));
    let snapshot = ok(DispatchSnapshot::new(
        world,
        at(0.0),
        RoutingEpoch(at(0.0)),
        &provider,
        &fixture.anchors,
    ));
    let mut candidates = Vec::new();
    let mut count = 0;
    for (r, profile) in &world.data().profiles {
        if world.data().riders[r].availability != Availability::Available
            || profile.routing_profile != provider.provenance().profile
        {
            continue;
        }
        let p = &inputs.projections[r];
        let baseline = ok(ok(evaluate_whole_plan(
            &snapshot,
            inputs,
            *r,
            &world.data().plans[r],
            None,
        )));
        if !baseline.breaches.is_empty() {
            continue;
        }
        let size = p.suffix.stops.len() + 2;
        for i in 0..size {
            for j in 0..size {
                if i >= j {
                    continue;
                }
                let mut old = p.suffix.stops.iter();
                let mut stops = Vec::new();
                for k in 0..size {
                    stops.push(if k == i {
                        Stop::Pickup(o)
                    } else if k == j {
                        Stop::Dropoff(o)
                    } else {
                        *ok(old.next().ok_or("old stop"))
                    });
                }
                if let Some(frozen) = &p.frozen {
                    stops.insert(0, frozen.timeline.stop);
                }
                count += 1;
                if let Ok(e) = ok(evaluate_whole_plan(
                    &snapshot,
                    inputs,
                    *r,
                    &RiderPlan {
                        stops: stops.clone(),
                    },
                    Some(o),
                )) {
                    if e.breaches.is_empty() {
                        candidates.push((
                            *r,
                            stops,
                            e.travel.value() - baseline.travel.value(),
                            e.distance.value() - baseline.distance.value(),
                        ));
                    }
                }
            }
        }
    }
    candidates.sort_by(|a, b| {
        a.2.total_cmp(&b.2)
            .then_with(|| a.3.total_cmp(&b.3))
            .then_with(|| a.0.cmp(&b.0))
            .then_with(|| a.1.cmp(&b.1))
    });
    let (r, stops, t, d) = ok(candidates.into_iter().next().ok_or("oracle feasible"));
    (r, stops, t, d, count)
}

#[test]
fn seeded_tiny_oracle_repeated_insertion_and_unordered_construction() {
    for seed in 0..32_u64 {
        let mut fixture = Fixture::new();
        let demand = 1 + seed % 2;
        ok(fixture
            .data
            .orders
            .get_mut(&OrderId::new(2))
            .ok_or("fixture field"))
        .demand = CapacityUnits::new(demand);
        let mut world = ok(World::new(seed, fixture.data.clone()));
        for order in [OrderId::new(1), OrderId::new(2), OrderId::new(3)] {
            let mut inputs = fixture.inputs(&world, at(0.0));
            for (o, p) in &mut inputs.policies {
                p.pickup_service = secs(f64::from(ok(u32::try_from((seed + o.value()) % 5))));
            }
            let expected = oracle(&fixture, &world, order, &inputs);
            let d = ok(fixture.decide(&world, order, inputs.clone()));
            let p = chosen(&d);
            assert_eq!(p.rider(), expected.0);
            assert_eq!(p.evaluation().plan.stops, expected.1);
            close(p.incremental_travel(), expected.2);
            close(p.incremental_distance(), expected.3);
            assert_eq!(d.evidence().work, expected.4);
            assert!(
                d.evidence()
                    .riders
                    .iter()
                    .all(|r| r.evaluated + r.pruned == r.pairs_total)
            );
            let before = world.data().accepted.clone();
            commit(&mut world, &d);
            for (o, a) in before {
                assert_eq!(world.data().accepted[&o], a);
            }
            ok(validate_world(world.data()));
            let after = world.data().clone();
            let mut reordered = after.clone();
            reordered.orders = after.orders.into_iter().rev().collect();
            reordered.profiles = after.profiles.into_iter().rev().collect();
            let other = ok(World::new(seed, reordered));
            assert_eq!(other.data(), world.data());
            // Equivalent construction at the same identity/version is checked below on fresh worlds.
        }
        let reversed = fixture
            .data
            .orders
            .iter()
            .rev()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        let mut data = fixture.data.clone();
        data.orders = reversed;
        let a = ok(World::new(seed, fixture.data.clone()));
        let b = ok(World::new(seed, data));
        assert_eq!(
            ok(fixture.decide(&a, OrderId::new(1), fixture.inputs(&a, at(0.0)))),
            ok(fixture.decide(&b, OrderId::new(1), fixture.inputs(&b, at(0.0))))
        );
    }
}

#[test]
fn incomplete_and_complete_infeasible_never_leak_responsibility_stops_or_terms() {
    let mut fixture = Fixture::new();
    let mut world = ok(World::new(17, fixture.data.clone()));
    let before = world.data().clone();
    let mut inputs = fixture.inputs(&world, at(0.0));
    inputs.work_budget = 1;
    let d = ok(fixture.decide(&world, OrderId::new(1), inputs));
    assert_eq!(
        d.evidence().termination,
        InsertionTermination::SearchIncomplete
    );
    assert!(d.proposal().is_none());
    assert_eq!(
        world.commit_insertion(&d, &d.evidence().context),
        Err(CommitError::InvalidTransition)
    );
    assert_eq!(world.data(), &before);
    ok(fixture
        .data
        .orders
        .get_mut(&OrderId::new(1))
        .ok_or("fixture field"))
    .demand = CapacityUnits::new(4);
    let world = ok(World::new(17, fixture.data.clone()));
    let d = ok(fixture.decide(&world, OrderId::new(1), fixture.inputs(&world, at(0.0))));
    assert_eq!(
        d.evidence().termination,
        InsertionTermination::NoFeasibleInsertion
    );
    assert!(d.evidence().search_complete);
    assert_eq!(d.evidence().work, 2);
    assert!(
        d.evidence()
            .riders
            .iter()
            .all(|r| r.rejections == [(InsertionRejection::Capacity, 1)])
    );
}

#[test]
fn required_forecasts_and_versions_fail_the_whole_fleet_not_candidate_rejection() {
    let fixture = Fixture::new();
    let world = ok(World::new(17, fixture.data.clone()));
    let mut inputs = fixture.inputs(&world, at(0.0));
    inputs.forecasts.remove(&OrderId::new(1));
    assert_eq!(
        fixture.decide(&world, OrderId::new(1), inputs),
        Err(PoolingError::PredictionUnavailable(OrderId::new(1)))
    );
    let mut inputs = fixture.inputs(&world, at(0.0));
    ok(inputs
        .policies
        .get_mut(&OrderId::new(1))
        .ok_or("fixture field"))
    .version = 99;
    assert_eq!(
        fixture.decide(&world, OrderId::new(1), inputs),
        Err(PoolingError::UnsupportedPolicy)
    );
    let mut inputs = fixture.inputs(&world, at(0.0));
    ok(inputs
        .policies
        .get_mut(&OrderId::new(1))
        .ok_or("fixture field"))
    .deadline = AdmissionDeadline::Hard;
    assert_eq!(
        fixture.decide(&world, OrderId::new(1), inputs),
        Err(PoolingError::UnsupportedPolicy)
    );
    let mut inputs = fixture.inputs(&world, at(0.0));
    ok(inputs
        .policies
        .get_mut(&OrderId::new(1))
        .ok_or("fixture field"))
    .readiness = ReadinessRule::CreatedAtFallbackV1;
    inputs.forecasts.clear();
    assert!(
        ok(fixture.decide(&world, OrderId::new(1), inputs))
            .proposal()
            .is_some()
    );
}

#[test]
fn exact_stale_contexts_reject_without_any_semantic_mutation() {
    let fixture = Fixture::new();
    let mut world = ok(World::new(17, fixture.data.clone()));
    let d = ok(fixture.decide(&world, OrderId::new(1), fixture.inputs(&world, at(0.0))));
    let before = world.data().clone();
    for kind in 0..9 {
        let mut c = d.evidence().context.clone();
        match kind {
            0 => c.at = at(1.0),
            1 => c.epoch = RoutingEpoch(at(1.0)),
            2 => c.inputs.identity.routing.graph_digest = "changed".into(),
            3 => c.inputs.identity.routing.traffic = TrafficIdentity::Static("changed".into()),
            4 => c.inputs.identity.service = "changed".into(),
            5 => c.inputs.identity.prediction = "changed".into(),
            6 => {
                ok(c.inputs
                    .forecasts
                    .get_mut(&OrderId::new(1))
                    .ok_or("fixture field"))
                .valid_until = at(99.0);
            }
            7 => {
                ok(c.inputs
                    .policies
                    .get_mut(&OrderId::new(1))
                    .ok_or("fixture field"))
                .dropoff_service = secs(99.0);
            }
            _ => {
                ok(c.inputs
                    .projections
                    .get_mut(&RiderId::new(1))
                    .ok_or("fixture field"))
                .at = at(1.0);
            }
        }
        assert_eq!(world.commit_insertion(&d, &c), Err(CommitError::Stale));
        assert_eq!(world.data(), &before);
        assert_eq!(world.version(), WorldVersion::new(0));
    }
    let another = ok(fixture.decide(&world, OrderId::new(2), fixture.inputs(&world, at(0.0))));
    commit(&mut world, &another);
    let replaced = world.data().clone();
    assert_eq!(
        world.commit_insertion(&d, &d.evidence().context),
        Err(CommitError::Stale)
    );
    assert_eq!(world.data(), &replaced);
    let mut other_world = ok(World::new(99, fixture.data.clone()));
    assert_eq!(
        other_world.commit_insertion(&d, &d.evidence().context),
        Err(CommitError::Stale)
    );
    ok(world.estimate_readiness(OrderId::new(2), at(16.0)));
    let before = world.data().clone();
    assert_eq!(
        world.commit_insertion(&d, &d.evidence().context),
        Err(CommitError::Stale)
    );
    assert_eq!(world.data(), &before);
}

#[test]
fn immutable_cumulative_reference_deadline_and_baseline_breach() {
    let mut fixture = Fixture::new();
    ok(fixture
        .data
        .riders
        .get_mut(&RiderId::new(2))
        .ok_or("fixture field"))
    .availability = Availability::Unavailable;
    let mut world = ok(World::new(17, fixture.data.clone()));
    let mut inputs = fixture.inputs(&world, at(0.0));
    ok(inputs
        .policies
        .get_mut(&OrderId::new(1))
        .ok_or("fixture field"))
    .max_completion_delay = Some(secs(0.0));
    let d = ok(fixture.decide(&world, OrderId::new(1), inputs));
    commit(&mut world, &d);
    let accepted = world.data().accepted[&OrderId::new(1)].clone();
    let mut inputs = fixture.inputs(&world, at(0.0));
    ok(inputs
        .policies
        .get_mut(&OrderId::new(1))
        .ok_or("fixture field"))
    .max_completion_delay = None; // cannot weaken accepted zero allowance
    let d = ok(fixture.decide(&world, OrderId::new(2), inputs));
    assert!(
        d.evidence().riders[0]
            .rejections
            .iter()
            .any(|(r, _)| *r == InsertionRejection::CumulativeDelay(OrderId::new(1)))
    );
    commit(&mut world, &d);
    assert_eq!(world.data().accepted[&OrderId::new(1)], accepted);
    let d = ok(fixture.decide(&world, OrderId::new(3), fixture.inputs(&world, at(0.0))));
    if let Some(p) = d.proposal() {
        assert_eq!(
            p.impacts()
                .iter()
                .find(|i| i.order == OrderId::new(1))
                .and_then(|i| i.accepted),
            Some(accepted.completion_reference)
        );
        commit(&mut world, &d);
    }
    assert_eq!(world.data().accepted[&OrderId::new(1)], accepted);
    // Strengthened hard deadline is already breached by baseline: rider isolated unchanged.
    let mut data = world.data().clone();
    ok(data.orders.get_mut(&OrderId::new(1)).ok_or("fixture field")).deadline = Some(at(1.0));
    data.orders
        .insert(OrderId::new(4), data.orders[&OrderId::new(3)].clone());
    ok(data.orders.get_mut(&OrderId::new(4)).ok_or("fixture field")).id = OrderId::new(4);
    data.readiness
        .insert(OrderId::new(4), data.readiness[&OrderId::new(3)]);
    data.fulfillment
        .insert(OrderId::new(4), FulfillmentState::AwaitingPickup);
    fixture.anchors.pickups.insert(
        OrderId::new(4),
        fixture.anchors.pickups[&OrderId::new(3)].clone(),
    );
    fixture.anchors.dropoffs.insert(
        OrderId::new(4),
        fixture.anchors.dropoffs[&OrderId::new(3)].clone(),
    );
    let world = ok(World::new(18, data));
    let mut inputs = fixture.inputs(&world, at(0.0));
    ok(inputs
        .policies
        .get_mut(&OrderId::new(1))
        .ok_or("fixture field"))
    .deadline = AdmissionDeadline::Hard;
    let d = ok(fixture.decide(&world, OrderId::new(4), inputs));
    assert_eq!(
        d.evidence().termination,
        InsertionTermination::NoFeasibleInsertion
    );
    assert_eq!(d.evidence().riders[0].evaluated, 0);
    assert_eq!(
        d.evidence().riders[0].pruned,
        d.evidence().riders[0].pairs_total
    );
    assert!(
        d.evidence().riders[0]
            .baseline
            .breaches
            .contains(&InsertionRejection::HardDeadline(OrderId::new(1)))
    );
}

#[test]
fn custody_exact_fulfillment_capacity_prefix_and_invalid_migration() {
    let fixture = Fixture::new();
    let r = RiderId::new(1);
    let a = OrderId::new(1);
    let b = OrderId::new(2);
    let mut data = fixture.data.clone();
    data.assignments
        .insert(a, CommittedAssignment { order: a, rider: r });
    data.fulfillment.insert(
        a,
        FulfillmentState::PickedUp {
            rider: r,
            at: at(0.0),
        },
    );
    ok(data.readiness.get_mut(&a).ok_or("fixture field")).observed_at = Some(at(0.0));
    data.plans.insert(
        r,
        RiderPlan {
            stops: vec![Stop::Dropoff(a)],
        },
    );
    ok(data.profiles.get_mut(&r).ok_or("fixture field")).max_capacity = CapacityUnits::new(1);
    ok(validate_world(&data));
    let world = ok(World::new(17, data.clone()));
    let mut inputs = fixture.inputs(&world, at(0.0));
    ok(inputs.policies.get_mut(&a).ok_or("fixture field")).max_completion_delay = Some(secs(100.0));
    assert_eq!(
        fixture.decide(&world, b, inputs),
        Err(PoolingError::UnsupportedMigration)
    );
    data.assignments
        .insert(b, CommittedAssignment { order: b, rider: r });
    assert_eq!(
        validate_plan(
            &data,
            r,
            &RiderPlan {
                stops: vec![Stop::Pickup(b), Stop::Dropoff(b), Stop::Dropoff(a)]
            }
        ),
        Err(PlanValidityError::CapacityExceeded)
    );
    ok(validate_plan(
        &data,
        r,
        &RiderPlan {
            stops: vec![Stop::Dropoff(a), Stop::Pickup(b), Stop::Dropoff(b)],
        },
    ));
    assert_eq!(
        validate_plan(
            &data,
            r,
            &RiderPlan {
                stops: vec![
                    Stop::Pickup(a),
                    Stop::Dropoff(a),
                    Stop::Pickup(b),
                    Stop::Dropoff(b)
                ]
            }
        ),
        Err(PlanValidityError::InvalidPlan)
    );
    ok(data.assignments.get_mut(&a).ok_or("fixture field")).rider = RiderId::new(2);
    assert!(validate_world(&data).is_err());
}

#[test]
fn explicit_frozen_projection_inserts_only_after_active_pickup_and_validates_effects() {
    let mut fixture = Fixture::new();
    ok(fixture.data.riders.get_mut(&RiderId::new(2)).ok_or("rider")).availability =
        Availability::Unavailable;
    let mut world = ok(World::new(17, fixture.data.clone()));
    let d = ok(fixture.decide(&world, OrderId::new(1), fixture.inputs(&world, at(0.0))));
    let rider = chosen(&d).rider();
    commit(&mut world, &d);
    let mut inputs = fixture.inputs(&world, at(0.0));
    let frozen = FrozenPrefix {
        execution_id: 77,
        timeline: StopTimeline::new(
            Stop::Pickup(OrderId::new(1)),
            at(0.0),
            secs(15.0),
            secs(2.0),
        )
        .unwrap_or_else(|e| panic!("{e}")),
        anchor: fixture.anchors.pickups[&OrderId::new(1)].clone(),
    };
    let projection = ok(project_execution(
        world.data(),
        rider,
        at(0.0),
        fixture.anchors.riders[&rider].clone(),
        Some(frozen.clone()),
    ));
    assert_eq!(projection.at, at(17.0));
    assert_eq!(projection.load, CapacityUnits::new(1));
    assert_eq!(projection.custody[&OrderId::new(1)], at(17.0));
    inputs.projections.insert(rider, projection);
    let d = ok(fixture.decide(&world, OrderId::new(2), inputs));
    let p = chosen(&d);
    assert_eq!(p.rider(), rider);
    assert_eq!(
        p.evaluation().plan.stops.first(),
        Some(&frozen.timeline.stop)
    );
    assert_eq!(
        d.evidence()
            .riders
            .iter()
            .find(|r| r.rider == rider)
            .map(|r| r.pairs_total),
        Some(3)
    );
    let before = world.data().clone();
    let mut stale = d.evidence().context.clone();
    ok(
        ok(stale.inputs.projections.get_mut(&rider).ok_or("projection"))
            .frozen
            .as_mut()
            .ok_or("frozen"),
    )
    .execution_id += 1;
    assert_eq!(world.commit_insertion(&d, &stale), Err(CommitError::Stale));
    assert_eq!(world.data(), &before);
    commit(&mut world, &d);
    assert_eq!(
        world.data().plans[&rider].stops.first(),
        Some(&frozen.timeline.stop)
    );
    let accepted = world.data().accepted.clone();
    ok(world.observe_ready(OrderId::new(1), at(15.0)));
    ok(world.pickup(rider, OrderId::new(1), at(17.0)));
    assert_eq!(world.data().accepted, accepted);
    assert!(
        matches!(world.data().fulfillment[&OrderId::new(1)],FulfillmentState::PickedUp{rider:r,..} if r==rider)
    );
}

#[test]
fn time_dependent_whole_plan_propagation_allows_signed_negative_road_delta() {
    use roadrunner_core::cost::{
        TimeDependentTrafficSnapshot, TrafficMultiplier, TrafficPoint, TrafficProfile,
    };
    let mut fixture = Fixture::new();
    ok(fixture.data.riders.get_mut(&RiderId::new(2)).ok_or("rider")).availability =
        Availability::Unavailable;
    let dropoff = fixture.anchors.dropoffs[&OrderId::new(1)].clone();
    ok(fixture.data.orders.get_mut(&OrderId::new(2)).ok_or("order")).dropoff = dropoff.coordinate;
    fixture.anchors.dropoffs.insert(OrderId::new(2), dropoff);
    let profiles = fixture
        .graph
        .edges()
        .iter()
        .map(|edge| TrafficProfile {
            edge_id: edge.id(),
            points: vec![
                TrafficPoint {
                    departure_seconds: secs(0.0),
                    multiplier: ok(TrafficMultiplier::new(2.0)),
                },
                TrafficPoint {
                    departure_seconds: secs(100.0),
                    multiplier: ok(TrafficMultiplier::new(1.0)),
                },
            ],
        })
        .collect::<Vec<_>>();
    let traffic = ok(TimeDependentTrafficSnapshot::new(&fixture.graph, profiles));
    let provider = ok(CoreRouteProvider::new(
        &fixture.graph,
        TrafficContext::TimeDependent(&traffic),
    ));
    let mut world = ok(World::new(17, fixture.data.clone()));
    let decide = |world: &World, order: OrderId| {
        let snapshot = ok(DispatchSnapshot::new(
            world,
            at(0.0),
            RoutingEpoch(at(0.0)),
            &provider,
            &fixture.anchors,
        ));
        let mut inputs = fixture.inputs(world, at(0.0));
        inputs.identity.routing = provider.provenance();
        ok(insert_order(&snapshot, order, inputs))
    };
    let first = decide(&world, OrderId::new(1));
    commit(&mut world, &first);
    close(
        chosen(&first).evaluation().stops[1]
            .timeline
            .arrival
            .value(),
        17.0 + 20.0 * (2.0 - 17.0 / 100.0),
    );
    let second = decide(&world, OrderId::new(2));
    let proposal = chosen(&second);
    assert!(proposal.incremental_travel() < 0.0);
    close(proposal.incremental_travel(), -0.4);
    close(
        proposal.evaluation().stops[2].timeline.arrival.value(),
        19.0 + 20.0 * (2.0 - 19.0 / 100.0),
    );
    assert_eq!(
        proposal.evaluation().plan.stops,
        vec![
            Stop::Pickup(OrderId::new(1)),
            Stop::Pickup(OrderId::new(2)),
            Stop::Dropoff(OrderId::new(1)),
            Stop::Dropoff(OrderId::new(2))
        ]
    );
}

#[test]
fn unevaluable_existing_eligible_rider_prevents_publication_and_dropoff_projection_releases_only_its_load()
 {
    let mut fixture = Fixture::new();
    ok(fixture.data.riders.get_mut(&RiderId::new(2)).ok_or("rider")).availability =
        Availability::Unavailable;
    let mut world = ok(World::new(17, fixture.data.clone()));
    let first = ok(fixture.decide(&world, OrderId::new(1), fixture.inputs(&world, at(0.0))));
    commit(&mut world, &first);
    let mut inputs = fixture.inputs(&world, at(0.0));
    inputs.forecasts.remove(&OrderId::new(1));
    assert_eq!(
        fixture.decide(&world, OrderId::new(2), inputs),
        Err(PoolingError::PredictionUnavailable(OrderId::new(1)))
    );
    ok(world.observe_ready(OrderId::new(1), at(0.0)));
    ok(world.pickup(RiderId::new(1), OrderId::new(1), at(0.0)));
    let mut inputs = fixture.inputs(&world, at(0.0));
    let frozen = FrozenPrefix {
        execution_id: 88,
        timeline: ok(StopTimeline::new(
            Stop::Dropoff(OrderId::new(1)),
            at(20.0),
            Seconds::ZERO,
            secs(3.0),
        )),
        anchor: fixture.anchors.dropoffs[&OrderId::new(1)].clone(),
    };
    let projection = ok(project_execution(
        world.data(),
        RiderId::new(1),
        at(0.0),
        fixture.anchors.riders[&RiderId::new(1)].clone(),
        Some(frozen),
    ));
    assert_eq!(projection.load.value(), 0);
    assert!(projection.custody.is_empty());
    assert_eq!(projection.suffix.stops, [] as [Stop; 0]);
    inputs.projections.insert(RiderId::new(1), projection);
    let second = ok(fixture.decide(&world, OrderId::new(2), inputs));
    commit(&mut world, &second);
    ok(world.deliver(RiderId::new(1), OrderId::new(1), at(23.0)));
    assert!(world.data().assignments.contains_key(&OrderId::new(2)));
    assert!(!world.data().assignments.contains_key(&OrderId::new(1)));
    assert_eq!(
        world.data().plans[&RiderId::new(1)].stops,
        vec![
            Stop::Pickup(OrderId::new(2)),
            Stop::Dropoff(OrderId::new(2))
        ]
    );
}

#[test]
fn hand_worked_marginal_cumulative_consumption_and_repeated_insertion_bound() {
    let mut fixture = Fixture::new();
    ok(fixture.data.riders.get_mut(&RiderId::new(2)).ok_or("rider")).availability =
        Availability::Unavailable;
    ok(fixture.data.orders.get_mut(&OrderId::new(1)).ok_or("order")).deadline = Some(at(70.0));
    let mut world = ok(World::new(17, fixture.data.clone()));
    let mut inputs = fixture.inputs(&world, at(0.0));
    let first_policy = ok(inputs.policies.get_mut(&OrderId::new(1)).ok_or("policy"));
    first_policy.deadline = AdmissionDeadline::Hard;
    first_policy.max_completion_delay = Some(secs(26.0));
    let first = ok(fixture.decide(&world, OrderId::new(1), inputs));
    close(
        chosen(&first).evaluation().completions[&OrderId::new(1)].value(),
        40.0,
    );
    commit(&mut world, &first);
    let second = ok(fixture.decide(&world, OrderId::new(2), fixture.inputs(&world, at(0.0))));
    let effect = ok(chosen(&second)
        .impacts()
        .iter()
        .find(|i| i.order == OrderId::new(1))
        .ok_or("impact"));
    close(effect.candidate.value(), 65.0);
    assert_eq!(effect.marginal_seconds, Some(25.0));
    assert_eq!(effect.cumulative_seconds, Some(25.0));
    assert_eq!(effect.consumed_seconds, Some(25.0));
    assert_eq!(effect.deadline_slack_seconds, Some(5.0));
    assert_eq!(effect.accepted_deadline_slack_seconds, Some(5.0));
    assert_eq!(effect.cumulative_allowance_remaining_seconds, Some(1.0));
    commit(&mut world, &second);
    let inputs = fixture.inputs(&world, at(0.0));
    let provider = ok(CoreRouteProvider::new(
        &fixture.graph,
        TrafficContext::FreeFlow,
    ));
    let snapshot = ok(DispatchSnapshot::new(
        &world,
        at(0.0),
        RoutingEpoch(at(0.0)),
        &provider,
        &fixture.anchors,
    ));
    let plan = RiderPlan {
        stops: vec![
            Stop::Pickup(OrderId::new(1)),
            Stop::Pickup(OrderId::new(2)),
            Stop::Pickup(OrderId::new(3)),
            Stop::Dropoff(OrderId::new(2)),
            Stop::Dropoff(OrderId::new(1)),
            Stop::Dropoff(OrderId::new(3)),
        ],
    };
    let evaluation = ok(ok(evaluate_whole_plan(
        &snapshot,
        &inputs,
        RiderId::new(1),
        &plan,
        Some(OrderId::new(3)),
    )));
    close(evaluation.completions[&OrderId::new(1)].value(), 67.0);
    assert!(
        evaluation
            .breaches
            .contains(&InsertionRejection::CumulativeDelay(OrderId::new(1)))
    );
    // Two marginal seconds consume 27 cumulative seconds against original 40, never a reset 65.
    let third = ok(fixture.decide(&world, OrderId::new(3), inputs));
    assert!(
        third.evidence().riders[0]
            .rejections
            .iter()
            .any(|(r, _)| *r == InsertionRejection::CumulativeDelay(OrderId::new(1)))
    );
    commit(&mut world, &third);
    assert_eq!(
        world.data().accepted[&OrderId::new(1)].completion_reference,
        at(40.0)
    );
}

#[test]
fn improvements_consume_zero_and_do_not_bank_future_allowance() {
    let mut fixture = Fixture::new();
    ok(fixture.data.riders.get_mut(&RiderId::new(2)).ok_or("rider")).availability =
        Availability::Unavailable;
    let mut world = ok(World::new(17, fixture.data.clone()));
    let mut inputs = fixture.inputs(&world, at(0.0));
    ok(inputs.policies.get_mut(&OrderId::new(1)).ok_or("policy")).max_completion_delay =
        Some(Seconds::ZERO);
    let first = ok(fixture.decide(&world, OrderId::new(1), inputs));
    commit(&mut world, &first);
    ok(world.estimate_readiness(OrderId::new(1), at(0.0)));
    ok(world.estimate_readiness(OrderId::new(2), at(0.0)));
    let mut inputs = fixture.inputs(&world, at(0.0));
    ok(inputs.policies.get_mut(&OrderId::new(2)).ok_or("policy")).pickup_service = secs(16.0);
    let provider = ok(CoreRouteProvider::new(
        &fixture.graph,
        TrafficContext::FreeFlow,
    ));
    let snapshot = ok(DispatchSnapshot::new(
        &world,
        at(0.0),
        RoutingEpoch(at(0.0)),
        &provider,
        &fixture.anchors,
    ));
    let baseline = ok(ok(evaluate_whole_plan(
        &snapshot,
        &inputs,
        RiderId::new(1),
        &world.data().plans[&RiderId::new(1)],
        None,
    )));
    close(baseline.completions[&OrderId::new(1)].value(), 25.0);
    assert_eq!(baseline.breaches, [] as [InsertionRejection; 0]);
    let plan = RiderPlan {
        stops: vec![
            Stop::Pickup(OrderId::new(1)),
            Stop::Pickup(OrderId::new(2)),
            Stop::Dropoff(OrderId::new(1)),
            Stop::Dropoff(OrderId::new(2)),
        ],
    };
    let candidate = ok(ok(evaluate_whole_plan(
        &snapshot,
        &inputs,
        RiderId::new(1),
        &plan,
        Some(OrderId::new(2)),
    )));
    close(candidate.completions[&OrderId::new(1)].value(), 41.0);
    assert!(
        candidate
            .breaches
            .contains(&InsertionRejection::CumulativeDelay(OrderId::new(1)))
    );
    assert_eq!(
        world.data().accepted[&OrderId::new(1)].completion_reference,
        at(40.0)
    );
}
