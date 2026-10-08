use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use roadrunner_core::cost::TrafficSnapshot;
use roadrunner_core::geo::{Meters, Seconds};
use roadrunner_core::graph::{EdgeId, FrozenGraph, NodeId};
use roadrunner_dispatch::{
    AssignmentDecision, Availability, CandidatePolicy, CandidateResult, CapacityUnits,
    CoreRouteProvider, DecisionId, DispatchDecisionOutcome, DispatchInstant, DispatchSnapshot,
    DispatchStrategy, Order, OrderId, OrderReadiness, PreparationAwareStrategy, RiderId, RiderPlan,
    RiderProfile, RiderState, RouteOutcome, RouteProvider, RoutingAnchor, RoutingAnchors,
    RoutingEpoch, TrafficContext, TrafficIdentity, World, WorldData, basic_dispatch,
    preparation_aware_dispatch, strategy_dispatch,
};

use crate::metrics::{SimulationInsertionRecord, summarize};
use crate::scenario::ReadinessRng;
use crate::{
    ActualReadiness, DispatchPolicy, ExecutedLeg, OrderOutcome, RecordedEvent, RiderMetrics,
    SimulationError, SimulationEvent, SimulationResult, SimulationScenario, TrafficOverride,
};
use roadrunner_dispatch::{
    FleetContext, FleetInputs, FrozenPrefix, InsertionDecision, PoolingContext, PoolingInputs,
    PredictionIdentity, ReadinessForecast, Stop, StopTimeline, effective_readiness, insert_order,
    optimize_fleet, project_execution,
};

fn invalid(message: &str) -> SimulationError {
    SimulationError::InvalidScenario(message.into())
}
fn instant(seconds: Seconds) -> Result<DispatchInstant, SimulationError> {
    Ok(DispatchInstant::new(seconds.value())?)
}

#[derive(Debug)]
enum Action {
    Traffic(TrafficSnapshot),
    Create(OrderId),
    Ready(OrderId),
    Assign(OrderId),
    Fleet,
    Recover,
    Dynamic(crate::DynamicChange),
    Move {
        order: OrderId,
        rider: RiderId,
        leg: ExecutedLeg,
        pickup: bool,
        execution_id: u64,
    },
    Arrive(OrderId, RiderId, u64),
    Pickup(OrderId, RiderId, u64),
    Deliver(OrderId, RiderId, u64),
}

#[derive(Debug)]
struct QueuedEvent {
    at: DispatchInstant,
    sequence: u64,
    action: Action,
}
impl PartialEq for QueuedEvent {
    fn eq(&self, other: &Self) -> bool {
        self.at == other.at && self.sequence == other.sequence
    }
}
// DispatchInstant is validated finite/non-negative, so exact equality is reflexive.
impl Eq for QueuedEvent {}
impl PartialOrd for QueuedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for QueuedEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .at
            .value()
            .total_cmp(&self.at.value())
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

#[derive(Clone)]
struct PreparedOrder {
    order: Order,
    readiness: OrderReadiness,
    actual_ready_at: DispatchInstant,
}

#[derive(Clone)]
struct ActiveStop {
    id: u64,
    stop: Stop,
    arrival: DispatchInstant,
    service_end: Option<DispatchInstant>,
    service_wait: Option<Seconds>,
    anchor: RoutingAnchor,
    leg: Option<ExecutedLeg>,
}

struct Engine<'a> {
    graph: &'a FrozenGraph,
    scenario: &'a SimulationScenario,
    now: DispatchInstant,
    end: DispatchInstant,
    epoch: RoutingEpoch,
    world: World,
    traffic: TrafficSnapshot,
    anchors: RoutingAnchors,
    queue: BinaryHeap<QueuedEvent>,
    next_sequence: u64,
    next_decision: u64,
    prepared: BTreeMap<OrderId, PreparedOrder>,
    pending: BTreeSet<OrderId>,
    queued_assignments: BTreeSet<OrderId>,
    waiting: BTreeMap<OrderId, RiderId>,
    busy_since: BTreeMap<RiderId, DispatchInstant>,
    rider_metrics: BTreeMap<RiderId, RiderMetrics>,
    outcomes: BTreeMap<OrderId, OrderOutcome>,
    events: Vec<RecordedEvent>,
    decisions: Vec<AssignmentDecision>,
    insertions: Vec<SimulationInsertionRecord>,
    fleets: Vec<crate::SimulationFleetRecord>,
    fleet_queued: bool,
    recovery_queued: bool,
    recovery_triggers: BTreeSet<String>,
    last_recovery: Option<DispatchInstant>,
    forecast_generated: BTreeMap<OrderId, DispatchInstant>,
    recoveries: Vec<crate::SimulationRecoveryRecord>,
    active: BTreeMap<RiderId, ActiveStop>,
    next_execution: u64,
    prediction_failures: Vec<crate::SimulationPredictionFailure>,
}

/// Executes a validated fixed scenario through its inclusive horizon without sleeping.
///
/// All seed-derived readiness is materialized before dispatch. Every invocation starts
/// from fresh equivalent world state. Inputs and graph are never mutated.
///
/// # Errors
/// Rejects invalid scenario/graph binding, shared evaluation/transition failures,
/// invalid traffic, clock/metric overflow, and sequence exhaustion.
pub fn simulate(
    graph: &FrozenGraph,
    scenario: &SimulationScenario,
) -> Result<SimulationResult, SimulationError> {
    Engine::new(graph, scenario)?.run()
}

fn traffic(
    graph: &FrozenGraph,
    overrides: &[TrafficOverride],
) -> Result<TrafficSnapshot, SimulationError> {
    Ok(TrafficSnapshot::new(
        graph,
        overrides
            .iter()
            .map(|o| (EdgeId::new(o.edge_id), o.multiplier)),
    )?)
}

impl<'a> Engine<'a> {
    fn anchor(&self, node: u32) -> Result<RoutingAnchor, SimulationError> {
        let node = NodeId::new(node);
        Ok(RoutingAnchor {
            node,
            coordinate: self
                .graph
                .node(node)
                .ok_or_else(|| invalid("absent graph node"))?
                .coordinate(),
            graph_digest: self.graph.metadata().snapshot_digest().to_owned(),
        })
    }

    fn new(
        graph: &'a FrozenGraph,
        scenario: &'a SimulationScenario,
    ) -> Result<Self, SimulationError> {
        let start = instant(scenario.start_seconds)?;
        let end = instant(scenario.end_seconds)?;
        let epoch = RoutingEpoch(instant(scenario.routing_epoch_seconds)?);
        let pooling = matches!(
            scenario.dispatch,
            DispatchPolicy::MultiOrder { .. }
                | DispatchPolicy::FleetBatch { .. }
                | DispatchPolicy::Dynamic { .. }
        );
        let schema = if matches!(scenario.dispatch, DispatchPolicy::Dynamic { .. }) {
            4
        } else if matches!(scenario.dispatch, DispatchPolicy::FleetBatch { .. }) {
            3
        } else if pooling {
            2
        } else {
            1
        };
        if scenario.schema_version != schema || end < start {
            return Err(invalid(
                "schema must be 4 for dynamic, 3 for fleet_batch, 2 for multi_order, 1 for legacy, and horizon must not precede start",
            ));
        }
        if pooling && scenario.scenario_id.as_ref().is_none_or(String::is_empty) {
            return Err(invalid(
                "pooling scenarios require a named versioned scenario_id",
            ));
        }
        epoch.departure_seconds(start)?;
        if scenario
            .graph_snapshot_digest
            .as_ref()
            .is_some_and(|d| d != graph.metadata().snapshot_digest())
        {
            return Err(invalid("graph snapshot digest mismatch"));
        }
        if let DispatchPolicy::PreparationAware {
            idle_penalty_weight,
        } = scenario.dispatch
        {
            PreparationAwareStrategy::new(idle_penalty_weight)?;
        }
        if let DispatchPolicy::Dynamic { recovery, .. } = scenario.dispatch {
            if recovery.version != 1 {
                return Err(invalid("unsupported recovery policy version"));
            }
        }
        let world = World::new(15, WorldData::default())?;
        let mut engine = Self {
            graph,
            scenario,
            now: start,
            end,
            epoch,
            world,
            traffic: traffic(graph, &scenario.initial_traffic)?,
            anchors: RoutingAnchors::default(),
            queue: BinaryHeap::new(),
            next_sequence: 0,
            next_decision: 0,
            prepared: BTreeMap::new(),
            pending: BTreeSet::new(),
            queued_assignments: BTreeSet::new(),
            waiting: BTreeMap::new(),
            busy_since: BTreeMap::new(),
            rider_metrics: BTreeMap::new(),
            outcomes: BTreeMap::new(),
            events: Vec::new(),
            decisions: Vec::new(),
            insertions: Vec::new(),
            fleets: Vec::new(),
            fleet_queued: false,
            recovery_queued: false,
            recovery_triggers: BTreeSet::new(),
            last_recovery: None,
            forecast_generated: BTreeMap::new(),
            recoveries: Vec::new(),
            active: BTreeMap::new(),
            next_execution: 0,
            prediction_failures: Vec::new(),
        };
        engine.initialize_riders()?;
        engine.initialize_orders()?;
        engine.initialize_events()?;
        Ok(engine)
    }

    fn initialize_riders(&mut self) -> Result<(), SimulationError> {
        let mut data = WorldData::default();
        for input in &self.scenario.riders {
            let rider = RiderId::new(input.id);
            let origin = self.anchor(input.node)?;
            if data
                .profiles
                .insert(
                    rider,
                    RiderProfile {
                        id: rider,
                        routing_profile: self.graph.metadata().routing_profile().to_owned(),
                        max_capacity: CapacityUnits::new(input.capacity),
                    },
                )
                .is_some()
            {
                return Err(invalid("duplicate rider identity"));
            }
            data.riders.insert(
                rider,
                RiderState {
                    coordinate: origin.coordinate,
                    availability: if input.available {
                        Availability::Available
                    } else {
                        Availability::Unavailable
                    },
                },
            );
            data.plans.insert(rider, RiderPlan::default());
            self.anchors.riders.insert(rider, origin);
            self.rider_metrics.insert(
                rider,
                RiderMetrics {
                    rider,
                    available: input.available || scenario_dynamic(self.scenario),
                    busy_seconds: Seconds::ZERO,
                    idle_seconds: None,
                    completed_distance_meters: Meters::ZERO,
                },
            );
        }
        self.world = World::new(15, data)?;
        Ok(())
    }

    fn validate_admission(&self, input: &crate::OrderInput) -> Result<(), SimulationError> {
        if matches!(
            self.scenario.dispatch,
            DispatchPolicy::MultiOrder { .. }
                | DispatchPolicy::FleetBatch { .. }
                | DispatchPolicy::Dynamic { .. }
        ) && input.admission.is_none()
        {
            return Err(invalid(
                "multi_order requires explicit per-order admission policies",
            ));
        }
        if let Some(policy) = &input.admission {
            if !matches!(
                self.scenario.dispatch,
                DispatchPolicy::MultiOrder { .. }
                    | DispatchPolicy::FleetBatch { .. }
                    | DispatchPolicy::Dynamic { .. }
            ) {
                return Err(invalid(
                    "legacy schema cannot apply Phase 17 admission terms",
                ));
            }
            policy.validate()?;
            if policy.readiness == roadrunner_dispatch::ReadinessRule::LegacyV1
                || (policy.deadline == roadrunner_dispatch::AdmissionDeadline::Hard
                    && input.deadline_seconds.is_none())
            {
                return Err(invalid(
                    "new pooling policies require explicit readiness and hard deadline data",
                ));
            }
        }
        Ok(())
    }

    fn initialize_orders(&mut self) -> Result<(), SimulationError> {
        let mut inputs: Vec<_> = self.scenario.orders.iter().collect();
        inputs.sort_by_key(|o| o.id);
        let mut rng = ReadinessRng::new(self.scenario.seed);
        for input in inputs {
            let id = OrderId::new(input.id);
            if self.prepared.contains_key(&id) {
                return Err(invalid("duplicate order identity"));
            }
            let created_at = instant(input.created_at_seconds)?;
            if created_at < self.now {
                return Err(invalid("order creation precedes simulation start"));
            }
            let actual_ready_at = match input.actual_readiness {
                ActualReadiness::Fixed { at_seconds } => instant(at_seconds)?,
                ActualReadiness::SeededDelay {
                    min_seconds,
                    max_seconds,
                } => {
                    if max_seconds < min_seconds {
                        return Err(invalid("reversed seeded readiness interval"));
                    }
                    let delay = Seconds::new(
                        min_seconds.value()
                            + (max_seconds.value() - min_seconds.value()) * rng.uniform(),
                    )?;
                    created_at.checked_add(delay)?
                }
            };
            self.validate_admission(input)?;
            let readiness = OrderReadiness {
                expected_at: input.expected_ready_at_seconds.map(instant).transpose()?,
                observed_at: None,
            };
            if matches!(
                self.scenario.dispatch,
                DispatchPolicy::PreparationAware { .. } | DispatchPolicy::LowestCompletionTime
            ) && readiness.expected_at.is_none()
                && actual_ready_at > created_at
            {
                return Err(invalid(
                    "preparation-aware orders need a forecast until actual readiness is known",
                ));
            }
            let pickup = self.anchor(input.pickup_node)?;
            let dropoff = self.anchor(input.dropoff_node)?;
            let order = Order {
                id,
                pickup: pickup.coordinate,
                dropoff: dropoff.coordinate,
                created_at,
                deadline: input.deadline_seconds.map(instant).transpose()?,
                demand: CapacityUnits::new(input.demand),
            };
            self.outcomes.insert(
                id,
                OrderOutcome {
                    order: id,
                    realized_ready_at: actual_ready_at,
                    created_at: None,
                    observed_ready_at: None,
                    assigned_at: None,
                    rider: None,
                    predicted_eta: None,
                    pickup_arrival: None,
                    picked_up_at: None,
                    delivered_at: None,
                    cancelled_at: None,
                    deadline: order.deadline,
                    partial_wait_seconds: None,
                },
            );
            self.prepared.insert(
                id,
                PreparedOrder {
                    order,
                    readiness,
                    actual_ready_at,
                },
            );
            // Order anchors become visible only at creation, not before future requests exist.
        }
        Ok(())
    }

    fn initialize_events(&mut self) -> Result<(), SimulationError> {
        let mut external = Vec::new();
        for (id, prepared) in &self.prepared {
            external.push((
                prepared.order.created_at,
                1_u8,
                id.value(),
                Action::Create(*id),
            ));
            let receipt = if prepared.actual_ready_at > prepared.order.created_at {
                prepared.actual_ready_at
            } else {
                prepared.order.created_at
            };
            external.push((receipt, 2, id.value(), Action::Ready(*id)));
        }
        for (index, change) in self.scenario.traffic_changes.iter().enumerate() {
            let at = instant(change.at_seconds)?;
            if at < self.now {
                return Err(invalid("traffic event precedes simulation start"));
            }
            external.push((
                at,
                0,
                u64::try_from(index).map_err(|_| SimulationError::SequenceExhausted)?,
                Action::Traffic(traffic(self.graph, &change.overrides)?),
            ));
        }
        if !scenario_dynamic(self.scenario) && !self.scenario.dynamic_events.is_empty() {
            return Err(invalid("dynamic_events require schema 4 dynamic policy"));
        }
        for (index, event) in self.scenario.dynamic_events.iter().enumerate() {
            let at = instant(event.at_seconds)?;
            if at < self.now {
                return Err(invalid("dynamic event precedes start"));
            }
            let known = match &event.change {
                crate::DynamicChange::Availability { rider, .. }
                | crate::DynamicChange::RoadDelay { rider, .. } => {
                    self.world.data().riders.contains_key(&RiderId::new(*rider))
                }
                crate::DynamicChange::Forecast { order, .. }
                | crate::DynamicChange::Cancel { order } => self
                    .prepared
                    .get(&OrderId::new(*order))
                    .is_some_and(|o| o.order.created_at <= at),
            };
            if !known {
                return Err(invalid(
                    "dynamic event refers to absent/not-yet-created entity",
                ));
            }
            external.push((
                at,
                3,
                u64::try_from(index).map_err(|_| SimulationError::SequenceExhausted)?,
                Action::Dynamic(event.change.clone()),
            ));
        }
        external.sort_by(|a, b| {
            a.0.value()
                .total_cmp(&b.0.value())
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
        });
        for (at, _, _, action) in external {
            self.push(at, action)?;
        }
        Ok(())
    }

    fn push(&mut self, at: DispatchInstant, action: Action) -> Result<(), SimulationError> {
        if at < self.now {
            return Err(invalid("event scheduling would reverse simulation time"));
        }
        let sequence = self.next_sequence;
        self.next_sequence = sequence
            .checked_add(1)
            .ok_or(SimulationError::SequenceExhausted)?;
        self.queue.push(QueuedEvent {
            at,
            sequence,
            action,
        });
        Ok(())
    }

    fn queue_pending(&mut self) -> Result<(), SimulationError> {
        if scenario_dynamic(self.scenario) {
            return self.queue_recovery("domain-update");
        }
        if matches!(self.scenario.dispatch, DispatchPolicy::FleetBatch { .. }) {
            if !self.pending.is_empty() && !self.fleet_queued {
                self.fleet_queued = true;
                self.push(self.now, Action::Fleet)?;
            }
            return Ok(());
        }
        let pending: Vec<_> = self.pending.iter().copied().collect();
        for order in pending {
            if self.queued_assignments.insert(order) {
                self.push(self.now, Action::Assign(order))?;
            }
        }
        Ok(())
    }

    fn provider(&self) -> Result<CoreRouteProvider<'_>, SimulationError> {
        Ok(CoreRouteProvider::new(
            self.graph,
            TrafficContext::Static(&self.traffic),
        )?)
    }

    fn outcome(&mut self, order: OrderId) -> Result<&mut OrderOutcome, SimulationError> {
        self.outcomes
            .get_mut(&order)
            .ok_or_else(|| invalid("absent outcome"))
    }

    fn create(&mut self, id: OrderId) -> Result<SimulationEvent, SimulationError> {
        let prepared = self
            .prepared
            .get(&id)
            .cloned()
            .ok_or_else(|| invalid("absent order input"))?;
        self.world
            .register_order(prepared.order, prepared.readiness)?;
        let input = self
            .scenario
            .orders
            .iter()
            .find(|o| o.id == id.value())
            .ok_or_else(|| invalid("absent input"))?;
        self.anchors
            .pickups
            .insert(id, self.anchor(input.pickup_node)?);
        self.anchors
            .dropoffs
            .insert(id, self.anchor(input.dropoff_node)?);
        self.outcome(id)?.created_at = Some(self.now);
        self.pending.insert(id);
        if scenario_dynamic(self.scenario) {
            self.queue_recovery("order-created")?;
        } else {
            self.queue_pending()?;
        }
        Ok(SimulationEvent::OrderCreated { order: id })
    }

    fn ready(&mut self, id: OrderId) -> Result<SimulationEvent, SimulationError> {
        let at = self
            .prepared
            .get(&id)
            .ok_or_else(|| invalid("absent readiness"))?
            .actual_ready_at;
        self.world.observe_ready(id, at)?;
        self.outcome(id)?.observed_ready_at = Some(at);
        if let Some(rider) = self.waiting.remove(&id) {
            self.schedule_service(id, rider, true)?;
        }
        if scenario_dynamic(self.scenario) {
            self.queue_recovery("readiness-observed")?;
        } else {
            self.queue_pending()?;
        }
        Ok(SimulationEvent::OrderReady {
            order: id,
            ready_at: at,
        })
    }

    fn record_prediction_failure(
        &mut self,
        order: OrderId,
        unavailable_order: OrderId,
    ) -> SimulationEvent {
        self.prediction_failures
            .push(crate::SimulationPredictionFailure {
                order,
                unavailable_order,
                at: self.now,
                world_identity: self.world.identity(),
                world_version: self.world.version(),
                coverage: crate::PredictionFailureCoverage {
                    input_complete: false,
                    riders_complete: false,
                    search_complete: false,
                },
                work: 0,
                reason: "PredictionUnavailable/v1".into(),
                committed: false,
            });
        SimulationEvent::DispatchPredictionFailed {
            order,
            unavailable_order,
        }
    }

    fn assign(&mut self, order: OrderId) -> Result<SimulationEvent, SimulationError> {
        self.queued_assignments.remove(&order);
        let id = DecisionId::new(self.next_decision);
        self.next_decision = self
            .next_decision
            .checked_add(1)
            .ok_or(SimulationError::SequenceExhausted)?;
        if matches!(self.scenario.dispatch, DispatchPolicy::MultiOrder { .. }) {
            return match self.assign_pooled(order, id) {
                Err(SimulationError::Pooling(
                    roadrunner_dispatch::PoolingError::PredictionUnavailable(unavailable_order),
                )) => Ok(self.record_prediction_failure(order, unavailable_order)),
                other => other,
            };
        }
        let decision = {
            let provider = self.provider()?;
            let snapshot =
                DispatchSnapshot::new(&self.world, self.now, self.epoch, &provider, &self.anchors)?;
            match self.scenario.dispatch {
                DispatchPolicy::MultiOrder { .. }
                | DispatchPolicy::FleetBatch { .. }
                | DispatchPolicy::Dynamic { .. } => {
                    return Err(invalid("pooling branch invariant"));
                }
                DispatchPolicy::Basic => {
                    basic_dispatch(&snapshot, order, id, CandidatePolicy::Exhaustive)?
                }
                DispatchPolicy::NearestRider
                | DispatchPolicy::LowestPickupEta
                | DispatchPolicy::LowestCompletionTime => strategy_dispatch(
                    &snapshot,
                    order,
                    id,
                    CandidatePolicy::Exhaustive,
                    match self.scenario.dispatch {
                        DispatchPolicy::NearestRider => DispatchStrategy::NearestRider,
                        DispatchPolicy::LowestPickupEta => DispatchStrategy::LowestPickupEta,
                        _ => DispatchStrategy::LowestCompletionTime,
                    },
                )?,
                DispatchPolicy::PreparationAware {
                    idle_penalty_weight,
                } => preparation_aware_dispatch(
                    &snapshot,
                    order,
                    id,
                    CandidatePolicy::Exhaustive,
                    PreparationAwareStrategy::new(idle_penalty_weight)?,
                )?,
            }
        };
        let event = if let DispatchDecisionOutcome::Assigned(proposal) = decision.outcome() {
            let rider = proposal.rider;
            let predicted = decision
                .evidence()
                .candidates
                .iter()
                .find_map(|c| match &c.result {
                    CandidateResult::Feasible { evaluation, .. } if c.rider == rider => {
                        Some(evaluation.completion_time)
                    }
                    _ => None,
                })
                .ok_or_else(|| invalid("missing selected evaluation"))?;
            self.world.commit(&decision)?;
            self.pending.remove(&order);
            self.busy_since.entry(rider).or_insert(self.now);
            let now = self.now;
            let outcome = self.outcome(order)?;
            outcome.assigned_at = Some(now);
            outcome.rider = Some(rider);
            outcome.predicted_eta = Some(predicted);
            self.schedule_next(rider)?;
            SimulationEvent::RiderAssigned {
                order,
                rider,
                decision: id,
            }
        } else {
            SimulationEvent::DispatchUnassigned {
                order,
                decision: id,
            }
        };
        self.decisions.push(decision);
        Ok(event)
    }

    fn frozen_projection(
        &self,
        active: &ActiveStop,
        inputs: &PoolingInputs,
    ) -> Result<FrozenPrefix, SimulationError> {
        let p = &inputs.policies[&active.stop.order()];
        let (wait, service) = match active.stop {
            Stop::Pickup(o) => {
                let (ready, _) = effective_readiness(self.world.data(), inputs, o, self.now)?;
                (
                    if ready > active.arrival {
                        ready.duration_since(active.arrival)?
                    } else {
                        Seconds::ZERO
                    },
                    p.pickup_service,
                )
            }
            Stop::Dropoff(_) => (Seconds::ZERO, p.dropoff_service),
        };
        let mut timeline = StopTimeline::new(active.stop, active.arrival, wait, service)?;
        // Service already started is pinned to its actual completion, including forecast error.
        if let Some(end) = active.service_end {
            timeline.waiting = active
                .service_wait
                .ok_or_else(|| invalid("missing frozen service wait"))?;
            timeline.departure = end;
        }
        if timeline.departure < self.now {
            return Err(roadrunner_dispatch::PoolingError::PredictionUnavailable(
                active.stop.order(),
            )
            .into());
        }
        Ok(FrozenPrefix {
            execution_id: active.id,
            timeline,
            anchor: active.anchor.clone(),
        })
    }

    fn pooling_inputs(&self) -> Result<PoolingInputs, SimulationError> {
        Ok(self.planning_inputs()?.0)
    }

    fn pooling_configuration(&self) -> Result<(u64, Seconds, bool), SimulationError> {
        Ok(match self.scenario.dispatch {
            DispatchPolicy::MultiOrder {
                work_budget,
                forecast_validity_seconds,
            } => (work_budget, forecast_validity_seconds, false),
            DispatchPolicy::FleetBatch {
                work_budget,
                forecast_validity_seconds,
                ..
            }
            | DispatchPolicy::Dynamic {
                work_budget,
                forecast_validity_seconds,
                ..
            } => (work_budget, forecast_validity_seconds, true),
            _ => return Err(invalid("not a pooling scenario")),
        })
    }

    fn planning_inputs(
        &self,
    ) -> Result<
        (
            PoolingInputs,
            BTreeMap<RiderId, roadrunner_dispatch::UnavailableExecution>,
        ),
        SimulationError,
    > {
        let (work_budget, forecast_validity_seconds, fleet) = self.pooling_configuration()?;
        let mut unavailable = BTreeMap::new();
        let identity = PredictionIdentity {
            prediction: format!(
                "{}:readiness/v1",
                self.scenario.scenario_id.as_deref().unwrap_or("legacy")
            ),
            service: "per-order-deterministic/v1".into(),
            optimizer: if fleet {
                "fleet-greedy-local/v1"
            } else {
                "exhaustive-insertion/v1"
            }
            .into(),
            routing: self.provider()?.provenance(),
        };
        let mut inputs = PoolingInputs {
            identity,
            policies: BTreeMap::new(),
            forecasts: BTreeMap::new(),
            projections: BTreeMap::new(),
            work_budget,
        };
        for input in &self.scenario.orders {
            let o = OrderId::new(input.id);
            if !self.world.data().orders.contains_key(&o) {
                continue;
            }
            inputs.policies.insert(
                o,
                input
                    .admission
                    .clone()
                    .ok_or_else(|| invalid("missing admission policy"))?,
            );
            if let Some(expected_at) = self.world.data().readiness[&o].expected_at {
                let generated_at = self
                    .forecast_generated
                    .get(&o)
                    .copied()
                    .unwrap_or(self.world.data().orders[&o].created_at);
                inputs.forecasts.insert(
                    o,
                    ReadinessForecast {
                        id: format!("scenario-order-{}/v1", input.id),
                        generated_at,
                        valid_until: generated_at.checked_add(forecast_validity_seconds)?,
                        expected_at,
                    },
                );
            }
        }
        for rider in self.world.data().profiles.keys().copied() {
            if self.world.data().riders[&rider].availability != Availability::Available
                && !scenario_dynamic(self.scenario)
            {
                continue;
            }
            let frozen = if let Some(active) = self.active.get(&rider) {
                match self.frozen_projection(active, &inputs) {
                    Ok(frozen) => Some(frozen),
                    Err(SimulationError::Pooling(
                        roadrunner_dispatch::PoolingError::PredictionUnavailable(o),
                    )) if fleet => {
                        unavailable.insert(
                            rider,
                            roadrunner_dispatch::UnavailableExecution {
                                execution_id: active.id,
                                order: o,
                                arrival: active.arrival,
                                anchor: active.anchor.clone(),
                            },
                        );
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            } else {
                None
            };
            inputs.projections.insert(
                rider,
                project_execution(
                    self.world.data(),
                    rider,
                    self.now,
                    self.anchors.riders[&rider].clone(),
                    frozen,
                )?,
            );
        }
        Ok((inputs, unavailable))
    }

    fn fleet_inputs(&self) -> Result<FleetInputs, SimulationError> {
        let (DispatchPolicy::FleetBatch { algorithm, .. }
        | DispatchPolicy::Dynamic { algorithm, .. }) = self.scenario.dispatch
        else {
            return Err(invalid("not a fleet scenario"));
        };
        let (pooling, unavailable_projections) = self.planning_inputs()?;
        Ok(FleetInputs {
            pooling,
            unavailable_projections,
            algorithm,
        })
    }

    fn queue_recovery(&mut self, trigger: &str) -> Result<(), SimulationError> {
        self.recovery_triggers.insert(trigger.into());
        if !self.recovery_queued {
            self.recovery_queued = true;
            self.push(self.now, Action::Recover)?;
        }
        Ok(())
    }

    fn recovery_context(
        &self,
        trigger: String,
    ) -> Result<roadrunner_dispatch::RecoveryContext, SimulationError> {
        let DispatchPolicy::Dynamic { recovery, .. } = self.scenario.dispatch else {
            return Err(invalid("not a dynamic scenario"));
        };
        let (mut inputs, unavailable) = self.planning_inputs()?;
        if let Some(execution) = unavailable.values().next() {
            return Err(
                roadrunner_dispatch::PoolingError::PredictionUnavailable(execution.order).into(),
            );
        }
        inputs.identity.optimizer = "dynamic-recovery/v1".into();
        let provider = self.provider()?;
        let snapshot =
            DispatchSnapshot::new(&self.world, self.now, self.epoch, &provider, &self.anchors)?;
        Ok(roadrunner_dispatch::RecoveryContext::new(
            &snapshot,
            inputs,
            recovery,
            trigger,
            self.last_recovery,
        )?)
    }

    fn reconcile_responsibility(&mut self) -> Result<(), SimulationError> {
        for rider in self
            .world
            .data()
            .profiles
            .keys()
            .copied()
            .collect::<Vec<_>>()
        {
            if self.world.data().plans[&rider].stops.is_empty() && !self.active.contains_key(&rider)
            {
                if let Some(began) = self.busy_since.remove(&rider) {
                    let metrics = self
                        .rider_metrics
                        .get_mut(&rider)
                        .ok_or_else(|| invalid("absent rider"))?;
                    metrics.busy_seconds = metrics
                        .busy_seconds
                        .checked_add(self.now.duration_since(began)?)?;
                }
            } else {
                self.busy_since.entry(rider).or_insert(self.now);
            }
        }
        for assignment in self.world.data().assignments.values() {
            self.outcomes
                .get_mut(&assignment.order)
                .ok_or_else(|| invalid("absent order"))?
                .rider = Some(assignment.rider);
        }
        Ok(())
    }

    fn plan_recovery(&mut self) -> Result<SimulationEvent, SimulationError> {
        self.recovery_queued = false;
        let trigger = self
            .recovery_triggers
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("+");
        self.recovery_triggers.clear();
        let evaluated = (|| -> Result<roadrunner_dispatch::RecoveryDecision, SimulationError> {
            let context = self.recovery_context(trigger.clone())?;
            let provider = self.provider()?;
            let snapshot =
                DispatchSnapshot::new(&self.world, self.now, self.epoch, &provider, &self.anchors)?;
            Ok(roadrunner_dispatch::recover_fleet(&snapshot, context)?)
        })();
        let (decision, failure, committed, result) = match evaluated {
            Ok(decision) => {
                let committed = decision.proposal().is_some();
                if committed {
                    let current = self.recovery_context(trigger.clone())?;
                    self.world.commit_recovery(&decision, &current)?;
                    self.last_recovery = Some(self.now);
                    self.reconcile_responsibility()?;
                }
                let result = format!("{:?}", decision.termination);
                (Some(decision), None, committed, result)
            }
            Err(SimulationError::Pooling(
                error @ (roadrunner_dispatch::PoolingError::PredictionUnavailable(_)
                | roadrunner_dispatch::PoolingError::BaselineUnavailable(_)),
            )) => {
                let reason = format!("{error:?}");
                (None, Some(reason.clone()), false, reason)
            }
            Err(error) => return Err(error),
        };
        self.recoveries.push(crate::SimulationRecoveryRecord {
            decision,
            failure,
            committed,
            world_version_after: self.world.version(),
        });
        // Start only current plan work. A failed recovery never abandons obligations.
        for rider in self
            .world
            .data()
            .profiles
            .keys()
            .copied()
            .collect::<Vec<_>>()
        {
            if !self.active.contains_key(&rider)
                && !self.world.data().plans[&rider].stops.is_empty()
            {
                self.schedule_next(rider)?;
            }
        }
        if !self.pending.is_empty() && !self.fleet_queued {
            self.fleet_queued = true;
            self.push(self.now, Action::Fleet)?;
        }
        Ok(SimulationEvent::RecoveryPlanned {
            trigger,
            committed,
            result,
        })
    }

    fn dynamic_change(
        &mut self,
        change: crate::DynamicChange,
    ) -> Result<SimulationEvent, SimulationError> {
        use crate::DynamicChange;
        let mut applied = true;
        let mut reason = "Applied".to_owned();
        let trigger = match change {
            DynamicChange::Availability { rider, available } => {
                let r = RiderId::new(rider);
                let state = self.world.data().riders[&r];
                self.world.update_rider_state(
                    r,
                    RiderState {
                        availability: if available {
                            Availability::Available
                        } else {
                            Availability::Unavailable
                        },
                        ..state
                    },
                )?;
                "availability"
            }
            DynamicChange::Forecast {
                order,
                expected_at_seconds,
            } => {
                self.world
                    .estimate_readiness(OrderId::new(order), instant(expected_at_seconds)?)?;
                self.forecast_generated
                    .insert(OrderId::new(order), self.now);
                "readiness-forecast"
            }
            DynamicChange::Cancel { order } => {
                let o = OrderId::new(order);
                let active = self.active.values().map(|a| a.stop.order()).collect();
                match self.world.cancel_order(o, self.now, &active)? {
                    Ok(()) => {
                        self.pending.remove(&o);
                        let now = self.now;
                        self.outcome(o)?.cancelled_at = Some(now);
                        self.reconcile_responsibility()?;
                    }
                    Err(refusal) => {
                        applied = false;
                        reason = format!("{refusal:?}");
                    }
                }
                "cancellation"
            }
            DynamicChange::RoadDelay {
                rider,
                additional_seconds,
            } => {
                let r = RiderId::new(rider);
                let moving = self.active.get(&r).and_then(|a| a.leg.as_ref()).is_some();
                if moving {
                    let id = self.next_execution;
                    self.next_execution = id
                        .checked_add(1)
                        .ok_or(SimulationError::SequenceExhausted)?;
                    let active = self
                        .active
                        .get_mut(&r)
                        .ok_or_else(|| invalid("absent execution"))?;
                    active.id = id; // An explicit observation supersedes the old arrival generation.
                    active.arrival = active.arrival.checked_add(additional_seconds)?;
                    let leg = active.leg.as_mut().ok_or_else(|| invalid("absent leg"))?;
                    leg.travel = leg.travel.checked_add(additional_seconds)?;
                    let action = Action::Move {
                        order: active.stop.order(),
                        rider: r,
                        leg: leg.clone(),
                        pickup: matches!(active.stop, Stop::Pickup(_)),
                        execution_id: id,
                    };
                    let at = active.arrival;
                    self.push(at, action)?;
                } else {
                    applied = false;
                    reason = "NoDepartedRoadLeg".into();
                }
                "observed-road-delay"
            }
        };
        if applied {
            self.queue_recovery(trigger)?;
        }
        Ok(SimulationEvent::DynamicChanged {
            change,
            applied,
            reason,
        })
    }

    fn plan_fleet(&mut self) -> Result<SimulationEvent, SimulationError> {
        self.fleet_queued = false;
        let batch: Vec<_> = self.pending.iter().copied().collect();
        let inputs = self.fleet_inputs()?;
        let decision = {
            let provider = self.provider()?;
            let snapshot =
                DispatchSnapshot::new(&self.world, self.now, self.epoch, &provider, &self.anchors)?;
            optimize_fleet(&snapshot, &batch, inputs)?
        };
        let mut admitted = Vec::new();
        let committed = decision.proposal().is_some();
        if let Some(proposal) = decision.proposal() {
            let current = {
                let provider = self.provider()?;
                let snapshot = DispatchSnapshot::new(
                    &self.world,
                    self.now,
                    self.epoch,
                    &provider,
                    &self.anchors,
                )?;
                FleetContext::new(&snapshot, self.fleet_inputs()?)?
            };
            self.world.commit_fleet(&decision, &current)?;
            for (order, assignment) in proposal.assignments() {
                self.pending.remove(order);
                self.busy_since.entry(assignment.rider).or_insert(self.now);
                let predicted = proposal.evaluations()[&assignment.rider].completions[order]
                    .duration_since(self.now)?;
                let now = self.now;
                let outcome = self.outcome(*order)?;
                outcome.assigned_at = Some(now);
                outcome.rider = Some(assignment.rider);
                outcome.predicted_eta = Some(predicted);
                admitted.push(*assignment);
            }
            for rider in proposal.plans().keys() {
                if !self.active.contains_key(rider)
                    && !self.world.data().plans[rider].stops.is_empty()
                {
                    self.schedule_next(*rider)?;
                }
            }
        }
        self.fleets.push(crate::SimulationFleetRecord {
            decision,
            committed,
            world_version_after: self.world.version(),
        });
        Ok(SimulationEvent::FleetPlanned {
            batch,
            admitted,
            committed,
        })
    }

    fn assign_pooled(
        &mut self,
        order: OrderId,
        id: DecisionId,
    ) -> Result<SimulationEvent, SimulationError> {
        let inputs = self.pooling_inputs()?;
        let decision: InsertionDecision = {
            let provider = self.provider()?;
            let snapshot =
                DispatchSnapshot::new(&self.world, self.now, self.epoch, &provider, &self.anchors)?;
            insert_order(&snapshot, order, inputs)?
        };
        let event = if let Some(p) = decision.proposal() {
            let rider = p.rider();
            let predicted = p.evaluation().completions[&order].duration_since(self.now)?;
            // Rebuild the current complete context, rather than echoing the proposal binding.
            let current = {
                let provider = self.provider()?;
                let snapshot = DispatchSnapshot::new(
                    &self.world,
                    self.now,
                    self.epoch,
                    &provider,
                    &self.anchors,
                )?;
                PoolingContext::new(&snapshot, self.pooling_inputs()?)?
            };
            self.world.commit_insertion(&decision, &current)?;
            self.pending.remove(&order);
            self.busy_since.entry(rider).or_insert(self.now);
            let now = self.now;
            let outcome = self.outcome(order)?;
            outcome.assigned_at = Some(now);
            outcome.rider = Some(rider);
            outcome.predicted_eta = Some(predicted);
            if !self.active.contains_key(&rider) {
                self.schedule_next(rider)?;
            }
            SimulationEvent::RiderAssigned {
                order,
                rider,
                decision: id,
            }
        } else {
            SimulationEvent::DispatchUnassigned {
                order,
                decision: id,
            }
        };
        self.insertions.push(SimulationInsertionRecord {
            committed: decision.proposal().is_some(),
            world_version_after: self.world.version(),
            decision,
        });
        Ok(event)
    }

    fn schedule_next(&mut self, rider: RiderId) -> Result<(), SimulationError> {
        if self.active.contains_key(&rider) {
            return Err(invalid("active execution already exists"));
        }
        let Some(stop) = self.world.data().plans[&rider].stops.first().copied() else {
            return Ok(());
        };
        let (to, pickup) = match stop {
            Stop::Pickup(o) => (self.anchors.pickups[&o].node, true),
            Stop::Dropoff(o) => (self.anchors.dropoffs[&o].node, false),
        };
        self.schedule_leg(
            stop.order(),
            rider,
            self.anchors.riders[&rider].node,
            to,
            pickup,
        )
    }

    fn schedule_service(
        &mut self,
        order: OrderId,
        rider: RiderId,
        pickup: bool,
    ) -> Result<(), SimulationError> {
        let service = if matches!(
            self.scenario.dispatch,
            DispatchPolicy::MultiOrder { .. }
                | DispatchPolicy::FleetBatch { .. }
                | DispatchPolicy::Dynamic { .. }
        ) {
            let p = self
                .scenario
                .orders
                .iter()
                .find(|o| o.id == order.value())
                .and_then(|o| o.admission.as_ref())
                .ok_or_else(|| invalid("missing service policy"))?;
            if pickup {
                p.pickup_service
            } else {
                p.dropoff_service
            }
        } else {
            Seconds::ZERO
        };
        let end = self.now.checked_add(service)?;
        let active = self
            .active
            .get_mut(&rider)
            .ok_or_else(|| invalid("missing active execution"))?;
        active.service_end = Some(end);
        active.service_wait = Some(self.now.duration_since(active.arrival)?);
        let id = active.id;
        self.push(
            end,
            if pickup {
                Action::Pickup(order, rider, id)
            } else {
                Action::Deliver(order, rider, id)
            },
        )
    }

    fn schedule_leg(
        &mut self,
        order: OrderId,
        rider: RiderId,
        from: NodeId,
        to: NodeId,
        pickup: bool,
    ) -> Result<(), SimulationError> {
        let provider = self.provider()?;
        let RouteOutcome::RouteFound(leg) =
            provider.route(from, to, self.epoch.departure_seconds(self.now)?)?
        else {
            return Err(SimulationError::NoExecutionRoute);
        };
        let arrival = self.now.checked_add(leg.route.elapsed_travel_time())?;
        let executed = ExecutedLeg {
            from,
            to,
            departed_at: self.now,
            travel: leg.route.elapsed_travel_time(),
            distance: leg.route.total_distance(),
            nodes: leg.route.path().to_vec(),
            edges: leg.route.edges().to_vec(),
            routing: leg.provenance,
        };
        let execution_id = self.next_execution;
        self.next_execution = self
            .next_execution
            .checked_add(1)
            .ok_or(SimulationError::SequenceExhausted)?;
        self.active.insert(
            rider,
            ActiveStop {
                id: execution_id,
                stop: if pickup {
                    Stop::Pickup(order)
                } else {
                    Stop::Dropoff(order)
                },
                arrival,
                service_end: None,
                service_wait: None,
                anchor: self.anchor(to.value())?,
                leg: Some(executed.clone()),
            },
        );
        self.push(
            arrival,
            Action::Move {
                order,
                rider,
                leg: executed,
                pickup,
                execution_id,
            },
        )
    }

    fn moved(
        &mut self,
        order: OrderId,
        rider: RiderId,
        leg: ExecutedLeg,
        pickup: bool,
    ) -> Result<SimulationEvent, SimulationError> {
        let anchor = self.anchor(leg.to.value())?;
        if let Some(active) = self.active.get_mut(&rider) {
            active.leg = None;
        }
        let state = self.world.data().riders[&rider];
        self.world.update_rider_state(
            rider,
            RiderState {
                coordinate: anchor.coordinate,
                ..state
            },
        )?;
        self.anchors.riders.insert(rider, anchor);
        let metrics = self
            .rider_metrics
            .get_mut(&rider)
            .ok_or_else(|| invalid("absent rider metrics"))?;
        metrics.completed_distance_meters = metrics
            .completed_distance_meters
            .checked_add(leg.distance)?;
        if pickup {
            let id = self.active[&rider].id;
            self.push(self.now, Action::Arrive(order, rider, id))?;
        } else {
            self.schedule_service(order, rider, false)?;
        }
        Ok(SimulationEvent::RiderMoved { order, rider, leg })
    }

    fn arrived(
        &mut self,
        order: OrderId,
        rider: RiderId,
    ) -> Result<SimulationEvent, SimulationError> {
        self.outcome(order)?.pickup_arrival = Some(self.now);
        if self.world.data().readiness[&order].observed_at.is_some() {
            self.schedule_service(order, rider, true)?;
        } else {
            self.waiting.insert(order, rider);
        }
        Ok(SimulationEvent::RiderArrivedPickup { order, rider })
    }

    fn pickup(
        &mut self,
        order: OrderId,
        rider: RiderId,
    ) -> Result<SimulationEvent, SimulationError> {
        self.world.pickup(rider, order, self.now)?;
        self.outcome(order)?.picked_up_at = Some(self.now);
        self.active.remove(&rider);
        if scenario_dynamic(self.scenario) {
            self.queue_recovery("pickup-boundary")?;
        } else {
            self.schedule_next(rider)?;
        }
        Ok(SimulationEvent::OrderPickedUp { order, rider })
    }

    fn deliver(
        &mut self,
        order: OrderId,
        rider: RiderId,
    ) -> Result<SimulationEvent, SimulationError> {
        self.world.deliver(rider, order, self.now)?;
        self.outcome(order)?.delivered_at = Some(self.now);
        self.active.remove(&rider);
        if self.world.data().plans[&rider].stops.is_empty() {
            let began = self
                .busy_since
                .remove(&rider)
                .ok_or_else(|| invalid("missing responsibility interval"))?;
            let metrics = self
                .rider_metrics
                .get_mut(&rider)
                .ok_or_else(|| invalid("absent rider"))?;
            metrics.busy_seconds = metrics
                .busy_seconds
                .checked_add(self.now.duration_since(began)?)?;
        } else if !scenario_dynamic(self.scenario) {
            self.schedule_next(rider)?;
        }
        self.queue_pending()?;
        Ok(SimulationEvent::OrderDelivered { order, rider })
    }

    fn process(&mut self, action: Action) -> Result<Option<SimulationEvent>, SimulationError> {
        // Only started work is queued. Its identity survives replacement plans.
        // A duplicate/superseded action cannot mutate any domain or metrics state.
        let execution = match &action {
            Action::Move {
                rider,
                execution_id,
                ..
            } => Some((*rider, *execution_id)),
            Action::Arrive(_, rider, id)
            | Action::Pickup(_, rider, id)
            | Action::Deliver(_, rider, id) => Some((*rider, *id)),
            _ => None,
        };
        if execution.is_some_and(|(r, id)| self.active.get(&r).is_none_or(|a| a.id != id)) {
            return Ok(None);
        }
        let event = match action {
            Action::Traffic(snapshot) => {
                self.traffic = snapshot;
                if scenario_dynamic(self.scenario) {
                    self.queue_recovery("traffic-change")?;
                } else {
                    self.queue_pending()?;
                }
                Ok(SimulationEvent::TrafficChanged {
                    traffic: TrafficIdentity::Static(
                        self.traffic.traffic_snapshot_digest().to_owned(),
                    ),
                })
            }
            Action::Create(id) => self.create(id),
            Action::Ready(id) => self.ready(id),
            Action::Assign(id) => self.assign(id),
            Action::Fleet => self.plan_fleet(),
            Action::Recover => self.plan_recovery(),
            Action::Dynamic(change) => self.dynamic_change(change),
            Action::Move {
                order,
                rider,
                leg,
                pickup,
                execution_id: _,
            } => self.moved(order, rider, leg, pickup),
            Action::Arrive(order, rider, _) => self.arrived(order, rider),
            Action::Pickup(order, rider, _) => self.pickup(order, rider),
            Action::Deliver(order, rider, _) => self.deliver(order, rider),
        }?;
        Ok(Some(event))
    }

    fn run(mut self) -> Result<SimulationResult, SimulationError> {
        while self.queue.peek().is_some_and(|event| event.at <= self.end) {
            let event = self.queue.pop().ok_or_else(|| invalid("queue invariant"))?;
            self.now = event.at;
            let processed = self.process(event.action)?;
            if let Some(processed) = processed {
                self.events.push(RecordedEvent {
                    at: self.now,
                    sequence: event.sequence,
                    event: processed,
                });
            }
        }
        self.finish()
    }

    fn finish(mut self) -> Result<SimulationResult, SimulationError> {
        let start = instant(self.scenario.start_seconds)?;
        let window = self.end.duration_since(start)?;
        for (rider, began) in &self.busy_since {
            let metrics = self
                .rider_metrics
                .get_mut(rider)
                .ok_or_else(|| invalid("absent busy rider"))?;
            metrics.busy_seconds = metrics
                .busy_seconds
                .checked_add(self.end.duration_since(*began)?)?;
        }
        for metrics in self.rider_metrics.values_mut() {
            if metrics.available {
                metrics.idle_seconds =
                    Some(Seconds::new(window.value() - metrics.busy_seconds.value())?);
            }
        }
        for outcome in self.outcomes.values_mut() {
            if outcome.picked_up_at.is_none() {
                outcome.partial_wait_seconds = outcome
                    .pickup_arrival
                    .map(|arrival| self.end.duration_since(arrival))
                    .transpose()?;
            }
        }
        let orders: Vec<_> = self.outcomes.into_values().collect();
        let riders: Vec<_> = self.rider_metrics.into_values().collect();
        let summary = summarize(&orders, &riders, start, self.end)?;
        Ok(SimulationResult {
            scenario_id: self.scenario.scenario_id.clone(),
            schema_version: self.scenario.schema_version,
            randomness: "splitmix64-upper53/v1".into(),
            seed: self.scenario.seed,
            graph_snapshot_digest: self.graph.metadata().snapshot_digest().to_owned(),
            graph_nodes: self.graph.node_count(),
            graph_edges: self.graph.edge_count(),
            dispatch: self.scenario.dispatch,
            utilization_basis: scenario_dynamic(self.scenario)
                .then(|| "fleet-window-responsibility/v1".into()),
            started_at: start,
            ended_at: self.end,
            future_events: self.queue.len(),
            events: self.events,
            decisions: self.decisions,
            prediction_failures: self.prediction_failures,
            realized_protections: crate::metrics::realized_protections(self.world.data()),
            insertions: self.insertions,
            fleets: self.fleets,
            recoveries: self.recoveries,
            final_state: (self.scenario.schema_version >= 2).then(|| self.world.data().clone()),
            orders,
            riders,
            summary,
        })
    }
}

fn scenario_dynamic(scenario: &SimulationScenario) -> bool {
    matches!(scenario.dispatch, DispatchPolicy::Dynamic { .. })
}

#[cfg(test)]
mod tests {
    use super::*;
    use roadrunner_core::graph::{
        GraphBuildIdentity, GraphBuilder, GraphMetadata, GraphSnapshotId,
    };

    #[test]
    fn obsolete_execution_identity_cannot_mutate_world_or_execution() {
        let graph = GraphBuilder::new(
            GraphSnapshotId::new(17),
            GraphMetadata::new(
                "test",
                "test",
                GraphBuildIdentity::new("test", "v1", "v1", "synthetic"),
            ),
        )
        .finalize()
        .unwrap_or_else(|e| panic!("{e}"));
        let scenario: SimulationScenario = serde_json::from_value(serde_json::json!({
            "schema_version":1,"seed":17,"start_seconds":0,"end_seconds":100,
            "routing_epoch_seconds":0,"dispatch":{"kind":"basic"},"riders":[],"orders":[]
        }))
        .unwrap_or_else(|e| panic!("{e}"));
        let mut engine = Engine::new(&graph, &scenario).unwrap_or_else(|e| panic!("{e}"));
        let before = engine.world.data().clone();
        let version = engine.world.version();
        for action in [
            Action::Pickup(OrderId::new(1), RiderId::new(1), 999),
            Action::Deliver(OrderId::new(1), RiderId::new(1), 999),
            Action::Arrive(OrderId::new(1), RiderId::new(1), 999),
        ] {
            assert!(
                engine
                    .process(action)
                    .unwrap_or_else(|e| panic!("{e}"))
                    .is_none()
            );
            assert_eq!(engine.world.data(), &before);
            assert_eq!(engine.world.version(), version);
            assert!(engine.queue.is_empty());
            assert!(engine.waiting.is_empty());
            assert!(engine.active.is_empty());
            assert_eq!(engine.events, [] as [RecordedEvent; 0]);
        }
    }

    #[test]
    fn queue_rejects_reversed_time_and_exhausted_sequences_without_insertion() {
        let graph = GraphBuilder::new(
            GraphSnapshotId::new(15),
            GraphMetadata::new(
                "test",
                "test",
                GraphBuildIdentity::new("test", "v1", "v1", "synthetic"),
            ),
        )
        .finalize()
        .unwrap_or_else(|e| panic!("graph: {e}"));
        let scenario: SimulationScenario = serde_json::from_value(serde_json::json!({
            "schema_version":1, "seed":15, "start_seconds":10, "end_seconds":20,
            "routing_epoch_seconds":0, "dispatch":{"kind":"basic"}, "riders":[], "orders":[],
        }))
        .unwrap_or_else(|e| panic!("scenario: {e}"));
        let mut engine = Engine::new(&graph, &scenario).unwrap_or_else(|e| panic!("engine: {e}"));
        let earlier = DispatchInstant::new(9.0).unwrap_or_else(|e| panic!("time: {e}"));
        assert!(matches!(
            engine.push(earlier, Action::Create(OrderId::new(1))),
            Err(SimulationError::InvalidScenario(_))
        ));
        assert_eq!(engine.next_sequence, 0);
        assert_eq!(engine.queue.len(), 0);
        engine.next_sequence = u64::MAX;
        assert!(matches!(
            engine.push(engine.now, Action::Create(OrderId::new(1))),
            Err(SimulationError::SequenceExhausted)
        ));
        assert_eq!(engine.queue.len(), 0);
        engine.next_decision = u64::MAX;
        assert!(matches!(
            engine.assign(OrderId::new(1)),
            Err(SimulationError::SequenceExhausted)
        ));
        assert!(engine.world.data().orders.is_empty());
    }
}
