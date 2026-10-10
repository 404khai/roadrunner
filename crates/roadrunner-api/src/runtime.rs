use crate::{
    Api, Credential,
    error::ApiError,
    model::{
        CreateOrder, CreateRider, DelayRequest, DispatchRequest, EffectRequest, Envelope,
        FleetRequest, ForecastRequest, NamespaceRequest, Observation, ObserveRequest,
        RecoveryRequest, SimulationRequest, StartRequest, TrafficRequest, UpdateLocation,
    },
    routing::anchor,
};
use axum::http::StatusCode;
use roadrunner_core::{
    cost::{TrafficMultiplier, TrafficSnapshot},
    geo::Seconds,
    graph::EdgeId,
};
use roadrunner_dispatch::{
    ActionId, AdmissionDeadline, AdoptedContextIdentity, AppliedExecutionEffectId, Availability,
    CapacityUnits, CoreRouteProvider, DispatchInstant, DispatchSnapshot, DispatchTimeError,
    EffectOutcome, ExecutionEffect, FleetAlgorithm, FleetContext, FleetInputs, FrozenExecutionLeg,
    OperationalClock, OperationalPlanningContext, OperationalPlanningSnapshot, OperationalRevision,
    OperationalState, Order, OrderId, OrderPolicy, OrderReadiness, PlanRevision, PoolingContext,
    PoolingError, PoolingInputs, PredictionIdentity, ReadinessForecast, RecoveryContext, RiderId,
    RiderProfile, RiderState, RouteOutcome, RouteProvider, RoutingAnchor, RoutingAnchors,
    RoutingEpoch, ScheduleGeneration, Stop, TemporalError, TemporalProposal, TrafficContext,
    TrafficIdentity, project_execution,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) struct StoredCommand {
    pub intent: Value,
    pub command: Command,
    pub status: StatusCode,
    pub envelope: Envelope,
}
pub(crate) struct Coordinator {
    pub state: OperationalState,
    pub anchors: RoutingAnchors,
    policies: BTreeMap<OrderId, OrderPolicy>,
    forecasts: BTreeMap<OrderId, ReadinessForecast>,
    pub traffic: Option<TrafficSnapshot>,
    pub commands: BTreeMap<(String, String), StoredCommand>,
    next_command: u64,
    observations: BTreeMap<(String, String), (Value, Value)>,
    sequences: BTreeMap<(String, String), (u64, DispatchInstant)>,
}
impl Coordinator {
    pub fn new(state: OperationalState) -> Self {
        Self {
            state,
            anchors: RoutingAnchors::default(),
            policies: BTreeMap::new(),
            forecasts: BTreeMap::new(),
            traffic: None,
            commands: BTreeMap::new(),
            next_command: 1,
            observations: BTreeMap::new(),
            sequences: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Serialize)]
#[serde(tag = "operation", content = "intent")]
pub(crate) enum Command {
    CreateOrder(CreateOrder),
    CreateRider(CreateRider),
    Location(String, UpdateLocation),
    Dispatch(DispatchRequest),
    Fleet(FleetRequest),
    Recovery(RecoveryRequest),
    Cancel(String, NamespaceRequest),
    Ready(String, ObserveRequest),
    Forecast(String, ForecastRequest),
    Start(String, StartRequest),
    Service(String, NamespaceRequest),
    Wait(String, NamespaceRequest),
    Arrival(String, EffectRequest),
    Completion(String, EffectRequest),
    Delay(String, DelayRequest),
    Traffic(TrafficRequest),
    Simulation(SimulationRequest),
}
impl Command {
    pub fn namespace(&self) -> &str {
        match self {
            Self::CreateOrder(r) => &r.namespace,
            Self::CreateRider(r) => &r.namespace,
            Self::Location(_, r) => &r.namespace,
            Self::Dispatch(r) => &r.namespace,
            Self::Fleet(r) => &r.namespace,
            Self::Recovery(r) => &r.namespace,
            Self::Cancel(_, r) | Self::Service(_, r) | Self::Wait(_, r) => &r.namespace,
            Self::Ready(_, r) => &r.namespace,
            Self::Forecast(_, r) => &r.namespace,
            Self::Start(_, r) => &r.namespace,
            Self::Arrival(_, r) | Self::Completion(_, r) => &r.namespace,
            Self::Delay(_, r) => &r.namespace,
            Self::Traffic(r) => &r.namespace,
            Self::Simulation(r) => &r.namespace,
        }
    }
    pub fn observation(&self) -> Option<(&str, &Observation)> {
        match self {
            Self::Location(id, r) => Some((id, &r.observation)),
            Self::Ready(id, r) => Some((id, &r.observation)),
            Self::Arrival(id, r) | Self::Completion(id, r) => Some((id, &r.observation)),
            Self::Delay(id, r) => Some((id, &r.observation)),
            _ => None,
        }
    }
    pub fn namespace_wide(&self) -> bool {
        matches!(
            self,
            Self::Recovery(_)
                | Self::Traffic(_)
                | Self::CreateOrder(_)
                | Self::CreateRider(_)
                | Self::Simulation(_)
                | Self::Dispatch(_)
                | Self::Fleet(_)
        )
    }
    pub fn resources(&self) -> Vec<&str> {
        match self {
            Self::Dispatch(r) => vec![&r.order],
            Self::Fleet(r) => r.orders.iter().map(String::as_str).collect(),
            Self::Location(id, _)
            | Self::Cancel(id, _)
            | Self::Ready(id, _)
            | Self::Forecast(id, _)
            | Self::Start(id, _)
            | Self::Service(id, _)
            | Self::Wait(id, _)
            | Self::Arrival(id, _)
            | Self::Completion(id, _)
            | Self::Delay(id, _) => vec![id],
            _ => vec![],
        }
    }
}
pub(crate) fn authorize_resource(credential: &Credential, resource: &str) -> Result<(), ApiError> {
    if credential
        .resources
        .as_ref()
        .is_some_and(|ids| !ids.iter().any(|id| id == resource))
    {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "resource outside credential scope",
        ));
    }
    Ok(())
}
pub(crate) fn reference(api: &Api, kind: &str, resource: &str) -> Result<u64, ApiError> {
    let raw = resource.rsplit(':').next().ok_or_else(ApiError::missing)?;
    let id = raw.parse::<u64>().map_err(|_| ApiError::missing())?;
    if api.inner.namespace.reference(kind, id) != resource {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "namespace_or_kind_mismatch",
            "resource belongs to another namespace or kind; discarded volatile outcomes are unknown",
        ));
    }
    Ok(id)
}
pub(crate) fn value<T: Serialize>(v: T) -> Result<Value, ApiError> {
    serde_json::to_value(v).map_err(|_| ApiError::internal())
}
fn instant(v: f64) -> Result<DispatchInstant, ApiError> {
    DispatchInstant::new(v).map_err(ApiError::invalid)
}
fn secs(v: f64) -> Result<Seconds, ApiError> {
    Seconds::new(v).map_err(ApiError::invalid)
}

// Keep reservation and its capability checks in one auditable sequence.
#[allow(clippy::too_many_lines)]
pub(crate) async fn submit(
    api: Api,
    credential: Credential,
    key: String,
    mut command: Command,
) -> Result<(StatusCode, axum::Json<Envelope>), ApiError> {
    if command.namespace() != api.namespace() {
        return Err(ApiError::conflict(
            "namespace mismatch; outcomes in a discarded volatile namespace are unknown",
        ));
    }
    let observation = command.observation();
    let permitted = if matches!(command, Command::Simulation(_)) {
        credential.simulate
    } else if observation.is_some() {
        credential.observation_source.is_some()
    } else {
        credential.write
    };
    if !permitted {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "credential lacks the required capability",
        ));
    }
    if let Some((_, obs)) = observation {
        if credential.observation_source.as_deref() != Some(&obs.source) {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "untrusted_source",
                "observation source is not bound to this credential",
            ));
        }
    }
    for resource in command.resources() {
        authorize_resource(&credential, resource)?;
    }
    // Namespace-wide recovery/traffic/creation/simulation can affect resources outside a scoped credential.
    if credential.resources.is_some() && command.namespace_wide() {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "operation requires namespace-wide capability",
        ));
    }
    if key.is_empty() || key.len() > 128 || !key.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(ApiError::invalid(
            "Idempotency-Key must contain 1..=128 visible ASCII bytes",
        ));
    }
    match &mut command {
        Command::Simulation(r) => {
            let scenario: roadrunner_simulation::SimulationScenario =
                serde_json::from_value(r.scenario.clone()).map_err(ApiError::invalid)?;
            r.scenario = value(scenario)?;
        }
        Command::Traffic(r) => r.overrides.sort_by_key(|v| v.edge),
        Command::Fleet(r) => r.orders.sort(),
        _ => {}
    }
    let intent = value(&command)?;
    let ledger_key = (credential.principal.clone(), key);
    let permit = {
        let mut c = api
            .inner
            .coordinator
            .lock()
            .map_err(|_| ApiError::internal())?;
        if let Some(prior) = c.commands.get(&ledger_key) {
            if prior.intent != intent {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "idempotency_conflict",
                    "same principal/key has different canonical intent",
                ));
            }
            return Ok((prior.status, axum::Json(prior.envelope.clone())));
        }
        let permit = api.inner.workers.clone().try_acquire_owned().map_err(|_| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "workers_busy",
                "no command reserved; retry the same key after capacity becomes available",
            )
        })?;
        let id = c.next_command;
        c.next_command = id.checked_add(1).ok_or_else(ApiError::internal)?;
        let envelope = Envelope {
            schema_version: 1,
            namespace: api.namespace(),
            command: api.inner.namespace.reference("command", id),
            outcome: "pending".into(),
            published: false,
            revision: c.state.revision().number.to_string(),
            result: json!({}),
        };
        c.commands.insert(
            ledger_key.clone(),
            StoredCommand {
                command: command.clone(),
                intent,
                status: StatusCode::ACCEPTED,
                envelope,
            },
        );
        permit
    };
    // The spawned worker owns the reservation: disconnecting/lost response cannot cancel publication.
    let worker_api = api.clone();
    let worker_key = ledger_key.clone();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        execute(&worker_api, &worker_key, &command)
    })
    .await
    .map_err(|_| ApiError::internal())??;
    let c = api
        .inner
        .coordinator
        .lock()
        .map_err(|_| ApiError::internal())?;
    let stored = c.commands.get(&ledger_key).ok_or_else(ApiError::internal)?;
    Ok((stored.status, axum::Json(stored.envelope.clone())))
}
fn finish(
    c: &mut Coordinator,
    key: &(String, String),
    result: Result<(Value, bool), ApiError>,
) -> Result<(), ApiError> {
    let stored = c.commands.get_mut(key).ok_or_else(ApiError::internal)?;
    stored.envelope.revision = c.state.revision().number.to_string();
    match result {
        Ok((payload, published)) => {
            stored.status = StatusCode::OK;
            stored.envelope.outcome = if published {
                "committed"
            } else if payload.get("noncommit").is_some() {
                "noncommit"
            } else {
                "completed"
            }
            .into();
            stored.envelope.published = published;
            stored.envelope.result = payload;
        }
        Err(error) => {
            stored.status = error.status;
            stored.envelope.outcome = "rejected".into();
            stored.envelope.result = value(error.body)?;
        }
    }
    Ok(())
}
fn execute(api: &Api, key: &(String, String), command: &Command) -> Result<(), ApiError> {
    if matches!(
        command,
        Command::Dispatch(_) | Command::Fleet(_) | Command::Recovery(_)
    ) {
        let result = planning(api, command);
        let mut c = api
            .inner
            .coordinator
            .lock()
            .map_err(|_| ApiError::internal())?;
        return match result {
            Ok(planned) => {
                let result = publish_plan(api, &mut c, planned);
                finish(&mut c, key, result)
            }
            Err(PlanningFailure::Noncommit(evidence)) => {
                finish(&mut c, key, Ok((json!({"noncommit":evidence}), false)))
            }
            Err(PlanningFailure::Error(error)) => finish(&mut c, key, Err(error)),
        };
    }
    if let Command::Start(resource, request) = command {
        let prepared = prepare_start(api, resource, request);
        let mut c = api
            .inner
            .coordinator
            .lock()
            .map_err(|_| ApiError::internal())?;
        let result=prepared.and_then(|(revision,rider,plan,mut leg,origin,destination)| {
            leg.departed_at=api.inner.clock.now().map_err(ApiError::invalid)?;
            let attempt=c.state.start_action(revision,rider,plan,leg,&origin,destination).map_err(ApiError::conflict)?;
            let active=&c.state.execution()[&rider];
            Ok((json!({"attempt":attempt.to_string(),"action":api.inner.namespace.reference("action",active.action_id.value()),"generation":active.generation.value().to_string(),"arrival_seconds":active.arrival}),true))
        });
        return finish(&mut c, key, result);
    }
    if let Command::Simulation(request) = command {
        let result = (|| {
            let scenario: roadrunner_simulation::SimulationScenario =
                serde_json::from_value(request.scenario.clone()).map_err(ApiError::invalid)?;
            if scenario.graph_snapshot_digest.as_deref()
                != Some(api.inner.graph.metadata().snapshot_digest())
            {
                return Err(ApiError::conflict(
                    "simulation must bind server graph digest",
                ));
            }
            if scenario.orders.len() > 10_000 || scenario.riders.len() > 10_000 {
                return Err(ApiError::invalid(
                    "HTTP simulation supports at most 10000 orders/riders",
                ));
            }
            value(
                roadrunner_simulation::simulate(&api.inner.graph, &scenario)
                    .map_err(ApiError::invalid)?,
            )
            .map(|v| (v, false))
        })();
        let mut c = api
            .inner
            .coordinator
            .lock()
            .map_err(|_| ApiError::internal())?;
        return finish(&mut c, key, result);
    }
    let mut c = api
        .inner
        .coordinator
        .lock()
        .map_err(|_| ApiError::internal())?;
    let result = mutation(api, &mut c, command);
    finish(&mut c, key, result)
}
fn temporal_error(error: TemporalError) -> ApiError {
    let code = match &error {
        TemporalError::Stale => "stale_evaluation",
        TemporalError::EvaluationContextExpired => "evaluation_context_expired",
        TemporalError::UnsupportedTemporalModel => "unsupported_temporal_model",
        TemporalError::Prediction(PoolingError::PredictionUnavailable(_)) => {
            "prediction_unavailable"
        }
        _ => "publication_rejected",
    };
    ApiError::new(StatusCode::CONFLICT, code, error)
}
struct ClockRef<'a>(&'a dyn OperationalClock);
impl OperationalClock for ClockRef<'_> {
    fn time_domain(&self) -> &str {
        self.0.time_domain()
    }
    fn now(&self) -> Result<DispatchInstant, DispatchTimeError> {
        self.0.now()
    }
}
struct EvaluationClock {
    at: DispatchInstant,
    domain: String,
}
impl OperationalClock for EvaluationClock {
    fn time_domain(&self) -> &str {
        &self.domain
    }
    fn now(&self) -> Result<DispatchInstant, DispatchTimeError> {
        Ok(self.at)
    }
}

enum PlanningFailure {
    Error(ApiError),
    Noncommit(Value),
}
impl From<ApiError> for PlanningFailure {
    fn from(e: ApiError) -> Self {
        Self::Error(e)
    }
}
impl From<TemporalError> for PlanningFailure {
    fn from(e: TemporalError) -> Self {
        Self::Error(temporal_error(e))
    }
}
type Planned = (
    OperationalPlanningSnapshot,
    TemporalProposal,
    Option<TrafficSnapshot>,
    Value,
);
// Assembly, adoption and detached certification follow one documented lifecycle.
#[allow(clippy::too_many_lines)]
fn planning(api: &Api, command: &Command) -> Result<Planned, PlanningFailure> {
    let (state, anchors, inputs, traffic, clock) = {
        let mut c = api
            .inner
            .coordinator
            .lock()
            .map_err(|_| ApiError::internal())?;
        let at = api.inner.clock.now().map_err(ApiError::invalid)?;
        let clock = EvaluationClock {
            at,
            domain: api.inner.clock.time_domain().into(),
        };
        let provider = CoreRouteProvider::new(
            &api.inner.graph,
            c.traffic
                .as_ref()
                .map_or(TrafficContext::FreeFlow, TrafficContext::Static),
        )
        .map_err(ApiError::invalid)?;
        let mut inputs = PoolingInputs {
            identity: PredictionIdentity {
                prediction: "http-forecast/v1".into(),
                service: "configured-stop-service/v1".into(),
                optimizer: match command {
                    Command::Dispatch(_) => "exhaustive-insertion/v1",
                    Command::Fleet(_) => "fleet-greedy-local/v1",
                    _ => "dynamic-recovery/v1",
                }
                .into(),
                routing: provider.provenance(),
            },
            policies: c.policies.clone(),
            forecasts: c.forecasts.clone(),
            projections: BTreeMap::new(),
            work_budget: api.inner.config.work_budget,
        };
        for rider in c.state.data().riders.keys() {
            let origin = c
                .anchors
                .riders
                .get(rider)
                .ok_or_else(ApiError::internal)?
                .clone();
            let frozen = c
                .state
                .frozen_prefix(*rider, &inputs, at)
                .map_err(|e| temporal_error(TemporalError::Prediction(e)))?;
            inputs.projections.insert(
                *rider,
                project_execution(c.state.data(), *rider, at, origin, frozen)
                    .map_err(|e| temporal_error(TemporalError::Prediction(e)))?,
            );
        }
        let dispatch = DispatchSnapshot::new(
            &c.state,
            at,
            RoutingEpoch(instant(0.0)?),
            &provider,
            &c.anchors,
        )
        .map_err(ApiError::invalid)?;
        let context = match command {
            Command::Fleet(_) => {
                FleetContext::new(
                    &dispatch,
                    FleetInputs {
                        pooling: inputs.clone(),
                        unavailable_projections: BTreeMap::new(),
                        algorithm: FleetAlgorithm::LocalSearch,
                    },
                )
                .map_err(ApiError::invalid)?
                .pooling
            }
            Command::Recovery(r) => {
                RecoveryContext::new(
                    &dispatch,
                    inputs.clone(),
                    api.inner.config.recovery,
                    r.trigger.clone(),
                    c.state.recovery_at(),
                )
                .map_err(ApiError::invalid)?
                .pooling
            }
            _ => PoolingContext::new(&dispatch, inputs.clone()).map_err(ApiError::invalid)?,
        };
        let revision = c.state.revision();
        c.state.adopt_planning_context(revision, &context)?;
        (
            c.state.snapshot(),
            c.anchors.clone(),
            inputs,
            c.traffic.clone(),
            clock,
        )
    };
    let provider = CoreRouteProvider::new(
        &api.inner.graph,
        traffic
            .as_ref()
            .map_or(TrafficContext::FreeFlow, TrafficContext::Static),
    )
    .map_err(ApiError::invalid)?;
    let context = OperationalPlanningContext {
        provider: &provider,
        anchors: &anchors,
        epoch: RoutingEpoch(instant(0.0)?),
        clock: &clock,
    };
    let evaluated = match command {
        Command::Dispatch(r) => state.evaluate_insertion(
            &context,
            OrderId::new(reference(api, "order", &r.order)?),
            inputs,
        )?,
        Command::Fleet(r) => {
            if r.orders.len() > 100 {
                return Err(ApiError::invalid("fleet batch limit is 100").into());
            }
            let batch = r
                .orders
                .iter()
                .map(|id| reference(api, "order", id).map(OrderId::new))
                .collect::<Result<Vec<_>, _>>()?;
            state.evaluate_fleet(
                &context,
                &batch,
                FleetInputs {
                    pooling: inputs,
                    unavailable_projections: BTreeMap::new(),
                    algorithm: FleetAlgorithm::LocalSearch,
                },
            )?
        }
        Command::Recovery(r) => state.evaluate_recovery(
            &context,
            inputs,
            api.inner.config.recovery,
            r.trigger.clone(),
        )?,
        _ => return Err(ApiError::internal().into()),
    };
    let (evidence, has_proposal) = if let Some(d) = evaluated.insertion() {
        (value(d)?, d.proposal().is_some())
    } else if let Some(d) = evaluated.fleet() {
        (value(d)?, d.proposal().is_some())
    } else if let Some(d) = evaluated.recovery() {
        (value(d)?, d.proposal().is_some())
    } else {
        return Err(ApiError::internal().into());
    };
    if !has_proposal {
        return Err(PlanningFailure::Noncommit(evidence));
    }
    let until = clock
        .at
        .checked_add(api.inner.config.certification_window)
        .map_err(ApiError::invalid)?;
    let certificate = state.certify(evaluated, &provider, until)?;
    Ok((state, certificate, traffic, evidence))
}
// Exhaustive intent-to-domain mapping; each arm validates before its atomic domain effect.
#[allow(clippy::too_many_lines)]
fn mutation(api: &Api, c: &mut Coordinator, command: &Command) -> Result<(Value, bool), ApiError> {
    let now = api.inner.clock.now().map_err(ApiError::invalid)?;
    let before = c.state.revision();
    let intent = value(command)?;
    let mut observation_key = None;
    if let Some((resource, obs)) = command.observation() {
        if obs.observation_id.is_empty() || obs.observation_id.len() > 128 {
            return Err(ApiError::invalid(
                "observation identity requires 1..=128 bytes",
            ));
        }
        let key = (obs.source.clone(), obs.observation_id.clone());
        if let Some((original, result)) = c.observations.get(&key) {
            if original != &intent {
                return Err(ApiError::conflict(
                    "observation identity reused with different intent",
                ));
            }
            return Ok((json!({"already_applied":true,"original":result}), false));
        }
        let at = instant(obs.observed_at_seconds)?;
        if at > now {
            return Err(ApiError::conflict("future observation"));
        }
        let previous = c.sequences.get(&(obs.source.clone(), resource.into()));
        let expected = previous.map_or(Some(1), |(sequence, _)| sequence.checked_add(1));
        if Some(obs.sequence) != expected {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "observation_sequence",
                "source/resource observation gap or superseded sequence; resolve before retrying with a new command",
            ));
        }
        if previous.is_some_and(|(_, prior)| at < *prior) {
            return Err(ApiError::conflict(
                "observation time regressed within source/resource",
            ));
        }
        observation_key = Some((
            key,
            (obs.source.clone(), resource.to_owned()),
            obs.sequence,
            at,
        ));
    }
    let result = match command {
        Command::CreateOrder(r) => {
            let pickup = anchor(&api.inner.graph, &r.pickup)?;
            let dropoff = anchor(&api.inner.graph, &r.dropoff)?;
            if r.demand == 0 {
                return Err(ApiError::invalid("demand must be positive"));
            }
            let policy = api
                .inner
                .config
                .policies
                .get(&r.policy)
                .ok_or_else(|| ApiError::invalid("unknown configured policy"))?
                .clone();
            let deadline = r.deadline_seconds.map(instant).transpose()?;
            if policy.deadline == AdmissionDeadline::Hard && deadline.is_none() {
                return Err(ApiError::invalid("hard policy requires deadline"));
            }
            let forecast = match (r.expected_ready_seconds, r.forecast_valid_until_seconds) {
                (Some(expected), Some(valid)) => {
                    let valid_until = instant(valid)?;
                    if valid_until < now {
                        return Err(ApiError::invalid("forecast already expired"));
                    }
                    Some(ReadinessForecast {
                        id: format!("http-forecast/{}/{}", api.namespace(), c.next_command),
                        generated_at: now,
                        valid_until,
                        expected_at: instant(expected)?,
                    })
                }
                (None, None) => None,
                _ => {
                    return Err(ApiError::invalid(
                        "expected_ready_seconds and forecast_valid_until_seconds are required together",
                    ));
                }
            };
            let id = c
                .state
                .create_order(
                    before,
                    Order {
                        id: OrderId::new(0),
                        pickup: pickup.coordinate,
                        dropoff: dropoff.coordinate,
                        created_at: now,
                        deadline,
                        demand: CapacityUnits::new(r.demand),
                    },
                    OrderReadiness {
                        expected_at: forecast.as_ref().map(|f| f.expected_at),
                        observed_at: None,
                    },
                )
                .map_err(ApiError::conflict)?;
            c.anchors.pickups.insert(id, pickup);
            c.anchors.dropoffs.insert(id, dropoff);
            c.policies.insert(id, policy);
            if let Some(f) = forecast {
                c.forecasts.insert(id, f);
            }
            json!({"order":api.inner.namespace.reference("order",id.value()),"created_at_seconds":now,"assigned":false})
        }
        Command::CreateRider(r) => {
            if r.capacity == 0 {
                return Err(ApiError::invalid("capacity must be positive"));
            }
            let location = anchor(&api.inner.graph, &r.location)?;
            let id = c
                .state
                .register_rider(
                    before,
                    RiderProfile {
                        id: RiderId::new(0),
                        routing_profile: api.inner.graph.metadata().routing_profile().into(),
                        max_capacity: CapacityUnits::new(r.capacity),
                    },
                    RiderState {
                        coordinate: location.coordinate,
                        availability: if r.available {
                            Availability::Available
                        } else {
                            Availability::Unavailable
                        },
                    },
                )
                .map_err(ApiError::conflict)?;
            c.anchors.riders.insert(id, location);
            json!({"rider":api.inner.namespace.reference("rider",id.value())})
        }
        Command::Location(id, r) => {
            let rider = RiderId::new(reference(api, "rider", id)?);
            let location = anchor(&api.inner.graph, &r.location)?;
            c.state
                .update_rider_state(
                    rider,
                    RiderState {
                        coordinate: location.coordinate,
                        availability: if r.available {
                            Availability::Available
                        } else {
                            Availability::Unavailable
                        },
                    },
                )
                .map_err(ApiError::conflict)?;
            c.anchors.riders.insert(rider, location);
            json!({"rider":id})
        }
        Command::Cancel(id, _) => {
            let order = OrderId::new(reference(api, "order", id)?);
            if !c.state.data().orders.contains_key(&order) {
                return Err(ApiError::missing());
            }
            match c
                .state
                .cancel_order(order, now)
                .map_err(ApiError::conflict)?
            {
                Ok(()) => json!({"cancelled":id}),
                Err(reason) => json!({"noncommit":{"cancellation_refusal":reason}}),
            }
        }
        Command::Ready(id, r) => {
            let order = OrderId::new(reference(api, "order", id)?);
            c.state
                .observe_ready(order, instant(r.observation.observed_at_seconds)?)
                .map_err(ApiError::conflict)?;
            json!({"order":id,"ready_at_seconds":r.observation.observed_at_seconds})
        }
        Command::Forecast(id, r) => {
            let order = OrderId::new(reference(api, "order", id)?);
            let expected = instant(r.expected_ready_seconds)?;
            let valid_until = instant(r.valid_until_seconds)?;
            if valid_until < now {
                return Err(ApiError::invalid("forecast already expired"));
            }
            c.state
                .estimate_readiness(order, expected)
                .map_err(ApiError::conflict)?;
            c.forecasts.insert(
                order,
                ReadinessForecast {
                    id: format!("http-forecast/{}/{}", api.namespace(), c.next_command),
                    generated_at: now,
                    valid_until,
                    expected_at: expected,
                },
            );
            json!({"order":id,"forecast_generated_at":now})
        }
        Command::Traffic(r) => {
            if r.graph_digest != api.inner.graph.metadata().snapshot_digest() {
                return Err(ApiError::conflict("graph snapshot mismatch"));
            }
            let overrides = r
                .overrides
                .iter()
                .map(|v| {
                    Ok((
                        EdgeId::new(v.edge),
                        TrafficMultiplier::new(v.multiplier).map_err(ApiError::invalid)?,
                    ))
                })
                .collect::<Result<Vec<_>, ApiError>>()?;
            let traffic =
                TrafficSnapshot::new(&api.inner.graph, overrides).map_err(ApiError::invalid)?;
            let identity = value(TrafficIdentity::Static(
                traffic.traffic_snapshot_digest().into(),
            ))?;
            // Match dispatch's canonical fingerprint of the typed traffic identity.
            let digest = format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&identity).map_err(|_| ApiError::internal())?)
            );
            c.state
                .adopt(
                    before,
                    AdoptedContextIdentity {
                        category: "traffic".into(),
                        content: digest,
                    },
                )
                .map_err(ApiError::conflict)?;
            let result = json!({"traffic_digest":traffic.traffic_snapshot_digest()});
            c.traffic = Some(traffic);
            result
        }
        Command::Service(id, _) => {
            let rider = RiderId::new(reference(api, "rider", id)?);
            let action = c
                .state
                .execution()
                .get(&rider)
                .ok_or_else(ApiError::missing)?;
            let terms = c
                .state
                .data()
                .accepted
                .get(&action.stop.order())
                .ok_or_else(ApiError::internal)?;
            let duration = if matches!(action.stop, Stop::Pickup(_)) {
                terms.policy.pickup_service
            } else {
                terms.policy.dropoff_service
            };
            c.state
                .start_service(before, rider, now, duration)
                .map_err(ApiError::conflict)?;
            json!({"rider":id,"service_started_at":now})
        }
        Command::Wait(id, _) => {
            let rider = RiderId::new(reference(api, "rider", id)?);
            c.state
                .wait_for_readiness(before, rider)
                .map_err(ApiError::conflict)?;
            json!({"rider":id,"waiting":true})
        }
        Command::Arrival(id, r) | Command::Completion(id, r) => {
            let rider = RiderId::new(reference(api, "rider", id)?);
            let action = ActionId::new(reference(api, "action", &r.action)?);
            let generation =
                ScheduleGeneration::new(r.generation.parse::<u64>().map_err(ApiError::invalid)?);
            let effect = AppliedExecutionEffectId::new(reference(api, "effect", &r.effect_id)?);
            let kind = if matches!(command, Command::Arrival(..)) {
                ExecutionEffect::Arrival
            } else {
                ExecutionEffect::Completion
            };
            let outcome = c
                .state
                .apply_effect(
                    before,
                    effect,
                    rider,
                    action,
                    generation,
                    kind,
                    instant(r.observation.observed_at_seconds)?,
                )
                .map_err(ApiError::conflict)?;
            // Arrival updates the authoritative endpoint coordinate, including its graph projection.
            if kind == ExecutionEffect::Arrival && outcome == EffectOutcome::Applied {
                let location = c
                    .state
                    .execution()
                    .get(&rider)
                    .ok_or_else(ApiError::internal)?
                    .anchor
                    .clone();
                c.anchors.riders.insert(rider, location);
            }
            json!({"effect":if outcome==EffectOutcome::Applied{"applied"}else{"already_applied"},"rider":id,"action":r.action})
        }
        Command::Delay(id, r) => {
            let rider = RiderId::new(reference(api, "rider", id)?);
            let action = c
                .state
                .delay_action(before, rider, secs(r.seconds)?)
                .map_err(ApiError::conflict)?;
            json!({"action":api.inner.namespace.reference("action",action.action_id.value()),"generation":action.generation.value().to_string(),"arrival_seconds":action.arrival})
        }
        Command::Start(_, _)
        | Command::Dispatch(_)
        | Command::Fleet(_)
        | Command::Recovery(_)
        | Command::Simulation(_) => return Err(ApiError::internal()),
    };
    let published = c.state.revision() != before;
    if let Some((key, sequence_key, sequence, at)) = observation_key {
        let result = json!({"effect":result,"observation":command.observation().map(|(_,o)|o),"received_at_seconds":now});
        c.observations.insert(key, (intent, result.clone()));
        c.sequences.insert(sequence_key, (sequence, at));
        Ok((result, published))
    } else {
        Ok((result, published))
    }
}

type PreparedStart = (
    OperationalRevision,
    RiderId,
    PlanRevision,
    FrozenExecutionLeg,
    RoutingAnchor,
    RoutingAnchor,
);
fn prepare_start(
    api: &Api,
    resource: &str,
    request: &StartRequest,
) -> Result<PreparedStart, ApiError> {
    let rider = RiderId::new(reference(api, "rider", resource)?);
    let plan = PlanRevision::new(
        request
            .plan_revision
            .parse::<u64>()
            .map_err(ApiError::invalid)?,
    );
    let (revision, origin, destination, traffic) = {
        let c = api
            .inner
            .coordinator
            .lock()
            .map_err(|_| ApiError::internal())?;
        let stop = c
            .state
            .next_stop(rider)
            .map_err(ApiError::conflict)?
            .ok_or_else(|| ApiError::conflict("rider has no committed next stop"))?;
        let destination = if matches!(stop, Stop::Pickup(_)) {
            c.anchors.pickups.get(&stop.order())
        } else {
            c.anchors.dropoffs.get(&stop.order())
        }
        .ok_or_else(ApiError::internal)?
        .clone();
        (
            c.state.revision(),
            c.anchors
                .riders
                .get(&rider)
                .ok_or_else(ApiError::missing)?
                .clone(),
            destination,
            c.traffic.clone(),
        )
    };
    let provider = CoreRouteProvider::new(
        &api.inner.graph,
        traffic
            .as_ref()
            .map_or(TrafficContext::FreeFlow, TrafficContext::Static),
    )
    .map_err(ApiError::invalid)?;
    let now = api.inner.clock.now().map_err(ApiError::invalid)?;
    let RouteOutcome::RouteFound(route) = provider
        .route(origin.node, destination.node, secs(now.value())?)
        .map_err(ApiError::invalid)?
    else {
        return Err(ApiError::conflict("no route to next committed stop"));
    };
    let leg = FrozenExecutionLeg {
        from: origin.node,
        to: destination.node,
        departed_at: now,
        travel: route.route.elapsed_travel_time(),
        distance: route.route.total_distance(),
        nodes: route.route.path().to_vec(),
        edges: route.route.edges().to_vec(),
        routing: route.provenance,
    };
    Ok((revision, rider, plan, leg, origin, destination))
}

fn publish_plan(
    api: &Api,
    c: &mut Coordinator,
    (snapshot, certificate, traffic, evidence): Planned,
) -> Result<(Value, bool), ApiError> {
    if c.state.revision() != snapshot.revision() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "stale_evaluation",
            "authority changed during evaluation; submit a new key for a new attempt",
        ));
    }
    let provider = CoreRouteProvider::new(
        &api.inner.graph,
        traffic
            .as_ref()
            .map_or(TrafficContext::FreeFlow, TrafficContext::Static),
    )
    .map_err(ApiError::invalid)?;
    c.state
        .publish_temporal(certificate, &ClockRef(api.inner.clock.as_ref()), &provider)
        .map(|p| {
            let assignments=c.state.data().assignments.iter().map(|(order,assignment)|json!({"order":api.inner.namespace.reference("order",order.value()),"rider":api.inner.namespace.reference("rider",assignment.rider.value())})).collect::<Vec<_>>();
            (json!({"decision":evidence,"publication":p,"assignments":assignments}), true)
        })
        .map_err(temporal_error)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{Command, PlanningFailure, planning, publish_plan};
    use crate::{
        model::DispatchRequest,
        test_support::{call, create_order, create_rider, fixture, observation},
    };
    use axum::http::StatusCode;
    use serde_json::json;

    #[tokio::test]
    async fn worker_saturation_preserves_recognition_without_reserving_new_commands() {
        let (api, _, digest) = fixture();
        let rider = create_rider(&api, &digest).await;
        let permits = api
            .inner
            .workers
            .clone()
            .acquire_many_owned(4)
            .await
            .unwrap();
        let original=call(&api,"POST","/v1/riders","rider",serde_json::json!({"namespace":api.namespace(),"location":crate::test_support::location(&digest,0),"capacity":3,"available":true})).await;
        assert_eq!(original.0, StatusCode::OK);
        assert_eq!(original.1["result"]["rider"], rider);
        let rejected=call(&api,"POST","/v1/riders","new-command",serde_json::json!({"namespace":api.namespace(),"location":crate::test_support::location(&digest,0),"capacity":3,"available":true})).await;
        assert_eq!(rejected.0, StatusCode::SERVICE_UNAVAILABLE);
        let c = api.inner.coordinator.lock().unwrap();
        assert!(
            !c.commands
                .contains_key(&("operator".into(), "new-command".into()))
        );
        assert_eq!(c.state.data().riders.len(), 1);
        drop(c);
        drop(permits);
    }
    #[tokio::test]
    async fn scoped_credentials_cannot_read_or_plan_outside_resource_authorization() {
        let (mut api, _, digest) = fixture();
        let ns = api.namespace();
        let scope = api.inner.namespace.reference("rider", 1);
        let credential = crate::Credential {
            principal: "operator".into(),
            read: true,
            write: true,
            simulate: false,
            observation_source: Some("scoped-source".into()),
            resources: Some(vec![scope.clone()]),
        };
        std::sync::Arc::get_mut(&mut api.inner)
            .unwrap()
            .config
            .credentials
            .insert("scoped-token-123456".into(), credential);
        create_rider(&api, &digest).await;
        let resource = crate::test_support::request(
            &api,
            "GET",
            &format!("/v1/riders/{scope}"),
            None,
            json!(null),
            Some("scoped-token-123456"),
        )
        .await;
        assert_eq!(resource.0, StatusCode::OK);
        let state = crate::test_support::request(
            &api,
            "GET",
            "/v1/state",
            None,
            json!(null),
            Some("scoped-token-123456"),
        )
        .await;
        assert_eq!(state.0, StatusCode::FORBIDDEN);
        let lookup = crate::test_support::request(
            &api,
            "GET",
            "/v1/commands/rider",
            None,
            json!(null),
            Some("scoped-token-123456"),
        )
        .await;
        assert_eq!(lookup.0, StatusCode::FORBIDDEN);
        let plan = crate::test_support::request(
            &api,
            "POST",
            "/v1/dispatch/recovery",
            Some("outside"),
            json!({"namespace":ns,"trigger":"test"}),
            Some("scoped-token-123456"),
        )
        .await;
        assert_eq!(plan.0, StatusCode::FORBIDDEN);
    }
    #[tokio::test]
    async fn competing_certified_assignments_have_one_legal_publication() {
        let (api, _, digest) = fixture();
        create_rider(&api, &digest).await;
        let order = create_order(&api, &digest, "order").await;
        let command = Command::Dispatch(DispatchRequest {
            namespace: api.namespace(),
            order,
        });
        let first = match planning(&api, &command) {
            Ok(p) => p,
            Err(PlanningFailure::Error(e)) => panic!("{e:?}"),
            Err(PlanningFailure::Noncommit(e)) => panic!("{e}"),
        };
        let second = match planning(&api, &command) {
            Ok(p) => p,
            Err(PlanningFailure::Error(e)) => panic!("{e:?}"),
            Err(PlanningFailure::Noncommit(e)) => panic!("{e}"),
        };
        let mut c = api.inner.coordinator.lock().unwrap();
        let before = c.state.revision();
        // The second context adoption supersedes the first evaluation (including ABA).
        assert_eq!(
            publish_plan(&api, &mut c, first).unwrap_err().body.code,
            "stale_evaluation"
        );
        assert_eq!(c.state.revision(), before);
        assert!(publish_plan(&api, &mut c, second).unwrap().1);
        c.state.validate().unwrap();
        assert_eq!(c.state.data().assignments.len(), 1);
        assert_eq!(c.state.publications().len(), 1);
        assert_eq!(c.state.history().last().unwrap().before, before);
    }
    #[tokio::test]
    async fn observation_and_traffic_between_evaluation_and_publication_reject_wholly() {
        let (api, _, digest) = fixture();
        let rider = create_rider(&api, &digest).await;
        let order = create_order(&api, &digest, "order").await;
        let command = Command::Dispatch(DispatchRequest {
            namespace: api.namespace(),
            order,
        });
        let planned = planning(&api, &command).unwrap_or_else(|_| panic!("planning failed"));
        let request = json!({"namespace":api.namespace(),"location":crate::test_support::location(&digest,0),"available":false,"observation":observation("offline",1,100)});
        assert_eq!(
            call(
                &api,
                "PATCH",
                &format!("/v1/riders/{rider}/location"),
                "offline",
                request
            )
            .await
            .0,
            StatusCode::OK
        );
        {
            let mut c = api.inner.coordinator.lock().unwrap();
            let before = c.state.revision();
            assert_eq!(
                publish_plan(&api, &mut c, planned).unwrap_err().body.code,
                "stale_evaluation"
            );
            assert_eq!(c.state.revision(), before);
            assert!(c.state.data().assignments.is_empty());
            c.state.validate().unwrap();
        }
        let (api, _, digest) = fixture();
        create_rider(&api, &digest).await;
        let order = create_order(&api, &digest, "traffic-order").await;
        let command = Command::Dispatch(DispatchRequest {
            namespace: api.namespace(),
            order,
        });
        let planned = planning(&api, &command).unwrap_or_else(|_| panic!("planning failed"));
        let result=call(&api,"POST","/v1/traffic","new-traffic",json!({"namespace":api.namespace(),"graph_digest":digest,"overrides":[{"edge":0,"multiplier":2.0}]})).await;
        assert_eq!(result.0, StatusCode::OK);
        let mut c = api.inner.coordinator.lock().unwrap();
        let before = c.state.snapshot();
        assert_eq!(
            publish_plan(&api, &mut c, planned).unwrap_err().body.code,
            "stale_evaluation"
        );
        assert_eq!(c.state.revision(), before.revision());
        assert_eq!(c.state.data(), before.data());
        assert_eq!(c.state.contexts(), before.contexts());
        c.state.validate().unwrap();
    }
    #[tokio::test]
    async fn elapsed_evaluation_time_can_publish_but_expiry_preserves_all_state() {
        let (api, clock, digest) = fixture();
        create_rider(&api, &digest).await;
        let order = create_order(&api, &digest, "order").await;
        let command = Command::Dispatch(DispatchRequest {
            namespace: api.namespace(),
            order,
        });
        let planned = planning(&api, &command).unwrap_or_else(|_| panic!("planning failed"));
        clock.set(101);
        {
            let mut c = api.inner.coordinator.lock().unwrap();
            let publication = publish_plan(&api, &mut c, planned).unwrap().0;
            assert_eq!(publication["publication"]["evaluated_at"], 100.0);
            assert_eq!(publication["publication"]["committed_at"], 101.0);
        }
        let order = create_order(&api, &digest, "second-order").await;
        let command = Command::Dispatch(DispatchRequest {
            namespace: api.namespace(),
            order,
        });
        let planned = planning(&api, &command).unwrap_or_else(|_| panic!("planning failed"));
        clock.set(104);
        let mut c = api.inner.coordinator.lock().unwrap();
        let before = c.state.snapshot();
        assert_eq!(
            publish_plan(&api, &mut c, planned).unwrap_err().body.code,
            "evaluation_context_expired"
        );
        assert_eq!(c.state.revision(), before.revision());
        assert_eq!(c.state.data(), before.data());
        assert_eq!(c.state.execution(), before.execution());
        assert_eq!(c.state.publications(), before.publications());
    }
}
