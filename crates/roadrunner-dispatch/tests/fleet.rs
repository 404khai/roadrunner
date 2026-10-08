//! Independent tiny fleet oracle and atomic fixed-owner invariants.
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
fn fleet_inputs(f: &Fixture, world: &World) -> FleetInputs {
    let mut pooling = f.inputs(world, at(0.0));
    pooling.identity.optimizer = "fleet-greedy-local/v1".into();
    FleetInputs {
        pooling,
        unavailable_projections: std::collections::BTreeMap::default(),
        algorithm: FleetAlgorithm::MultiStartLocal,
    }
}
fn fleet(f: &Fixture, world: &World, batch: &[OrderId], inputs: FleetInputs) -> FleetDecision {
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let snapshot = ok(DispatchSnapshot::new(
        world,
        at(0.0),
        RoutingEpoch(at(0.0)),
        &provider,
        &f.anchors,
    ));
    ok(optimize_fleet(&snapshot, batch, inputs))
}
fn objective_of(d: &FleetDecision) -> &FleetObjective {
    ok(d.evidence().selected_objective.as_ref().ok_or("objective"))
}

#[test]
fn greedy_trap_local_ejection_and_multistart_admit_two_and_match_exact_oracle() {
    let mut f = Fixture::new();
    f.anchors.pickups.remove(&OrderId::new(3));
    f.anchors.dropoffs.remove(&OrderId::new(3));
    let original_dropoffs = f.anchors.dropoffs.clone();
    f.data.orders.remove(&OrderId::new(3));
    f.data.readiness.remove(&OrderId::new(3));
    f.data.fulfillment.remove(&OrderId::new(3));
    // A goes 0→1, B goes 0→2; only rider1 can finish B's hard 20s deadline.
    for (id, node, deadline) in [(1, 1, 20.0), (2, 2, 20.0)] {
        let o = OrderId::new(id);
        let anchor = original_dropoffs[&OrderId::new(if node == 2 { 1 } else { 2 })].clone();
        f.anchors.dropoffs.insert(o, anchor.clone());
        let order = ok(f.data.orders.get_mut(&o).ok_or("order"));
        order.dropoff = anchor.coordinate;
        order.deadline = Some(at(deadline));
        f.data.readiness.insert(
            o,
            OrderReadiness {
                expected_at: Some(at(0.0)),
                observed_at: Some(at(0.0)),
            },
        );
    }
    for profile in f.data.profiles.values_mut() {
        profile.max_capacity = CapacityUnits::new(1);
    }
    let world = ok(World::new(18, f.data.clone()));
    let mut inputs = fleet_inputs(&f, &world);
    for p in inputs.pooling.policies.values_mut() {
        p.deadline = AdmissionDeadline::Hard;
        p.pickup_service = Seconds::ZERO;
        p.dropoff_service = Seconds::ZERO;
    }
    let batch = [OrderId::new(1), OrderId::new(2)];
    let mut greedy = inputs.clone();
    greedy.algorithm = FleetAlgorithm::Greedy;
    let g = fleet(&f, &world, &batch, greedy);
    assert_eq!(objective_of(&g).admitted_orders, 1);
    let mut local = inputs.clone();
    local.algorithm = FleetAlgorithm::LocalSearch;
    let l = fleet(&f, &world, &batch, local);
    assert_eq!(objective_of(&l).admitted_orders, 2);
    let result = fleet(&f, &world, &batch, inputs.clone());
    assert_eq!(objective_of(&result).admitted_orders, 2);
    let exact = oracle(&f, &world, &batch, &inputs);
    assert_eq!(*objective_of(&result), exact.0);
    assert_eq!(ok(result.proposal().ok_or("proposal")).plans(), &exact.1);
}

#[test]
fn publication_is_atomic_and_every_relevant_context_mismatch_is_stale() {
    let f = Fixture::new();
    let mut world = ok(World::new(18, f.data.clone()));
    let batch = [OrderId::new(1), OrderId::new(2)];
    let inputs = fleet_inputs(&f, &world);
    let d = fleet(&f, &world, &batch, inputs);
    let before = world.data().clone();
    let version = world.version();
    let base = d.evidence().context.clone();
    let mut contexts = vec![];
    let mut c = base.clone();
    c.pooling.at = at(1.0);
    contexts.push(c);
    let mut c = base.clone();
    c.pooling.epoch = RoutingEpoch(at(1.0));
    contexts.push(c);
    let mut c = base.clone();
    c.pooling.inputs.identity.service.push('x');
    contexts.push(c);
    let mut c = base.clone();
    c.pooling.inputs.identity.prediction.push('x');
    contexts.push(c);
    let mut c = base.clone();
    c.pooling.inputs.identity.routing.profile.push('x');
    contexts.push(c);
    let mut c = base.clone();
    c.pooling.inputs.identity.routing.graph_digest.push('x');
    contexts.push(c);
    let mut c = base.clone();
    c.pooling.inputs.identity.routing.traffic = TrafficIdentity::Static("changed/v1".into());
    contexts.push(c);
    let mut c = base.clone();
    c.pooling.inputs.identity.optimizer.push('x');
    contexts.push(c);
    let mut c = base.clone();
    c.pooling.inputs.forecasts.clear();
    contexts.push(c);
    let mut c = base.clone();
    c.pooling.inputs.policies.clear();
    contexts.push(c);
    let mut c = base.clone();
    c.pooling.inputs.projections.clear();
    contexts.push(c);
    let mut c = base.clone();
    c.pooling.anchors.dropoffs.clear();
    contexts.push(c);
    let mut c = base.clone();
    c.pooling.inputs.work_budget -= 1;
    contexts.push(c);
    let mut c = base.clone();
    c.algorithm = FleetAlgorithm::Greedy;
    contexts.push(c);
    for context in contexts {
        assert_eq!(world.commit_fleet(&d, &context), Err(CommitError::Stale));
        assert_eq!(*world.data(), before);
        assert_eq!(world.version(), version);
    }
    let p = ok(d.proposal().ok_or("proposal"));
    ok(world.commit_fleet(&d, &base));
    assert_eq!(world.data().assignments.len(), 2);
    assert_eq!(world.data().accepted.len(), 2);
    for (r, e) in p.evaluations() {
        assert_eq!(world.data().plans[r], e.plan);
    }
    for (o, t) in p.accepted() {
        assert_eq!(&world.data().accepted[o], t);
    }
    assert_eq!(world.commit_fleet(&d, &base), Err(CommitError::Stale));
    let mut other = ok(World::new(999, before));
    assert_eq!(other.commit_fleet(&d, &base), Err(CommitError::Stale));
}

#[test]
fn exhausted_search_never_leaks_an_incumbent_assignment_or_terms() {
    let f = Fixture::new();
    let mut world = ok(World::new(18, f.data.clone()));
    let original = world.data().clone();
    let full = fleet(
        &f,
        &world,
        &[OrderId::new(1), OrderId::new(2)],
        fleet_inputs(&f, &world),
    );
    for budget in [0, 1, full.evidence().work.total() - 1] {
        let mut inputs = fleet_inputs(&f, &world);
        inputs.pooling.work_budget = budget;
        let d = fleet(&f, &world, &[OrderId::new(1), OrderId::new(2)], inputs);
        assert_eq!(d.evidence().termination, FleetTermination::SearchIncomplete);
        assert_eq!(d.evidence().work.total(), budget);
        assert!(d.proposal().is_none());
        assert_eq!(
            world.commit_fleet(&d, &d.evidence().context),
            Err(CommitError::InvalidTransition)
        );
        assert_eq!(*world.data(), original);
    }
}

#[test]
fn unknown_inputs_are_isolated_without_discarding_healthy_remainder() {
    let mut f = Fixture::new();
    f.data.assignments.insert(
        OrderId::new(1),
        CommittedAssignment {
            order: OrderId::new(1),
            rider: RiderId::new(1),
        },
    );
    f.data.plans.insert(
        RiderId::new(1),
        RiderPlan {
            stops: vec![
                Stop::Pickup(OrderId::new(1)),
                Stop::Dropoff(OrderId::new(1)),
            ],
        },
    );
    let mut world = ok(World::new(18, f.data.clone()));
    let before = world.data().clone();
    let mut inputs = fleet_inputs(&f, &world);
    inputs.pooling.forecasts.remove(&OrderId::new(1));
    inputs.pooling.forecasts.remove(&OrderId::new(3));
    let d = fleet(&f, &world, &[OrderId::new(2), OrderId::new(3)], inputs);
    assert!(!d.evidence().input_complete);
    assert!(d.evidence().riders_complete);
    assert!(d.evidence().search_complete);
    assert_eq!(d.evidence().isolated_orders, vec![OrderId::new(3)]);
    assert_eq!(
        d.evidence().riders[0].isolation,
        Some(FleetIsolation::PredictionUnavailable(OrderId::new(1)))
    );
    ok(world.commit_fleet(&d, &d.evidence().context));
    assert_eq!(
        world.data().plans[&RiderId::new(1)],
        before.plans[&RiderId::new(1)]
    );
    assert_eq!(
        world.data().assignments[&OrderId::new(1)],
        before.assignments[&OrderId::new(1)]
    );
    assert_eq!(
        world.data().assignments[&OrderId::new(2)].rider,
        RiderId::new(2)
    );
    assert!(!world.data().accepted.contains_key(&OrderId::new(3)));
}

#[test]
fn hard_breached_baseline_is_unchanged_and_acceptance_is_not_reset() {
    let f = Fixture::new();
    let mut world = ok(World::new(18, f.data.clone()));
    let mut initial = f.inputs(&world, at(0.0));
    ok(initial.policies.get_mut(&OrderId::new(1)).ok_or("policy")).max_completion_delay =
        Some(secs(0.0));
    let insertion = ok(f.decide(&world, OrderId::new(1), initial));
    ok(world.commit_insertion(&insertion, &insertion.evidence().context));
    let original = world.data().clone();
    let terms = world.data().accepted[&OrderId::new(1)].clone();
    let mut inputs = fleet_inputs(&f, &world);
    inputs
        .pooling
        .policies
        .insert(OrderId::new(1), terms.policy.clone());
    ok(inputs
        .pooling
        .forecasts
        .get_mut(&OrderId::new(1))
        .ok_or("forecast"))
    .expected_at = at(50.0);
    // Change authoritative forecast coherently; a new world simulates a forecast update.
    let mut data = world.data().clone();
    ok(data.readiness.get_mut(&OrderId::new(1)).ok_or("readiness")).expected_at = Some(at(50.0));
    world = ok(World::new(19, data));
    let d = fleet(&f, &world, &[OrderId::new(2)], inputs);
    assert!(
        d.evidence()
            .riders
            .iter()
            .any(|r| matches!(r.isolation, Some(FleetIsolation::BaselineBreach(_))))
    );
    ok(world.commit_fleet(&d, &d.evidence().context));
    assert_eq!(world.data().accepted[&OrderId::new(1)], terms);
    let owner = original.assignments[&OrderId::new(1)].rider;
    assert_eq!(world.data().plans[&owner], original.plans[&owner]);
    assert_eq!(
        world.data().assignments[&OrderId::new(1)],
        original.assignments[&OrderId::new(1)]
    );
}

// Independent exact oracle: Cartesian ownership/unplaced choices and full stop permutations.
// No production candidate generator, pruning, objective comparator, or greedy/local routines.
fn permutations(stops: &mut [Stop], index: usize, output: &mut Vec<Vec<Stop>>) {
    if index == stops.len() {
        if stops.iter().enumerate().all(|(i, s)| match s {
            Stop::Pickup(o) => !stops[..i].contains(&Stop::Dropoff(*o)),
            Stop::Dropoff(_) => true,
        }) {
            output.push(stops.to_vec());
        }
        return;
    }
    for j in index..stops.len() {
        stops.swap(index, j);
        permutations(stops, index + 1, output);
        stops.swap(index, j);
    }
}
type OracleResult = (
    FleetObjective,
    std::collections::BTreeMap<RiderId, RiderPlan>,
);
#[allow(clippy::too_many_lines)]
fn oracle(f: &Fixture, world: &World, batch: &[OrderId], inputs: &FleetInputs) -> OracleResult {
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let snapshot = ok(DispatchSnapshot::new(
        world,
        at(0.0),
        RoutingEpoch(at(0.0)),
        &provider,
        &f.anchors,
    ));
    let riders: Vec<_> = world.data().profiles.keys().copied().collect();
    let choices = ok((riders.len() + 1)
        .checked_pow(ok(u32::try_from(batch.len())))
        .ok_or("choices"));
    let mut best: Option<OracleResult> = None;
    for code in 0..choices {
        let mut value = code;
        let mut lists: Vec<Vec<Stop>> = riders
            .iter()
            .map(|r| world.data().plans[r].stops.clone())
            .collect();
        let mut owners = vec![None; batch.len()];
        for (i, o) in batch.iter().enumerate() {
            let choice = value % (riders.len() + 1);
            value /= riders.len() + 1;
            if choice > 0 {
                owners[i] = Some(choice - 1);
                lists[choice - 1].extend([Stop::Pickup(*o), Stop::Dropoff(*o)]);
            }
        }
        let mut variants = Vec::new();
        for list in &mut lists {
            let mut p = vec![];
            permutations(list, 0, &mut p);
            variants.push(p);
        }
        let mut cursor = vec![0; riders.len()];
        loop {
            let mut plans = std::collections::BTreeMap::new();
            let mut seconds = 0.0;
            let mut meters = 0.0;
            let mut feasible = true;
            for (i, r) in riders.iter().enumerate() {
                let plan = RiderPlan {
                    stops: variants[i][cursor[i]].clone(),
                };
                let new: Vec<_> = batch
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| owners[*j] == Some(i))
                    .map(|(_, o)| *o)
                    .collect();
                match ok(evaluate_batch_plan(
                    &snapshot,
                    &inputs.pooling,
                    *r,
                    &plan,
                    &new,
                )) {
                    Ok(e) if e.breaches.is_empty() => {
                        seconds += e.travel.value();
                        meters += e.distance.value();
                    }
                    _ => feasible = false,
                }
                plans.insert(*r, plan);
            }
            if feasible {
                let candidate = (
                    FleetObjective {
                        admitted_orders: owners.iter().flatten().count(),
                        road_seconds: seconds,
                        road_meters: meters,
                    },
                    plans,
                );
                let replace = best.as_ref().is_none_or(|b| {
                    candidate.0.admitted_orders > b.0.admitted_orders
                        || (candidate.0.admitted_orders == b.0.admitted_orders
                            && (
                                candidate.0.road_seconds,
                                candidate.0.road_meters,
                                candidate
                                    .1
                                    .iter()
                                    .map(|(r, p)| (*r, p.stops.clone()))
                                    .collect::<Vec<_>>(),
                            ) < (
                                b.0.road_seconds,
                                b.0.road_meters,
                                b.1.iter()
                                    .map(|(r, p)| (*r, p.stops.clone()))
                                    .collect::<Vec<_>>(),
                            ))
                });
                if replace {
                    best = Some(candidate);
                }
            }
            let mut i = 0;
            while i < cursor.len() {
                cursor[i] += 1;
                if cursor[i] < variants[i].len() {
                    break;
                }
                cursor[i] = 0;
                i += 1;
            }
            if i == cursor.len() {
                break;
            }
        }
    }
    ok(best.ok_or("oracle"))
}

#[test]
fn seeded_tiny_quality_replay_and_unordered_construction() {
    for seed in 0..16 {
        let mut f = Fixture::new();
        for (i, o) in f.data.orders.values_mut().enumerate() {
            o.demand = CapacityUnits::new(1 + (seed + ok(u64::try_from(i))) % 2);
        }
        for r in f.data.profiles.values_mut() {
            r.max_capacity = CapacityUnits::new(1 + seed % 3);
        }
        let mut world = ok(World::new(18, f.data.clone()));
        let inputs = fleet_inputs(&f, &world);
        let batch = [OrderId::new(1), OrderId::new(2)];
        let d = fleet(&f, &world, &batch, inputs.clone());
        let reversed = fleet(&f, &world, &[batch[1], batch[0]], inputs.clone());
        assert_eq!(d, reversed);
        assert_eq!(d, fleet(&f, &world, &batch, inputs.clone()));
        let exact = oracle(&f, &world, &batch, &inputs);
        assert_eq!(*objective_of(&d), exact.0); // explicit tiny dataset quality, not universal optimum
        if let Some(p) = d.proposal() {
            let accepted = p.accepted().clone();
            ok(world.commit_fleet(&d, &d.evidence().context));
            ok(validate_world(world.data()));
            assert_eq!(world.data().accepted, accepted);
        }
    }
}

#[test]
fn committed_suffix_resequencing_fixed_custody_and_acceptance_references() {
    let f = Fixture::new();
    let mut data = f.data.clone();
    let rider = RiderId::new(1);
    for o in [OrderId::new(1), OrderId::new(2)] {
        data.assignments
            .insert(o, CommittedAssignment { order: o, rider });
    }
    data.plans.insert(
        rider,
        RiderPlan {
            stops: vec![
                Stop::Pickup(OrderId::new(1)),
                Stop::Dropoff(OrderId::new(1)),
                Stop::Pickup(OrderId::new(2)),
                Stop::Dropoff(OrderId::new(2)),
            ],
        },
    );
    let mut world = ok(World::new(18, data));
    let before = world.data().clone();
    let d = fleet(&f, &world, &[], fleet_inputs(&f, &world));
    assert!(objective_of(&d).road_seconds < d.evidence().baseline_objective.road_seconds);
    ok(world.commit_fleet(&d, &d.evidence().context));
    assert_eq!(world.data().assignments, before.assignments);
    assert_eq!(world.data().accepted, before.accepted);
    ok(validate_world(world.data()));
    // Onboard custody has no pickup and stays attached to its original owner.
    let mut data = before.clone();
    ok(data.readiness.get_mut(&OrderId::new(1)).ok_or("readiness")).observed_at = Some(at(0.0));
    data.fulfillment.insert(
        OrderId::new(1),
        FulfillmentState::PickedUp { rider, at: at(0.0) },
    );
    data.plans.get_mut(&rider).map(|p| p.stops.remove(0));
    let world = ok(World::new(19, data));
    let d = fleet(&f, &world, &[OrderId::new(3)], fleet_inputs(&f, &world));
    let p = ok(d.proposal().ok_or("p"));
    let stops = &p.plans()[&rider].stops;
    assert!(!stops.contains(&Stop::Pickup(OrderId::new(1))));
    assert_eq!(
        stops
            .iter()
            .filter(|s| **s == Stop::Dropoff(OrderId::new(1)))
            .count(),
        1
    );
}

#[test]
fn unsupported_policy_or_migration_is_not_input_isolation() {
    let mut f = Fixture::new();
    let rider = RiderId::new(1);
    let o = OrderId::new(1);
    f.data
        .assignments
        .insert(o, CommittedAssignment { order: o, rider });
    f.data.plans.insert(
        rider,
        RiderPlan {
            stops: vec![Stop::Pickup(o), Stop::Dropoff(o)],
        },
    );
    let world = ok(World::new(18, f.data.clone()));
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let snapshot = ok(DispatchSnapshot::new(
        &world,
        at(0.0),
        RoutingEpoch(at(0.0)),
        &provider,
        &f.anchors,
    ));
    let mut inputs = fleet_inputs(&f, &world);
    inputs.pooling.forecasts.clear();
    ok(inputs.pooling.policies.get_mut(&o).ok_or("p")).version = 99;
    assert_eq!(
        optimize_fleet(&snapshot, &[OrderId::new(2)], inputs),
        Err(PoolingError::UnsupportedPolicy)
    );
    let mut inputs = fleet_inputs(&f, &world);
    ok(inputs.pooling.policies.get_mut(&o).ok_or("p")).max_completion_delay = Some(secs(1.0));
    assert_eq!(
        optimize_fleet(&snapshot, &[OrderId::new(2)], inputs),
        Err(PoolingError::UnsupportedMigration)
    );
}

#[test]
fn tiny_generated_exact_oracle_quality_evidence() {
    let mut evidence = Vec::new();
    for seed in 0_u64..24 {
        let mut f = Fixture::new();
        for (id, o) in &mut f.data.orders {
            let n = if (seed.wrapping_mul(17) + id.value() * 11) % 3 == 0 {
                2
            } else {
                1
            };
            let anchor = f.anchors.dropoffs[&OrderId::new(if n == 2 { 1 } else { 2 })].clone();
            o.dropoff = anchor.coordinate;
            f.anchors.dropoffs.insert(*id, anchor);
            o.deadline =
                Some(at(35.0
                    + f64::from(ok(u32::try_from(
                        (seed * 7 + id.value() * 13) % 45,
                    )))));
            o.demand = CapacityUnits::new(1 + (seed + id.value()) % 2);
        }
        for p in f.data.profiles.values_mut() {
            p.max_capacity = CapacityUnits::new(1 + seed % 3);
        }
        let world = ok(World::new(18, f.data.clone()));
        let mut inputs = fleet_inputs(&f, &world);
        for p in inputs.pooling.policies.values_mut() {
            p.deadline = AdmissionDeadline::Hard;
        }
        let batch = [OrderId::new(1), OrderId::new(2), OrderId::new(3)];
        let exact = oracle(&f, &world, &batch, &inputs);
        for algorithm in [
            FleetAlgorithm::Greedy,
            FleetAlgorithm::LocalSearch,
            FleetAlgorithm::MultiStartLocal,
        ] {
            inputs.algorithm = algorithm;
            let d = fleet(&f, &world, &batch, inputs.clone());
            let found = objective_of(&d);
            assert!(found.admitted_orders <= exact.0.admitted_orders);
            if found.admitted_orders == exact.0.admitted_orders {
                assert!(found.road_seconds >= exact.0.road_seconds - 1e-8);
            }
            let replay = fleet(&f, &world, &batch, inputs.clone());
            assert_eq!(d, replay);
            evidence.push(serde_json::json!({"seed":seed,"algorithm":algorithm,"oracle":exact.0,"selected":found,"admission_gap":exact.0.admitted_orders-found.admitted_orders,"road_seconds_gap_if_equal_count":(found.admitted_orders==exact.0.admitted_orders).then_some(found.road_seconds-exact.0.road_seconds),"work":d.evidence().work,"termination":d.evidence().termination}));
        }
    }
    println!(
        "FLEET_ORACLE_EVIDENCE={}",
        ok(serde_json::to_string(
            &serde_json::json!({"schema_version":1,"dataset":"seeded-star-three-orders-two-riders/v1","seed_range":[0,24],"enumeration":"independent ownership/unplaced Cartesian product and all precedence-valid stop permutations","cases":evidence})
        ))
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Keep the hand-worked matrix and custody fixture beside its proof.
fn declared_local_optimum_has_a_measured_gap_to_independent_global_oracle() {
    // Directed metric closure: every single-stop relocation is worse at [2,4,1,3].
    // Global [1,3,2,4] needs more than one improving single-stop relocation.
    let matrix = [
        [0, 4, 11, 10, 19],
        [13, 0, 18, 6, 25],
        [13, 8, 0, 6, 8],
        [7, 11, 12, 0, 19],
        [17, 7, 25, 13, 0],
    ];
    let mut b = GraphBuilder::new(
        GraphSnapshotId::new(18),
        GraphMetadata::new(
            "test/v1",
            "synthetic",
            GraphBuildIdentity::new("local-trap", "v1", "v1", "synthetic"),
        ),
    );
    let points: Vec<_> = (0..5)
        .map(|i| ok(CanonicalCoordinate::new(0, i * 10_000)))
        .collect();
    for (id, c) in (0_u64..).zip(&points) {
        ok(b.add_node(BuilderNodeId::new(id), *c));
    }
    let mut segment = 0;
    for (from, row) in matrix.iter().enumerate() {
        for (to, time) in row.iter().enumerate() {
            if from == to {
                continue;
            }
            let d = haversine_distance(points[from].to_coordinate(), points[to].to_coordinate());
            let p = EdgeProperties::new(
                ok(KilometersPerHour::new(d.value() / f64::from(*time) * 3.6)),
                None,
                AccessClass::General,
            );
            ok(b.add_segment(
                BuilderSegmentId::new(segment),
                BuilderNodeId::new(ok(u64::try_from(from))),
                BuilderNodeId::new(ok(u64::try_from(to))),
                vec![points[from], points[to]],
                Some(p),
                None,
            ));
            segment += 1;
        }
    }
    let graph = ok(b.finalize());
    let anchor = |id: u32| RoutingAnchor {
        node: NodeId::new(id),
        coordinate: ok(graph.node(NodeId::new(id)).ok_or("node")).coordinate(),
        graph_digest: graph.metadata().snapshot_digest().into(),
    };
    let mut data = WorldData::default();
    let mut anchors = RoutingAnchors::default();
    let rider = RiderId::new(1);
    data.profiles.insert(
        rider,
        RiderProfile {
            id: rider,
            routing_profile: "test/v1".into(),
            max_capacity: CapacityUnits::new(4),
        },
    );
    data.riders.insert(
        rider,
        RiderState {
            coordinate: anchor(0).coordinate,
            availability: Availability::Available,
        },
    );
    anchors.riders.insert(rider, anchor(0));
    data.plans.insert(
        rider,
        RiderPlan {
            stops: [2, 4, 1, 3]
                .map(|id| Stop::Dropoff(OrderId::new(id)))
                .to_vec(),
        },
    );
    for id in 1..=4 {
        let o = OrderId::new(id);
        let dropoff = anchor(ok(u32::try_from(id)));
        let pickup = anchor(0);
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
                expected_at: Some(at(0.0)),
                observed_at: Some(at(0.0)),
            },
        );
        data.fulfillment
            .insert(o, FulfillmentState::PickedUp { rider, at: at(0.0) });
        data.assignments
            .insert(o, CommittedAssignment { order: o, rider });
        anchors.pickups.insert(o, pickup);
        anchors.dropoffs.insert(o, dropoff);
    }
    let f = Fixture {
        graph,
        data,
        anchors,
    };
    let world = ok(World::new(18, f.data.clone()));
    let inputs = fleet_inputs(&f, &world);
    let d = fleet(&f, &world, &[], inputs.clone());
    let exact = oracle(&f, &world, &[], &inputs);
    assert_eq!(d.evidence().termination, FleetTermination::LocalOptimum);
    assert!(d.proposal().is_none());
    assert!((objective_of(&d).road_seconds - 32.0).abs() < 1e-8);
    assert!((exact.0.road_seconds - 30.0).abs() < 1e-8);
    println!(
        "FLEET_LOCAL_MINIMUM_EVIDENCE={}",
        ok(serde_json::to_string(
            &serde_json::json!({"schema_version":1,"dataset":"directed-four-onboard-stops-relocation-trap/v1","matrix_road_seconds":matrix,"committed_owner":1,"initial_stop_order":[2,4,1,3],"selected":objective_of(&d),"oracle":exact.0,"road_seconds_gap":objective_of(&d).road_seconds-exact.0.road_seconds,"termination":d.evidence().termination,"work":d.evidence().work})
        ))
    );
}

#[test]
fn frozen_identity_world_revision_and_repeated_cumulative_reference_are_pinned() {
    let f = Fixture::new();
    let mut world = ok(World::new(18, f.data.clone()));
    let mut inputs = f.inputs(&world, at(0.0));
    ok(inputs.policies.get_mut(&OrderId::new(1)).ok_or("policy")).max_completion_delay =
        Some(secs(100.0));
    let first = ok(f.decide(&world, OrderId::new(1), inputs));
    ok(world.commit_insertion(&first, &first.evidence().context));
    let rider = ok(first.proposal().ok_or("first")).rider();
    let accepted = world.data().accepted[&OrderId::new(1)].clone();
    let mut inputs = fleet_inputs(&f, &world);
    inputs
        .pooling
        .policies
        .insert(OrderId::new(1), accepted.policy.clone());
    let frozen = FrozenPrefix {
        execution_id: 18,
        timeline: ok(StopTimeline::new(
            Stop::Pickup(OrderId::new(1)),
            at(0.0),
            secs(15.0),
            secs(2.0),
        )),
        anchor: f.anchors.pickups[&OrderId::new(1)].clone(),
    };
    inputs.pooling.projections.insert(
        rider,
        ok(project_execution(
            world.data(),
            rider,
            at(0.0),
            f.anchors.riders[&rider].clone(),
            Some(frozen),
        )),
    );
    let d = fleet(&f, &world, &[OrderId::new(2)], inputs);
    let before = world.data().clone();
    let mut current = d.evidence().context.clone();
    ok(ok(current
        .pooling
        .inputs
        .projections
        .get_mut(&rider)
        .ok_or("projection"))
    .frozen
    .as_mut()
    .ok_or("frozen"))
    .execution_id += 1;
    assert_eq!(world.commit_fleet(&d, &current), Err(CommitError::Stale));
    assert_eq!(*world.data(), before);
    ok(world.commit_fleet(&d, &d.evidence().context));
    assert_eq!(world.data().accepted[&OrderId::new(1)], accepted);
    assert_eq!(
        world.data().plans[&rider].stops.first(),
        Some(&Stop::Pickup(OrderId::new(1)))
    );
    // A second batch must keep the original reference even after the first plan rewrite.
    let mut inputs = fleet_inputs(&f, &world);
    inputs
        .pooling
        .policies
        .insert(OrderId::new(1), accepted.policy.clone());
    let third = fleet(&f, &world, &[OrderId::new(3)], inputs);
    let customer = ok(third
        .evidence()
        .riders
        .iter()
        .flat_map(|r| &r.impacts)
        .find(|i| i.order == OrderId::new(1))
        .ok_or("impact"));
    assert_eq!(customer.accepted, Some(accepted.completion_reference));
    assert_eq!(
        customer.cumulative_seconds,
        Some(customer.candidate.value() - accepted.completion_reference.value())
    );
    if third.proposal().is_some() {
        ok(world.commit_fleet(&third, &third.evidence().context));
        assert_eq!(world.data().accepted[&OrderId::new(1)], accepted);
    }
    let inputs = fleet_inputs(&f, &world);
    let d = fleet(&f, &world, &[], inputs);
    ok(world.observe_ready(OrderId::new(3), at(0.0)));
    let after = world.data().clone();
    assert_eq!(
        world.commit_fleet(&d, &d.evidence().context),
        Err(CommitError::Stale)
    );
    assert_eq!(*world.data(), after);
}
