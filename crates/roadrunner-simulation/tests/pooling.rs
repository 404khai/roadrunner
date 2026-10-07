//! Pooled execution against actual readiness, frozen actions and responsibility intervals.
#![allow(clippy::float_cmp)]
mod support;
use roadrunner_dispatch::{
    AdmissionDeadline, FulfillmentState, InsertionTermination, OrderId, OrderPolicy, ReadinessRule,
    Stop,
};
use roadrunner_simulation::{ActualReadiness, DispatchPolicy, SimulationEvent, simulate};
use support::{close, graph, ok, order, scenario, seconds};

fn pooled() -> roadrunner_simulation::SimulationScenario {
    let mut s = scenario();
    s.schema_version = 2;
    s.scenario_id = Some("pooling-tests/v1".into());
    s.end_seconds = seconds(200.0);
    s.dispatch = DispatchPolicy::MultiOrder {
        work_budget: 1000,
        forecast_validity_seconds: seconds(1000.0),
    };
    s.orders = vec![order(1, 0.0, 25.0), order(2, 1.0, 25.0)];
    for o in &mut s.orders {
        o.admission = Some(OrderPolicy {
            id: "pooling-test/v1".into(),
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
fn successful_pooling_reads_current_plan_and_releases_one_order_at_a_time() {
    let g = graph();
    let s = pooled();
    let result = ok(simulate(&g, &s));
    let replay = ok(simulate(&g, &s));
    assert_eq!(result, replay);
    assert_eq!(result.summary.delivered_orders, 2);
    assert_eq!(result.insertions.len(), 2);
    assert!(result.insertions.iter().all(|i| i.committed));
    let second = ok(result.insertions[1]
        .decision
        .proposal()
        .ok_or("second insertion"));
    assert_eq!(
        second.evaluation().plan.stops,
        vec![
            Stop::Pickup(OrderId::new(1)),
            Stop::Pickup(OrderId::new(2)),
            Stop::Dropoff(OrderId::new(1)),
            Stop::Dropoff(OrderId::new(2))
        ]
    );
    assert_eq!(second.evaluation().stops[0].load.value(), 2);
    assert!(result.orders[1].assigned_at < result.orders[0].picked_up_at);
    close(
        ok(result.orders[0].picked_up_at.ok_or("pickup")).value(),
        27.0,
    );
    close(
        ok(result.orders[1].picked_up_at.ok_or("pickup")).value(),
        29.0,
    );
    close(
        ok(result.orders[0].delivered_at.ok_or("delivery")).value(),
        62.0,
    );
    close(
        ok(result.orders[1].delivered_at.ok_or("delivery")).value(),
        65.0,
    );
    close(result.riders[0].busy_seconds.value(), 65.0); // union, never summed overlapping responsibility
    let final_state = ok(result.final_state.as_ref().ok_or("final domain"));
    assert!(final_state.assignments.is_empty());
    assert!(final_state.plans.values().all(|p| p.stops.is_empty()));
    assert_eq!(final_state.accepted.len(), 2);
    assert!(
        final_state
            .fulfillment
            .values()
            .all(|f| matches!(f, FulfillmentState::Delivered { .. }))
    );
    let first_leg = result.events.iter().find_map(|e| match &e.event {
        SimulationEvent::RiderMoved { leg, .. } => Some(leg),
        _ => None,
    });
    close(ok(first_leg.ok_or("leg")).travel.value(), 10.0);
}

#[test]
fn frozen_travel_wait_and_service_survive_rewrites() {
    for arrival_time in [1.0, 15.0, 26.0] {
        let g = graph();
        let mut s = pooled();
        s.orders[1].created_at_seconds = seconds(arrival_time);
        let result = ok(simulate(&g, &s));
        assert_eq!(result.summary.delivered_orders, 2);
        close(
            ok(result.orders[0].picked_up_at.ok_or("pickup")).value(),
            27.0,
        );
        let context = &result.insertions[1].decision.evidence().context;
        let projection = ok(context
            .inputs
            .projections
            .values()
            .find(|p| p.frozen.is_some())
            .ok_or("frozen projection"));
        let frozen = ok(projection.frozen.as_ref().ok_or("frozen"));
        assert_eq!(frozen.execution_id, 0);
        assert_eq!(frozen.timeline.stop, Stop::Pickup(OrderId::new(1)));
        close(frozen.timeline.departure.value(), 27.0);
        let actual_pickups=result.events.iter().filter(|e|matches!(e.event,SimulationEvent::OrderPickedUp{order,..} if order==OrderId::new(1))).count();
        assert_eq!(actual_pickups, 1);
    }
}

#[test]
fn hard_deadline_capacity_and_budget_rejections_are_distinct_and_replay() {
    let g = graph();
    let mut s = pooled();
    s.orders[1].deadline_seconds = Some(seconds(20.0));
    ok(s.orders[1].admission.as_mut().ok_or("policy")).deadline = AdmissionDeadline::Hard;
    let result = ok(simulate(&g, &s));
    assert_eq!(result.summary.delivered_orders, 1);
    assert!(result.insertions.iter().skip(1).all(|i| !i.committed));
    assert!(result.insertions.iter().skip(1).all(|i|i.decision.evidence().termination==InsertionTermination::NoFeasibleInsertion));
    let state = ok(result.final_state.as_ref().ok_or("domain"));
    assert!(!state.accepted.contains_key(&OrderId::new(2)));
    assert!(!state.assignments.contains_key(&OrderId::new(2)));
    let mut s = pooled();
    s.dispatch = DispatchPolicy::MultiOrder {
        work_budget: 1,
        forecast_validity_seconds: seconds(1000.0),
    };
    let result = ok(simulate(&g, &s));
    assert!(
        result
            .insertions
            .iter()
            .any(|i| i.decision.evidence().termination == InsertionTermination::SearchIncomplete)
    );
    assert!(
        result
            .insertions
            .iter()
            .filter(|i| i.decision.evidence().termination == InsertionTermination::SearchIncomplete)
            .all(|i| !i.committed)
    );
    let mut s = pooled();
    s.orders[1].demand = 3;
    let result = ok(simulate(&g, &s));
    assert_eq!(result.summary.delivered_orders, 1);
    assert!(result.insertions.iter().skip(1).all(|i| !i.committed));
}

#[test]
fn admission_valid_prediction_can_have_realized_readiness_violation() {
    let g = graph();
    let mut s = pooled();
    s.orders.truncate(1);
    s.orders[0].deadline_seconds = Some(seconds(70.0));
    ok(s.orders[0].admission.as_mut().ok_or("policy")).deadline = AdmissionDeadline::Hard;
    s.orders[0].actual_readiness = ActualReadiness::Fixed {
        at_seconds: seconds(80.0),
    };
    let result = ok(simulate(&g, &s));
    assert!(result.insertions[0].committed);
    assert!(
        ok(result.insertions[0].decision.proposal().ok_or("proposal"))
            .evaluation()
            .completions[&OrderId::new(1)]
            .value()
            < 70.0
    );
    assert_eq!(result.summary.late_deliveries, 1);
    assert_eq!(
        result.realized_protections[0].hard_deadline_missed,
        Some(true)
    );
    close(
        ok(result.orders[0].delivered_at.ok_or("delivery")).value(),
        115.0,
    );
}

#[test]
fn forecast_failures_are_typed_and_legacy_fixtures_remain_versioned() {
    let g = graph();
    let mut s = pooled();
    s.orders[0].expected_ready_at_seconds = None;
    let result = ok(simulate(&g, &s));
    assert_ne!(
        result.prediction_failures,
        [] as [roadrunner_simulation::SimulationPredictionFailure; 0]
    );
    assert!(
        result
            .prediction_failures
            .iter()
            .all(|f| !f.coverage.input_complete && !f.committed)
    );
    let mut s = pooled();
    s.schema_version = 1;
    assert!(simulate(&g, &s).is_err());
    let mut s = pooled();
    ok(s.orders[0].admission.as_mut().ok_or("policy")).version = 99;
    assert!(simulate(&g, &s).is_err());
}

#[test]
fn seeded_replays_and_canonical_external_collection_order() {
    let g = graph();
    for seed in 0..24 {
        let mut s = pooled();
        s.seed = seed;
        for o in &mut s.orders {
            let policy = ok(o.admission.as_mut().ok_or("policy"));
            policy.pickup_service = seconds(0.1);
            policy.dropoff_service = seconds(0.2);
            o.actual_readiness = ActualReadiness::SeededDelay {
                min_seconds: seconds(20.0),
                max_seconds: seconds(30.0),
            };
        }
        let a = ok(simulate(&g, &s));
        s.orders.reverse();
        s.riders.reverse();
        let b = ok(simulate(&g, &s));
        assert_eq!(a, b);
        assert_eq!(a.summary.delivered_orders, 2);
        assert!(a.riders[0].busy_seconds.value() <= 200.0);
    }
}

#[test]
fn unavailable_new_forecast_blocks_admission_without_stopping_committed_execution() {
    let g = graph();
    let mut s = pooled();
    s.orders[1].expected_ready_at_seconds = None;
    s.orders[1].actual_readiness = ActualReadiness::Fixed {
        at_seconds: seconds(500.0),
    };
    let result = ok(simulate(&g, &s));
    assert_eq!(result.summary.delivered_orders, 1);
    assert!(
        result
            .prediction_failures
            .iter()
            .all(|f| f.order == OrderId::new(2) && !f.committed)
    );
    close(
        ok(result.orders[0].picked_up_at.ok_or("pickup")).value(),
        27.0,
    );
    close(
        ok(result.orders[0].delivered_at.ok_or("delivery")).value(),
        60.0,
    );
    assert!(result.orders[1].assigned_at.is_none());
    assert!(
        !ok(result.final_state.as_ref().ok_or("domain"))
            .accepted
            .contains_key(&OrderId::new(2))
    );
}
