//! Dynamic triggers, committed recovery, frozen execution and terminal populations.
mod support;
use roadrunner_dispatch::{
    AdmissionDeadline, Availability, FleetAlgorithm, FulfillmentState, OrderId, OrderPolicy,
    ReadinessRule, RecoveryPolicy, RecoveryTermination, RiderId, validate_world,
};
use roadrunner_simulation::{
    DispatchPolicy, DynamicChange, DynamicEvent, RiderInput, SimulationEvent, simulate,
};
use support::{close, graph, ok, order, scenario, seconds};
fn dynamic() -> roadrunner_simulation::SimulationScenario {
    let mut s = scenario();
    s.schema_version = 4;
    s.scenario_id = Some("dynamic-test/v1".into());
    s.end_seconds = seconds(200.0);
    s.dispatch = DispatchPolicy::Dynamic {
        work_budget: 100_000,
        forecast_validity_seconds: seconds(1000.0),
        algorithm: FleetAlgorithm::MultiStartLocal,
        recovery: RecoveryPolicy {
            version: 1,
            reroute_penalty: seconds(1.0),
            assignment_stability_penalty: seconds(2.0),
            minimum_improvement: seconds(1.0),
            cooldown: seconds(10.0),
        },
    };
    s.orders = vec![order(1, 0.0, 25.0), order(2, 0.0, 25.0)];
    for o in &mut s.orders {
        o.admission = Some(OrderPolicy {
            id: "dynamic-order/v1".into(),
            version: 1,
            deadline: AdmissionDeadline::SoftObserved,
            max_completion_delay: None,
            pickup_service: seconds(2.0),
            dropoff_service: seconds(3.0),
            readiness: ReadinessRule::ValidForecastV1,
        });
    }
    s.riders.push(RiderInput {
        id: 2,
        node: 0,
        capacity: 2,
        available: false,
    });
    s
}
fn event(at: f64, change: DynamicChange) -> DynamicEvent {
    DynamicEvent {
        at_seconds: seconds(at),
        change,
    }
}
#[test]
fn offline_recovery_moves_only_unstarted_work_and_both_execute() {
    let mut s = dynamic();
    s.dynamic_events = vec![
        event(
            1.0,
            DynamicChange::Availability {
                rider: 2,
                available: true,
            },
        ),
        event(
            1.0,
            DynamicChange::Availability {
                rider: 1,
                available: false,
            },
        ),
    ];
    let r = ok(simulate(&graph(), &s));
    assert_eq!(r.summary.delivered_orders, 2);
    assert_eq!(r.orders[0].rider, Some(RiderId::new(1)));
    assert_eq!(r.orders[1].rider, Some(RiderId::new(2)));
    let repair = r
        .recoveries
        .iter()
        .find(|r| r.committed)
        .unwrap_or_else(|| panic!("repair"));
    assert!(
        repair
            .decision
            .as_ref()
            .unwrap_or_else(|| panic!("decision"))
            .evidence()
            .baseline_requires_repair
    );
    let initial = r.fleets[0]
        .decision
        .proposal()
        .unwrap_or_else(|| panic!("admission"));
    let state = r.final_state.as_ref().unwrap_or_else(|| panic!("state"));
    assert_eq!(state.accepted, initial.accepted().clone());
    ok(validate_world(state));
    close(
        r.orders[0]
            .picked_up_at
            .unwrap_or_else(|| panic!("pickup"))
            .value(),
        27.0,
    );
    // Two responsibility intervals overlap but are measured once per rider.
    close(
        r.riders[0].busy_seconds.value(),
        r.orders[0]
            .delivered_at
            .unwrap_or_else(|| panic!("delivery"))
            .value(),
    );
    close(
        r.riders[1].busy_seconds.value(),
        r.orders[1]
            .delivered_at
            .unwrap_or_else(|| panic!("delivery"))
            .value()
            - 1.0,
    );
    assert_eq!(r, ok(simulate(&graph(), &s)));
}
#[test]
fn cancellation_refuses_frozen_and_custody_but_removes_unstarted_work() {
    let mut s = dynamic();
    s.dynamic_events = vec![
        event(1.0, DynamicChange::Cancel { order: 1 }),
        event(2.0, DynamicChange::Cancel { order: 2 }),
        event(28.0, DynamicChange::Cancel { order: 1 }),
    ];
    let r = ok(simulate(&graph(), &s));
    assert_eq!(r.summary.delivered_orders, 1);
    assert_eq!(r.summary.cancelled_orders, 1);
    assert_eq!(r.summary.outstanding_orders, 0);
    assert_eq!(r.summary.assigned_unfinished_orders, 0);
    let reasons: Vec<_> = r
        .events
        .iter()
        .filter_map(|e| match &e.event {
            SimulationEvent::DynamicChanged { reason, .. } => Some(reason.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(reasons, ["FrozenExecution", "Applied", "NotAwaitingPickup"]);
    let state = r.final_state.as_ref().unwrap_or_else(|| panic!("state"));
    assert!(matches!(
        state.fulfillment[&OrderId::new(2)],
        FulfillmentState::Cancelled { .. }
    ));
    assert_eq!(state.accepted.len(), 2);
    ok(validate_world(state));
}
#[test]
fn observed_road_delay_supersedes_old_arrival_without_double_movement() {
    let mut s = dynamic();
    s.orders.truncate(1);
    s.orders[0].actual_readiness = roadrunner_simulation::ActualReadiness::Fixed {
        at_seconds: seconds(0.0),
    };
    s.orders[0].expected_ready_at_seconds = Some(seconds(0.0));
    s.dynamic_events = vec![event(
        1.0,
        DynamicChange::RoadDelay {
            rider: 1,
            additional_seconds: seconds(20.0),
        },
    )];
    let r = ok(simulate(&graph(), &s));
    close(
        r.orders[0]
            .pickup_arrival
            .unwrap_or_else(|| panic!("arrival"))
            .value(),
        30.0,
    );
    assert_eq!(
        r.events
            .iter()
            .filter(|e| matches!(e.event, SimulationEvent::RiderMoved { .. }))
            .count(),
        2
    );
    assert_eq!(r.summary.delivered_orders, 1);
    assert_eq!(r, ok(simulate(&graph(), &s)));
}
#[test]
fn unknown_readiness_recovery_fails_but_preserves_execution() {
    let mut s = dynamic();
    s.orders.truncate(1);
    s.orders[0].actual_readiness = roadrunner_simulation::ActualReadiness::Fixed {
        at_seconds: seconds(60.0),
    };
    s.dynamic_events = vec![event(
        30.0,
        DynamicChange::Availability {
            rider: 2,
            available: true,
        },
    )];
    let r = ok(simulate(&graph(), &s));
    assert_eq!(r.summary.delivered_orders, 1);
    assert!(r.recoveries.iter().any(|r| {
        r.failure
            .as_ref()
            .is_some_and(|e| e.contains("PredictionUnavailable"))
    }));
    close(
        r.orders[0]
            .picked_up_at
            .unwrap_or_else(|| panic!("pickup"))
            .value(),
        62.0,
    );
}
#[test]
fn no_recovery_is_scoped_and_does_not_implicitly_abandon_offline_obligations() {
    let mut s = dynamic();
    s.dynamic_events = vec![event(
        1.0,
        DynamicChange::Availability {
            rider: 1,
            available: false,
        },
    )];
    let r = ok(simulate(&graph(), &s));
    assert!(r.recoveries.iter().any(|r| {
        r.decision
            .as_ref()
            .is_some_and(|d| d.evidence().termination == RecoveryTermination::NoRecovery)
    }));
    assert_eq!(r.summary.delivered_orders, 2);
    let state = r.final_state.as_ref().unwrap_or_else(|| panic!("state"));
    assert_eq!(
        state.riders[&RiderId::new(1)].availability,
        Availability::Unavailable
    );
}
#[test]
fn forecast_update_is_fresh_and_never_overwrites_actual_observation() {
    let mut s = dynamic();
    s.orders.truncate(1);
    s.dynamic_events = vec![event(
        15.0,
        DynamicChange::Forecast {
            order: 1,
            expected_at_seconds: seconds(100.0),
        },
    )];
    let r = ok(simulate(&graph(), &s));
    close(
        r.orders[0]
            .picked_up_at
            .unwrap_or_else(|| panic!("pickup"))
            .value(),
        27.0,
    );
    let state = r.final_state.as_ref().unwrap_or_else(|| panic!("state"));
    close(
        state.readiness[&OrderId::new(1)]
            .observed_at
            .unwrap_or_else(|| panic!("actual"))
            .value(),
        25.0,
    );
    assert!(
        r.recoveries
            .iter()
            .filter_map(|r| r.decision.as_ref())
            .any(
                |d| (d.evidence().context.pooling.inputs.forecasts[&OrderId::new(1)]
                    .generated_at
                    .value()
                    - 15.0)
                    .abs()
                    < 1e-9
            )
    );
}

#[test]
fn cancellation_before_assignment_is_neither_unassigned_nor_delivered() {
    let mut s = dynamic();
    s.orders.truncate(1);
    s.dynamic_events = vec![event(0.0, DynamicChange::Cancel { order: 1 })];
    let r = ok(simulate(&graph(), &s));
    assert_eq!(r.summary.assigned_orders, 0);
    assert_eq!(r.summary.delivered_orders, 0);
    assert_eq!(r.summary.unassigned_orders, 0);
    assert_eq!(r.summary.cancelled_orders, 1);
    assert_eq!(r.summary.outstanding_orders, 0);
    assert!(
        r.final_state
            .as_ref()
            .unwrap_or_else(|| panic!("state"))
            .accepted
            .is_empty()
    );
}
