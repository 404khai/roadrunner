//! Paired objectives, shared feasibility, reproducibility, and metric populations.
#![allow(clippy::float_cmp)]
mod support;

use roadrunner_core::cost::TrafficMultiplier;
use roadrunner_dispatch::{CandidateResult, RiderId};
use roadrunner_simulation::{
    DispatchPolicy, RiderInput, TrafficOverride, compare_strategies, simulate,
};
use support::{close, graph, ok, scenario, seconds};

fn two_riders() -> roadrunner_simulation::SimulationScenario {
    let mut input = scenario();
    input.riders.push(RiderInput {
        id: 2,
        node: 2,
        capacity: 2,
        available: true,
    });
    input
}

#[test]
fn completion_ties_and_wait_penalties_select_different_riders_with_explainable_scores() {
    let input = two_riders();
    let original = input.clone();
    let result = ok(compare_strategies(&graph(), &input, 1.0));
    assert_eq!(input, original);
    assert_eq!(result.runs.len(), 4);
    for (index, run) in result.runs.iter().enumerate() {
        assert_eq!(
            run.orders[0].rider,
            Some(RiderId::new(if index == 3 { 2 } else { 1 }))
        );
        close(
            ok(run.orders[0].delivered_at.ok_or("delivered")).value(),
            55.0,
        );
        close(
            ok(run.summary.delivered_duration.mean_seconds.ok_or("mean")).value(),
            55.0,
        );
        close(
            ok(run.summary.delivered_duration.p95_seconds.ok_or("p95")).value(),
            55.0,
        );
        close(
            ok(run.summary.pickup_waiting.mean_seconds.ok_or("waiting")).value(),
            if index == 3 { 5.0 } else { 15.0 },
        );
        assert_eq!(run.summary.delivered_orders, 1);
        assert_eq!(run.summary.late_deliveries, 0);
        assert_eq!(run.summary.outstanding_orders, 0);
        let mut standalone = input.clone();
        standalone.dispatch = run.dispatch;
        assert_eq!(*run, ok(simulate(&graph(), &standalone)));
    }
    let nearest = &result.runs[0].decisions[0];
    assert!(
        nearest
            .explanation()
            .contains("straight-line pickup distance")
    );
    let CandidateResult::Feasible { score, .. } = &nearest.evidence().candidates[0].result else {
        panic!("feasible")
    };
    close(
        ok(score.nearest_distance.ok_or("meters")).value(),
        111.195_080_233_532_92,
    );
    let completion = &result.runs[2].decisions[0];
    assert_eq!(completion.evidence().strategy, "lowest-completion-time/v1");
    assert!(
        completion
            .explanation()
            .contains("readiness-aware completion time")
    );
    assert_eq!(
        ok(completion.evidence().preparation.ok_or("readiness")).idle_penalty_weight,
        0.0
    );
    let CandidateResult::Feasible { score, .. } =
        &result.runs[3].decisions[0].evidence().candidates[1].result
    else {
        panic!("feasible")
    };
    close(score.total.value(), 60.0);
    close(
        ok(score.preparation.ok_or("breakdown"))
            .idle_penalty
            .value(),
        5.0,
    );
}

#[test]
fn nearest_distance_and_pickup_eta_diverge_under_traffic() {
    let mut input = two_riders();
    // Obtain the directed traversal from nearest rider node 1 to pickup node 0.
    let roads = graph();
    input.dispatch = DispatchPolicy::NearestRider;
    let baseline = ok(simulate(&roads, &input));
    let edge = baseline.events.iter().find_map(|event| match &event.event {
        roadrunner_simulation::SimulationEvent::RiderMoved { leg, .. } if leg.from.value() == 1 => {
            leg.edges.first().copied()
        }
        _ => None,
    });
    input.initial_traffic = vec![TrafficOverride {
        edge_id: ok(edge.ok_or("pickup traversal")).value(),
        multiplier: ok(TrafficMultiplier::new(4.0)),
    }];
    let result = ok(compare_strategies(&roads, &input, 1.0));
    assert_eq!(result.runs[0].orders[0].rider, Some(RiderId::new(1)));
    assert_eq!(result.runs[1].orders[0].rider, Some(RiderId::new(2)));
    assert!(
        result.runs[1].decisions[0]
            .explanation()
            .contains("pickup road travel")
    );
}

#[test]
fn pairing_preserves_readiness_replays_and_canonical_ordering() {
    let mut input = two_riders();
    input.orders[0].actual_readiness = roadrunner_simulation::ActualReadiness::SeededDelay {
        min_seconds: seconds(20.0),
        max_seconds: seconds(40.0),
    };
    let first = ok(compare_strategies(&graph(), &input, 2.0));
    input.riders.reverse();
    let second = ok(compare_strategies(&graph(), &input, 2.0));
    assert_eq!(
        ok(serde_json::to_vec(&first)),
        ok(serde_json::to_vec(&second))
    );
    for run in &first.runs {
        assert_eq!(
            run.orders[0].realized_ready_at,
            first.runs[0].orders[0].realized_ready_at
        );
        assert_eq!(run.seed, input.seed);
    }
    let zero = ok(compare_strategies(&graph(), &input, 0.0));
    assert_eq!(zero.runs[2].orders, zero.runs[3].orders);
    assert_eq!(zero.runs[2].summary, zero.runs[3].summary);
}

#[test]
fn horizon_empty_populations_and_shared_rejections_are_retained() {
    let mut input = two_riders();
    input.end_seconds = seconds(5.0);
    input.orders[0].deadline_seconds = Some(seconds(1.0));
    for run in ok(compare_strategies(&graph(), &input, 1.0)).runs {
        assert_eq!(run.summary.assigned_unfinished_orders, 1);
        assert_eq!(run.summary.delivered_duration.samples, 0);
        assert_eq!(run.summary.delivered_duration.mean_seconds, None);
        assert_eq!(run.summary.outstanding_past_deadline, 1);
    }
    input.orders[0].demand = 3;
    for run in ok(compare_strategies(&graph(), &input, 1.0)).runs {
        assert_eq!(run.summary.unassigned_orders, 1);
        assert_eq!(run.orders[0].rider, None);
    }
    input.riders.clear();
    for run in ok(compare_strategies(&graph(), &input, 1.0)).runs {
        assert_eq!(run.summary.rider_utilization, None);
        assert_eq!(run.summary.mean_rider_idle_seconds, None);
    }
    input.orders.clear();
    for run in ok(compare_strategies(&graph(), &input, 1.0)).runs {
        assert_eq!(run.summary.created_orders, 0);
        assert_eq!(run.summary.pickup_waiting.samples, 0);
    }
}

#[test]
fn invalid_comparisons_fail_without_partial_results_or_input_mutation() {
    let mut input = two_riders();
    for weight in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(compare_strategies(&graph(), &input, weight).is_err());
    }
    input.orders[0].expected_ready_at_seconds = None;
    let original = input.clone();
    // Non-readiness policies can run, but a four-way comparison requires a forecast.
    input.dispatch = DispatchPolicy::NearestRider;
    assert!(simulate(&graph(), &input).is_ok());
    input.dispatch = original.dispatch;
    assert!(compare_strategies(&graph(), &input, 1.0).is_err());
    assert_eq!(input, original);
}
