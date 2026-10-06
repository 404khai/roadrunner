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

use crate::metrics::summarize;
use crate::scenario::ReadinessRng;
use crate::{
    ActualReadiness, DispatchPolicy, ExecutedLeg, OrderOutcome, RecordedEvent, RiderMetrics,
    SimulationError, SimulationEvent, SimulationResult, SimulationScenario, TrafficOverride,
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
    Move {
        order: OrderId,
        rider: RiderId,
        leg: ExecutedLeg,
        pickup: bool,
    },
    Arrive(OrderId, RiderId),
    Pickup(OrderId, RiderId),
    Deliver(OrderId, RiderId),
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
        if scenario.schema_version != 1 || end < start {
            return Err(invalid(
                "schema must be 1 and horizon must not precede start",
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
                    available: input.available,
                    busy_seconds: Seconds::ZERO,
                    idle_seconds: None,
                    completed_distance_meters: Meters::ZERO,
                },
            );
        }
        self.world = World::new(15, data)?;
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
        self.queue_pending()?;
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
            self.push(self.now, Action::Pickup(id, rider))?;
        }
        self.queue_pending()?;
        Ok(SimulationEvent::OrderReady {
            order: id,
            ready_at: at,
        })
    }

    fn assign(&mut self, order: OrderId) -> Result<SimulationEvent, SimulationError> {
        self.queued_assignments.remove(&order);
        let id = DecisionId::new(self.next_decision);
        self.next_decision = self
            .next_decision
            .checked_add(1)
            .ok_or(SimulationError::SequenceExhausted)?;
        let decision = {
            let provider = self.provider()?;
            let snapshot =
                DispatchSnapshot::new(&self.world, self.now, self.epoch, &provider, &self.anchors)?;
            match self.scenario.dispatch {
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
            self.busy_since.insert(rider, self.now);
            let now = self.now;
            let outcome = self.outcome(order)?;
            outcome.assigned_at = Some(now);
            outcome.rider = Some(rider);
            outcome.predicted_eta = Some(predicted);
            let from = self.anchors.riders[&rider].node;
            let to = self.anchors.pickups[&order].node;
            self.schedule_leg(order, rider, from, to, true)?;
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
        self.push(
            arrival,
            Action::Move {
                order,
                rider,
                leg: executed,
                pickup,
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
        self.push(
            self.now,
            if pickup {
                Action::Arrive(order, rider)
            } else {
                Action::Deliver(order, rider)
            },
        )?;
        Ok(SimulationEvent::RiderMoved { order, rider, leg })
    }

    fn arrived(
        &mut self,
        order: OrderId,
        rider: RiderId,
    ) -> Result<SimulationEvent, SimulationError> {
        self.outcome(order)?.pickup_arrival = Some(self.now);
        if self.world.data().readiness[&order].observed_at.is_some() {
            self.push(self.now, Action::Pickup(order, rider))?;
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
        self.schedule_leg(
            order,
            rider,
            self.anchors.pickups[&order].node,
            self.anchors.dropoffs[&order].node,
            false,
        )?;
        Ok(SimulationEvent::OrderPickedUp { order, rider })
    }

    fn deliver(
        &mut self,
        order: OrderId,
        rider: RiderId,
    ) -> Result<SimulationEvent, SimulationError> {
        self.world.deliver(rider, order, self.now)?;
        self.outcome(order)?.delivered_at = Some(self.now);
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
        self.queue_pending()?;
        Ok(SimulationEvent::OrderDelivered { order, rider })
    }

    fn process(&mut self, action: Action) -> Result<SimulationEvent, SimulationError> {
        match action {
            Action::Traffic(snapshot) => {
                self.traffic = snapshot;
                self.queue_pending()?;
                Ok(SimulationEvent::TrafficChanged {
                    traffic: TrafficIdentity::Static(
                        self.traffic.traffic_snapshot_digest().to_owned(),
                    ),
                })
            }
            Action::Create(id) => self.create(id),
            Action::Ready(id) => self.ready(id),
            Action::Assign(id) => self.assign(id),
            Action::Move {
                order,
                rider,
                leg,
                pickup,
            } => self.moved(order, rider, leg, pickup),
            Action::Arrive(order, rider) => self.arrived(order, rider),
            Action::Pickup(order, rider) => self.pickup(order, rider),
            Action::Deliver(order, rider) => self.deliver(order, rider),
        }
    }

    fn run(mut self) -> Result<SimulationResult, SimulationError> {
        while self.queue.peek().is_some_and(|event| event.at <= self.end) {
            let event = self.queue.pop().ok_or_else(|| invalid("queue invariant"))?;
            self.now = event.at;
            let processed = self.process(event.action)?;
            self.events.push(RecordedEvent {
                at: self.now,
                sequence: event.sequence,
                event: processed,
            });
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
            schema_version: 1,
            randomness: "splitmix64-upper53/v1".into(),
            seed: self.scenario.seed,
            graph_snapshot_digest: self.graph.metadata().snapshot_digest().to_owned(),
            graph_nodes: self.graph.node_count(),
            graph_edges: self.graph.edge_count(),
            dispatch: self.scenario.dispatch,
            started_at: start,
            ended_at: self.end,
            future_events: self.queue.len(),
            events: self.events,
            decisions: self.decisions,
            orders,
            riders,
            summary,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roadrunner_core::graph::{
        GraphBuildIdentity, GraphBuilder, GraphMetadata, GraphSnapshotId,
    };

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
