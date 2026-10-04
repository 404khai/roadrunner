//! Deterministic event ordering, shared transitions, actual execution, and metric populations.
#![allow(clippy::float_cmp)]
mod support;

use roadrunner_core::cost::TrafficMultiplier;
use roadrunner_core::graph::NodeId;
use roadrunner_dispatch::{CandidateResult, DispatchDecisionOutcome, RiderId};
use roadrunner_simulation::{
    ActualReadiness, DispatchPolicy, RiderInput, SimulationError, SimulationEvent, TrafficChange,
    TrafficOverride, simulate,
};
use support::{close, graph, ok, order, scenario, seconds};

#[test]
fn lifecycle_metrics_and_event_order_are_derived_from_actual_execution() {
    let result = ok(simulate(&graph(), &scenario()));
    assert_eq!(result.summary.created_orders, 1);
    assert_eq!(result.summary.assigned_orders, 1);
    assert_eq!(result.summary.delivered_orders, 1);
    assert_eq!(result.summary.outstanding_orders, 0);
    close(
        ok(result
            .summary
            .delivered_duration
            .median_seconds
            .ok_or("median"))
        .value(),
        55.0,
    );
    close(
        ok(result.summary.pickup_waiting.mean_seconds.ok_or("waiting")).value(),
        15.0,
    );
    close(
        ok(result.summary.rider_utilization.ok_or("utilization")),
        0.55,
    );
    close(
        ok(result.summary.mean_rider_idle_seconds.ok_or("idle")).value(),
        45.0,
    );
    assert_eq!(result.events.len(), 8);
    let times: Vec<_> = result.events.iter().map(|e| e.at.value()).collect();
    assert_eq!(times, vec![0.0, 0.0, 10.0, 10.0, 25.0, 25.0, 55.0, 55.0]);
    assert!(matches!(
        result.events[0].event,
        SimulationEvent::OrderCreated { .. }
    ));
    assert!(matches!(
        result.events[1].event,
        SimulationEvent::RiderAssigned { .. }
    ));
    assert!(matches!(
        result.events[2].event,
        SimulationEvent::RiderMoved { .. }
    ));
    assert!(matches!(
        result.events[3].event,
        SimulationEvent::RiderArrivedPickup { .. }
    ));
    assert!(matches!(
        result.events[4].event,
        SimulationEvent::OrderReady { .. }
    ));
    assert!(matches!(
        result.events[5].event,
        SimulationEvent::OrderPickedUp { .. }
    ));
    assert!(matches!(
        result.events[7].event,
        SimulationEvent::OrderDelivered { .. }
    ));
    let completed_distance: f64 = result
        .events
        .iter()
        .filter_map(|e| match &e.event {
            SimulationEvent::RiderMoved { leg, .. } => Some(leg.distance.value()),
            _ => None,
        })
        .sum();
    close(
        result.summary.completed_distance_meters.value(),
        completed_distance,
    );
    for pair in result.events.windows(2) {
        assert!(
            pair[0].at < pair[1].at
                || (pair[0].at == pair[1].at && pair[0].sequence < pair[1].sequence)
        );
    }
}

#[test]
fn baseline_forecast_never_overrides_actual_readiness_and_rider_is_reused() {
    let mut s = scenario();
    s.dispatch = DispatchPolicy::Basic;
    s.orders.push(order(2, 5.0, 5.0));
    s.end_seconds = seconds(200.0);
    let result = ok(simulate(&graph(), &s));
    assert_eq!(result.summary.delivered_orders, 2);
    close(
        ok(result.orders[0].predicted_eta.ok_or("forecast")).value(),
        40.0,
    );
    close(
        ok(result.orders[0].delivered_at.ok_or("delivery")).value(),
        55.0,
    );
    close(
        ok(result.orders[1].assigned_at.ok_or("assignment")).value(),
        55.0,
    );
    close(
        ok(result.orders[1].pickup_arrival.ok_or("arrival")).value(),
        85.0,
    );
    close(
        ok(result.orders[1].delivered_at.ok_or("delivery")).value(),
        115.0,
    );
    close(
        ok(result
            .summary
            .delivered_duration
            .median_seconds
            .ok_or("median"))
        .value(),
        82.5,
    );
    close(
        ok(result.summary.delivered_duration.p95_seconds.ok_or("p95")).value(),
        110.0,
    );
    assert!(
        result
            .decisions
            .iter()
            .any(|d| matches!(d.outcome(), DispatchDecisionOutcome::Unassigned { .. }))
    );
    assert!(
        result
            .events
            .iter()
            .any(|e| matches!(e.event, SimulationEvent::DispatchUnassigned { .. }))
    );
    let assigned_riders: Vec<_> = result.orders.iter().map(|o| o.rider).collect();
    assert_eq!(
        assigned_riders,
        vec![Some(RiderId::new(1)), Some(RiderId::new(1))]
    );
}

#[test]
fn horizon_reports_unfinished_uncreated_and_late_work_without_biased_populations() {
    let mut s = scenario();
    s.orders[0].deadline_seconds = Some(seconds(50.0));
    let mut second = order(2, 5.0, 5.0);
    second.deadline_seconds = Some(seconds(90.0));
    s.orders.extend([second, order(3, 120.0, 120.0)]);
    let result = ok(simulate(&graph(), &s));
    let m = &result.summary;
    assert_eq!(
        (m.scheduled_orders, m.created_orders, m.uncreated_orders),
        (3, 2, 1)
    );
    assert_eq!(
        (
            m.assigned_orders,
            m.delivered_orders,
            m.assigned_unfinished_orders
        ),
        (2, 1, 1)
    );
    assert_eq!(
        (
            m.outstanding_orders,
            m.late_deliveries,
            m.outstanding_past_deadline
        ),
        (1, 1, 1)
    );
    assert_eq!(m.delivered_duration.samples, 1);
    assert_eq!(m.predicted_eta.samples, 2);
    assert_eq!(m.pickup_waiting.samples, 2);
    close(
        ok(m.pickup_waiting.mean_seconds.ok_or("wait mean")).value(),
        7.5,
    );
    close(ok(m.rider_utilization.ok_or("busy")), 1.0);
    assert!(result.orders[1].delivered_at.is_none());
    assert!(result.orders[2].created_at.is_none());
    assert_eq!(result.future_events, 3);
    assert!(result.events.iter().all(|e| e.at.value() <= 100.0));
    // The unfinished final delivery leg contributes no fabricated partial distance.
    let moved = result
        .events
        .iter()
        .filter(|e| matches!(e.event, SimulationEvent::RiderMoved { .. }))
        .count();
    assert_eq!(moved, 3);
}

#[test]
fn waiting_at_horizon_is_reported_separately_from_completed_pickup_samples() {
    let mut s = scenario();
    s.end_seconds = seconds(20.0);
    let result = ok(simulate(&graph(), &s));
    assert_eq!(result.summary.delivered_orders, 0);
    assert_eq!(result.summary.assigned_unfinished_orders, 1);
    assert_eq!(result.summary.pickup_waiting.samples, 0);
    assert!(result.summary.pickup_waiting.mean_seconds.is_none());
    close(
        ok(result.orders[0].partial_wait_seconds.ok_or("partial wait")).value(),
        10.0,
    );
    close(
        ok(result.summary.rider_utilization.ok_or("utilization")),
        1.0,
    );
}

#[test]
fn traffic_changes_freeze_inflight_legs_and_apply_at_next_departure() {
    let g = graph();
    let mut s = scenario();
    let edge = ok(g.outgoing_edges(NodeId::new(0)))
        .iter()
        .find(|e| e.to() == NodeId::new(3))
        .map(|e| e.id().value());
    s.traffic_changes = vec![
        TrafficChange {
            at_seconds: seconds(5.0),
            overrides: vec![TrafficOverride {
                edge_id: 3,
                multiplier: ok(TrafficMultiplier::new(10.0)),
            }],
        },
        TrafficChange {
            at_seconds: seconds(12.0),
            overrides: vec![TrafficOverride {
                edge_id: ok(edge.ok_or("edge")),
                multiplier: ok(TrafficMultiplier::new(2.0)),
            }],
        },
        TrafficChange {
            at_seconds: seconds(40.0),
            overrides: vec![],
        },
    ];
    let result = ok(simulate(&g, &s));
    close(
        ok(result.orders[0].pickup_arrival.ok_or("arrival")).value(),
        10.0,
    );
    close(
        ok(result.orders[0].predicted_eta.ok_or("forecast")).value(),
        55.0,
    );
    close(
        ok(result.orders[0].delivered_at.ok_or("delivery")).value(),
        85.0,
    );
    let legs: Vec<_> = result
        .events
        .iter()
        .filter_map(|e| match &e.event {
            SimulationEvent::RiderMoved { leg, .. } => Some(leg),
            _ => None,
        })
        .collect();
    close(legs[0].travel.value(), 10.0);
    close(legs[1].travel.value(), 60.0);
    close(legs[1].departed_at.value(), 25.0);
    assert_ne!(legs[0].routing.traffic, legs[1].routing.traffic);
    assert_eq!(
        result
            .events
            .iter()
            .filter(|e| matches!(e.event, SimulationEvent::TrafficChanged { .. }))
            .count(),
        3
    );
}

#[test]
fn seed_replays_exactly_and_is_materialized_independently_of_policy_and_input_order() {
    let g = graph();
    let mut s = scenario();
    s.end_seconds = seconds(300.0);
    s.orders.push(order(2, 1.0, 20.0));
    for order in &mut s.orders {
        order.actual_readiness = ActualReadiness::SeededDelay {
            min_seconds: seconds(10.0),
            max_seconds: seconds(30.0),
        };
    }
    let first = ok(simulate(&g, &s));
    let second = ok(simulate(&g, &s));
    assert_eq!(first, second);
    assert_eq!(
        ok(serde_json::to_vec(&first)),
        ok(serde_json::to_vec(&second))
    );
    s.orders.reverse();
    assert_eq!(first, ok(simulate(&g, &s)));
    s.dispatch = DispatchPolicy::Basic;
    let baseline = ok(simulate(&g, &s));
    assert_eq!(
        first
            .orders
            .iter()
            .map(|o| o.realized_ready_at)
            .collect::<Vec<_>>(),
        baseline
            .orders
            .iter()
            .map(|o| o.realized_ready_at)
            .collect::<Vec<_>>()
    );
    s.seed = 16;
    let changed = ok(simulate(&g, &s));
    assert_ne!(
        baseline.orders[0].realized_ready_at,
        changed.orders[0].realized_ready_at
    );
}

#[test]
fn simultaneous_creations_serialize_commits_and_do_not_double_assign_a_rider() {
    let g = graph();
    let mut s = scenario();
    s.end_seconds = seconds(300.0);
    s.orders = vec![order(2, 0.0, 0.0), order(1, 0.0, 0.0)];
    let result = ok(simulate(&g, &s));
    assert_eq!(result.summary.delivered_orders, 2);
    close(
        ok(result.orders[0].assigned_at.ok_or("first assignment")).value(),
        0.0,
    );
    close(
        ok(result.orders[1].assigned_at.ok_or("second assignment")).value(),
        40.0,
    );
    let creations: Vec<_> = result
        .events
        .iter()
        .filter_map(|e| match e.event {
            SimulationEvent::OrderCreated { order } => Some(order.value()),
            _ => None,
        })
        .collect();
    assert_eq!(creations, vec![1, 2]);
}

#[test]
fn unavailable_empty_and_capacity_limited_fleets_report_valid_unassigned_results() {
    let g = graph();
    for mode in 0..3 {
        let mut s = scenario();
        match mode {
            0 => s.riders.clear(),
            1 => s.riders[0].available = false,
            _ => s.riders[0].capacity = 0,
        }
        let result = ok(simulate(&g, &s));
        assert_eq!(result.summary.unassigned_orders, 1);
        assert_eq!(result.summary.delivered_duration.samples, 0);
        assert!(result.summary.delivered_duration.median_seconds.is_none());
        assert_eq!(result.summary.completed_distance_meters.value(), 0.0);
        if mode < 2 {
            assert!(result.summary.rider_utilization.is_none());
        } else {
            assert_eq!(result.summary.rider_utilization, Some(0.0));
        }
    }
}

#[test]
fn preparation_strategy_reuses_far_rider_preference_and_zero_wait_baseline() {
    let g = graph();
    let mut s = scenario();
    s.riders.push(RiderInput {
        id: 2,
        node: 2,
        capacity: 1,
        available: true,
    });
    let aware = ok(simulate(&g, &s));
    assert_eq!(aware.orders[0].rider, Some(RiderId::new(2)));
    close(
        ok(aware.summary.pickup_waiting.mean_seconds.ok_or("wait")).value(),
        5.0,
    );
    s.dispatch = DispatchPolicy::Basic;
    let baseline = ok(simulate(&g, &s));
    assert_eq!(baseline.orders[0].rider, Some(RiderId::new(1)));
    close(
        ok(baseline.summary.pickup_waiting.mean_seconds.ok_or("wait")).value(),
        15.0,
    );
    assert_eq!(
        aware.orders[0].delivered_at,
        baseline.orders[0].delivered_at
    );
}

#[test]
fn already_ready_stock_nonzero_epoch_and_zero_time_events_are_supported() {
    let g = graph();
    let mut s = scenario();
    s.start_seconds = seconds(1000.0);
    s.end_seconds = seconds(1100.0);
    s.routing_epoch_seconds = seconds(900.0);
    s.orders = vec![order(1, 1000.0, 990.0)];
    s.orders[0].expected_ready_at_seconds = None;
    let result = ok(simulate(&g, &s));
    close(
        ok(result.orders[0].observed_ready_at.ok_or("ready")).value(),
        990.0,
    );
    close(
        ok(result.orders[0].delivered_at.ok_or("delivery")).value(),
        1040.0,
    );
    assert_eq!(
        result.decisions[0].evidence().routing_epoch.0.value(),
        900.0
    );
    s = scenario();
    s.end_seconds = seconds(0.0);
    s.riders[0].node = 0;
    s.orders = vec![order(1, 0.0, 0.0)];
    s.orders[0].dropoff_node = 0;
    let zero = ok(simulate(&g, &s));
    assert_eq!(zero.summary.delivered_orders, 1);
    assert_eq!(
        zero.summary.delivered_duration.median_seconds,
        Some(seconds(0.0))
    );
    assert!(zero.summary.rider_utilization.is_none());
}

#[test]
fn disconnected_orders_are_infeasible_not_engine_failures() {
    let mut s = scenario();
    s.orders[0].dropoff_node = 4;
    let result = ok(simulate(&graph(), &s));
    assert_eq!(result.summary.unassigned_orders, 1);
    assert!(result.decisions.iter().all(|d| {
        d.evidence()
            .candidates
            .iter()
            .all(|c| matches!(c.result, CandidateResult::Rejected(_)))
    }));
}

#[test]
fn scenario_validation_rejects_invalid_schema_references_times_and_seed_bounds() {
    let g = graph();
    let mut variants = Vec::new();
    let mut s = scenario();
    s.schema_version = 2;
    variants.push(s);
    let mut s = scenario();
    s.riders.push(s.riders[0].clone());
    variants.push(s);
    let mut s = scenario();
    s.orders.push(s.orders[0].clone());
    variants.push(s);
    let mut s = scenario();
    s.orders[0].pickup_node = 99;
    variants.push(s);
    let mut s = scenario();
    s.riders[0].node = 99;
    variants.push(s);
    let mut s = scenario();
    s.start_seconds = seconds(2.0);
    variants.push(s);
    let mut s = scenario();
    s.start_seconds = seconds(101.0);
    variants.push(s);
    let mut s = scenario();
    s.routing_epoch_seconds = seconds(1.0);
    variants.push(s);
    let mut s = scenario();
    s.graph_snapshot_digest = Some("wrong".into());
    variants.push(s);
    let mut s = scenario();
    s.dispatch = DispatchPolicy::PreparationAware {
        idle_penalty_weight: f64::INFINITY,
    };
    variants.push(s);
    let mut s = scenario();
    s.orders[0].expected_ready_at_seconds = None;
    variants.push(s);
    let mut s = scenario();
    s.orders[0].actual_readiness = ActualReadiness::SeededDelay {
        min_seconds: seconds(20.0),
        max_seconds: seconds(10.0),
    };
    variants.push(s);
    let mut s = scenario();
    s.initial_traffic.push(TrafficOverride {
        edge_id: 99,
        multiplier: TrafficMultiplier::NORMAL,
    });
    variants.push(s);
    for input in variants {
        assert!(
            simulate(&g, &input).is_err(),
            "invalid fixture accepted: {input:?}"
        );
    }
    let mut s = scenario();
    s.traffic_changes.push(TrafficChange {
        at_seconds: seconds(101.0),
        overrides: vec![TrafficOverride {
            edge_id: 99,
            multiplier: TrafficMultiplier::NORMAL,
        }],
    });
    // Even traffic scheduled beyond the horizon is validated before execution.
    assert!(matches!(simulate(&g, &s), Err(SimulationError::Traffic(_))));
}

#[test]
fn dispatch_errors_are_not_relabelled_as_unassigned_and_inputs_remain_unchanged() {
    let g = graph();
    let mut s = scenario();
    s.orders[0].expected_ready_at_seconds = Some(seconds(f64::MAX));
    let before = s.clone();
    assert!(matches!(
        simulate(&g, &s),
        Err(SimulationError::Dispatch(_))
    ));
    assert_eq!(s, before);
    assert_eq!(g.node_count(), 5);
}

#[test]
fn empty_scenario_runs_full_window_and_events_at_horizon_are_inclusive() {
    let g = graph();
    let mut s = scenario();
    s.orders.clear();
    let empty = ok(simulate(&g, &s));
    assert_eq!(empty.summary.created_orders, 0);
    assert_eq!(empty.summary.predicted_eta.samples, 0);
    assert_eq!(empty.summary.rider_utilization, Some(0.0));
    assert_eq!(empty.summary.mean_rider_idle_seconds, Some(seconds(100.0)));
    assert_eq!(empty.events.len(), 0);
    s.orders = vec![order(1, 0.0, 25.0)];
    s.end_seconds = seconds(55.0);
    let inclusive = ok(simulate(&g, &s));
    assert_eq!(inclusive.summary.delivered_orders, 1);
    assert_eq!(inclusive.future_events, 0);
}

#[test]
fn simultaneous_traffic_replacements_preserve_input_order_before_assignments() {
    let g = graph();
    let mut s = scenario();
    let congestion = TrafficChange {
        at_seconds: seconds(0.0),
        overrides: vec![TrafficOverride {
            edge_id: 2,
            multiplier: ok(TrafficMultiplier::new(2.0)),
        }],
    };
    let normal = TrafficChange {
        at_seconds: seconds(0.0),
        overrides: vec![],
    };
    s.traffic_changes = vec![congestion.clone(), normal.clone()];
    let normal_last = ok(simulate(&g, &s));
    assert!(matches!(
        normal_last.events[0].event,
        SimulationEvent::TrafficChanged { .. }
    ));
    assert!(matches!(
        normal_last.events[1].event,
        SimulationEvent::TrafficChanged { .. }
    ));
    assert!(matches!(
        normal_last.events[2].event,
        SimulationEvent::OrderCreated { .. }
    ));
    close(
        ok(normal_last.orders[0].delivered_at.ok_or("delivery")).value(),
        55.0,
    );
    s.traffic_changes = vec![normal, congestion];
    let congestion_last = ok(simulate(&g, &s));
    close(
        ok(congestion_last.orders[0].delivered_at.ok_or("delivery")).value(),
        85.0,
    );
}
