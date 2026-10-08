//! Batch formation, frozen execution, input isolation and deterministic actual outcomes.
mod support;
use roadrunner_dispatch::{
    AdmissionDeadline, FleetAlgorithm, FleetIsolation, FleetTermination, OrderPolicy,
    ReadinessRule, validate_world,
};
use roadrunner_simulation::{ActualReadiness, DispatchPolicy, SimulationEvent, simulate};
use support::{close, graph, ok, order, scenario, seconds};
fn pooled() -> roadrunner_simulation::SimulationScenario {
    let mut s = scenario();
    s.schema_version = 3;
    s.scenario_id = Some("fleet-tests/v1".into());
    s.end_seconds = seconds(200.0);
    s.dispatch = DispatchPolicy::FleetBatch {
        work_budget: 100_000,
        forecast_validity_seconds: seconds(1000.0),
        algorithm: FleetAlgorithm::MultiStartLocal,
    };
    s.orders = vec![order(1, 0.0, 25.0), order(2, 0.0, 25.0)];
    for o in &mut s.orders {
        o.admission = Some(OrderPolicy {
            id: "fleet-policy/v1".into(),
            version: 1,
            deadline: AdmissionDeadline::SoftObserved,
            max_completion_delay: None,
            pickup_service: seconds(2.0),
            dropoff_service: seconds(3.0),
            readiness: ReadinessRule::ValidForecastV1,
        });
    }
    s
}
#[test]
fn same_instant_orders_form_one_batch_one_commit_and_correct_union_occupancy() {
    let s = pooled();
    let r = ok(simulate(&graph(), &s));
    assert_eq!(r.summary.delivered_orders, 2);
    assert_eq!(r.fleets.len(), 1);
    assert!(r.fleets[0].committed);
    let p = ok(r.fleets[0].decision.proposal().ok_or("proposal"));
    assert_eq!(p.assignments().len(), 2);
    assert_eq!(p.accepted().len(), 2);
    let state = ok(r.final_state.as_ref().ok_or("state"));
    ok(validate_world(state));
    assert!(state.assignments.is_empty());
    assert!(state.plans.values().all(|p| p.stops.is_empty()));
    assert_eq!(state.accepted, p.accepted().clone());
    let last = ok(r
        .orders
        .iter()
        .filter_map(|o| o.delivered_at)
        .max_by(|a, b| a.value().total_cmp(&b.value()))
        .ok_or("last"));
    close(r.riders[0].busy_seconds.value(), last.value());
    assert_eq!(
        r.events
            .iter()
            .filter(|e| matches!(
                e.event,
                SimulationEvent::FleetPlanned {
                    committed: true,
                    ..
                }
            ))
            .count(),
        1
    );
}
#[test]
fn frozen_travel_wait_service_and_existing_acceptance_survive_joint_rewrite() {
    for creation in [1.0, 15.0, 26.0] {
        let mut s = pooled();
        s.orders[1].created_at_seconds = seconds(creation);
        let r = ok(simulate(&graph(), &s));
        assert_eq!(r.summary.delivered_orders, 2);
        close(ok(r.orders[0].picked_up_at.ok_or("pickup")).value(), 27.0);
        let first = ok(r.fleets[0].decision.proposal().ok_or("first"));
        let accepted = first.accepted().clone();
        let state = ok(r.final_state.as_ref().ok_or("state"));
        for (o, a) in accepted {
            assert_eq!(state.accepted[&o], a);
        }
        assert!(
            r.fleets[1]
                .decision
                .evidence()
                .context
                .pooling
                .inputs
                .projections
                .values()
                .any(|p| p.frozen.is_some())
        );
        assert_eq!(r.events.iter().filter(|e|matches!(e.event,SimulationEvent::OrderPickedUp {order,..} if order.value()==1)).count(),1);
    }
}
#[test]
fn unavailable_order_isolated_then_observation_allows_new_admission() {
    let mut s = pooled();
    s.orders[0].expected_ready_at_seconds = None;
    s.orders[0].actual_readiness = ActualReadiness::Fixed {
        at_seconds: seconds(40.0),
    };
    let r = ok(simulate(&graph(), &s));
    assert_eq!(r.summary.delivered_orders, 2);
    let first = &r.fleets[0];
    assert!(!first.decision.evidence().input_complete);
    assert!(first.committed);
    assert_eq!(
        ok(first.decision.proposal().ok_or("p")).assignments().len(),
        1
    );
    close(
        ok(r.orders[0].assigned_at.ok_or("assignment")).value(),
        40.0,
    );
}
#[test]
fn unknown_active_prefix_is_preserved_while_a_healthy_rider_admits_work() {
    let mut s = pooled();
    s.riders.push(roadrunner_simulation::RiderInput {
        id: 2,
        node: 3,
        capacity: 2,
        available: true,
    });
    s.orders[0].expected_ready_at_seconds = Some(seconds(15.0));
    s.orders[0].actual_readiness = ActualReadiness::Fixed {
        at_seconds: seconds(60.0),
    };
    s.orders[1].created_at_seconds = seconds(30.0);
    let r = ok(simulate(&graph(), &s));
    assert_eq!(r.summary.delivered_orders, 2);
    let d = &r.fleets[1].decision;
    assert!(!d.evidence().input_complete);
    assert!(
        d.evidence()
            .riders
            .iter()
            .any(|r| matches!(r.isolation, Some(FleetIsolation::PredictionUnavailable(_))))
    );
    assert_eq!(
        r.orders[1].rider.map(roadrunner_dispatch::RiderId::value),
        Some(2)
    );
    close(ok(r.orders[0].picked_up_at.ok_or("pickup")).value(), 62.0);
}
#[test]
fn budget_exhaustion_publishes_nothing_even_after_feasible_work() {
    let mut s = pooled();
    s.dispatch = DispatchPolicy::FleetBatch {
        work_budget: 2,
        forecast_validity_seconds: seconds(1000.0),
        algorithm: FleetAlgorithm::MultiStartLocal,
    };
    let r = ok(simulate(&graph(), &s));
    assert_eq!(r.summary.assigned_orders, 0);
    assert_eq!(r.summary.delivered_orders, 0);
    assert!(r.fleets.iter().all(|f| !f.committed
        && f.decision.proposal().is_none()
        && f.decision.evidence().termination == FleetTermination::SearchIncomplete));
    let state = ok(r.final_state.as_ref().ok_or("state"));
    assert!(state.accepted.is_empty());
    assert!(state.assignments.is_empty());
}
#[test]
fn generated_seeded_batch_replays_preserve_semantics_under_reversed_input() {
    for seed in 0..24 {
        let mut s = pooled();
        s.seed = seed;
        for o in &mut s.orders {
            o.actual_readiness = ActualReadiness::SeededDelay {
                min_seconds: seconds(20.0),
                max_seconds: seconds(40.0),
            };
        }
        let first = ok(simulate(&graph(), &s));
        let replay = ok(simulate(&graph(), &s));
        assert_eq!(first, replay);
        s.orders.reverse();
        s.riders.reverse();
        let reversed = ok(simulate(&graph(), &s));
        assert_eq!(first, reversed);
        assert_eq!(
            ok(serde_json::to_vec(&first)),
            ok(serde_json::to_vec(&replay))
        );
        assert_eq!(first.summary.delivered_orders, 2);
        ok(validate_world(ok(first
            .final_state
            .as_ref()
            .ok_or("state"))));
    }
}
