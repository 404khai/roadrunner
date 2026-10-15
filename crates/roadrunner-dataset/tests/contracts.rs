//! Independent small graph/time oracles and adversarial publication cases.
#[allow(dead_code)]
#[path = "../../roadrunner-simulation/tests/support/mod.rs"]
mod support;
use roadrunner_dataset::*;
use roadrunner_dispatch::{
    AdmissionDeadline, AppliedExecutionEffectId, DispatchInstant, ExecutionEffect, FleetAlgorithm,
    OrderPolicy, ReadinessRule, RecoveryPolicy, RiderId,
};
use roadrunner_simulation::{
    DispatchPolicy, DynamicChange, DynamicEvent, SimulationScenario, simulate,
};
use std::collections::{BTreeMap, BTreeSet};
use support::{close, graph, ok, order, scenario, seconds};

fn lineage() -> SyntheticLineage {
    SyntheticLineage {
        generator: "deterministic-scripted/v1".into(),
        family: "fixture-family/v1".into(),
        base_world: "fixture-world/v1".into(),
        realization: "fixed/v1".into(),
        policy: "preparation-aware/v1".into(),
        dependencies: BTreeSet::new(),
    }
}
fn source(s: &SimulationScenario, run: &str) -> SourceCapture {
    ok(SourceCapture::simulate(&graph(), s, run, lineage()))
}
fn reproduction() -> Reproduction {
    Reproduction {
        base_revision: "fixture-code/v1".into(),
        toolchain: "fixture: binary64/float_roundtrip".into(),
        patch: RetainedBytes::new(String::new()),
        files: [
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            "crates/roadrunner-dataset/src/lib.rs",
        ]
        .into_iter()
        .map(|s| (s.into(), RetainedBytes::new("fixture".into())))
        .collect(),
    }
}
fn request(sources: &[SourceCapture]) -> SplitRequest {
    SplitRequest {
        claim: "descriptive-synthetic/v1".into(),
        assignments: sources
            .iter()
            .map(|s| (s.run.clone(), Partition::Descriptive))
            .collect(),
        excluded_reasons: BTreeMap::new(),
    }
}
fn bundle(sources: Vec<SourceCapture>) -> DatasetBundle {
    let req = request(&sources);
    ok(build_dataset(sources, "test/v1", reproduction(), req))
}
fn label() -> LabelEvidence {
    LabelEvidence {
        start_effective_at: ok(DispatchInstant::new(10.0)),
        arrival_reported_at: Some(ok(DispatchInstant::new(30.0))),
        arrival_processing_sample_at: Some(ok(DispatchInstant::new(30.0))),
        boundary: ok(DispatchInstant::new(50.0)),
        non_arrival_through: None,
        source_closed: true,
        disputed: false,
        clock: ClockWitness::SyntheticLogicalV1,
        measurement: MeasurementSupport::SyntheticControlled,
    }
}
fn dynamic() -> SimulationScenario {
    let mut s = scenario();
    s.schema_version = 4;
    s.scenario_id = Some("dataset-dynamic/v1".into());
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
    s.orders = vec![order(1, 0.0, 0.0)];
    s.orders[0].admission = Some(OrderPolicy {
        id: "test/v1".into(),
        version: 1,
        deadline: AdmissionDeadline::SoftObserved,
        max_completion_delay: None,
        pickup_service: seconds(2.0),
        dropoff_service: seconds(3.0),
        readiness: ReadinessRule::ValidForecastV1,
    });
    s
}
#[test]
fn independent_all_started_oracle_includes_two_legs_and_original_b0() {
    // Fixture roads: rider node1 -> pickup node0 = 10s; ready25; node0 -> dropoff3 =30s.
    let b = bundle(vec![source(&scenario(), "run-1")]);
    assert_eq!(b.rows.len(), 2);
    assert_eq!(b.quality.exact_labels, 2);
    assert_eq!(b.quality.outcomes[&Lifecycle::Observed], 2);
    for (row, (action, start, arrival, b0)) in b
        .rows
        .iter()
        .zip([(0, 0.0, 10.0, 10.0), (1, 25.0, 55.0, 30.0)])
    {
        assert_eq!(row.key.action.value(), action);
        assert_eq!(row.rider, RiderId::new(1));
        close(row.label.evidence.start_effective_at.value(), start);
        close(
            row.label
                .evidence
                .arrival_reported_at
                .unwrap_or_else(|| panic!("endpoint"))
                .value(),
            arrival,
        );
        close(row.features.b0_seconds.value(), b0);
        close(
            row.label
                .duration_seconds
                .unwrap_or_else(|| panic!("duration"))
                .value(),
            b0,
        );
        assert_eq!(
            row.provenance["label"].measurement_support,
            MeasurementSupport::SyntheticControlled
        );
    }
    assert!(matches!(
        b.evaluation.physical,
        EvaluationAvailability::EvaluationUnavailable(_)
    ));
}
#[test]
fn horizon_start_denominator_is_not_completed_movements() {
    let mut s = scenario();
    s.end_seconds = seconds(5.0);
    let b = bundle(vec![source(&s, "horizon")]);
    assert_eq!(b.rows.len(), 1);
    assert_eq!(b.quality.exact_labels, 0);
    let row = &b.rows[0];
    assert_eq!(row.label.outcome, Lifecycle::RightCensored);
    close(
        row.label
            .lower_bound_seconds
            .unwrap_or_else(|| panic!("bound"))
            .value(),
        5.0,
    );
    let (_, trace) = ok(b.sources[0].verify());
    assert_eq!(trace.committed_start_count, 1);
    assert_eq!(trace.final_active.len(), 1);
    assert_ne!(
        trace.pending,
        [] as [roadrunner_simulation::PendingEvent; 0]
    );
}
#[test]
fn arrival_without_pickup_or_fulfillment_is_an_exact_label() {
    let mut s = scenario();
    s.end_seconds = seconds(12.0);
    let b = bundle(vec![source(&s, "arrived-waiting")]);
    assert_eq!(b.rows.len(), 1);
    assert_eq!(b.quality.exact_labels, 1);
    close(
        b.rows[0]
            .label
            .duration_seconds
            .unwrap_or_else(|| panic!("label"))
            .value(),
        10.0,
    );
}
#[test]
fn never_started_and_future_orders_do_not_create_fake_legs() {
    let mut s = scenario();
    s.riders[0].available = false;
    s.orders.push(order(2, 150.0, 150.0));
    let b = bundle(vec![source(&s, "no-starts")]);
    assert_eq!(b.rows, [] as [DatasetRow; 0]);
    assert_eq!(b.quality.committed_starts, 0);
}
#[test]
fn reused_namespace_and_replay_are_run_qualified_but_not_independent() {
    let b = bundle(vec![source(&scenario(), "a"), source(&scenario(), "b")]);
    assert_eq!(b.rows.len(), 4);
    assert_ne!(b.rows[0].key, b.rows[2].key);
    assert_eq!(b.rows[0].key.namespace, b.rows[2].key.namespace);
    assert_eq!(b.quality.dependency_groups, 1);
    assert!(matches!(
        b.splits.evaluation,
        EvaluationAvailability::EvaluationUnavailable(_)
    ));
}
#[test]
fn duplicate_source_run_rejected() {
    let a = source(&scenario(), "same");
    assert!(build_dataset(vec![a.clone(), a], "v1", reproduction(), request(&[])).is_err());
}
#[test]
fn duplicate_arrival_aliases_do_not_multiply_census() {
    let (_, mut t) = ok(source(&scenario(), "alias").verify());
    let mut e = t
        .effects
        .iter()
        .find(|e| e.effect == ExecutionEffect::Arrival)
        .cloned()
        .unwrap_or_else(|| panic!("arrival"));
    e.id = AppliedExecutionEffectId::new(99);
    t.effects.push(e);
    ok(reconcile_trace(&t));
    assert_eq!(t.starts.len(), 2);
    t.effects.last_mut().unwrap_or_else(|| panic!("alias")).at = ok(DispatchInstant::new(11.0));
    assert!(reconcile_trace(&t).is_err());
}
#[test]
fn delay_generations_preserve_original_ten_second_b0() {
    let mut s = dynamic();
    s.dynamic_events = vec![DynamicEvent {
        at_seconds: seconds(1.0),
        change: DynamicChange::RoadDelay {
            rider: 1,
            additional_seconds: seconds(20.0),
        },
    }];
    let src = source(&s, "delay");
    let (_, t) = ok(src.verify());
    assert_eq!(t.schedules.len(), 1);
    assert_eq!(t.schedules[0].execution.generation.value(), 1);
    let b = bundle(vec![src]);
    close(b.rows[0].features.b0_seconds.value(), 10.0);
    close(
        b.rows[0]
            .label
            .duration_seconds
            .unwrap_or_else(|| panic!("label"))
            .value(),
        30.0,
    );
    assert_eq!(b.rows[0].key.action.value(), 0);
    assert_eq!(t.schedules[0].execution.route, t.starts[0].original.route);
}
#[test]
fn cancellation_cannot_erase_already_started_action() {
    let mut s = dynamic();
    s.dynamic_events = vec![DynamicEvent {
        at_seconds: seconds(1.0),
        change: DynamicChange::Cancel { order: 1 },
    }];
    let b = bundle(vec![source(&s, "cancel")]);
    assert_eq!(b.rows[0].key.action.value(), 0);
    close(b.rows[0].features.b0_seconds.value(), 10.0);
    assert!(b.rows[0].label.duration_seconds.is_some());
}
#[test]
fn reassignment_and_frozen_replacement_retain_actual_started_owner() {
    let mut s = dynamic();
    s.orders.push(s.orders[0].clone());
    s.orders[1].id = 2;
    s.riders.push(roadrunner_simulation::RiderInput {
        id: 2,
        node: 0,
        capacity: 2,
        available: false,
    });
    s.dynamic_events = vec![
        DynamicEvent {
            at_seconds: seconds(1.0),
            change: DynamicChange::Availability {
                rider: 2,
                available: true,
            },
        },
        DynamicEvent {
            at_seconds: seconds(1.0),
            change: DynamicChange::Availability {
                rider: 1,
                available: false,
            },
        },
    ];
    let src = source(&s, "reassign");
    let (_, t) = ok(src.verify());
    assert_eq!(t.starts[0].rider, RiderId::new(1));
    assert!(t.starts.iter().any(|a| a.rider == RiderId::new(2)));
    let b = bundle(vec![src]);
    assert_eq!(b.rows[0].rider, RiderId::new(1));
}
#[test]
fn source_corruption_blocks_even_when_digest_is_recomputed() {
    let mut s = source(&scenario(), "corrupt");
    let mut v: serde_json::Value = ok(serde_json::from_str(&s.result.utf8));
    v["trace"]["starts"] = serde_json::json!([]);
    s.result = RetainedBytes::new(ok(serde_json::to_string(&v)));
    assert!(s.verify().is_err());
}
#[test]
fn wrong_owner_hole_and_pending_evidence_fail_census() {
    let (_, t) = ok(source(&scenario(), "census").verify());
    let mut bad = t.clone();
    bad.starts[0].rider = RiderId::new(99);
    assert!(reconcile_trace(&bad).is_err());
    let mut bad = t;
    bad.starts.remove(0);
    assert!(reconcile_trace(&bad).is_err());
    let mut s = scenario();
    s.end_seconds = seconds(5.0);
    let (_, mut t) = ok(source(&s, "pending").verify());
    t.pending.clear();
    assert!(reconcile_trace(&t).is_err());
}
#[test]
fn source_bytes_missing_graph_and_legacy_semantics_fail_publication() {
    let mut a = source(&scenario(), "bytes");
    a.result.utf8.push(' ');
    assert!(a.verify().is_err());
    let mut a = source(&scenario(), "graph");
    a.graph = RetainedBytes::new(String::new());
    assert!(a.verify().is_err());
    let mut a = source(&scenario(), "legacy");
    let mut v: serde_json::Value = ok(serde_json::from_str(&a.graph.utf8));
    v["schema_version"] = serde_json::json!(3);
    a.graph = RetainedBytes::new(ok(serde_json::to_string(&v)));
    assert!(a.verify().is_err());
}
#[test]
fn label_partition_zero_negative_pending_missing_dispute_and_late() {
    let mut e = label();
    e.arrival_reported_at = Some(e.start_effective_at);
    assert_eq!(assess_label(e).duration_seconds, Some(seconds(0.0)));
    let mut e = label();
    e.arrival_reported_at = Some(ok(DispatchInstant::new(9.0)));
    let a = assess_label(e);
    assert_eq!(a.outcome, Lifecycle::DisputedOrInvalid);
    assert!(a.duration_seconds.is_none());
    let mut e = label();
    e.arrival_reported_at = None;
    e.arrival_processing_sample_at = None;
    assert_eq!(assess_label(e.clone()).outcome, Lifecycle::MissingArrival);
    e.source_closed = false;
    assert_eq!(assess_label(e).outcome, Lifecycle::Pending);
    let mut e = label();
    e.disputed = true;
    assert_eq!(assess_label(e).outcome, Lifecycle::DisputedOrInvalid);
    let mut e = label();
    e.arrival_processing_sample_at = Some(ok(DispatchInstant::new(51.0)));
    assert_eq!(assess_label(e).outcome, Lifecycle::MissingArrival);
}
#[test]
fn incompatible_clocks_and_physical_silence_never_get_synthetic_labels_or_bounds() {
    for support in [
        MeasurementSupport::OperationalReportInterval,
        MeasurementSupport::PhysicalMeasurementSupported,
        MeasurementSupport::InsufficientEvidence,
    ] {
        let mut e = label();
        e.measurement = support;
        e.clock = ClockWitness::Unsupported;
        assert!(assess_label(e.clone()).duration_seconds.is_none());
        e.arrival_reported_at = None;
        e.non_arrival_through = Some(e.boundary);
        let a = assess_label(e);
        assert_eq!(a.outcome, Lifecycle::MissingArrival);
        assert!(a.lower_bound_seconds.is_none());
    }
}
#[test]
fn rejected_correction_does_not_replace_original_endpoint_and_new_dispute_version_is_explicit() {
    let old = assess_label(label());
    let rejected = ok(DispatchInstant::new(35.0));
    assert_ne!(Some(rejected), old.evidence.arrival_reported_at);
    let mut next = old.evidence.clone();
    next.disputed = true;
    let new = assess_label(next);
    assert_eq!(new.outcome, Lifecycle::DisputedOrInvalid);
    assert_eq!(old.duration_seconds, Some(seconds(20.0)));
    assert_eq!(
        new.evidence.arrival_reported_at,
        old.evidence.arrival_reported_at
    );
}
#[test]
fn archival_loss_is_not_predictor_time_absence_or_modern_backfill() {
    let (_, t) = ok(source(&scenario(), "feature").verify());
    let f = ok(extract_features(&t.starts[0], None));
    assert_eq!(
        f.origin,
        Availability::Unavailable(UnavailableReason::ArchivalLoss)
    );
    assert_ne!(
        UnavailableReason::ArchivalLoss,
        UnavailableReason::PredictorTimeAbsent
    );
    assert_eq!(
        f.day_of_week,
        Availability::Unavailable(UnavailableReason::NotApplicable)
    );
    assert_eq!(
        f.road_class,
        Availability::Unavailable(UnavailableReason::UnsupportedSemantics)
    );
    let mut start = t.starts[0].clone();
    start.original.route.routing.graph_digest = "modern-wrong-graph".into();
    assert!(extract_features(&start, Some(&graph())).is_err());
}
#[test]
fn future_traffic_readiness_delay_arrival_and_horizon_do_not_change_original_features() {
    let s = dynamic();
    let a = source(&s, "same");
    let (_, old) = ok(a.verify());
    let mut next = s;
    next.end_seconds = seconds(40.0);
    next.traffic_changes
        .push(roadrunner_simulation::TrafficChange {
            at_seconds: seconds(1.0),
            overrides: vec![roadrunner_simulation::TrafficOverride {
                edge_id: 0,
                multiplier: ok(roadrunner_core::cost::TrafficMultiplier::new(2.0)),
            }],
        });
    next.dynamic_events = vec![
        DynamicEvent {
            at_seconds: seconds(1.0),
            change: DynamicChange::Forecast {
                order: 1,
                expected_at_seconds: seconds(100.0),
            },
        },
        DynamicEvent {
            at_seconds: seconds(2.0),
            change: DynamicChange::RoadDelay {
                rider: 1,
                additional_seconds: seconds(60.0),
            },
        },
    ];
    let (_, new) = ok(source(&next, "same").verify());
    assert_eq!(old.starts[0], new.starts[0]);
    assert_eq!(
        ok(extract_features(&old.starts[0], Some(&graph()))),
        ok(extract_features(&new.starts[0], Some(&graph())))
    );
    // No scenario, final state, arrival, readiness, aggregate or latent-input field exists on StartFeatures.
    let v = ok(serde_json::to_value(ok(extract_features(
        &new.starts[0],
        Some(&graph()),
    ))));
    for key in [
        "scenario",
        "seed",
        "arrival",
        "realized_delay",
        "readiness",
        "aggregate",
        "policy",
    ] {
        assert!(v.get(key).is_none());
    }
}
#[test]
fn same_second_later_adoption_and_later_generation_cannot_impersonate_original_inputs() {
    let (_, t) = ok(source(&scenario(), "order").verify());
    let mut start = t.starts[0].clone();
    start.traffic[0].multiplier = ok(roadrunner_core::cost::TrafficMultiplier::new(2.0));
    assert!(extract_features(&start, Some(&graph())).is_err());
    start = t.starts[0].clone();
    start.original.generation = roadrunner_dispatch::ScheduleGeneration::new(1);
    assert!(extract_features(&start, Some(&graph())).is_err());
}
#[test]
fn dependency_closure_rejects_policy_replay_episode_and_transitive_leakage() {
    let mut first = source(&scenario(), "first");
    let mut second = source(&scenario(), "second");
    let mut third = source(&scenario(), "third");
    first.lineage.dependencies.insert("episode-1".into());
    second
        .lineage
        .dependencies
        .extend(["episode-1".into(), "episode-2".into()]);
    third.lineage.dependencies.insert("episode-2".into());
    second.lineage.policy = "other-policy".into();
    let src = vec![first, second, third];
    let mut req = request(&src);
    req.claim = "unseen-synthetic-components/v1".into();
    req.assignments.insert("first".into(), Partition::Train);
    req.assignments
        .insert("second".into(), Partition::Validation);
    req.assignments.insert("third".into(), Partition::Test);
    assert!(plan_splits(&src, req).is_err());
    let plan = ok(plan_splits(&src, request(&src)));
    assert_eq!(plan.groups.values().collect::<BTreeSet<_>>().len(), 1);
    assert!(matches!(
        plan.evaluation,
        EvaluationAvailability::EvaluationUnavailable(_)
    ));
}
#[test]
fn exact_split_reproducibility_and_exclusions_require_reasons() {
    let src = vec![source(&scenario(), "a")];
    let r = request(&src);
    assert_eq!(ok(plan_splits(&src, r.clone())), ok(plan_splits(&src, r)));
    let mut r = request(&src);
    r.assignments.insert("a".into(), Partition::Excluded);
    assert!(plan_splits(&src, r.clone()).is_err());
    r.excluded_reasons
        .insert("a".into(), "predeclared source exclusion".into());
    ok(plan_splits(&src, r));
}
#[test]
fn semantic_reproducibility_and_changed_boundaries_are_distinct_versions() {
    let src = source(&scenario(), "stable");
    let a = bundle(vec![src.clone()]);
    let b = bundle(vec![src]);
    assert_eq!(ok(canonical(&a)), ok(canonical(&b)));
    ok(verify_dataset(&a));
    let mut s = scenario();
    s.end_seconds = seconds(5.0);
    let changed = bundle(vec![source(&s, "stable")]);
    assert_ne!(ok(digest_bundle(&a)), ok(digest_bundle(&changed)));
}
fn digest_bundle(b: &DatasetBundle) -> Result<String, DatasetError> {
    Ok(digest(&canonical(b)?))
}
#[test]
fn corrupted_features_quality_splits_and_label_cannot_publish() {
    let b = bundle(vec![source(&scenario(), "integrity")]);
    let mut bad = b.clone();
    bad.rows[0].features.b0_seconds = seconds(999.0);
    assert!(verify_dataset(&bad).is_err());
    let mut bad = b.clone();
    bad.rows[0].label.duration_seconds = Some(seconds(0.0));
    assert!(verify_dataset(&bad).is_err());
    let mut bad = b.clone();
    bad.quality.committed_starts = 0;
    assert!(verify_dataset(&bad).is_err());
    let mut bad = b;
    bad.rows[0].dependency_group = "fake".into();
    assert!(verify_dataset(&bad).is_err());
}
#[test]
fn immutable_publication_roundtrip_rejects_overwrite_and_corruption() {
    let b = bundle(vec![source(&scenario(), "publish")]);
    let root = std::env::temp_dir().join(format!("eta-dataset-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    ok(publish_dataset(&b, &root));
    assert_eq!(ok(load_dataset(&root)), b);
    assert!(publish_dataset(&b, &root).is_err());
    ok(std::fs::write(root.join("bundle.json"), "{}"));
    assert!(load_dataset(&root).is_err());
    ok(std::fs::remove_dir_all(root));
}
#[test]
fn reverse_input_order_keeps_default_replay_identity_and_full_trace_stable() {
    let mut s = scenario();
    s.orders.push(order(2, 100.0, 100.0));
    let a = ok(simulate(&graph(), &s));
    s.orders.reverse();
    assert_eq!(a, ok(simulate(&graph(), &s)));
}
#[test]
fn malformed_units_nan_infinity_and_unsupported_source_schema_reject() {
    assert!(serde_json::from_str::<roadrunner_core::geo::Seconds>("-1").is_err());
    assert!(serde_json::from_str::<roadrunner_core::geo::Seconds>("1e999").is_err());
    assert!(serde_json::from_str::<roadrunner_core::geo::Seconds>("NaN").is_err());
    let mut s = source(&scenario(), "schema");
    s.schema = "eta-source/v999".into();
    assert!(s.verify().is_err());
}

#[test]
fn lossless_scoped_identity_preserves_u64_max_without_float_conversion() {
    let key = ActionKey {
        run: "run".into(),
        namespace: roadrunner_dispatch::OperationalWorldNamespace::from_bytes([255; 16]),
        action: roadrunner_dispatch::ActionId::new(u64::MAX),
    };
    let bytes = ok(canonical(&key));
    assert!(String::from_utf8_lossy(&bytes).contains("18446744073709551615"));
    assert_eq!(ok(serde_json::from_slice::<ActionKey>(&bytes)), key);
}

#[test]
fn genuine_separate_graph_components_can_receive_frozen_protected_assignments() {
    use roadrunner_core::geo::{CanonicalCoordinate, KilometersPerHour, haversine_distance};
    use roadrunner_core::graph::{
        AccessClass, BuilderNodeId, BuilderSegmentId, EdgeProperties, GraphBuildIdentity,
        GraphBuilder, GraphMetadata, GraphSnapshotId,
    };
    let mut sources = Vec::new();
    let mut assignments = BTreeMap::new();
    for (index, role) in [
        (1_u32, Partition::Train),
        (2, Partition::Validation),
        (3, Partition::Test),
    ] {
        let mut builder = GraphBuilder::new(
            GraphSnapshotId::new(u64::from(index)),
            GraphMetadata::new(
                "synthetic-independent/v1",
                "test",
                GraphBuildIdentity::new("fixture", "v1", "v1", "synthetic"),
            ),
        );
        let latitude = i32::try_from(index * 1000).unwrap_or_else(|e| panic!("{e}"));
        let points: Vec<_> = [0, 10_000, 20_000, 30_000]
            .into_iter()
            .map(|longitude| ok(CanonicalCoordinate::new(latitude, longitude)))
            .collect();
        for (id, point) in (0_u64..).zip(&points) {
            ok(builder.add_node(BuilderNodeId::new(id), *point));
        }
        for id in 1_usize..4 {
            let distance =
                haversine_distance(points[0].to_coordinate(), points[id].to_coordinate());
            let properties = EdgeProperties::new(
                ok(KilometersPerHour::new(distance.value() / 10.0 * 3.6)),
                None,
                AccessClass::General,
            );
            let target = u64::try_from(id).unwrap_or_else(|e| panic!("{e}"));
            ok(builder.add_segment(
                BuilderSegmentId::new(target),
                BuilderNodeId::new(0),
                BuilderNodeId::new(target),
                vec![points[0], points[id]],
                Some(properties),
                Some(properties),
            ));
        }
        let graph = ok(builder.finalize());
        let run = format!("independent-{index}");
        let mut ancestry = lineage();
        ancestry.family = format!("family-{index}");
        ancestry.base_world = format!("world-{index}");
        ancestry.realization = format!("realization-{index}");
        sources.push(ok(SourceCapture::simulate(
            &graph,
            &scenario(),
            &run,
            ancestry,
        )));
        assignments.insert(run, role);
    }
    let request = SplitRequest {
        claim: "unseen-synthetic-components/v1".into(),
        assignments,
        excluded_reasons: BTreeMap::new(),
    };
    let result = ok(build_dataset(
        sources,
        "three-components/v1",
        reproduction(),
        request,
    ));
    assert_eq!(result.splits.evaluation, EvaluationAvailability::Available);
    assert_eq!(result.quality.dependency_groups, 3);
    assert!(matches!(
        result.evaluation.physical,
        EvaluationAvailability::EvaluationUnavailable(_)
    ));
}
