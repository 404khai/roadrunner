//! Durable coordinator reconstruction and staged SQL publication. No credentials persist.
use super::{Coordinator, StoredCommand, value};
use crate::{Api, ApiConfig, HostClock, Inner, error::ApiError};
use axum::http::StatusCode;
use roadrunner_core::{
    cost::{TrafficMultiplier, TrafficSnapshot},
    graph::{EdgeId, FrozenGraph},
};
use roadrunner_dispatch::{
    DispatchInstant, OperationalClock, OperationalState, OrderId, OrderPolicy, ReadinessForecast,
    RoutingAnchors, TrafficIdentity,
};
use roadrunner_storage::{DurableHead, DurableImage, PgStore, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) mod status_code {
    use axum::http::StatusCode;
    use serde::{Deserialize, Deserializer, Serializer};
    #[allow(clippy::trivially_copy_pass_by_ref)] // serde with-module callback requires a reference.
    pub fn serialize<S: Serializer>(value: &StatusCode, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u16(value.as_u16())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<StatusCode, D::Error> {
        StatusCode::from_u16(u16::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrafficRecord {
    graph_digest: String,
    digest: String,
    overrides: Vec<(u32, TrafficMultiplier)>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandRecord {
    principal: String,
    key: String,
    record: StoredCommand,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationRecord {
    source: String,
    id: String,
    record: (Value, Value),
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SequenceRecord {
    source: String,
    resource: String,
    sequence: u64,
    at: DispatchInstant,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdapterRecord {
    schema_version: u32,
    anchors: RoutingAnchors,
    policies: BTreeMap<OrderId, OrderPolicy>,
    forecasts: BTreeMap<OrderId, ReadinessForecast>,
    traffic: Option<TrafficRecord>,
    commands: Vec<CommandRecord>,
    next_command: u64,
    observations: Vec<ObservationRecord>,
    sequences: Vec<SequenceRecord>,
}
impl Coordinator {
    pub(crate) fn image(
        &self,
        graph: &FrozenGraph,
        domain: &str,
        floor: DispatchInstant,
    ) -> Result<DurableImage, ApiError> {
        let traffic = self
            .traffic
            .as_ref()
            .map(|t| -> Result<TrafficRecord, ApiError> {
                Ok(TrafficRecord {
                    graph_digest: t.graph_snapshot_digest().into(),
                    digest: t.traffic_snapshot_digest().into(),
                    overrides: graph
                        .edges()
                        .iter()
                        .map(|e| {
                            t.multiplier(e.id())
                                .map(|m| (e.id().value(), m))
                                .ok_or_else(ApiError::internal)
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .transpose()?;
        let record = AdapterRecord {
            schema_version: 1,
            anchors: self.anchors.clone(),
            policies: self.policies.clone(),
            forecasts: self.forecasts.clone(),
            traffic,
            commands: self
                .commands
                .iter()
                .map(|((p, k), r)| CommandRecord {
                    principal: p.clone(),
                    key: k.clone(),
                    record: r.clone(),
                })
                .collect(),
            next_command: self.next_command,
            observations: self
                .observations
                .iter()
                .map(|((source, id), record)| ObservationRecord {
                    source: source.clone(),
                    id: id.clone(),
                    record: record.clone(),
                })
                .collect(),
            sequences: self
                .sequences
                .iter()
                .map(|((source, resource), (sequence, at))| SequenceRecord {
                    source: source.clone(),
                    resource: resource.clone(),
                    sequence: *sequence,
                    at: *at,
                })
                .collect(),
        };
        Ok(DurableImage {
            operational: self.state.checkpoint().map_err(ApiError::invalid)?,
            adapter: value(record)?,
            time_domain: domain.into(),
            clock_floor: floor.value(),
        })
    }
    pub(crate) fn image_for(
        &self,
        api: &Api,
        floor: DispatchInstant,
    ) -> Result<DurableImage, ApiError> {
        self.image(&api.inner.graph, api.inner.clock.time_domain(), floor)
    }
    pub(crate) fn restore(graph: &FrozenGraph, image: &DurableImage) -> Result<Self, ApiError> {
        let state =
            OperationalState::restore_checkpoint(&image.operational).map_err(ApiError::invalid)?;
        let record: AdapterRecord =
            serde_json::from_value(image.adapter.clone()).map_err(ApiError::invalid)?;
        if state.checkpoint().map_err(ApiError::invalid)? != image.operational
            || record.next_command == 0
            || record.schema_version != 1
        {
            return Err(ApiError::invalid("unsupported adapter storage schema"));
        }
        let traffic = record
            .traffic
            .map(|t| {
                if t.graph_digest != graph.metadata().snapshot_digest() {
                    return Err(ApiError::invalid("traffic graph mismatch"));
                }
                let traffic = TrafficSnapshot::new(
                    graph,
                    t.overrides.into_iter().map(|(id, m)| (EdgeId::new(id), m)),
                )
                .map_err(ApiError::invalid)?;
                if traffic.traffic_snapshot_digest() != t.digest {
                    return Err(ApiError::invalid("traffic digest mismatch"));
                }
                Ok(traffic)
            })
            .transpose()?;
        let mut c = Self {
            state,
            anchors: record.anchors,
            policies: record.policies,
            forecasts: record.forecasts,
            traffic,
            commands: BTreeMap::new(),
            next_command: record.next_command,
            observations: BTreeMap::new(),
            sequences: BTreeMap::new(),
            clock_floor: image.clock_floor,
        };
        for item in record.commands {
            let r = &item.record;
            let id = r
                .envelope
                .command
                .rsplit(':')
                .next()
                .and_then(|v| v.parse::<u64>().ok())
                .ok_or_else(ApiError::internal)?;
            let ns = c.state.revision().namespace;
            if r.intent != value(&r.command)?
                || r.command.namespace() != ns.reference("world", 0)
                || ns.reference("command", id) != r.envelope.command
                || id >= c.next_command
                || item.principal.is_empty()
                || item.key.is_empty()
                || r.envelope.schema_version != 1
            {
                return Err(ApiError::invalid("invalid durable command recognition"));
            }
            if c.commands
                .insert((item.principal, item.key), item.record)
                .is_some()
            {
                return Err(ApiError::invalid("duplicate durable command"));
            }
        }
        for item in record.observations {
            if c.observations
                .insert((item.source, item.id), item.record)
                .is_some()
            {
                return Err(ApiError::invalid("duplicate durable observation"));
            }
        }
        for item in record.sequences {
            if c.sequences
                .insert((item.source, item.resource), (item.sequence, item.at))
                .is_some()
            {
                return Err(ApiError::invalid("duplicate durable sequence"));
            }
        }
        c.validate_artifacts(graph)?;
        Ok(c)
    }
    fn validate_artifacts(&self, graph: &FrozenGraph) -> Result<(), ApiError> {
        for (id, order) in &self.state.data().orders {
            if self
                .anchors
                .pickups
                .get(id)
                .is_none_or(|a| a.validate(graph, order.pickup).is_err())
                || self
                    .anchors
                    .dropoffs
                    .get(id)
                    .is_none_or(|a| a.validate(graph, order.dropoff).is_err())
                || self.policies.get(id).is_none_or(|p| p.validate().is_err())
            {
                return Err(ApiError::invalid("missing/corrupt order planning context"));
            }
        }
        for (id, rider) in &self.state.data().riders {
            if self
                .anchors
                .riders
                .get(id)
                .is_none_or(|a| a.validate(graph, rider.coordinate).is_err())
                || self.state.data().profiles[id].routing_profile
                    != graph.metadata().routing_profile()
            {
                return Err(ApiError::invalid("missing/corrupt rider planning context"));
            }
        }
        for action in self.state.execution().values() {
            let leg = &action.route;
            if leg.routing.graph_digest != graph.metadata().snapshot_digest()
                || leg.routing.profile != graph.metadata().routing_profile()
                || leg.nodes.first() != Some(&leg.from)
                || leg.nodes.last() != Some(&leg.to)
                || leg.nodes.len() != leg.edges.len() + 1
            {
                return Err(ApiError::invalid("frozen execution graph/route mismatch"));
            }
            for (i, id) in leg.edges.iter().enumerate() {
                let e = graph.edge(*id).ok_or_else(ApiError::internal)?;
                if e.from() != leg.nodes[i]
                    || e.to() != leg.nodes[i + 1]
                    || (i > 0 && !graph.is_maneuver_allowed(leg.edges[i - 1], *id))
                {
                    return Err(ApiError::invalid("invalid frozen route traversals"));
                }
            }
        }
        Ok(())
    }
}
impl Api {
    /// Opens the persistent single-writer API, preserving namespace, counters, terms,
    /// frozen work, recognition and history. Run `PgStore::migrate` offline first.
    /// This synchronous constructor must run on a blocking thread.
    /// # Errors
    /// Rejects stale writer/head, corrupt artifacts/configuration, backwards clock or schema.
    pub fn open_postgres(
        graph: FrozenGraph,
        config: ApiConfig,
        url: &str,
        world_key: &str,
        anchor_path: &std::path::Path,
        expected: Option<&DurableHead>,
    ) -> Result<Self, String> {
        let mut api = Self::new(graph.clone(), config.clone())?;
        let c = api
            .inner
            .coordinator
            .lock()
            .map_err(|_| "coordinator poisoned")?;
        let initial = c
            .image_for(&api, api.inner.clock.now().map_err(|e| e.to_string())?)
            .map_err(|e| e.body.message)?;
        drop(c);
        let config_value = json!({"semantics":"roadrunner.http-runtime/v1","policies":config.policies,"recovery":config.recovery,"work_budget":config.work_budget,"certification_window":config.certification_window});
        let config_digest = format!("{:x}", Sha256::digest(config_value.to_string().as_bytes()));
        let (mut storage, image) = PgStore::open(
            url,
            world_key,
            graph.clone(),
            &config_digest,
            initial,
            anchor_path,
            expected,
        )
        .map_err(|e| e.to_string())?;
        let mut c = Coordinator::restore(&graph, &image).map_err(|e| e.body.message)?;
        for action in c.state.execution().values() {
            if let TrafficIdentity::Static(digest) = &action.route.routing.traffic {
                let artifact = storage
                    .traffic_artifact(digest)
                    .map_err(|e| e.to_string())?;
                let t: TrafficRecord =
                    serde_json::from_value(artifact).map_err(|e| e.to_string())?;
                if t.graph_digest != graph.metadata().snapshot_digest() || &t.digest != digest {
                    return Err("frozen traffic provenance mismatch".into());
                }
                let traffic = TrafficSnapshot::new(
                    &graph,
                    t.overrides.into_iter().map(|(id, m)| (EdgeId::new(id), m)),
                )
                .map_err(|e| e.to_string())?;
                if traffic.traffic_snapshot_digest() != digest {
                    return Err("frozen traffic artifact mismatch".into());
                }
            }
        }
        let origin = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs_f64();
        if origin < image.clock_floor {
            return Err("host clock precedes durable clock floor".into());
        }
        let clock = Arc::new(HostClock {
            origin,
            started: Instant::now(),
            domain: image.time_domain.clone(),
        });
        // A new exclusive fenced owner sees terminal COMMITs or durable reservations.
        // Remaining pending records prove that effects and terminal publication never committed.
        let interrupted=c.commands.values_mut().filter(|r|r.envelope.outcome=="pending").map(|r|{r.status=StatusCode::CONFLICT;r.envelope.outcome="interrupted_before_publication".into();r.envelope.result=json!({"code":"interrupted_before_publication","message":"previous writer ended before atomic business publication; use a new key for a new attempt"});}).count();
        if interrupted > 0 {
            let now = clock.now().map_err(|e| e.to_string())?;
            let mut next = c
                .image(&graph, &image.time_domain, now)
                .map_err(|e| e.body.message)?;
            next.adapter["traffic"] = image.adapter["traffic"].clone();
            let next = storage
                .commit(
                    &image,
                    &next,
                    || Ok(next.clone()),
                    &[json!({"kind":"commands.interrupted","count":interrupted})],
                )
                .map_err(|e| e.to_string())?;
            c = Coordinator::restore(&graph, &next).map_err(|e| e.body.message)?;
        }
        let namespace = c.state.revision().namespace;
        let inner = Arc::get_mut(&mut api.inner).ok_or("unexpected shared bootstrap")?;
        *inner = Inner {
            graph: Arc::new(graph),
            workers: Arc::new(tokio::sync::Semaphore::new(config.workers)),
            export_admission: Arc::new(tokio::sync::Semaphore::new(config.workers)),
            config,
            clock,
            namespace,
            coordinator: Mutex::new(c),
            storage: Some(Mutex::new(storage)),
        };
        Ok(api)
    }
    /// Reads the exact durable head for backup/restore watermarking. Volatile APIs return None.
    /// # Errors
    /// Rejects a lost/fenced/uncertain database writer.
    pub fn durable_head(&self) -> Result<Option<DurableHead>, String> {
        self.inner
            .storage
            .as_ref()
            .map(|store| {
                let mut s = store.lock().map_err(|_| "storage poisoned".to_owned())?;
                s.check().map_err(|e| e.to_string())?;
                Ok(s.head())
            })
            .transpose()
    }
}
pub(crate) fn storage_error(error: StorageError) -> ApiError {
    let code = match error {
        StorageError::OutcomeUnknown => "commit_outcome_unknown",
        StorageError::Expired => "evaluation_context_expired",
        StorageError::Fenced => "writer_fenced",
        _ => "storage_unavailable",
    };
    ApiError::new(StatusCode::SERVICE_UNAVAILABLE, code, error)
}
/// Stage, validate and persist before installing local visibility. A failed SQL operation
/// discards the complete private replacement; uncertain COMMIT stops the cache authority.
pub(crate) fn write<T>(
    api: &Api,
    c: &mut Coordinator,
    apply: impl FnOnce(&mut Coordinator) -> Result<T, ApiError>,
) -> Result<T, ApiError> {
    let Some(store) = &api.inner.storage else {
        return apply(c);
    };
    let mut store = store.lock().map_err(|_| ApiError::internal())?;
    store.check().map_err(storage_error)?;
    let floor = DispatchInstant::new(c.clock_floor).map_err(ApiError::invalid)?;
    let before = c.image_for(api, floor)?;
    let mut next = Coordinator::restore(&api.inner.graph, &before)?;
    let result = apply(&mut next)?;
    let now = api.inner.clock.now().map_err(ApiError::invalid)?;
    let provisional = next.image_for(api, now)?;
    let mut events = Vec::new();
    for transition in next.state.history().iter().skip(c.state.history().len()) {
        events.push(json!({"kind":"operational.transition","record":transition}));
    }
    for (key, record) in &next.commands {
        if record.envelope.outcome != "pending"
            && c.commands
                .get(key)
                .is_none_or(|old| old.envelope.outcome == "pending")
        {
            events.push(json!({"kind":"command.completed","command":record.envelope.command,"principal":key.0,"key":key.1}));
        }
    }
    let committed = store
        .commit(
            &before,
            &provisional,
            || {
                let at = api
                    .inner
                    .clock
                    .now()
                    .map_err(|e| StorageError::Staging(e.to_string()))?;
                next.state
                    .stamp_storage_commit(&c.state, at)
                    .map_err(|e| StorageError::Staging(e.to_string()))?;
                for r in next.commands.values_mut() {
                    if let Some(publication) = r.envelope.result.get_mut("publication") {
                        if let Some(p) = next.state.publications().iter().find(|p| {
                            publication["committed_revision"]
                                == serde_json::to_value(p.committed_revision).unwrap_or(Value::Null)
                        }) {
                            *publication = serde_json::to_value(p)
                                .map_err(|e| StorageError::Staging(e.to_string()))?;
                        }
                    }
                }
                next.image_for(api, at)
                    .map_err(|e| StorageError::Staging(e.body.message))
            },
            &events,
        )
        .map_err(storage_error)?;
    *c = Coordinator::restore(&api.inner.graph, &committed)?;
    Ok(result)
}
pub(crate) fn check(api: &Api) -> Result<(), ApiError> {
    if let Some(store) = &api.inner.storage {
        store
            .lock()
            .map_err(|_| ApiError::internal())?
            .check()
            .map_err(storage_error)?;
    }
    Ok(())
}
pub(crate) async fn check_async(api: &Api) -> Result<(), ApiError> {
    if api.inner.storage.is_some() {
        let api = api.clone();
        tokio::task::spawn_blocking(move || check(&api))
            .await
            .map_err(|_| ApiError::internal())??;
    }
    Ok(())
}
