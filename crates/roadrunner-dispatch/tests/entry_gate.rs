//! Pre-Phase-13 architecture entry gate, using real core routing plus focused failures.
#![allow(clippy::float_cmp)]

use std::cell::RefCell;
use std::fmt::Debug;

use roadrunner_core::cost::{
    TimeDependentTrafficSnapshot, TrafficMultiplier, TrafficPoint, TrafficProfile,
};
use roadrunner_core::geo::{CanonicalCoordinate, Coordinate, KilometersPerHour, Meters, Seconds};
use roadrunner_core::graph::{
    AccessClass, BuilderNodeId, BuilderSegmentId, EdgeProperties, FrozenGraph, GraphBuildIdentity,
    GraphBuilder, GraphMetadata, GraphSnapshotId, NodeId,
};
use roadrunner_core::routing::RoutingError;
use roadrunner_dispatch::*;

fn ok<T, E: Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("fixture: {error:?}"))
}
fn instant(value: f64) -> DispatchInstant {
    ok(DispatchInstant::new(value))
}
fn seconds(value: f64) -> Seconds {
    ok(Seconds::new(value))
}
fn order_id() -> OrderId {
    OrderId::new(1)
}
fn near() -> RiderId {
    RiderId::new(9)
}
fn fast() -> RiderId {
    RiderId::new(2)
}
fn policy() -> CandidatePolicy {
    CandidatePolicy::Exhaustive
}
fn profile() -> String {
    "dispatch-motorcycle/v1".to_owned()
}

struct Fixture {
    graph: FrozenGraph,
    data: WorldData,
    anchors: RoutingAnchors,
}

impl Fixture {
    fn new() -> Self {
        let mut graph = GraphBuilder::new(
            GraphSnapshotId::new(7),
            GraphMetadata::new(
                profile(),
                "test",
                GraphBuildIdentity::new("dispatch-entry-gate", "v1", "v1", "test"),
            ),
        );
        let points: Vec<_> = (0..5)
            .map(|n| ok(CanonicalCoordinate::new(0, n * 10_000)))
            .collect();
        for (id, point) in points.iter().copied().enumerate() {
            ok(graph.add_node(BuilderNodeId::new(id as u64), point));
        }
        // Near rider's only road is slow; farther rider reaches pickup much sooner.
        for (id, from, to, speed) in [(0, 1_usize, 0_usize, 3.6), (1, 2, 0, 72.0), (2, 0, 3, 36.0)]
        {
            ok(graph.add_segment(
                BuilderSegmentId::new(id),
                BuilderNodeId::new(from as u64),
                BuilderNodeId::new(to as u64),
                vec![points[from], points[to]],
                Some(EdgeProperties::new(
                    ok(KilometersPerHour::new(speed)),
                    None,
                    AccessClass::General,
                )),
                None,
            ));
        }
        let graph = ok(graph.finalize());
        let location =
            |node: u32| ok(graph.node(NodeId::new(node)).ok_or("absent node")).coordinate();
        let mut data = WorldData::default();
        data.orders.insert(
            order_id(),
            Order {
                id: order_id(),
                pickup: location(0),
                dropoff: location(3),
                created_at: instant(0.0),
                deadline: Some(instant(101.0)),
                demand: CapacityUnits::new(2),
            },
        );
        data.readiness.insert(order_id(), OrderReadiness::default());
        data.fulfillment
            .insert(order_id(), FulfillmentState::AwaitingPickup);
        let anchor = |node: u32| RoutingAnchor {
            coordinate: location(node),
            graph_digest: graph.metadata().snapshot_digest().to_owned(),
            node: NodeId::new(node),
        };
        let mut anchors = RoutingAnchors::default();
        anchors.pickups.insert(order_id(), anchor(0));
        anchors.dropoffs.insert(order_id(), anchor(3));
        for (rider, node) in [(near(), 1), (fast(), 2)] {
            data.profiles.insert(
                rider,
                RiderProfile {
                    id: rider,
                    routing_profile: profile(),
                    max_capacity: CapacityUnits::new(3),
                },
            );
            data.riders.insert(
                rider,
                RiderState {
                    coordinate: location(node),
                    availability: Availability::Available,
                },
            );
            data.plans.insert(rider, RiderPlan::default());
            anchors.riders.insert(rider, anchor(node));
        }
        Self {
            graph,
            data,
            anchors,
        }
    }
    fn world(&self) -> World {
        ok(World::new(7, self.data.clone()))
    }
    fn decide(
        &self,
        world: &World,
        policy: CandidatePolicy,
    ) -> Result<AssignmentDecision, DispatchEvaluationError> {
        let provider = CoreRouteProvider::new(&self.graph, TrafficContext::FreeFlow)?;
        decide(world, &provider, &self.anchors, policy)
    }
}

fn decide(
    world: &World,
    provider: &dyn RouteProvider,
    anchors: &RoutingAnchors,
    policy: CandidatePolicy,
) -> Result<AssignmentDecision, DispatchEvaluationError> {
    let snapshot = DispatchSnapshot::new(
        world,
        instant(100.0),
        RoutingEpoch(instant(0.0)),
        provider,
        anchors,
    )?;
    basic_dispatch(&snapshot, order_id(), DecisionId::new(42), policy)
}
fn selected(decision: &AssignmentDecision) -> RiderId {
    let DispatchDecisionOutcome::Assigned(p) = decision.outcome() else {
        panic!("expected assignment")
    };
    p.rider
}
fn feasible(
    decision: &AssignmentDecision,
    rider: RiderId,
) -> (&PlanEvaluation, &ScoreContributions) {
    let c = ok(decision
        .evidence()
        .candidates
        .iter()
        .find(|c| c.rider == rider)
        .ok_or("absent evidence"));
    let CandidateResult::Feasible { evaluation, score } = &c.result else {
        panic!("expected feasible")
    };
    (evaluation, score)
}

#[test]
fn exhaustive_routed_best_differs_from_spatial_nearest_and_records_coverage() {
    let f = Fixture::new();
    let world = f.world();
    let exhaustive = ok(f.decide(&world, policy()));
    assert_eq!(selected(&exhaustive), fast());
    assert_eq!(exhaustive.evidence().candidates.len(), 2);
    assert_eq!(exhaustive.evidence().coverage, CandidateCoverage::Complete);
    let shortlist = ok(f.decide(
        &world,
        CandidatePolicy::Spatial {
            radius: ok(Meters::new(1000.0)),
            limit: 1,
        },
    ));
    assert_eq!(selected(&shortlist), near());
    assert_eq!(
        shortlist.evidence().coverage,
        CandidateCoverage::PotentiallyIncomplete
    );
    assert_eq!(shortlist.evidence().candidates.len(), 1);
    assert_eq!(world.version(), WorldVersion::new(0));
    assert!(world.data().assignments.is_empty());
}

#[test]
fn eligibility_filters_unavailable_and_coherent_busy_riders_before_screening() {
    let mut f = Fixture::new();
    ok(f.data.riders.get_mut(&near()).ok_or("rider")).availability = Availability::Unavailable;
    let d = ok(f.decide(&f.world(), policy()));
    assert_eq!(d.evidence().candidates.len(), 1);
    let mut world = f.world();
    ok(world.commit(&d));
    assert!(!ok(basic_dispatch_eligible(world.data(), fast())));
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let snapshot = ok(DispatchSnapshot::new(
        &world,
        instant(100.0),
        RoutingEpoch(instant(0.0)),
        &provider,
        &f.anchors,
    ));
    assert_eq!(
        ok(generate_candidates(&snapshot, order_id(), policy())),
        [] as [RiderId; 0]
    );
}

#[test]
fn anchors_missing_wrong_graph_absent_node_and_wrong_location_are_input_errors() {
    let mut f = Fixture::new();
    let world = f.world();
    assert!(f.decide(&world, policy()).is_ok());
    let original = ok(f.anchors.pickups.remove(&order_id()).ok_or("pickup"));
    assert_eq!(
        f.decide(&world, policy()),
        Err(DispatchEvaluationError::MissingAnchor)
    );
    let mut bad = original.clone();
    bad.graph_digest = "other-graph".into();
    f.anchors.pickups.insert(order_id(), bad);
    assert_eq!(
        f.decide(&world, policy()),
        Err(DispatchEvaluationError::InvalidAnchor)
    );
    let mut bad = original.clone();
    bad.node = NodeId::new(999);
    f.anchors.pickups.insert(order_id(), bad);
    assert_eq!(
        f.decide(&world, policy()),
        Err(DispatchEvaluationError::InvalidAnchor)
    );
    let mut bad = original.clone();
    bad.coordinate = ok(Coordinate::new(1.0, 1.0));
    f.anchors.pickups.insert(order_id(), bad);
    assert_eq!(
        f.decide(&world, policy()),
        Err(DispatchEvaluationError::InvalidAnchor)
    );
    f.anchors.pickups.insert(order_id(), original);
    f.anchors.riders.remove(&near());
    assert_eq!(
        f.decide(&world, policy()),
        Err(DispatchEvaluationError::MissingAnchor)
    );
}

#[derive(Clone, Copy)]
enum Fault {
    None,
    Failure,
    Graph,
    Traffic,
    Profile,
    Departure,
}
struct RecordingProvider<'a> {
    core: CoreRouteProvider<'a>,
    calls: RefCell<Vec<Seconds>>,
    fault: Fault,
}
impl RouteProvider for RecordingProvider<'_> {
    fn graph(&self) -> &FrozenGraph {
        self.core.graph()
    }
    fn provenance(&self) -> RoutingProvenance {
        self.core.provenance()
    }
    fn route(
        &self,
        from: NodeId,
        to: NodeId,
        departure: Seconds,
    ) -> Result<RouteOutcome, DispatchEvaluationError> {
        self.calls.borrow_mut().push(departure);
        if matches!(self.fault, Fault::Failure) {
            return Err(DispatchEvaluationError::Routing(
                RoutingError::EvaluatorGraphMismatch,
            ));
        }
        let mut outcome = self.core.route(from, to, departure)?;
        if let RouteOutcome::RouteFound(leg) = &mut outcome {
            match self.fault {
                Fault::Graph => leg.provenance.graph_digest = "wrong".into(),
                Fault::Traffic => leg.provenance.traffic = TrafficIdentity::Static("wrong".into()),
                Fault::Profile => leg.provenance.profile = "wrong".into(),
                Fault::Departure => leg.departure = seconds(999.0),
                Fault::None | Fault::Failure => {}
            }
        }
        Ok(outcome)
    }
}

#[test]
fn no_route_is_candidate_infeasibility_but_routing_failure_is_evaluation_error() {
    let mut f = Fixture::new();
    let point = ok(f.graph.node(NodeId::new(4)).ok_or("node")).coordinate();
    ok(f.data.orders.get_mut(&order_id()).ok_or("order")).dropoff = point;
    f.anchors.dropoffs.insert(
        order_id(),
        RoutingAnchor {
            coordinate: point,
            graph_digest: f.graph.metadata().snapshot_digest().into(),
            node: NodeId::new(4),
        },
    );
    let d = ok(f.decide(&f.world(), policy()));
    assert_eq!(
        d.outcome(),
        &DispatchDecisionOutcome::Unassigned {
            scope: UnassignedScope::EligibleFleet
        }
    );
    assert!(
        d.evidence()
            .candidates
            .iter()
            .all(|c| c.result == CandidateResult::Rejected(CandidateRejection::NoRoute))
    );
    let provider = RecordingProvider {
        core: ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow)),
        calls: RefCell::default(),
        fault: Fault::Failure,
    };
    assert!(matches!(
        decide(&f.world(), &provider, &f.anchors, policy()),
        Err(DispatchEvaluationError::Routing(_))
    ));
}

#[test]
fn all_route_provenance_components_and_departure_are_checked() {
    let f = Fixture::new();
    for fault in [
        Fault::Graph,
        Fault::Traffic,
        Fault::Profile,
        Fault::Departure,
    ] {
        let provider = RecordingProvider {
            core: ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow)),
            calls: RefCell::default(),
            fault,
        };
        assert_eq!(
            decide(&f.world(), &provider, &f.anchors, policy()),
            Err(DispatchEvaluationError::ProvenanceMismatch)
        );
    }
}

#[test]
fn scalar_capacity_and_candidate_profile_are_enforced_before_routing() {
    let mut f = Fixture::new();
    ok(f.data.profiles.get_mut(&near()).ok_or("rider")).max_capacity = CapacityUnits::new(1);
    ok(f.data.profiles.get_mut(&fast()).ok_or("rider")).routing_profile = "bicycle".into();
    let provider = RecordingProvider {
        core: ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow)),
        calls: RefCell::default(),
        fault: Fault::None,
    };
    let d = ok(decide(&f.world(), &provider, &f.anchors, policy()));
    assert!(provider.calls.borrow().is_empty());
    assert!(d.evidence().candidates.contains(&CandidateEvidence {
        rider: near(),
        result: CandidateResult::Rejected(CandidateRejection::CapacityExceeded)
    }));
    assert!(d.evidence().candidates.contains(&CandidateEvidence {
        rider: fast(),
        result: CandidateResult::Rejected(CandidateRejection::UnsupportedCandidateProfile)
    }));
}

#[test]
fn deadlines_are_soft_observed_and_leg_departures_are_propagated() {
    let mut f = Fixture::new();
    let world = f.world();
    let provider = RecordingProvider {
        core: ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow)),
        calls: RefCell::default(),
        fault: Fault::None,
    };
    let late = ok(decide(&world, &provider, &f.anchors, policy()));
    let (evaluation, score) = feasible(&late, fast());
    assert!(evaluation.lateness > Seconds::ZERO);
    assert_eq!(evaluation.delivery_completed_at, evaluation.dropoff_arrival);
    assert_eq!(score.total, evaluation.travel);
    assert_eq!(evaluation.waiting, Seconds::ZERO);
    assert_eq!(evaluation.service, Seconds::ZERO);
    let calls = provider.calls.borrow();
    assert_eq!(calls[0], seconds(100.0));
    assert_eq!(
        calls[1],
        seconds(feasible(&late, fast()).0.pickup_departure.value())
    );
    assert_eq!(
        late.evidence().deadline_policy,
        DeadlinePolicy::SoftObserved
    );
    ok(f.data.orders.get_mut(&order_id()).ok_or("order")).deadline = None;
    let without = ok(f.decide(&f.world(), policy()));
    assert_eq!(selected(&late), selected(&without));
    assert_eq!(score, feasible(&without, fast()).1);
}

#[test]
fn already_expired_deadline_and_early_readiness_are_not_hidden_hard_constraints() {
    let mut f = Fixture::new();
    let request = ok(f.data.orders.get_mut(&order_id()).ok_or("order"));
    request.created_at = instant(50.0);
    request.deadline = Some(instant(20.0));
    // Stock can already be ready before the fulfillment request is created.
    f.data.readiness.insert(
        order_id(),
        OrderReadiness {
            expected_at: Some(instant(10.0)),
            observed_at: Some(instant(10.0)),
        },
    );
    let d = ok(f.decide(&f.world(), policy()));
    assert_eq!(selected(&d), fast());
    assert!(feasible(&d, fast()).0.lateness > seconds(80.0));
}

#[test]
fn exact_tie_and_permutations_use_lower_rider_id_and_canonical_evidence() {
    let mut f = Fixture::new();
    let state = f.data.riders[&near()];
    f.data.riders.insert(fast(), state);
    let anchor = f.anchors.riders[&near()].clone();
    f.anchors.riders.insert(fast(), anchor);
    let d = ok(f.decide(&f.world(), policy()));
    assert_eq!(selected(&d), fast());
    assert_eq!(
        d.evidence().selection_reason,
        Some(SelectionReason::ExactScoreThenRiderId {
            tied_riders: vec![fast(), near()]
        })
    );
    let mut candidates: Vec<_> = [near(), fast()]
        .into_iter()
        .map(|rider| FeasibleCandidate {
            rider,
            evaluation: feasible(&d, rider).0.clone(),
        })
        .collect();
    let first = ok(BaselineStrategy.rank(&candidates));
    candidates.reverse();
    assert_eq!(first, ok(BaselineStrategy.rank(&candidates)));
    let mut reversed = f.data.clone();
    reversed.riders = reversed.riders.into_iter().rev().collect();
    reversed.profiles = reversed.profiles.into_iter().rev().collect();
    assert_eq!(d, ok(f.decide(&ok(World::new(7, reversed)), policy())));
    // A real difference below an arbitrary epsilon must not be treated as a tie.
    candidates[0].evaluation.pickup_travel = seconds(1.0);
    candidates[0].evaluation.delivery_travel = Seconds::ZERO;
    candidates[1].evaluation.pickup_travel = seconds(1.0 - 1e-12);
    candidates[1].evaluation.delivery_travel = Seconds::ZERO;
    assert_eq!(
        ok(BaselineStrategy.rank(&candidates)).selected,
        Some(near())
    );
    candidates[0].evaluation.pickup_travel = seconds(f64::MAX);
    candidates[0].evaluation.delivery_travel = seconds(f64::MAX);
    assert_eq!(
        BaselineStrategy.rank(&candidates),
        Err(DispatchEvaluationError::InvalidMetric)
    );
}

#[test]
fn invalid_assignment_plan_custody_and_fulfillment_worlds_are_rejected() {
    let f = Fixture::new();
    let mut bad = f.data.clone();
    bad.assignments.insert(
        order_id(),
        CommittedAssignment {
            order: order_id(),
            rider: near(),
        },
    );
    assert!(matches!(
        World::new(7, bad),
        Err(DispatchEvaluationError::InvalidWorldState)
    ));
    let mut bad = f.data.clone();
    bad.plans.insert(
        near(),
        RiderPlan {
            stops: vec![Stop::Pickup(order_id()), Stop::Dropoff(order_id())],
        },
    );
    assert!(matches!(
        World::new(7, bad),
        Err(DispatchEvaluationError::InvalidWorldState)
    ));
    let mut bad = f.data.clone();
    bad.fulfillment.insert(
        order_id(),
        FulfillmentState::PickedUp {
            rider: near(),
            at: instant(1.0),
        },
    );
    assert!(matches!(
        World::new(7, bad),
        Err(DispatchEvaluationError::InvalidWorldState)
    ));
    let mut bad = f.data.clone();
    bad.fulfillment.insert(
        order_id(),
        FulfillmentState::Delivered {
            rider: near(),
            picked_up_at: instant(0.5),
            at: instant(1.0),
        },
    );
    bad.assignments.insert(
        order_id(),
        CommittedAssignment {
            order: order_id(),
            rider: near(),
        },
    );
    assert!(matches!(
        World::new(7, bad),
        Err(DispatchEvaluationError::InvalidWorldState)
    ));
}

#[test]
fn stale_commit_preserves_every_assignment_and_plan_and_repeated_commit_is_stale() {
    let f = Fixture::new();
    let mut world = f.world();
    let d = ok(f.decide(&world, policy()));
    let mut state = world.data().riders[&fast()];
    state.availability = Availability::Unavailable;
    ok(world.update_rider_state(fast(), state));
    let before = world.data().clone();
    assert_eq!(world.commit(&d), Err(CommitError::Stale));
    assert_eq!(world.data(), &before);
    assert_eq!(world.version(), WorldVersion::new(1));
    let mut fresh = f.world();
    ok(fresh.commit(&d));
    assert_eq!(fresh.data().assignments[&order_id()].rider, fast());
    assert_eq!(
        fresh.data().plans[&fast()].stops,
        [Stop::Pickup(order_id()), Stop::Dropoff(order_id())]
    );
    let before = fresh.data().clone();
    assert_eq!(fresh.commit(&d), Err(CommitError::Stale));
    assert_eq!(fresh.data(), &before);
    let mut other = ok(World::new(8, f.data.clone()));
    assert_eq!(other.commit(&d), Err(CommitError::Stale));
}

#[test]
fn structured_unassigned_scope_differs_for_complete_and_bounded_search() {
    let mut f = Fixture::new();
    for rider in f.data.profiles.values_mut() {
        rider.max_capacity = CapacityUnits::new(1);
    }
    let world = f.world();
    let complete = ok(f.decide(&world, policy()));
    let bounded = ok(f.decide(
        &world,
        CandidatePolicy::Spatial {
            radius: ok(Meters::new(1000.0)),
            limit: 1,
        },
    ));
    assert_eq!(
        complete.outcome(),
        &DispatchDecisionOutcome::Unassigned {
            scope: UnassignedScope::EligibleFleet
        }
    );
    assert_eq!(
        bounded.outcome(),
        &DispatchDecisionOutcome::Unassigned {
            scope: UnassignedScope::EvaluatedCandidates
        }
    );
    assert_eq!(
        bounded.evidence().coverage,
        CandidateCoverage::PotentiallyIncomplete
    );
    let empty = ok(f.decide(
        &world,
        CandidatePolicy::Spatial {
            radius: Meters::ZERO,
            limit: 0,
        },
    ));
    assert_eq!(empty.outcome(), bounded.outcome());
    assert_eq!(empty.evidence().candidates, [] as [CandidateEvidence; 0]);
    let json = ok(serde_json::to_value(&complete));
    assert!(json["evidence"]["routing"]["graph_digest"].is_string());
    assert!(json["evidence"]["candidates"].is_array());
    assert_eq!(json["evidence"]["strategy"], "basic-road-travel/v1");
}

#[test]
fn shared_transitions_advance_custody_and_remaining_plan_atomically() {
    let f = Fixture::new();
    let mut world = f.world();
    let d = ok(f.decide(&world, policy()));
    ok(world.commit(&d));
    let before = world.data().clone();
    assert_eq!(
        world.pickup(fast(), order_id(), instant(101.0)),
        Err(CommitError::InvalidTransition)
    );
    assert_eq!(world.data(), &before);
    ok(world.observe_ready(order_id(), instant(101.0)));
    let version = world.version();
    assert_eq!(
        world.observe_ready(order_id(), instant(102.0)),
        Err(CommitError::InvalidTransition)
    );
    assert_eq!(world.version(), version);
    ok(world.pickup(fast(), order_id(), instant(102.0)));
    assert_eq!(
        ok(onboard_load(world.data(), fast())),
        CapacityUnits::new(2)
    );
    assert_eq!(
        world.data().plans[&fast()].stops,
        [Stop::Dropoff(order_id())]
    );
    let before = world.data().clone();
    assert_eq!(
        world.deliver(near(), order_id(), instant(103.0)),
        Err(CommitError::InvalidTransition)
    );
    assert_eq!(
        world.deliver(fast(), order_id(), instant(101.0)),
        Err(CommitError::InvalidTransition)
    );
    assert_eq!(world.data(), &before);
    ok(world.deliver(fast(), order_id(), instant(103.0)));
    assert_eq!(
        ok(onboard_load(world.data(), fast())),
        CapacityUnits::new(0)
    );
    assert_eq!(world.data().plans[&fast()].stops, [] as [Stop; 0]);
    assert!(world.data().assignments.is_empty());
    assert_eq!(
        world.data().fulfillment[&order_id()],
        FulfillmentState::Delivered {
            rider: fast(),
            picked_up_at: instant(102.0),
            at: instant(103.0),
        }
    );
    assert!(ok(basic_dispatch_eligible(world.data(), fast())));
}

#[test]
fn general_remaining_plans_validate_custody_and_load_at_every_stop() {
    let f = Fixture::new();
    let mut data = f.data.clone();
    let b = OrderId::new(2);
    let mut request = data.orders[&order_id()].clone();
    request.id = b;
    request.demand = CapacityUnits::new(1);
    data.orders.insert(b, request);
    data.readiness.insert(b, OrderReadiness::default());
    data.fulfillment.insert(b, FulfillmentState::AwaitingPickup);
    for order in [order_id(), b] {
        data.assignments.insert(
            order,
            CommittedAssignment {
                order,
                rider: near(),
            },
        );
    }
    let plan = RiderPlan {
        stops: vec![
            Stop::Pickup(order_id()),
            Stop::Pickup(b),
            Stop::Dropoff(order_id()),
            Stop::Dropoff(b),
        ],
    };
    data.plans.insert(near(), plan.clone());
    assert!(validate_world(&data).is_ok());
    ok(data.profiles.get_mut(&near()).ok_or("profile")).max_capacity = CapacityUnits::new(2);
    assert_eq!(
        validate_plan(&data, near(), &plan),
        Err(PlanValidityError::CapacityExceeded)
    );
    ok(data.profiles.get_mut(&near()).ok_or("profile")).max_capacity = CapacityUnits::new(3);
    data.fulfillment.insert(
        order_id(),
        FulfillmentState::PickedUp {
            rider: near(),
            at: instant(2.0),
        },
    );
    ok(data.readiness.get_mut(&order_id()).ok_or("readiness")).observed_at = Some(instant(1.0));
    let remaining = RiderPlan {
        stops: vec![Stop::Pickup(b), Stop::Dropoff(order_id()), Stop::Dropoff(b)],
    };
    data.plans.insert(near(), remaining.clone());
    assert!(validate_world(&data).is_ok());
    assert_eq!(ok(onboard_load(&data, near())), CapacityUnits::new(2));
    data.fulfillment.insert(
        order_id(),
        FulfillmentState::PickedUp {
            rider: fast(),
            at: instant(2.0),
        },
    );
    assert_eq!(
        validate_plan(&data, near(), &remaining),
        Err(PlanValidityError::CustodyViolation)
    );
    assert_eq!(
        validate_world(&data),
        Err(DispatchEvaluationError::InvalidWorldState)
    );
}

#[test]
fn typed_time_epoch_and_stop_timeline_reject_invalid_conversion_and_overflow() {
    assert!(DispatchInstant::new(f64::INFINITY).is_err());
    assert!(DispatchInstant::new(-1.0).is_err());
    let epoch = RoutingEpoch(instant(100.0));
    assert_eq!(ok(epoch.departure_seconds(instant(102.5))), seconds(2.5));
    assert!(epoch.departure_seconds(instant(99.0)).is_err());
    assert!(instant(f64::MAX).checked_add(seconds(f64::MAX)).is_err());
    assert!(instant(f64::MAX).checked_add(seconds(1.0)).is_err());
    let stop = ok(StopTimeline::new(
        Stop::Dropoff(order_id()),
        instant(100.0),
        seconds(2.0),
        seconds(3.0),
    ));
    assert_eq!(stop.departure, instant(105.0));
    let f = Fixture::new();
    let world = f.world();
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    assert!(matches!(
        DispatchSnapshot::new(
            &world,
            instant(100.0),
            RoutingEpoch(instant(101.0)),
            &provider,
            &f.anchors
        ),
        Err(DispatchEvaluationError::Time(_))
    ));
}

#[test]
fn fifo_traffic_identity_and_nonzero_epoch_are_used_for_each_leg() {
    let f = Fixture::new();
    let world = f.world();
    let edge = ok(f.graph.outgoing_edges(NodeId::new(0)))[0].id();
    let traffic = ok(TimeDependentTrafficSnapshot::new(
        &f.graph,
        [TrafficProfile {
            edge_id: edge,
            points: vec![
                TrafficPoint {
                    departure_seconds: seconds(0.0),
                    multiplier: ok(TrafficMultiplier::new(1.0)),
                },
                TrafficPoint {
                    departure_seconds: seconds(200.0),
                    multiplier: ok(TrafficMultiplier::new(2.0)),
                },
            ],
        }],
    ));
    let provider = RecordingProvider {
        core: ok(CoreRouteProvider::new(
            &f.graph,
            TrafficContext::TimeDependent(&traffic),
        )),
        calls: RefCell::default(),
        fault: Fault::None,
    };
    let snapshot = ok(DispatchSnapshot::new(
        &world,
        instant(100.0),
        RoutingEpoch(instant(90.0)),
        &provider,
        &f.anchors,
    ));
    let d = ok(basic_dispatch(
        &snapshot,
        order_id(),
        DecisionId::new(43),
        policy(),
    ));
    assert_eq!(
        d.evidence().routing.traffic,
        TrafficIdentity::TimeDependent(traffic.traffic_snapshot_digest().into())
    );
    assert_eq!(provider.calls.borrow()[0], seconds(10.0));
    assert_eq!(
        provider.calls.borrow()[1],
        seconds(feasible(&d, fast()).0.pickup_departure.value() - 90.0)
    );
}
