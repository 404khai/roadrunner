//! Adversarial complete-state/history checks and hand-calculated temporal proof fixtures.
#![allow(clippy::float_cmp)]
#[path = "support/operational_fixture.rs"]
mod fixture;
use fixture::{Fixture, at, close, ok, secs};
use roadrunner_dispatch::*;

fn state(f: &Fixture) -> OperationalState {
    ok(OperationalState::new(
        OperationalWorldNamespace::from_bytes([17; 16]),
        ok(World::new(17, f.data.clone())),
    ))
}
fn adopt(
    s: &mut OperationalState,
    f: &Fixture,
    provider: &CoreRouteProvider<'_>,
    inputs: PoolingInputs,
) {
    let snapshot = ok(DispatchSnapshot::new(
        s,
        inputs.projections.values().next().map_or(at(0.0), |p| p.at),
        RoutingEpoch(at(0.0)),
        provider,
        &f.anchors,
    ));
    let context = match inputs.identity.optimizer.as_str() {
        "fleet-greedy-local/v1" => {
            ok(FleetContext::new(
                &snapshot,
                FleetInputs {
                    pooling: inputs,
                    unavailable_projections: std::collections::BTreeMap::default(),
                    algorithm: FleetAlgorithm::MultiStartLocal,
                },
            ))
            .pooling
        }
        "dynamic-recovery/v1" => {
            ok(RecoveryContext::new(
                &snapshot,
                inputs,
                RecoveryPolicy {
                    version: 1,
                    reroute_penalty: secs(0.0),
                    assignment_stability_penalty: secs(0.0),
                    minimum_improvement: secs(0.0),
                    cooldown: secs(0.0),
                },
                "fixture".into(),
                None,
            ))
            .pooling
        }
        _ => ok(PoolingContext::new(&snapshot, inputs)),
    };
    ok(s.adopt_planning_context(s.revision(), &context));
}
fn planning<'a>(
    f: &'a Fixture,
    provider: &'a CoreRouteProvider<'a>,
    clock: &'a LogicalOperationalClock,
) -> OperationalPlanningContext<'a> {
    OperationalPlanningContext {
        provider,
        anchors: &f.anchors,
        epoch: RoutingEpoch(at(0.0)),
        clock,
    }
}
fn leg(
    provider: &CoreRouteProvider<'_>,
    from: roadrunner_core::graph::NodeId,
    to: roadrunner_core::graph::NodeId,
) -> FrozenExecutionLeg {
    let RouteOutcome::RouteFound(r) = ok(provider.route(from, to, secs(0.0))) else {
        panic!("route");
    };
    FrozenExecutionLeg {
        from,
        to,
        departed_at: at(0.0),
        travel: r.route.elapsed_travel_time(),
        distance: r.route.total_distance(),
        nodes: r.route.path().to_vec(),
        edges: r.route.edges().to_vec(),
        routing: r.provenance,
    }
}
fn committed_fixture() -> (Fixture, OperationalState) {
    let f = Fixture::new();
    let mut world = ok(World::new(17, f.data.clone()));
    let d = ok(f.decide(&world, OrderId::new(2), f.inputs(&world, at(0.0))));
    ok(world.commit_insertion(&d, &d.evidence().context));
    let s = ok(OperationalState::new(
        OperationalWorldNamespace::from_bytes([17; 16]),
        world,
    ));
    (f, s)
}
fn history_valid(s: &OperationalState) {
    for (i, record) in s.history().iter().enumerate() {
        assert_eq!(
            record.before.number,
            u64::try_from(i).unwrap_or_else(|e| panic!("{e}"))
        );
        assert_eq!(record.after.number, record.before.number + 1);
        assert_eq!(record.before.namespace, record.after.namespace);
    }
    assert_eq!(
        s.revision().number,
        u64::try_from(s.history().len()).unwrap_or_else(|e| panic!("{e}"))
    );
    ok(s.validate());
}

#[test]
fn delayed_publication_preserves_reference_and_stamps_authentic_commit() {
    let f = Fixture::new();
    let mut s = state(&f);
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let inputs = f.inputs(&s, at(0.0));
    adopt(&mut s, &f, &provider, inputs.clone());
    let clock = ok(LogicalOperationalClock::new(
        "fixture-origin-0/v1".into(),
        at(0.0),
    ));
    let evaluated =
        ok(s.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(2), inputs));
    let d = evaluated.insertion().unwrap_or_else(|| panic!("insertion"));
    let original = d
        .proposal()
        .unwrap_or_else(|| panic!("proposal"))
        .evaluation()
        .completions[&OrderId::new(2)];
    close(original.value(), 30.0); // wait to 15 + pickup 2 + road 10 + dropoff 3.
    assert!(s.commit_insertion(d, &d.evidence().context).is_err()); // no weaker operational path
    let proof = ok(s.certify(evaluated, &provider, at(2.0)));
    ok(clock.advance(at(1.0)));
    let before = s.revision();
    let record = ok(s.publish_temporal(proof, &clock, &provider));
    assert_eq!(record.committed_revision.number, before.number + 1);
    assert_eq!(record.evaluated_at, at(0.0));
    assert_eq!(record.committed_at, at(1.0));
    assert_eq!(
        s.data().accepted[&OrderId::new(2)].completion_reference,
        original
    );
    assert_eq!(s.data().accepted[&OrderId::new(2)].accepted_at, at(1.0));
    assert_eq!(
        record.commitment_record.payload["world"]["accepted"]["2"]["accepted_at"],
        serde_json::json!(1.0)
    );
    assert_eq!(record.commitment_record.operational_revision, s.revision());
    assert_eq!(s.publications().len(), 1);
    history_valid(&s);
}

#[test]
fn competing_decisions_have_one_legal_publication_not_just_one_final_owner() {
    let f = Fixture::new();
    let mut s = state(&f);
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let inputs = f.inputs(&s, at(0.0));
    adopt(&mut s, &f, &provider, inputs.clone());
    let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
    let detached = s.snapshot();
    let a = ok(detached.evaluate_insertion(
        &planning(&f, &provider, &clock),
        OrderId::new(2),
        inputs.clone(),
    ));
    let b =
        ok(detached.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(2), inputs));
    let pa = ok(s.certify(a, &provider, at(2.0)));
    let pb = ok(s.certify(b, &provider, at(2.0)));
    ok(clock.advance(at(1.0)));
    ok(s.publish_temporal(pa, &clock, &provider));
    let before = s.snapshot();
    assert!(matches!(
        s.publish_temporal(pb, &clock, &provider),
        Err(TemporalError::Stale)
    ));
    assert_eq!(&s, &*before);
    assert_eq!(s.publications().len(), 1);
    history_valid(&s);
}

#[test]
fn aba_adoption_and_unrelated_authoritative_change_cannot_rebind_old_evaluation() {
    let f = Fixture::new();
    let mut s = state(&f);
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let inputs = f.inputs(&s, at(0.0));
    adopt(&mut s, &f, &provider, inputs.clone());
    let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
    let d = ok(s.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(2), inputs));
    let original = s.contexts()["traffic"].identity.clone();
    ok(s.adopt(
        s.revision(),
        AdoptedContextIdentity {
            category: "traffic".into(),
            content: "B".into(),
        },
    ));
    ok(s.adopt(s.revision(), original));
    assert_eq!(s.contexts()["traffic"].revision.value(), 3);
    assert!(matches!(
        s.certify(d, &provider, at(2.0)),
        Err(TemporalError::Stale)
    ));
    history_valid(&s);
}

#[test]
fn expiry_and_backward_clock_never_publish_tentative_state() {
    let f = Fixture::new();
    let mut s = state(&f);
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let inputs = f.inputs(&s, at(0.0));
    adopt(&mut s, &f, &provider, inputs.clone());
    let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
    let d = ok(s.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(2), inputs));
    let proof = ok(s.certify(d, &provider, at(2.0)));
    ok(clock.advance(at(3.0)));
    assert!(clock.advance(at(2.0)).is_err());
    let before = s.snapshot();
    assert!(matches!(
        s.publish_temporal(proof, &clock, &provider),
        Err(TemporalError::EvaluationContextExpired)
    ));
    assert_eq!(&s, &*before);
}

#[test]
fn deadline_cumulative_bound_and_forecast_expiry_reject_endpoint_certification() {
    for mode in 0..3 {
        let mut f = Fixture::new();
        if mode == 0 {
            f.data
                .orders
                .get_mut(&OrderId::new(2))
                .unwrap_or_else(|| panic!("order"))
                .deadline = Some(at(31.0));
        }
        let mut s = state(&f);
        let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
        let mut inputs = f.inputs(&s, at(0.0));
        if mode == 0 {
            inputs
                .policies
                .get_mut(&OrderId::new(2))
                .unwrap_or_else(|| panic!("policy"))
                .deadline = AdmissionDeadline::Hard;
        }
        if mode == 1 {
            inputs
                .policies
                .get_mut(&OrderId::new(2))
                .unwrap_or_else(|| panic!("policy"))
                .max_completion_delay = Some(secs(0.0));
        }
        if mode == 2 {
            inputs
                .forecasts
                .get_mut(&OrderId::new(2))
                .unwrap_or_else(|| panic!("forecast"))
                .valid_until = at(1.0);
        }
        adopt(&mut s, &f, &provider, inputs.clone());
        let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
        let d = ok(s.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(2), inputs));
        assert!(d.insertion().is_some_and(|d| d.proposal().is_some()));
        let before = s.snapshot();
        assert!(s.certify(d, &provider, at(20.0)).is_err());
        assert_eq!(&s, &*before);
    }
}

#[test]
fn time_dependent_ranking_is_not_authorized_by_static_proof() {
    use roadrunner_core::cost::{
        TimeDependentTrafficSnapshot, TrafficMultiplier, TrafficPoint, TrafficProfile,
    };
    let f = Fixture::new();
    let mut s = state(&f);
    let edge = f
        .graph
        .edges()
        .first()
        .unwrap_or_else(|| panic!("edge"))
        .id();
    let traffic = ok(TimeDependentTrafficSnapshot::new(
        &f.graph,
        [TrafficProfile {
            edge_id: edge,
            points: vec![
                TrafficPoint {
                    departure_seconds: secs(0.0),
                    multiplier: ok(TrafficMultiplier::new(2.0)),
                },
                TrafficPoint {
                    departure_seconds: secs(1000.0),
                    multiplier: TrafficMultiplier::NORMAL,
                },
            ],
        }],
    ));
    let provider = ok(CoreRouteProvider::new(
        &f.graph,
        TrafficContext::TimeDependent(&traffic),
    ));
    let mut inputs = f.inputs(&s, at(0.0));
    inputs.identity.routing = provider.provenance();
    adopt(&mut s, &f, &provider, inputs.clone());
    let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
    let d = ok(s.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(2), inputs));
    assert!(matches!(
        s.certify(d, &provider, at(1.0)),
        Err(TemporalError::UnsupportedTemporalModel)
    ));
}

#[test]
fn same_effect_distinct_ids_and_conflicting_reuse_have_no_second_business_effect() {
    let (f, mut s) = committed_fixture();
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let rider = s.data().assignments[&OrderId::new(2)].rider;
    let origin = f.anchors.riders[&rider].clone();
    let target = f.anchors.pickups[&OrderId::new(2)].clone();
    ok(s.start_action(
        s.revision(),
        rider,
        s.plan_revisions()[&rider],
        leg(&provider, origin.node, target.node),
        &origin,
        target,
    ));
    let a = s.execution()[&rider].clone();
    ok(s.apply_effect(
        s.revision(),
        AppliedExecutionEffectId::new(10),
        rider,
        a.action_id,
        a.generation,
        ExecutionEffect::Arrival,
        at(0.0),
    ));
    ok(s.observe_ready(OrderId::new(2), at(15.0)));
    ok(s.start_service(s.revision(), rider, at(15.0), secs(2.0)));
    let revision = s.revision();
    assert!(
        s.apply_effect(
            revision,
            AppliedExecutionEffectId::new(11),
            rider,
            a.action_id,
            a.generation,
            ExecutionEffect::Completion,
            at(16.0)
        )
        .is_err()
    );
    assert_eq!(s.revision(), revision);
    ok(s.apply_effect(
        revision,
        AppliedExecutionEffectId::new(11),
        rider,
        a.action_id,
        a.generation,
        ExecutionEffect::Completion,
        at(17.0),
    ));
    let committed = s.revision();
    assert_eq!(
        s.data().fulfillment[&OrderId::new(2)],
        FulfillmentState::PickedUp {
            rider,
            at: at(17.0)
        }
    );
    assert_eq!(
        s.data().plans[&rider].stops,
        vec![Stop::Dropoff(OrderId::new(2))]
    );
    assert!(s.execution().is_empty());
    assert_eq!(
        ok(s.apply_effect(
            revision,
            AppliedExecutionEffectId::new(11),
            rider,
            a.action_id,
            a.generation,
            ExecutionEffect::Completion,
            at(17.0)
        )),
        EffectOutcome::AlreadyApplied
    );
    assert_eq!(
        ok(s.apply_effect(
            committed,
            AppliedExecutionEffectId::new(12),
            rider,
            a.action_id,
            a.generation,
            ExecutionEffect::Completion,
            at(17.0)
        )),
        EffectOutcome::AlreadyApplied
    );
    assert_eq!(s.revision(), committed);
    assert!(
        s.apply_effect(
            committed,
            AppliedExecutionEffectId::new(12),
            rider,
            a.action_id,
            a.generation,
            ExecutionEffect::Arrival,
            at(17.0)
        )
        .is_err()
    );
    history_valid(&s);
}

#[test]
fn rescheduling_preserves_action_and_rejects_old_generation_atomically() {
    let (f, mut s) = committed_fixture();
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let rider = s.data().assignments[&OrderId::new(2)].rider;
    let origin = f.anchors.riders[&rider].clone();
    let target = f.anchors.pickups[&OrderId::new(2)].clone();
    let mut wrong_profile = leg(&provider, origin.node, target.node);
    wrong_profile.routing.profile = "wrong-profile/v1".into();
    let before = s.snapshot();
    assert!(
        s.start_action(
            s.revision(),
            rider,
            s.plan_revisions()[&rider],
            wrong_profile,
            &origin,
            target.clone()
        )
        .is_err()
    );
    assert_eq!(&s, &*before);
    ok(s.start_action(
        s.revision(),
        rider,
        s.plan_revisions()[&rider],
        leg(&provider, origin.node, target.node),
        &origin,
        target,
    ));
    let original = s.execution()[&rider].clone();
    let delayed = ok(s.delay_action(s.revision(), rider, secs(5.0)));
    assert_eq!(original.action_id, delayed.action_id);
    assert_ne!(original.id, delayed.id);
    assert_eq!(original.route, delayed.route);
    assert_ne!(original.generation, delayed.generation);
    let before = s.snapshot();
    assert!(
        s.apply_effect(
            s.revision(),
            AppliedExecutionEffectId::new(10),
            rider,
            original.action_id,
            original.generation,
            ExecutionEffect::Arrival,
            at(5.0)
        )
        .is_err()
    );
    assert_eq!(&s, &*before);
    assert_eq!(
        ok(s.cancel_order(OrderId::new(2), at(5.0))),
        Err(CancellationRefusal::FrozenExecution)
    );
    assert_eq!(&s, &*before);
    history_valid(&s);
}

#[test]
fn fresh_namespaces_checked_allocation_and_no_stale_creation_leaks() {
    let f = Fixture::new();
    let mut s = state(&f);
    let a = ok(OperationalWorldNamespace::fresh());
    let b = ok(OperationalWorldNamespace::fresh());
    assert_ne!(a, b);
    assert_ne!(
        a.reference("order", u64::MAX),
        b.reference("order", u64::MAX)
    );
    let old = s.revision();
    let order = f.data.orders[&OrderId::new(2)].clone();
    let id = ok(s.create_order(old, order.clone(), OrderReadiness::default()));
    assert_eq!(id, OrderId::new(4));
    let before = s.snapshot();
    assert!(
        s.create_order(old, order, OrderReadiness::default())
            .is_err()
    );
    assert_eq!(&s, &*before);
    let mut exhausted = f.data.clone();
    let old = OrderId::new(3);
    let max = OrderId::new(u64::MAX);
    let mut order = exhausted
        .orders
        .remove(&old)
        .unwrap_or_else(|| panic!("order"));
    order.id = max;
    exhausted.orders.insert(max, order);
    let ready = exhausted
        .readiness
        .remove(&old)
        .unwrap_or_else(|| panic!("ready"));
    exhausted.readiness.insert(max, ready);
    let fulfillment = exhausted
        .fulfillment
        .remove(&old)
        .unwrap_or_else(|| panic!("fulfillment"));
    exhausted.fulfillment.insert(max, fulfillment);
    assert!(matches!(
        OperationalState::new(a, ok(World::new(17, exhausted))),
        Err(CommitError::VersionOverflow)
    ));
    history_valid(&s);
}

#[test]
fn fleet_incumbent_and_recovery_publish_after_nonzero_evaluation_time() {
    let f = Fixture::new();
    let mut s = state(&f);
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let mut pooling = f.inputs(&s, at(0.0));
    pooling.identity.optimizer = "fleet-greedy-local/v1".into();
    adopt(&mut s, &f, &provider, pooling.clone());
    let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
    let evaluated = ok(s.evaluate_fleet(
        &planning(&f, &provider, &clock),
        &[OrderId::new(1)],
        FleetInputs {
            pooling,
            unavailable_projections: std::collections::BTreeMap::default(),
            algorithm: FleetAlgorithm::MultiStartLocal,
        },
    ));
    let original = evaluated
        .fleet()
        .unwrap_or_else(|| panic!("fleet"))
        .proposal()
        .unwrap_or_else(|| panic!("proposal"))
        .plans()
        .clone();
    let proof = ok(s.certify(evaluated, &provider, at(2.0)));
    ok(clock.advance(at(1.0)));
    ok(s.publish_temporal(proof, &clock, &provider));
    for (r, p) in original {
        assert_eq!(s.data().plans[&r], p);
    }
    let accepted = s.data().accepted.clone();
    let owner = s.data().assignments[&OrderId::new(1)].rider;
    ok(s.update_rider_state(
        owner,
        RiderState {
            availability: Availability::Unavailable,
            ..s.data().riders[&owner]
        },
    ));
    let mut pooling = f.inputs(&s, at(1.0));
    pooling.identity.optimizer = "dynamic-recovery/v1".into();
    adopt(&mut s, &f, &provider, pooling.clone());
    let evaluated = ok(s.evaluate_recovery(
        &planning(&f, &provider, &clock),
        pooling,
        RecoveryPolicy {
            version: 1,
            reroute_penalty: secs(0.0),
            assignment_stability_penalty: secs(0.0),
            minimum_improvement: secs(0.0),
            cooldown: secs(0.0),
        },
        "offline".into(),
    ));
    assert!(
        evaluated
            .recovery()
            .unwrap_or_else(|| panic!("recovery"))
            .proposal()
            .is_some()
    );
    let proof = ok(s.certify(evaluated, &provider, at(3.0)));
    ok(clock.advance(at(2.0)));
    ok(s.publish_temporal(proof, &clock, &provider));
    assert_ne!(s.data().assignments[&OrderId::new(1)].rider, owner);
    assert_eq!(s.data().accepted, accepted);
    assert_eq!(s.recovery_at(), Some(at(2.0)));
    assert_eq!(
        s.history()
            .last()
            .unwrap_or_else(|| panic!("history"))
            .recovery_after,
        Some(at(2.0))
    );
    history_valid(&s);
}

#[test]
fn expiry_during_atomic_staging_rolls_back_the_entire_publication() {
    struct AdvancingClock(std::cell::Cell<u8>);
    impl OperationalClock for AdvancingClock {
        fn time_domain(&self) -> &'static str {
            "fixture/v1"
        }
        fn now(&self) -> Result<DispatchInstant, DispatchTimeError> {
            let calls = self.0.get();
            self.0.set(calls + 1);
            Ok(at(if calls == 0 { 1.0 } else { 3.0 }))
        }
    }
    let f = Fixture::new();
    let mut s = state(&f);
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let inputs = f.inputs(&s, at(0.0));
    adopt(&mut s, &f, &provider, inputs.clone());
    let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
    let evaluated =
        ok(s.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(2), inputs));
    let proof = ok(s.certify(evaluated, &provider, at(2.0)));
    let before = s.snapshot();
    assert!(matches!(
        s.publish_temporal(proof, &AdvancingClock(std::cell::Cell::new(0)), &provider),
        Err(TemporalError::EvaluationContextExpired)
    ));
    assert_eq!(&s, &*before);
}

#[test]
#[allow(clippy::too_many_lines)] // Complete travel/wait/service and rewrite history is one adversarial sequence.
fn frozen_action_survives_suffix_publication_and_old_graph_adoption() {
    let (f, mut s) = committed_fixture();
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let rider = s.data().assignments[&OrderId::new(2)].rider;
    for r in [RiderId::new(1), RiderId::new(2)] {
        if r != rider {
            ok(s.update_rider_state(
                r,
                RiderState {
                    availability: Availability::Unavailable,
                    ..s.data().riders[&r]
                },
            ));
        }
    }
    let origin = f.anchors.riders[&rider].clone();
    let target = f.anchors.pickups[&OrderId::new(2)].clone();
    ok(s.start_action(
        s.revision(),
        rider,
        s.plan_revisions()[&rider],
        leg(&provider, origin.node, target.node),
        &origin,
        target,
    ));
    let action = s.execution()[&rider].clone();
    let old_plan = s.plan_revisions()[&rider];
    let mut inputs = f.inputs(&s, at(0.0));
    let prefix = ok(s.frozen_prefix(rider, &inputs, at(0.0)));
    inputs.projections.insert(
        rider,
        ok(project_execution(s.data(), rider, at(0.0), origin, prefix)),
    );
    adopt(&mut s, &f, &provider, inputs.clone());
    let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
    let evaluated =
        ok(s.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(3), inputs));
    let proof = ok(s.certify(evaluated, &provider, at(2.0)));
    ok(clock.advance(at(1.0)));
    ok(s.publish_temporal(proof, &clock, &provider));
    assert_eq!(s.execution()[&rider], action);
    assert!(s.plan_revisions()[&rider] > old_plan);
    let originating_graph = action.route.routing.graph_digest.clone();
    ok(s.adopt(
        s.revision(),
        AdoptedContextIdentity {
            category: "graph".into(),
            content: "different-graph".into(),
        },
    ));
    assert_eq!(
        s.execution()[&rider].route.routing.graph_digest,
        originating_graph
    );
    ok(s.apply_effect(
        s.revision(),
        AppliedExecutionEffectId::new(100),
        rider,
        action.action_id,
        action.generation,
        ExecutionEffect::Arrival,
        at(1.0),
    ));
    ok(s.wait_for_readiness(s.revision(), rider));
    assert!(
        s.cancel_order(OrderId::new(2), at(2.0))
            .unwrap_or_else(|e| panic!("{e:?}"))
            .is_err()
    );
    ok(s.observe_ready(OrderId::new(2), at(15.0)));
    ok(s.start_service(s.revision(), rider, at(15.0), secs(2.0)));
    let before = s.snapshot();
    assert!(
        s.apply_effect(
            s.revision(),
            AppliedExecutionEffectId::new(101),
            rider,
            action.action_id,
            action.generation,
            ExecutionEffect::Completion,
            at(16.0)
        )
        .is_err()
    );
    assert_eq!(&s, &*before);
    ok(s.apply_effect(
        s.revision(),
        AppliedExecutionEffectId::new(101),
        rider,
        action.action_id,
        action.generation,
        ExecutionEffect::Completion,
        at(17.0),
    ));
    let revision = s.revision();
    assert_eq!(
        ok(s.apply_effect(
            revision,
            AppliedExecutionEffectId::new(102),
            rider,
            action.action_id,
            action.generation,
            ExecutionEffect::Completion,
            at(17.0)
        )),
        EffectOutcome::AlreadyApplied
    );
    assert_eq!(s.revision(), revision);
    assert!(
        matches!(s.data().fulfillment[&OrderId::new(2)],FulfillmentState::PickedUp{rider:r,..} if r==rider)
    );
    assert!(
        s.next_stop(rider)
            .unwrap_or_else(|e| panic!("{e:?}"))
            .is_some()
    );
    let before = s.snapshot();
    let origin = f.anchors.pickups[&OrderId::new(2)].clone();
    let stop = ok(s.next_stop(rider)).unwrap_or_else(|| panic!("next"));
    let target = match stop {
        Stop::Pickup(o) => f.anchors.pickups[&o].clone(),
        Stop::Dropoff(o) => f.anchors.dropoffs[&o].clone(),
    };
    assert!(
        s.start_action(
            s.revision(),
            rider,
            old_plan,
            leg(&provider, origin.node, target.node),
            &origin,
            target.clone()
        )
        .is_err()
    );
    assert_eq!(&s, &*before); // superseded future plan cannot start
    assert!(
        s.start_action(
            s.revision(),
            rider,
            s.plan_revisions()[&rider],
            leg(&provider, origin.node, target.node),
            &origin,
            target
        )
        .is_err()
    );
    assert_eq!(&s, &*before); // no backdated or old-graph future departure
    history_valid(&s);
}

#[test]
#[ignore = "external benchmark collector runs this release-only measurement explicitly"]
fn operational_boundary_measurement() {
    let f = Fixture::new();
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let mut evaluations = Vec::new();
    let mut certificates = Vec::new();
    let mut publications = Vec::new();
    let mut work = 0;
    for run in 0..102 {
        let mut s = state(&f);
        let inputs = f.inputs(&s, at(0.0));
        adopt(&mut s, &f, &provider, inputs.clone());
        let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
        let begin = std::time::Instant::now();
        let evaluated =
            ok(s.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(2), inputs));
        let evaluation_ns = begin.elapsed().as_nanos();
        let count = evaluated
            .insertion()
            .unwrap_or_else(|| panic!("insertion"))
            .evidence()
            .riders
            .iter()
            .map(|r| r.evaluated)
            .sum::<u64>();
        if run == 0 {
            work = count;
        } else {
            assert_eq!(count, work);
        }
        let begin = std::time::Instant::now();
        let proof = ok(s.certify(evaluated, &provider, at(2.0)));
        let certificate_ns = begin.elapsed().as_nanos();
        ok(clock.advance(at(1.0)));
        let begin = std::time::Instant::now();
        ok(s.publish_temporal(proof, &clock, &provider));
        let publication_ns = begin.elapsed().as_nanos();
        assert_eq!(s.data().accepted[&OrderId::new(2)].accepted_at, at(1.0));
        history_valid(&s);
        if run > 0 {
            evaluations.push(evaluation_ns);
            certificates.push(certificate_ns);
            publications.push(publication_ns);
        }
    }
    println!(
        "OPERATIONAL_BOUNDARY_MEASUREMENT={}",
        serde_json::json!({"schema_version":1,"fixture":"three-node/two-rider/three-order/v1","graph_digest":f.graph.metadata().snapshot_digest(),"nodes":3,"edges":4,"riders":2,"orders":3,"evaluations_per_run":work,"warmups":1,"runs":101,"evaluation_raw_ns":evaluations,"certification_raw_ns":certificates,"publication_raw_ns":publications,"validity_policy":"static-road-monotone/v1","evaluation_at":0,"publication_at":1,"valid_until":2})
    );
}

#[test]
fn fifo_time_passage_changes_exhaustive_winner_without_any_operational_mutation() {
    use roadrunner_core::cost::{
        TimeDependentTrafficSnapshot, TrafficMultiplier, TrafficPoint, TrafficProfile,
    };
    use roadrunner_core::graph::NodeId;
    let mut f = Fixture::new();
    let node = NodeId::new(2);
    let coordinate = f
        .graph
        .node(node)
        .unwrap_or_else(|| panic!("node"))
        .coordinate();
    f.data
        .riders
        .get_mut(&RiderId::new(1))
        .unwrap_or_else(|| panic!("rider"))
        .coordinate = coordinate;
    f.anchors.riders.insert(
        RiderId::new(1),
        RoutingAnchor {
            coordinate,
            node,
            graph_digest: f.graph.metadata().snapshot_digest().into(),
        },
    );
    f.data
        .readiness
        .get_mut(&OrderId::new(2))
        .unwrap_or_else(|| panic!("order"))
        .observed_at = Some(at(0.0));
    let edge = f
        .graph
        .edges()
        .iter()
        .find(|e| e.from() == NodeId::new(1) && e.to() == NodeId::new(0))
        .unwrap_or_else(|| panic!("edge"))
        .id();
    let traffic = ok(TimeDependentTrafficSnapshot::new(
        &f.graph,
        [TrafficProfile {
            edge_id: edge,
            points: vec![
                TrafficPoint {
                    departure_seconds: secs(0.0),
                    multiplier: TrafficMultiplier::NORMAL,
                },
                TrafficPoint {
                    departure_seconds: secs(100.0),
                    multiplier: ok(TrafficMultiplier::new(4.0)),
                },
            ],
        }],
    ));
    let provider = ok(CoreRouteProvider::new(
        &f.graph,
        TrafficContext::TimeDependent(&traffic),
    ));
    let mut s = state(&f);
    let mut inputs = f.inputs(&s, at(0.0));
    inputs.identity.routing = provider.provenance();
    adopt(&mut s, &f, &provider, inputs.clone());
    let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
    let early = ok(s.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(2), inputs));
    assert_eq!(
        early
            .insertion()
            .unwrap_or_else(|| panic!("decision"))
            .proposal()
            .unwrap_or_else(|| panic!("proposal"))
            .rider(),
        RiderId::new(2)
    );
    let revision = s.revision();
    ok(clock.advance(at(100.0)));
    let mut inputs = f.inputs(&s, at(100.0));
    inputs.identity.routing = provider.provenance();
    let late = ok(s.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(2), inputs));
    assert_eq!(
        late.insertion()
            .unwrap_or_else(|| panic!("decision"))
            .proposal()
            .unwrap_or_else(|| panic!("proposal"))
            .rider(),
        RiderId::new(1)
    );
    assert_eq!(s.revision(), revision);
    let before = s.snapshot();
    assert!(matches!(
        s.certify(early, &provider, at(100.0)),
        Err(TemporalError::UnsupportedTemporalModel)
    ));
    assert_eq!(&s, &*before);
}

#[test]
#[allow(clippy::too_many_lines)] // Seeded complete execution histories exercise every prefix and duplicate.
fn generated_execution_histories_preserve_custody_load_terms_and_once_only_effects() {
    for seed in 0..24_u64 {
        let mut f = Fixture::new();
        for ready in f.data.readiness.values_mut() {
            ready.observed_at = Some(at(0.0));
        }
        let mut s = ok(OperationalState::new(
            OperationalWorldNamespace::from_bytes(u128::from(seed).to_be_bytes()),
            ok(World::new(17, f.data.clone())),
        ));
        let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
        let mut inputs = f.inputs(&s, at(0.0));
        inputs.identity.optimizer = "fleet-greedy-local/v1".into();
        adopt(&mut s, &f, &provider, inputs.clone());
        let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
        let evaluated = ok(s.evaluate_fleet(
            &planning(&f, &provider, &clock),
            &[OrderId::new(1), OrderId::new(2)],
            FleetInputs {
                pooling: inputs,
                unavailable_projections: std::collections::BTreeMap::default(),
                algorithm: FleetAlgorithm::MultiStartLocal,
            },
        ));
        let proof = ok(s.certify(evaluated, &provider, at(2.0)));
        ok(clock.advance(at(1.0)));
        ok(s.publish_temporal(proof, &clock, &provider));
        let accepted = s.data().accepted.clone();
        let owners = s.data().assignments.clone();
        let mut effects = seed * 1000 + 100;
        for rider in [RiderId::new(1), RiderId::new(2)] {
            let mut now = at(1.0);
            let mut anchor = f.anchors.riders[&rider].clone();
            while let Some(stop) = ok(s.next_stop(rider)) {
                assert_eq!(owners[&stop.order()].rider, rider);
                let target = match stop {
                    Stop::Pickup(o) => f.anchors.pickups[&o].clone(),
                    Stop::Dropoff(o) => f.anchors.dropoffs[&o].clone(),
                };
                let RouteOutcome::RouteFound(result) =
                    ok(provider.route(anchor.node, target.node, secs(now.value())))
                else {
                    panic!("route");
                };
                let route = FrozenExecutionLeg {
                    from: anchor.node,
                    to: target.node,
                    departed_at: now,
                    travel: result.route.elapsed_travel_time(),
                    distance: result.route.total_distance(),
                    nodes: result.route.path().to_vec(),
                    edges: result.route.edges().to_vec(),
                    routing: result.provenance,
                };
                ok(s.start_action(
                    s.revision(),
                    rider,
                    s.plan_revisions()[&rider],
                    route,
                    &anchor,
                    target.clone(),
                ));
                if seed % 2 == 1 {
                    ok(s.delay_action(s.revision(), rider, secs(1.0)));
                }
                let active = s.execution()[&rider].clone();
                now = active.arrival;
                ok(s.apply_effect(
                    s.revision(),
                    AppliedExecutionEffectId::new(effects),
                    rider,
                    active.action_id,
                    active.generation,
                    ExecutionEffect::Arrival,
                    now,
                ));
                effects += 1;
                let service = match stop {
                    Stop::Pickup(_) => secs(2.0),
                    Stop::Dropoff(_) => secs(3.0),
                };
                ok(s.start_service(s.revision(), rider, now, service));
                now = ok(now.checked_add(service));
                ok(s.apply_effect(
                    s.revision(),
                    AppliedExecutionEffectId::new(effects),
                    rider,
                    active.action_id,
                    active.generation,
                    ExecutionEffect::Completion,
                    now,
                ));
                effects += 1;
                let before = s.snapshot();
                assert_eq!(
                    ok(s.apply_effect(
                        s.revision(),
                        AppliedExecutionEffectId::new(effects),
                        rider,
                        active.action_id,
                        active.generation,
                        ExecutionEffect::Completion,
                        now
                    )),
                    EffectOutcome::AlreadyApplied
                );
                effects += 1;
                assert_eq!(s.data(), before.data());
                assert_eq!(s.execution(), before.execution());
                assert_eq!(s.revision(), before.revision());
                assert_eq!(s.history(), before.history());
                let expected_load=s.data().fulfillment.iter().filter(|(_,status)|matches!(status,FulfillmentState::PickedUp{rider:r,..} if *r==rider)).map(|(order,_)|s.data().orders[order].demand.value()).sum::<u64>();
                assert_eq!(ok(onboard_load(s.data(), rider)).value(), expected_load);
                assert_eq!(s.data().accepted, accepted);
                ok(s.validate());
                anchor = target;
            }
        }
        assert!(s.data().assignments.is_empty());
        assert!(matches!(
            s.data().fulfillment[&OrderId::new(1)],
            FulfillmentState::Delivered { .. }
        ));
        assert!(matches!(
            s.data().fulfillment[&OrderId::new(2)],
            FulfillmentState::Delivered { .. }
        ));
        history_valid(&s);
    }
}

#[test]
fn pickup_progress_wins_over_an_evaluated_fleet_proposal() {
    let (f, mut s) = committed_fixture();
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let rider = s.data().assignments[&OrderId::new(2)].rider;
    let origin = f.anchors.riders[&rider].clone();
    let target = f.anchors.pickups[&OrderId::new(2)].clone();
    ok(s.start_action(
        s.revision(),
        rider,
        s.plan_revisions()[&rider],
        leg(&provider, origin.node, target.node),
        &origin,
        target,
    ));
    let active = s.execution()[&rider].clone();
    ok(s.observe_execution(rider, ExecutionEffect::Arrival, at(0.0)));
    ok(s.observe_ready(OrderId::new(2), at(15.0)));
    ok(s.start_service(s.revision(), rider, at(15.0), secs(2.0)));
    let mut inputs = f.inputs(&s, at(15.0));
    inputs.identity.optimizer = "fleet-greedy-local/v1".into();
    let frozen = ok(s.frozen_prefix(rider, &inputs, at(15.0)));
    inputs.projections.insert(
        rider,
        ok(project_execution(s.data(), rider, at(15.0), origin, frozen)),
    );
    adopt(&mut s, &f, &provider, inputs.clone());
    let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(15.0)));
    let detached = s.snapshot();
    let evaluated = ok(detached.evaluate_fleet(
        &planning(&f, &provider, &clock),
        &[OrderId::new(3)],
        FleetInputs {
            pooling: inputs,
            unavailable_projections: std::collections::BTreeMap::default(),
            algorithm: FleetAlgorithm::MultiStartLocal,
        },
    ));
    let proof = ok(s.certify(evaluated, &provider, at(16.0)));
    ok(s.apply_effect(
        s.revision(),
        AppliedExecutionEffectId::new(100),
        rider,
        active.action_id,
        active.generation,
        ExecutionEffect::Completion,
        at(17.0),
    ));
    ok(clock.advance(at(17.0)));
    let before = s.snapshot();
    assert!(matches!(
        s.publish_temporal(proof, &clock, &provider),
        Err(TemporalError::Stale)
    ));
    assert_eq!(&s, &*before);
    assert!(!s.data().accepted.contains_key(&OrderId::new(3)));
    assert!(!s.data().assignments.contains_key(&OrderId::new(3)));
    history_valid(&s);
}

#[test]
fn every_adopted_context_category_invalidates_publication_atomically() {
    for category in ["graph", "traffic", "readiness", "profile", "policies"] {
        let f = Fixture::new();
        let mut s = state(&f);
        let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
        let inputs = f.inputs(&s, at(0.0));
        adopt(&mut s, &f, &provider, inputs.clone());
        let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
        let decision =
            ok(s.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(2), inputs));
        let proof = ok(s.certify(decision, &provider, at(2.0)));
        ok(s.adopt(
            s.revision(),
            AdoptedContextIdentity {
                category: category.into(),
                content: "new-authoritative-content".into(),
            },
        ));
        let before = s.snapshot();
        assert!(matches!(
            s.publish_temporal(proof, &clock, &provider),
            Err(TemporalError::Stale)
        ));
        assert_eq!(&s, &*before);
        history_valid(&s);
    }
}

#[test]
fn observed_readiness_remains_authoritative_when_its_unused_forecast_expires() {
    let mut f = Fixture::new();
    for ready in f.data.readiness.values_mut() {
        ready.observed_at = Some(at(0.0));
    }
    let mut s = state(&f);
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let mut inputs = f.inputs(&s, at(0.0));
    for forecast in inputs.forecasts.values_mut() {
        forecast.valid_until = at(0.0);
    }
    adopt(&mut s, &f, &provider, inputs.clone());
    let clock = ok(LogicalOperationalClock::new("fixture/v1".into(), at(0.0)));
    let decision =
        ok(s.evaluate_insertion(&planning(&f, &provider, &clock), OrderId::new(2), inputs));
    let proof = ok(s.certify(decision, &provider, at(2.0)));
    ok(clock.advance(at(1.0)));
    ok(s.publish_temporal(proof, &clock, &provider));
    history_valid(&s);
}
