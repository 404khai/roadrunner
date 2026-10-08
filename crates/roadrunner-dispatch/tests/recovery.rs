//! Independent tiny recovery oracle and commitment/execution invariants.
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
fn churn() -> RecoveryPolicy {
    RecoveryPolicy {
        version: 1,
        reroute_penalty: Seconds::ZERO,
        assignment_stability_penalty: Seconds::ZERO,
        minimum_improvement: Seconds::ZERO,
        cooldown: Seconds::ZERO,
    }
}
fn committed() -> (Fixture, World) {
    let mut f = Fixture::new();
    f.data
        .riders
        .get_mut(&RiderId::new(1))
        .unwrap_or_else(|| panic!("rider"))
        .availability = Availability::Unavailable;
    let mut world = ok(World::new(19, f.data.clone()));
    let decision = ok(f.decide(&world, OrderId::new(1), f.inputs(&world, at(0.0))));
    let context = decision.evidence().context.clone();
    ok(world.commit_insertion(&decision, &context));
    let state = RiderState {
        availability: Availability::Available,
        ..world.data().riders[&RiderId::new(1)]
    };
    ok(world.update_rider_state(RiderId::new(1), state));
    (f, world)
}
fn context(f: &Fixture, world: &World, policy: RecoveryPolicy) -> RecoveryContext {
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let snapshot = ok(DispatchSnapshot::new(
        world,
        at(0.0),
        RoutingEpoch(at(0.0)),
        &provider,
        &f.anchors,
    ));
    let mut inputs = f.inputs(world, at(0.0));
    inputs.identity.optimizer = "dynamic-recovery/v1".into();
    ok(RecoveryContext::new(
        &snapshot,
        inputs,
        policy,
        "test-input/v1".into(),
        None,
    ))
}
fn recovery(
    f: &Fixture,
    world: &World,
    ctx: RecoveryContext,
) -> Result<RecoveryDecision, PoolingError> {
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let snapshot = ok(DispatchSnapshot::new(
        world,
        at(0.0),
        RoutingEpoch(at(0.0)),
        &provider,
        &f.anchors,
    ));
    recover_fleet(&snapshot, ctx)
}
#[test]
fn atomic_reassignment_retains_authentic_terms_and_releases_only_original_owner() {
    let (f, mut world) = committed();
    let accepted = world.data().accepted.clone();
    let ctx = context(&f, &world, churn());
    let d = ok(recovery(&f, &world, ctx.clone()));
    let p = d.proposal().unwrap_or_else(|| panic!("improvement"));
    assert_eq!(p.assignments()[&OrderId::new(1)].rider, RiderId::new(1));
    assert_ne!(p.impacts.len(), 0);
    assert_eq!(
        p.impacts[0].accepted,
        Some(accepted[&OrderId::new(1)].completion_reference)
    );
    let expected = p.plans().clone();
    let version = world.version().value();
    ok(world.commit_recovery(&d, &ctx));
    assert_eq!(world.version().value(), version + 1);
    assert_eq!(world.data().plans, expected);
    assert_eq!(world.data().accepted, accepted);
    ok(validate_world(world.data()));
    let before = world.data().clone();
    assert_eq!(world.commit_recovery(&d, &ctx), Err(CommitError::Stale));
    assert_eq!(*world.data(), before);
}
#[test]
fn complete_context_mismatches_are_stale_with_zero_mutation() {
    let (f, mut world) = committed();
    let ctx = context(&f, &world, churn());
    let d = ok(recovery(&f, &world, ctx.clone()));
    for field in 0..12 {
        let mut current = ctx.clone();
        match field {
            0 => current.pooling.at = at(1.0),
            1 => current.pooling.inputs.identity.prediction.push('x'),
            2 => current.pooling.inputs.identity.service.push('x'),
            3 => current.policy.reroute_penalty = secs(1.0),
            4 => current.trigger.push('x'),
            5 => current.last_applied = Some(at(0.0)),
            6 => {
                current
                    .pooling
                    .inputs
                    .forecasts
                    .get_mut(&OrderId::new(1))
                    .unwrap_or_else(|| panic!("forecast"))
                    .valid_until = at(99.0);
            }
            7 => {
                current
                    .pooling
                    .inputs
                    .projections
                    .get_mut(&RiderId::new(1))
                    .unwrap_or_else(|| panic!("projection"))
                    .at = at(1.0);
            }
            8 => {
                current.pooling.inputs.identity.routing.traffic =
                    TrafficIdentity::Static("changed".into());
            }
            9 => current
                .pooling
                .inputs
                .identity
                .routing
                .graph_digest
                .push('x'),
            10 => current.pooling.inputs.identity.routing.profile.push('x'),
            _ => current.pooling.inputs.identity.optimizer.push('x'),
        }
        let before = world.data().clone();
        let version = world.version();
        assert_eq!(world.commit_recovery(&d, &current), Err(CommitError::Stale));
        assert_eq!(*world.data(), before);
        assert_eq!(world.version(), version);
    }
}
#[test]
fn budget_prediction_and_churn_failures_publish_nothing() {
    let (f, world) = committed();
    let before = world.data().clone();
    let mut ctx = context(&f, &world, churn());
    ctx.pooling.inputs.work_budget = 0;
    let d = ok(recovery(&f, &world, ctx));
    assert_eq!(d.termination, RecoveryTermination::SearchIncomplete);
    assert!(d.proposal().is_none());
    let mut ctx = context(&f, &world, churn());
    ctx.pooling.inputs.forecasts.remove(&OrderId::new(1));
    assert_eq!(
        recovery(&f, &world, ctx),
        Err(PoolingError::PredictionUnavailable(OrderId::new(1)))
    );
    let mut policy = churn();
    policy.assignment_stability_penalty = secs(100.0);
    assert!(
        ok(recovery(&f, &world, context(&f, &world, policy)))
            .proposal()
            .is_none()
    );
    let mut ctx = context(&f, &world, churn());
    ctx.policy.cooldown = secs(100.0);
    ctx.last_applied = Some(at(0.0));
    assert_eq!(
        ok(recovery(&f, &world, ctx)).termination,
        RecoveryTermination::Cooldown
    );
    assert_eq!(*world.data(), before);
}
#[test]
fn frozen_pickup_and_custody_are_never_transferred_or_cancelled() {
    let (mut f, mut world) = committed();
    let mut ctx = context(&f, &world, churn());
    let rider = RiderId::new(2);
    let order = OrderId::new(1);
    let frozen = FrozenPrefix {
        execution_id: 9,
        timeline: ok(StopTimeline::new(
            Stop::Pickup(order),
            at(10.0),
            secs(5.0),
            secs(2.0),
        )),
        anchor: f.anchors.pickups[&order].clone(),
    };
    ctx.pooling.inputs.projections.insert(
        rider,
        ok(project_execution(
            world.data(),
            rider,
            at(0.0),
            f.anchors.riders[&rider].clone(),
            Some(frozen),
        )),
    );
    assert!(ok(recovery(&f, &world, ctx)).proposal().is_none());
    assert_eq!(
        ok(world.cancel_order(order, at(0.0), &[order].into_iter().collect())),
        Err(CancellationRefusal::FrozenExecution)
    );
    ok(world.observe_ready(order, at(0.0)));
    ok(world.pickup(rider, order, at(0.0)));
    f.anchors
        .riders
        .insert(rider, f.anchors.pickups[&order].clone());
    let d = ok(recovery(&f, &world, context(&f, &world, churn())));
    assert!(d.proposal().is_none());
    assert_eq!(
        ok(world.cancel_order(order, at(0.0), &std::collections::BTreeSet::default())),
        Err(CancellationRefusal::NotAwaitingPickup)
    );
    ok(validate_world(world.data()));
}
#[test]
fn unavailable_owner_repair_bypasses_cooldown_but_never_accepted_protections() {
    let (f, mut world) = committed();
    let state = RiderState {
        availability: Availability::Unavailable,
        ..world.data().riders[&RiderId::new(2)]
    };
    ok(world.update_rider_state(RiderId::new(2), state));
    let mut ctx = context(&f, &world, churn());
    ctx.policy.cooldown = secs(100.0);
    ctx.last_applied = Some(at(0.0));
    let d = ok(recovery(&f, &world, ctx));
    assert!(d.baseline_requires_repair);
    assert!(d.proposal().is_some());
    let terms = world.data().accepted.clone();
    let ctx = d.context.clone();
    ok(world.commit_recovery(&d, &ctx));
    assert_eq!(world.data().accepted, terms);
}
#[test]
fn cancellation_keeps_historical_acceptance_and_removes_exact_work() {
    let (_, mut world) = committed();
    let order = OrderId::new(1);
    let accepted = world.data().accepted.clone();
    assert_eq!(
        ok(world.cancel_order(order, at(0.0), &std::collections::BTreeSet::default())),
        Ok(())
    );
    assert_eq!(world.data().accepted, accepted);
    assert!(world.data().assignments.is_empty());
    assert!(world.data().plans.values().all(|p| p.stops.is_empty()));
    ok(validate_world(world.data()));
}
#[test]
fn independently_ranked_single_order_oracle_and_seeded_replay() {
    for seed in 0..32 {
        let (f, world) = committed();
        let mut policy = churn();
        policy.assignment_stability_penalty = secs(f64::from(seed % 13));
        policy.reroute_penalty = secs(f64::from(seed % 3));
        let ctx = context(&f, &world, policy);
        let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
        let snapshot = ok(DispatchSnapshot::new(
            &world,
            at(0.0),
            RoutingEpoch(at(0.0)),
            &provider,
            &f.anchors,
        ));
        // Independent enumeration: one order has precisely one valid sequence per owner.
        let mut values = Vec::new();
        for owner in [RiderId::new(1), RiderId::new(2)] {
            let mut assignments = world.data().assignments.clone();
            assignments.insert(
                OrderId::new(1),
                CommittedAssignment {
                    order: OrderId::new(1),
                    rider: owner,
                },
            );
            let mut travel = 0.0;
            let mut changed_plans = 0;
            for rider in [RiderId::new(1), RiderId::new(2)] {
                let plan = RiderPlan {
                    stops: if rider == owner {
                        vec![
                            Stop::Pickup(OrderId::new(1)),
                            Stop::Dropoff(OrderId::new(1)),
                        ]
                    } else {
                        Vec::new()
                    },
                };
                let e = ok(ok(evaluate_recovery_plan(
                    &snapshot,
                    &ctx.pooling.inputs,
                    rider,
                    &plan,
                    &assignments,
                )));
                assert_eq!(e.breaches, Vec::new());
                travel += e.travel.value();
                changed_plans += u32::from(plan != world.data().plans[&rider]);
            }
            let score = travel
                + f64::from(changed_plans) * policy.reroute_penalty.value()
                + f64::from(u32::from(owner != RiderId::new(2)))
                    * policy.assignment_stability_penalty.value();
            values.push((owner, score));
        }
        let d = ok(recovery(&f, &world, ctx.clone()));
        assert_eq!(d, ok(recovery(&f, &world, ctx)));
        let selected = d
            .proposal()
            .map_or(RiderId::new(2), |p| p.assignments()[&OrderId::new(1)].rider);
        // Equal penalized cost does not justify optional churn.
        let expected = if values[0].1 < values[1].1 {
            RiderId::new(1)
        } else {
            RiderId::new(2)
        };
        assert_eq!(selected, expected, "seed {seed}");
    }
    println!("RECOVERY_ORACLE_EVIDENCE={{\"single_order_seeds\":32,\"matched\":32}}");
}
#[test]
fn hard_baseline_breach_can_repair_but_cumulative_reference_never_resets() {
    let mut f = Fixture::new();
    let order = OrderId::new(1);
    let rider = RiderId::new(2);
    f.data
        .orders
        .get_mut(&order)
        .unwrap_or_else(|| panic!("order"))
        .deadline = Some(at(44.0));
    f.data
        .riders
        .get_mut(&RiderId::new(1))
        .unwrap_or_else(|| panic!("rider"))
        .availability = Availability::Unavailable;
    let mut world = ok(World::new(19, f.data.clone()));
    let mut inputs = f.inputs(&world, at(0.0));
    let p = inputs
        .policies
        .get_mut(&order)
        .unwrap_or_else(|| panic!("policy"));
    p.deadline = AdmissionDeadline::Hard;
    p.max_completion_delay = Some(Seconds::ZERO);
    let d = ok(f.decide(&world, order, inputs));
    ok(world.commit_insertion(&d, &d.evidence().context));
    let terms = world.data().accepted.clone();
    ok(world.observe_ready(order, at(0.0)));
    let mut state = world.data().riders[&rider];
    state.coordinate = f.anchors.dropoffs[&order].coordinate;
    ok(world.update_rider_state(rider, state));
    f.anchors
        .riders
        .insert(rider, f.anchors.dropoffs[&order].clone());
    let state = RiderState {
        availability: Availability::Available,
        ..world.data().riders[&RiderId::new(1)]
    };
    ok(world.update_rider_state(RiderId::new(1), state));
    let mut ctx = context(&f, &world, churn());
    ctx.policy.assignment_stability_penalty = secs(100.0);
    let d = ok(recovery(&f, &world, ctx.clone()));
    assert!(d.baseline_requires_repair);
    assert_ne!(d.baseline[&rider].breaches.len(), 0);
    let proposal = d.proposal().unwrap_or_else(|| panic!("repair"));
    assert!(
        proposal
            .impacts
            .iter()
            .all(|i| i.consumed_seconds == Some(0.0))
    );
    ok(world.commit_recovery(&d, &ctx));
    assert_eq!(world.data().accepted, terms);
    // Both riders now project farther away; neither can erase the original reference.
    let r = RiderId::new(1);
    let state = RiderState {
        coordinate: f.anchors.dropoffs[&order].coordinate,
        ..world.data().riders[&r]
    };
    ok(world.update_rider_state(r, state));
    f.anchors
        .riders
        .insert(r, f.anchors.dropoffs[&order].clone());
    let d = ok(recovery(&f, &world, context(&f, &world, churn())));
    assert!(d.baseline_requires_repair);
    assert_eq!(d.termination, RecoveryTermination::NoRecovery);
    assert!(d.proposal().is_none());
    assert_eq!(world.data().accepted, terms);
}
#[test]
fn generated_multi_order_recoveries_preserve_exact_work_and_unordered_construction() {
    for seed in 0..16 {
        let mut f = Fixture::new();
        for (r, plan) in &mut f.data.plans {
            let orders: Vec<_> = (1..=3)
                .filter(|o| (*o + seed) % 2 + 1 == r.value())
                .map(OrderId::new)
                .collect();
            plan.stops = orders
                .iter()
                .copied()
                .map(Stop::Pickup)
                .chain(orders.iter().copied().map(Stop::Dropoff))
                .collect();
            for order in orders {
                f.data
                    .assignments
                    .insert(order, CommittedAssignment { order, rider: *r });
            }
        }
        let mut world = ok(World::new(19, f.data.clone()));
        let accepted = world.data().accepted.clone();
        let ctx = context(&f, &world, churn());
        let d = ok(recovery(&f, &world, ctx.clone()));
        let mut reordered = ctx.clone();
        reordered.pooling.inputs.policies = ctx
            .pooling
            .inputs
            .policies
            .iter()
            .rev()
            .map(|(o, p)| (*o, p.clone()))
            .collect();
        reordered.pooling.inputs.projections = ctx
            .pooling
            .inputs
            .projections
            .iter()
            .rev()
            .map(|(r, p)| (*r, p.clone()))
            .collect();
        assert_eq!(d, ok(recovery(&f, &world, reordered)));
        if d.proposal().is_some() {
            ok(world.commit_recovery(&d, &ctx));
        }
        ok(validate_world(world.data()));
        assert_eq!(world.data().assignments.len(), 3);
        assert_eq!(world.data().accepted, accepted);
        for order in world.data().assignments.keys() {
            assert_eq!(
                world
                    .data()
                    .plans
                    .values()
                    .flat_map(|p| &p.stops)
                    .filter(|s| **s == Stop::Pickup(*order))
                    .count(),
                1
            );
            assert_eq!(
                world
                    .data()
                    .plans
                    .values()
                    .flat_map(|p| &p.stops)
                    .filter(|s| **s == Stop::Dropoff(*order))
                    .count(),
                1
            );
        }
    }
}
