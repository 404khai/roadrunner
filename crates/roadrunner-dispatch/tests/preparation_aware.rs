//! Phase 14 correctness: readiness-aware timing, preference, and observed execution.
#![allow(clippy::float_cmp)]

#[path = "../examples/support/preparation_scenario.rs"]
mod scenario;

use std::cell::RefCell;
use std::fmt::Debug;

use roadrunner_core::cost::{
    TimeDependentTrafficSnapshot, TrafficMultiplier, TrafficPoint, TrafficProfile,
};
use roadrunner_core::geo::{Meters, Seconds};
use roadrunner_core::graph::{FrozenGraph, NodeId};
use roadrunner_core::routing::RoutingError;
use roadrunner_dispatch::{
    AssignmentDecision, CandidateCoverage, CandidatePolicy, CandidateRejection, CandidateResult,
    CapacityUnits, CommitError, CoreRouteProvider, DeadlinePolicy, DecisionId,
    DispatchDecisionOutcome, DispatchEvaluationError, DispatchInstant, DispatchSnapshot,
    FeasibleCandidate, OrderReadiness, PlanEvaluation, PreparationAwareStrategy, ReadinessSource,
    RiderId, RouteOutcome, RouteProvider, RoutingProvenance, ScoreContributions, SelectionReason,
    TrafficContext, UnassignedScope, World, basic_dispatch, preparation_aware_dispatch,
};
use scenario::{Scenario, run};

fn ok<T, E: Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("fixture: {error:?}"))
}
fn seconds(value: f64) -> Seconds {
    ok(Seconds::new(value))
}
fn close(actual: Seconds, expected: f64) {
    assert!(
        (actual.value() - expected).abs() < 1e-9,
        "{} != {expected}",
        actual.value()
    );
}
fn offset(s: &Scenario, value: f64) -> DispatchInstant {
    ok(s.at.checked_add(seconds(value)))
}
fn selected(d: &AssignmentDecision) -> RiderId {
    let DispatchDecisionOutcome::Assigned(proposal) = d.outcome() else {
        panic!("expected assigned")
    };
    proposal.rider
}
fn metrics(d: &AssignmentDecision, rider: RiderId) -> (&PlanEvaluation, &ScoreContributions) {
    let candidate = ok(d
        .evidence()
        .candidates
        .iter()
        .find(|c| c.rider == rider)
        .ok_or("missing candidate"));
    let CandidateResult::Feasible { evaluation, score } = &candidate.result else {
        panic!("expected feasible")
    };
    (evaluation, score)
}
fn decide(
    s: &Scenario,
    world: &World,
    provider: &dyn RouteProvider,
    policy: CandidatePolicy,
    strategy: PreparationAwareStrategy,
) -> Result<AssignmentDecision, DispatchEvaluationError> {
    let snapshot = DispatchSnapshot::new(world, s.at, s.epoch, provider, &s.anchors)?;
    preparation_aware_dispatch(&snapshot, s.order, DecisionId::new(14), policy, strategy)
}
fn prepared(s: &Scenario) -> AssignmentDecision {
    let provider = ok(CoreRouteProvider::new(&s.graph, TrafficContext::FreeFlow));
    ok(decide(
        s,
        &ok(s.world()),
        &provider,
        CandidatePolicy::Exhaustive,
        PreparationAwareStrategy::default(),
    ))
}

#[test]
fn long_preparation_reduces_observed_idle_time_with_equal_delivery_completion() {
    let s = ok(Scenario::new(1080.0));
    let ready = offset(&s, 1080.0);
    let baseline = ok(run(&s, false, ready));
    let aware = ok(run(&s, true, ready));
    assert_eq!(baseline.observed.rider, s.near);
    assert_eq!(aware.observed.rider, s.far);
    close(baseline.observed.waiting, 780.0);
    close(aware.observed.waiting, 180.0);
    close(baseline.observed.completion_time, 1680.0);
    close(aware.observed.completion_time, 1680.0);
    close(aware.observed.delivery_travel, 600.0);
    let (near, near_score) = metrics(&aware.decision, s.near);
    let (far, far_score) = metrics(&aware.decision, s.far);
    close(near.pickup_travel, 300.0);
    close(far.pickup_travel, 900.0);
    assert_eq!(near.pickup_departure, ready);
    assert_eq!(far.pickup_departure, ready);
    assert_eq!(near.stops[0].waiting, near.waiting);
    assert_eq!(far.stops[0].waiting, far.waiting);
    assert_eq!(near.service, Seconds::ZERO);
    close(near_score.total, 2460.0);
    close(far_score.total, 1860.0);
    assert_eq!(
        ok(far_score.preparation.ok_or("score")).completion_time,
        far.completion_time
    );
    assert!(baseline.decision.evidence().preparation.is_none());
    assert_eq!(metrics(&baseline.decision, s.near).0.waiting, Seconds::ZERO);
    close(metrics(&baseline.decision, s.near).0.completion_time, 900.0);
    assert!(aware.decision.explanation().contains("waiting penalty"));
    let json = ok(serde_json::to_value(&aware.decision));
    assert_eq!(
        json["evidence"]["strategy"],
        "preparation-completion-wait/v1"
    );
    assert_eq!(json["evidence"]["preparation"]["source"], "Expected");
    assert_eq!(json["evidence"]["preparation"]["idle_penalty_weight"], 1.0);
}

#[test]
fn observed_execution_uses_actual_readiness_even_when_forecast_is_wrong() {
    let s = ok(Scenario::new(1080.0));
    let actual = offset(&s, 1500.0);
    let baseline = ok(run(&s, false, actual));
    let aware = ok(run(&s, true, actual));
    close(metrics(&aware.decision, s.far).0.completion_time, 1680.0);
    close(aware.observed.completion_time, 2100.0);
    close(aware.observed.waiting, 600.0);
    close(baseline.observed.waiting, 1200.0);
    assert_eq!(
        aware.observed.delivery_completed_at,
        baseline.observed.delivery_completed_at
    );
}

#[test]
fn past_equal_and_intermediate_ready_times_have_nonnegative_waiting() {
    for ready_seconds in [0.0, 300.0, 600.0, 900.0, 1080.0] {
        let s = ok(Scenario::new(ready_seconds));
        let decision = prepared(&s);
        for (rider, arrival) in [(s.near, 300.0), (s.far, 900.0)] {
            let (evaluation, _) = metrics(&decision, rider);
            close(evaluation.waiting, (ready_seconds - arrival).max(0.0));
            assert_eq!(evaluation.stops[1].waiting, Seconds::ZERO);
        }
    }
    let mut s = ok(Scenario::new(0.0));
    ok(s.data.readiness.get_mut(&s.order).ok_or("readiness")).expected_at =
        Some(ok(DispatchInstant::new(s.at.value() - 10.0)));
    assert_eq!(metrics(&prepared(&s), s.near).0.waiting, Seconds::ZERO);
    let observed = ok(run(&s, true, s.at));
    assert_eq!(observed.observed.rider, s.near);
    assert_eq!(observed.observed.waiting, Seconds::ZERO);
}

#[test]
fn actual_ready_observation_overrides_forecast_and_missing_readiness_is_an_error() {
    let mut s = ok(Scenario::new(1080.0));
    let provider = ok(CoreRouteProvider::new(&s.graph, TrafficContext::FreeFlow));
    let mut world = ok(s.world());
    ok(world.observe_ready(s.order, s.at));
    let d = ok(decide(
        &s,
        &world,
        &provider,
        CandidatePolicy::Exhaustive,
        PreparationAwareStrategy::default(),
    ));
    assert_eq!(selected(&d), s.near);
    let evidence = ok(d.evidence().preparation.ok_or("readiness evidence"));
    assert_eq!(evidence.source, ReadinessSource::Observed);
    assert_eq!(evidence.effective_ready_at, s.at);
    assert_eq!(evidence.readiness.expected_at, Some(offset(&s, 1080.0)));
    assert_eq!(metrics(&d, s.near).0.waiting, Seconds::ZERO);
    s.data.readiness.insert(s.order, OrderReadiness::default());
    let world = ok(s.world());
    assert_eq!(
        decide(
            &s,
            &world,
            &provider,
            CandidatePolicy::Exhaustive,
            PreparationAwareStrategy::default()
        ),
        Err(DispatchEvaluationError::MissingReadiness)
    );
    let snapshot = ok(DispatchSnapshot::new(
        &world, s.at, s.epoch, &provider, &s.anchors,
    ));
    assert!(
        basic_dispatch(
            &snapshot,
            s.order,
            DecisionId::new(13),
            CandidatePolicy::Exhaustive
        )
        .is_ok()
    );
    let mut future = ok(s.world());
    ok(future.observe_ready(s.order, offset(&s, 1.0)));
    assert_eq!(
        decide(
            &s,
            &future,
            &provider,
            CandidatePolicy::Exhaustive,
            PreparationAwareStrategy::default()
        ),
        Err(DispatchEvaluationError::InvalidWorldState)
    );
}

struct RecordingProvider<'a> {
    core: CoreRouteProvider<'a>,
    calls: RefCell<Vec<(NodeId, NodeId, Seconds)>>,
    fail_delivery: bool,
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
        self.calls.borrow_mut().push((from, to, departure));
        if self.fail_delivery && from == NodeId::new(0) {
            return Err(DispatchEvaluationError::Routing(
                RoutingError::EvaluatorGraphMismatch,
            ));
        }
        self.core.route(from, to, departure)
    }
}

#[test]
fn waiting_advances_fifo_traffic_departure_with_nonzero_epoch() {
    let s = ok(Scenario::new(1080.0));
    let edge = ok(s.graph.outgoing_edges(NodeId::new(0)))[0].id();
    let traffic = ok(TimeDependentTrafficSnapshot::new(
        &s.graph,
        [TrafficProfile {
            edge_id: edge,
            points: vec![
                TrafficPoint {
                    departure_seconds: Seconds::ZERO,
                    multiplier: ok(TrafficMultiplier::new(1.0)),
                },
                TrafficPoint {
                    departure_seconds: seconds(1800.0),
                    multiplier: ok(TrafficMultiplier::new(2.0)),
                },
            ],
        }],
    ));
    let provider = RecordingProvider {
        core: ok(CoreRouteProvider::new(
            &s.graph,
            TrafficContext::TimeDependent(&traffic),
        )),
        calls: RefCell::default(),
        fail_delivery: false,
    };
    let world = ok(s.world());
    let d = ok(decide(
        &s,
        &world,
        &provider,
        CandidatePolicy::Exhaustive,
        PreparationAwareStrategy::default(),
    ));
    assert_eq!(
        provider.calls.borrow().as_slice(),
        &[
            (NodeId::new(1), NodeId::new(0), Seconds::ZERO),
            (NodeId::new(0), NodeId::new(3), seconds(1080.0)),
            (NodeId::new(2), NodeId::new(0), Seconds::ZERO),
            (NodeId::new(0), NodeId::new(3), seconds(1080.0)),
        ]
    );
    for rider in [s.near, s.far] {
        let (evaluation, _) = metrics(&d, rider);
        close(evaluation.delivery_travel, 960.0);
        close(evaluation.completion_time, 2040.0);
        close(evaluation.travel, evaluation.pickup_travel.value() + 960.0);
    }
    assert!(world.data().assignments.is_empty());
}

#[test]
fn readiness_updates_invalidate_commit_and_forecasts_do_not_authorize_pickup() {
    let s = ok(Scenario::new(1080.0));
    let provider = ok(CoreRouteProvider::new(&s.graph, TrafficContext::FreeFlow));
    let mut world = ok(s.world());
    let old = ok(decide(
        &s,
        &world,
        &provider,
        CandidatePolicy::Exhaustive,
        PreparationAwareStrategy::default(),
    ));
    ok(world.estimate_readiness(s.order, s.at));
    let before = world.data().clone();
    assert_eq!(world.commit(&old), Err(CommitError::Stale));
    assert_eq!(world.data(), &before);
    assert_eq!(
        ok(old.evidence().preparation.ok_or("evidence")).effective_ready_at,
        offset(&s, 1080.0)
    );
    let fresh = ok(decide(
        &s,
        &world,
        &provider,
        CandidatePolicy::Exhaustive,
        PreparationAwareStrategy::default(),
    ));
    assert_eq!(selected(&fresh), s.near);
    ok(world.commit(&fresh));
    let before = world.data().clone();
    assert_eq!(
        world.pickup(s.near, s.order, offset(&s, 300.0)),
        Err(CommitError::InvalidTransition)
    );
    assert_eq!(world.data(), &before);
    ok(world.observe_ready(s.order, s.at));
    ok(world.pickup(s.near, s.order, offset(&s, 300.0)));
    ok(world.deliver(s.near, s.order, offset(&s, 900.0)));
}

#[test]
fn deadlines_include_waiting_but_remain_soft_observed() {
    let mut s = ok(Scenario::new(1080.0));
    let baseline = prepared(&s);
    let deadline = offset(&s, 1200.0);
    ok(s.data.orders.get_mut(&s.order).ok_or("order")).deadline = Some(deadline);
    let late = prepared(&s);
    assert_eq!(selected(&baseline), selected(&late));
    assert_eq!(
        late.evidence().deadline_policy,
        DeadlinePolicy::SoftObserved
    );
    for rider in [s.near, s.far] {
        close(metrics(&late, rider).0.lateness, 480.0);
        assert_eq!(metrics(&baseline, rider).1, metrics(&late, rider).1);
    }
}

#[test]
fn spatial_scope_capacity_and_profile_rejections_reuse_basic_dispatch_rules() {
    let mut s = ok(Scenario::new(1080.0));
    let provider = ok(CoreRouteProvider::new(&s.graph, TrafficContext::FreeFlow));
    let spatial = CandidatePolicy::Spatial {
        radius: ok(Meters::new(1000.0)),
        limit: 1,
    };
    let d = ok(decide(
        &s,
        &ok(s.world()),
        &provider,
        spatial,
        PreparationAwareStrategy::default(),
    ));
    assert_eq!(selected(&d), s.near);
    assert_eq!(
        d.evidence().coverage,
        CandidateCoverage::PotentiallyIncomplete
    );
    ok(s.data.profiles.get_mut(&s.near).ok_or("profile")).max_capacity = CapacityUnits::new(0);
    ok(s.data.profiles.get_mut(&s.far).ok_or("profile")).routing_profile = "other".into();
    let recorder = RecordingProvider {
        core: provider,
        calls: RefCell::default(),
        fail_delivery: false,
    };
    let world = ok(s.world());
    let d = ok(decide(
        &s,
        &world,
        &recorder,
        CandidatePolicy::Exhaustive,
        PreparationAwareStrategy::default(),
    ));
    assert_eq!(
        d.outcome(),
        &DispatchDecisionOutcome::Unassigned {
            scope: UnassignedScope::EligibleFleet
        }
    );
    assert_eq!(d.evidence().coverage, CandidateCoverage::Complete);
    assert_eq!(
        d.evidence().candidates[0].result,
        CandidateResult::Rejected(CandidateRejection::CapacityExceeded)
    );
    assert_eq!(
        d.evidence().candidates[1].result,
        CandidateResult::Rejected(CandidateRejection::UnsupportedCandidateProfile)
    );
    assert_eq!(recorder.calls.borrow().len(), 0);
    let d = ok(decide(
        &s,
        &world,
        &recorder,
        spatial,
        PreparationAwareStrategy::default(),
    ));
    assert_eq!(
        d.outcome(),
        &DispatchDecisionOutcome::Unassigned {
            scope: UnassignedScope::EvaluatedCandidates
        }
    );
}

#[test]
fn no_route_is_rejection_and_delivery_routing_failure_is_error_after_waiting() {
    let mut s = ok(Scenario::new(1080.0));
    let recorder = RecordingProvider {
        core: ok(CoreRouteProvider::new(&s.graph, TrafficContext::FreeFlow)),
        calls: RefCell::default(),
        fail_delivery: true,
    };
    let world = ok(s.world());
    assert!(matches!(
        decide(
            &s,
            &world,
            &recorder,
            CandidatePolicy::Exhaustive,
            PreparationAwareStrategy::default()
        ),
        Err(DispatchEvaluationError::Routing(_))
    ));
    assert_eq!(recorder.calls.borrow()[1].2, seconds(1080.0));
    assert!(world.data().assignments.is_empty());
    let point = ok(s.graph.node(NodeId::new(4)).ok_or("node")).coordinate();
    ok(s.data.orders.get_mut(&s.order).ok_or("order")).dropoff = point;
    let anchor = ok(s.anchors.dropoffs.get_mut(&s.order).ok_or("anchor"));
    anchor.coordinate = point;
    anchor.node = NodeId::new(4);
    let d = prepared(&s);
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
}

#[test]
fn preparation_ranking_is_pure_exact_and_weighted_with_checked_overflow() {
    let s = ok(Scenario::new(1080.0));
    let d = prepared(&s);
    let mut candidates: Vec<_> = [s.far, s.near]
        .into_iter()
        .map(|rider| FeasibleCandidate {
            rider,
            evaluation: metrics(&d, rider).0.clone(),
        })
        .collect();
    let zero = ok(PreparationAwareStrategy::new(0.0));
    let tied = ok(zero.rank(&candidates));
    assert_eq!(tied.selected, Some(s.near));
    assert_eq!(
        tied.reason,
        Some(SelectionReason::ExactScoreThenRiderId {
            tied_riders: vec![s.near, s.far]
        })
    );
    let normal = ok(PreparationAwareStrategy::default().rank(&candidates));
    assert_eq!(normal.selected, Some(s.far));
    candidates.reverse();
    assert_eq!(
        normal,
        ok(PreparationAwareStrategy::default().rank(&candidates))
    );
    // Exact finite ordering: a tiny real difference is never rounded into a tie.
    candidates[0].evaluation.completion_time = seconds(1.0);
    candidates[1].evaluation.completion_time = seconds(1.0 - 1e-12);
    assert_eq!(ok(zero.rank(&candidates)).selected, Some(s.far));
    candidates[0].evaluation.waiting = seconds(f64::MAX);
    assert_eq!(
        ok(PreparationAwareStrategy::new(2.0)).rank(&candidates),
        Err(DispatchEvaluationError::InvalidMetric)
    );
    candidates[0].evaluation.waiting = seconds(f64::MAX);
    candidates[0].evaluation.completion_time = seconds(f64::MAX);
    assert_eq!(
        PreparationAwareStrategy::default().rank(&candidates),
        Err(DispatchEvaluationError::InvalidMetric)
    );
    candidates[0].evaluation.waiting = Seconds::ZERO;
    candidates[0].evaluation.completion_time = Seconds::ZERO;
    candidates[1] = candidates[0].clone();
    assert_eq!(
        zero.rank(&candidates),
        Err(DispatchEvaluationError::InvalidMetric)
    );
    for invalid in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            PreparationAwareStrategy::new(invalid),
            Err(DispatchEvaluationError::InvalidMetric)
        );
    }
    assert_eq!(ok(zero.rank(&[])).selected, None);
}

#[test]
fn policy_overflow_and_unrepresentable_departure_fail_without_world_mutation() {
    let mut s = ok(Scenario::new(1080.0));
    let provider = ok(CoreRouteProvider::new(&s.graph, TrafficContext::FreeFlow));
    let world = ok(s.world());
    let before = world.data().clone();
    assert_eq!(
        decide(
            &s,
            &world,
            &provider,
            CandidatePolicy::Exhaustive,
            ok(PreparationAwareStrategy::new(f64::MAX))
        ),
        Err(DispatchEvaluationError::InvalidMetric)
    );
    assert_eq!(world.data(), &before);
    ok(s.data.readiness.get_mut(&s.order).ok_or("readiness")).expected_at =
        Some(ok(DispatchInstant::new(f64::MAX)));
    let world = ok(s.world());
    let before = world.data().clone();
    assert!(matches!(
        decide(
            &s,
            &world,
            &provider,
            CandidatePolicy::Exhaustive,
            PreparationAwareStrategy::default()
        ),
        Err(DispatchEvaluationError::Time(_))
    ));
    assert_eq!(world.data(), &before);
}

#[test]
fn configurable_idle_penalty_explicitly_trades_completion_time_for_waiting() {
    let s = ok(Scenario::new(720.0));
    let provider = ok(CoreRouteProvider::new(&s.graph, TrafficContext::FreeFlow));
    let world = ok(s.world());
    let completion_only = ok(decide(
        &s,
        &world,
        &provider,
        CandidatePolicy::Exhaustive,
        ok(PreparationAwareStrategy::new(0.0)),
    ));
    let weighted = ok(decide(
        &s,
        &world,
        &provider,
        CandidatePolicy::Exhaustive,
        PreparationAwareStrategy::default(),
    ));
    assert_eq!(selected(&completion_only), s.near);
    assert_eq!(selected(&weighted), s.far);
    close(metrics(&weighted, s.near).0.completion_time, 1320.0);
    close(metrics(&weighted, s.far).0.completion_time, 1500.0);
    close(metrics(&weighted, s.near).1.total, 1740.0);
    close(metrics(&weighted, s.far).1.total, 1500.0);
    let lower_weight = ok(decide(
        &s,
        &world,
        &provider,
        CandidatePolicy::Exhaustive,
        ok(PreparationAwareStrategy::new(0.25)),
    ));
    assert_eq!(selected(&lower_weight), s.near);
    assert_eq!(
        ok(lower_weight.evidence().preparation.ok_or("policy")).idle_penalty_weight,
        0.25
    );
}
