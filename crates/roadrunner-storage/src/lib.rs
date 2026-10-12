//! PostgreSQL/PostGIS operational authority with one fenced session writer.
//! Relational current facts are authoritative; immutable records explain commits.
mod anchor;
mod entities;
use postgres::{Client, NoTls, Transaction};
use roadrunner_core::graph::{FrozenGraph, decode_graph_artifact, encode_graph_artifact};
use roadrunner_dispatch::OperationalState;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

const MIGRATION: &str = include_str!("../migrations/0001_operational.sql");
const MIGRATION_LOCK: i64 = 0x5252_4d49_4752;
/// Durable record version. Domain checkpoint and graph formats have independent versions.
pub const STORAGE_SCHEMA_VERSION: u32 = 1;
/// Complete adapter + domain before-image, reconstructed from relational current rows.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DurableImage {
    /// Versioned validated domain checkpoint.
    pub operational: Value,
    /// Versioned adapter projections/forecasts/commands/observation ordering. No credentials.
    pub adapter: Value,
    /// Named authoritative clock domain preserved across process restart.
    pub time_domain: String,
    /// Last persisted clock sample; backwards clock restoration rejects.
    pub clock_floor: f64,
}
impl DurableImage {
    /// Canonical content fingerprint, including domain and adapter recognition.
    #[must_use]
    pub fn digest(&self) -> String {
        digest(
            &json!({"operational":self.operational,"adapter":self.adapter,"time_domain":self.time_domain,"clock_floor":self.clock_floor}),
        )
    }
    fn validate(&self) -> Result<OperationalState, StorageError> {
        if self.time_domain.is_empty()
            || !self.clock_floor.is_finite()
            || self.clock_floor < 0.0
            || self.adapter["schema_version"] != 1
        {
            return Err(StorageError::Corrupt);
        }
        let state = OperationalState::restore_checkpoint(&self.operational)
            .map_err(|_| StorageError::Corrupt)?;
        if state.checkpoint().map_err(|_| StorageError::Corrupt)? != self.operational {
            return Err(StorageError::Corrupt);
        }
        Ok(state)
    }
}
/// Storage failure distinguishes exclusion, proven rejection, and uncertain COMMIT.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// PostgreSQL operation failed before COMMIT was attempted.
    #[error("PostgreSQL operation failed: {0}")]
    Database(#[from] postgres::Error),
    /// Unsupported schema or mismatched content/artifacts/installation.
    #[error("unsupported or corrupt durable authority")]
    Corrupt,
    /// Another session owns the namespace or an obsolete fence/head attempted publication.
    #[error("writer lease or expected durable head is unavailable/stale")]
    Fenced,
    /// Configured graph/policy or externally required head differs.
    #[error("durable restoration binding mismatch")]
    Binding,
    /// Publication expired at the transaction's final time gate.
    #[error("temporal certificate expired at durable commit gate")]
    Expired,
    /// Independent acknowledgement watermark could not be read/written durably.
    #[error("durability anchor unavailable: {0}")]
    Anchor(String),
    /// Connection failure while COMMIT was in flight; no noncommit claim is made.
    #[error(
        "commit outcome unknown; stop writes and restore/reconcile through a fresh fenced session"
    )]
    OutcomeUnknown,
    /// Staging or adapter validation failed without a durable effect.
    #[error("invalid staged transition: {0}")]
    Staging(String),
}
/// Last acknowledged durable head, suitable for an externally retained restore watermark.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DurableHead {
    /// Storage transaction revision, separate from domain revision.
    pub revision: i64,
    /// Full state/recognition content fingerprint.
    pub digest: String,
}
/// Synchronous PostgreSQL writer; use only on blocking threads. Dropping closes its
/// session lease. No reconnect or implicit leader takeover can reuse an obsolete fence.
pub struct PgStore {
    client: Client,
    namespace: String,
    fence: i64,
    head: DurableHead,
    graph: FrozenGraph,
    lease_key: i64,
    failed: bool,
    anchor: anchor::Anchor,
    world_key: String,
    installation: String,
}
impl PgStore {
    /// Applies the checksum-pinned initial schema offline. Active writers hold the
    /// shared migration lock, so schema changes cannot race operational publication.
    /// # Errors
    /// Rejects active writers, disabled fsync, unknown schema/checksum or clone binding.
    pub fn migrate(url: &str) -> Result<(), StorageError> {
        let mut c = connect(url)?;
        let locked: bool = c
            .query_one("SELECT pg_try_advisory_lock($1)", &[&MIGRATION_LOCK])?
            .get(0);
        if !locked {
            return Err(StorageError::Fenced);
        }
        let exists: bool = c
            .query_one("SELECT to_regclass('rr_installation') IS NOT NULL", &[])?
            .get(0);
        if exists {
            return verify_installation(&mut c);
        }
        let (cluster, oid) = installation(&mut c)?;
        let checksum = digest_bytes(MIGRATION.as_bytes());
        let mut tx = c.transaction()?;
        tx.batch_execute(MIGRATION)?;
        tx.execute("INSERT INTO rr_installation(singleton,cluster_id,database_oid,authority_table_oid,schema_version,migration_sha256) VALUES(true,$1,$2,'rr_worlds'::regclass::oid::bigint,1,$3)",&[&cluster,&oid,&checksum])?;
        tx.commit()?;
        Ok(())
    }
    /// Acquires the namespace writer, restores a coherent image, or atomically creates
    /// a fresh authority. Existing authority must match graph/configuration and optional
    /// externally retained head; installation binding rejects logical database clones.
    /// # Errors
    /// Rejects incompatible/corrupt state, active writer, missing artifacts or stale watermark.
    #[allow(clippy::too_many_lines)] // One atomic acquisition/restore/bootstrap sequence.
    pub fn open(
        url: &str,
        world_key: &str,
        graph: FrozenGraph,
        config_digest: &str,
        bootstrap: DurableImage,
        anchor_path: &Path,
        expected: Option<&DurableHead>,
    ) -> Result<(Self, DurableImage), StorageError> {
        if world_key.is_empty() || world_key.len() > 128 {
            return Err(StorageError::Binding);
        }
        let anchor = anchor::Anchor::acquire(anchor_path)?;
        let watermark = anchor.read()?;
        let mut client = connect(url)?;
        client.query_one(
            "SELECT set_config('application_name',$1,false)",
            &[&format!("roadrunner/{world_key}")],
        )?;
        verify_installation(&mut client)?;
        let shared: bool = client
            .query_one("SELECT pg_try_advisory_lock_shared($1)", &[&MIGRATION_LOCK])?
            .get(0);
        if !shared {
            return Err(StorageError::Fenced);
        }
        let lease_key = lock_key(world_key);
        let acquired: bool = client
            .query_one("SELECT pg_try_advisory_lock($1)", &[&lease_key])?
            .get(0);
        if !acquired {
            return Err(StorageError::Fenced);
        }
        let installation = installation_fingerprint(&mut client, url)?;
        if watermark
            .as_ref()
            .is_some_and(|w| w.world_key != world_key || w.installation != installation)
        {
            return Err(StorageError::Binding);
        }
        let graph_bytes = encode_graph_artifact(&graph).map_err(|_| StorageError::Corrupt)?;
        let graph_digest = graph.metadata().snapshot_digest().to_owned();
        let mut tx = client.transaction()?;
        tx.execute(
            "INSERT INTO rr_graphs(digest,sha256,artifact) VALUES($1,$2,$3) ON CONFLICT DO NOTHING",
            &[&graph_digest, &digest_bytes(&graph_bytes), &graph_bytes],
        )?;
        let row=tx.query_opt("SELECT namespace, fence,storage_revision,content_sha256,graph_digest,config_digest FROM rr_worlds WHERE world_key=$1 FOR UPDATE",&[&world_key])?;
        let (namespace, fence, head, image) = if let Some(row) = row {
            let namespace: String = row.get(0);
            let fence: i64 = row
                .get::<_, i64>(1)
                .checked_add(1)
                .ok_or(StorageError::Fenced)?;
            let head = DurableHead {
                revision: row.get(2),
                digest: row.get(3),
            };
            if row.get::<_, String>(4) != graph_digest || row.get::<_, String>(5) != config_digest {
                return Err(StorageError::Binding);
            }
            let watermark = watermark.as_ref().ok_or(StorageError::Binding)?;
            if watermark.namespace != namespace {
                return Err(StorageError::Binding);
            }
            verify_watermark(&mut tx, &namespace, &head, &watermark.head)?;
            if let Some(expected) = expected {
                verify_watermark(&mut tx, &namespace, &head, expected)?;
            }
            let stored = tx.query_one(
                "SELECT artifact,sha256 FROM rr_graphs WHERE digest=$1",
                &[&graph_digest],
            )?;
            let bytes: Vec<u8> = stored.get(0);
            if digest_bytes(&bytes) != stored.get::<_, String>(1)
                || decode_graph_artifact(&bytes)
                    .map_err(|_| StorageError::Corrupt)?
                    .metadata()
                    .snapshot_digest()
                    != graph_digest
            {
                return Err(StorageError::Corrupt);
            }
            let image = entities::load(&mut tx, &namespace)?;
            if image.digest() != head.digest
                || image.validate()?.revision().namespace.reference("world", 0) != namespace
            {
                return Err(StorageError::Corrupt);
            }
            tx.execute(
                "UPDATE rr_worlds SET fence=$2,certified_until=NULL WHERE namespace=$1",
                &[&namespace, &fence],
            )?;
            (namespace, fence, head, image)
        } else {
            if expected.is_some() || watermark.is_some() {
                return Err(StorageError::Binding);
            }
            let state = bootstrap.validate()?;
            let namespace = state.revision().namespace.reference("world", 0);
            let hash = bootstrap.digest();
            let metadata = entities::metadata(&bootstrap)?;
            let adapter = entities::adapter_metadata(&bootstrap);
            let op = state.revision().number.to_string();
            tx.execute("INSERT INTO rr_worlds(namespace,world_key,fence,storage_revision,operational_revision,content_sha256,graph_digest,config_digest,time_domain,clock_floor,metadata,adapter,gate_at) VALUES($1,$2,1,0,$3::text::rr_u64,$4,$5,$6,$7,$8,$9,$10,$8)",&[&namespace,&world_key,&op,&hash,&graph_digest,&config_digest,&bootstrap.time_domain,&bootstrap.clock_floor,&metadata,&adapter])?;
            tx.execute(
                "INSERT INTO rr_commits(namespace,revision,content_sha256) VALUES($1,0,$2)",
                &[&namespace, &hash],
            )?;
            entities::write_current(&mut tx, &namespace, &bootstrap, &graph)?;
            entities::write_evidence(&mut tx, &namespace, &bootstrap)?;
            (
                namespace,
                1,
                DurableHead {
                    revision: 0,
                    digest: hash,
                },
                bootstrap,
            )
        };
        tx.commit()?;
        anchor.write(world_key, &namespace, &installation, &head)?;
        Ok((
            Self {
                client,
                namespace,
                fence,
                head,
                graph,
                lease_key,
                failed: false,
                anchor,
                world_key: world_key.into(),
                installation,
            },
            image,
        ))
    }
    /// Current exact durable transaction head.
    #[must_use]
    pub fn head(&self) -> DurableHead {
        self.head.clone()
    }
    /// Immutable namespace-qualified world reference.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
    /// Checks that this connection still owns the fenced lease; stale processes may
    /// neither serve cached authority nor publish after a lease/connection loss.
    /// # Errors
    /// Rejects lost lease, changed head/fence or failed COMMIT reconciliation.
    pub fn check(&mut self) -> Result<(), StorageError> {
        if self.failed {
            return Err(StorageError::OutcomeUnknown);
        }
        let row=self.client.query_one("SELECT fence,storage_revision,content_sha256,EXISTS(SELECT 1 FROM pg_locks WHERE pid=pg_backend_pid() AND locktype='advisory' AND classid=$2::bigint::oid AND objid=$3::bigint::oid AND granted) FROM rr_worlds WHERE namespace=$1",&[&self.namespace,&i64::from(u32::from_be_bytes(self.lease_key.to_be_bytes()[..4].try_into().map_err(|_|StorageError::Corrupt)?)),&i64::from(u32::from_be_bytes(self.lease_key.to_be_bytes()[4..].try_into().map_err(|_|StorageError::Corrupt)?))]);
        let row = match row {
            Ok(row) => row,
            Err(e) => {
                self.failed = true;
                self.anchor.release();
                return Err(StorageError::Database(e));
            }
        };
        if row.get::<_, i64>(0) != self.fence
            || row.get::<_, i64>(1) != self.head.revision
            || row.get::<_, String>(2) != self.head.digest
            || !row.get::<_, bool>(3)
        {
            self.failed = true;
            self.anchor.release();
            return Err(StorageError::Fenced);
        }
        Ok(())
    }
    /// Publishes a complete staged replacement under the row/fence/before-image gate.
    /// Current relational facts are written first, then the callback samples the final
    /// clock/stamps authentic terms. Evidence, recognition and outbox commit together.
    /// A deferred SQL trigger rechecks temporal expiry at COMMIT.
    /// # Errors
    /// All pre-COMMIT failures roll back. Ambiguous COMMIT poisons this writer until restore.
    #[allow(clippy::too_many_lines)] // Keep atomic gate, facts, evidence, outbox and acknowledgement together.
    pub fn commit(
        &mut self,
        before: &DurableImage,
        provisional: &DurableImage,
        finalize: impl FnOnce() -> Result<DurableImage, StorageError>,
        events: &[Value],
    ) -> Result<DurableImage, StorageError> {
        self.check()?;
        if before.digest() != self.head.digest {
            return Err(StorageError::Fenced);
        }
        let state = provisional.validate()?;
        if state.revision().namespace.reference("world", 0) != self.namespace {
            return Err(StorageError::Binding);
        }
        let revision = self
            .head
            .revision
            .checked_add(1)
            .ok_or(StorageError::Fenced)?;
        let mut tx = self.client.transaction()?;
        gate(&mut tx, &self.namespace, self.fence, &self.head)?;
        if entities::load(&mut tx, &self.namespace)?.digest() != before.digest() {
            return Err(StorageError::Corrupt);
        }
        entities::write_current(&mut tx, &self.namespace, provisional, &self.graph)?;
        let image = finalize()?;
        verify_successor(before, &image)?;
        let restored = image.validate()?;
        if image.clock_floor < before.clock_floor
            || image.time_domain != before.time_domain
            || restored.revision().namespace != state.revision().namespace
        {
            return Err(StorageError::Binding);
        }
        entities::write_current(&mut tx, &self.namespace, &image, &self.graph)?;
        entities::write_evidence(&mut tx, &self.namespace, &image)?;
        let prior_publications = before.operational["state"]["publications"]
            .as_array()
            .ok_or(StorageError::Corrupt)?
            .len();
        let mut until = image.operational["state"]["publications"]
            .as_array()
            .ok_or(StorageError::Corrupt)?
            .iter()
            .skip(prior_publications)
            .filter_map(|p| p["valid_until"].as_f64())
            .reduce(f64::min);
        let old_terms = before.operational["state"]["world"]["data"]["accepted"]
            .as_object()
            .ok_or(StorageError::Corrupt)?;
        for (id, terms) in image.operational["state"]["world"]["data"]["accepted"]
            .as_object()
            .ok_or(StorageError::Corrupt)?
        {
            if !old_terms.contains_key(id) {
                let reference = terms["completion_reference"]
                    .as_f64()
                    .ok_or(StorageError::Corrupt)?;
                until = Some(until.map_or(reference, |t| t.min(reference)));
            }
        }
        let metadata = entities::metadata(&image)?;
        let adapter = entities::adapter_metadata(&image);
        let hash = image.digest();
        let op = restored.revision().number.to_string();
        tx.execute("UPDATE rr_worlds SET storage_revision=$2,operational_revision=$3::text::rr_u64,content_sha256=$4,metadata=$5,adapter=$6,clock_floor=$7,certified_until=$8,gate_at=$7 WHERE namespace=$1",&[&self.namespace,&revision,&op,&hash,&metadata,&adapter,&image.clock_floor,&until])?;
        for (ordinal, event) in events.iter().enumerate() {
            let ordinal = i32::try_from(ordinal).map_err(|_| StorageError::Corrupt)?;
            let id = format!("{}:event:{revision}:{ordinal}", self.namespace);
            let mut payload = event.clone();
            if event["kind"] == "operational.transition" {
                payload["committed_facts"] = facts_delta(before, &image)?;
            }
            if event["kind"] == "command.completed" {
                if let Some(record) = image.adapter["commands"]
                    .as_array()
                    .ok_or(StorageError::Corrupt)?
                    .iter()
                    .find(|r| r["principal"] == event["principal"] && r["key"] == event["key"])
                {
                    payload["record"] = record["record"].clone();
                }
            }
            let body = json!({"schema_version":1,"event_id":id,"storage_revision":revision,"operational_revision":restored.revision(),"ordinal":ordinal,"committed_at":image.clock_floor,"time_domain":image.time_domain,"previous_content_sha256":before.digest(),"content_sha256":hash,"writer_fence":self.fence,"payload":payload});
            tx.execute("INSERT INTO rr_outbox(namespace,storage_revision,ordinal,event_id,body) VALUES($1,$2,$3,$4,$5)",&[&self.namespace,&revision,&ordinal,&id,&body])?;
        }
        tx.execute("INSERT INTO rr_commits(namespace,revision,content_sha256,previous_sha256) VALUES($1,$2,$3,$4)",&[&self.namespace,&revision,&hash,&self.head.digest])?;
        match tx.commit() {
            Ok(()) => {
                let head = DurableHead {
                    revision,
                    digest: hash.clone(),
                };
                if self
                    .anchor
                    .write(&self.world_key, &self.namespace, &self.installation, &head)
                    .is_err()
                {
                    self.failed = true;
                    self.anchor.release();
                    return Err(StorageError::OutcomeUnknown);
                }
                self.head = DurableHead {
                    revision,
                    digest: hash,
                };
                Ok(image)
            }
            Err(e) if e.as_db_error().is_some() => {
                if e.as_db_error()
                    .is_some_and(|e| e.message().contains("temporal certificate"))
                {
                    Err(StorageError::Expired)
                } else {
                    Err(StorageError::Database(e))
                }
            }
            Err(_) => {
                self.failed = true;
                self.anchor.release();
                Err(StorageError::OutcomeUnknown)
            }
        }
    }
    /// Persists a read-only route query as immutable spatial evidence. No operational
    /// revision or business event is invented for this recording transaction.
    /// # Errors
    /// Rejects stale writer or failed spatial/integrity constraints.
    pub fn record_routes(
        &mut self,
        request: &Value,
        response: &Value,
    ) -> Result<Vec<String>, StorageError> {
        self.check()?;
        let revision = self
            .head
            .revision
            .checked_add(1)
            .ok_or(StorageError::Fenced)?;
        let mut tx = self.client.transaction()?;
        gate(&mut tx, &self.namespace, self.fence, &self.head)?;
        let routes = response["routes"].as_array().ok_or(StorageError::Corrupt)?;
        let records: Vec<_> = (0..routes.len())
            .map(|i| format!("{}:route:{revision}:{i}", self.namespace))
            .collect();
        let mut recorded = response.clone();
        recorded["route_records"] = json!(records);
        for (ordinal, route) in response["routes"]
            .as_array()
            .ok_or(StorageError::Corrupt)?
            .iter()
            .enumerate()
        {
            let ordinal = i32::try_from(ordinal).map_err(|_| StorageError::Corrupt)?;
            let geometry = route["geometry"].to_string();
            let digest = route["graph_digest"]
                .as_str()
                .ok_or(StorageError::Corrupt)?;
            tx.execute("INSERT INTO rr_routes(namespace,storage_revision,ordinal,graph_digest,request,response,geometry) VALUES($1,$2,$3,$4,$5,$6,rr_route_geometry($7))",&[&self.namespace,&revision,&ordinal,&digest,&request,&recorded,&geometry])?;
        }
        tx.execute(
            "UPDATE rr_worlds SET storage_revision=$2,certified_until=NULL WHERE namespace=$1",
            &[&self.namespace, &revision],
        )?;
        tx.execute("INSERT INTO rr_commits(namespace,revision,content_sha256,previous_sha256) VALUES($1,$2,$3,$3)",&[&self.namespace,&revision,&self.head.digest])?;
        match tx.commit() {
            Ok(()) => {
                let head = DurableHead {
                    revision,
                    digest: self.head.digest.clone(),
                };
                if self
                    .anchor
                    .write(&self.world_key, &self.namespace, &self.installation, &head)
                    .is_err()
                {
                    self.failed = true;
                    self.anchor.release();
                    return Err(StorageError::OutcomeUnknown);
                }
                self.head.revision = revision;
                Ok((0..response["routes"]
                    .as_array()
                    .ok_or(StorageError::Corrupt)?
                    .len())
                    .map(|i| format!("{}:route:{revision}:{i}", self.namespace))
                    .collect())
            }
            Err(e) if e.as_db_error().is_some() => Err(e.into()),
            Err(_) => {
                self.failed = true;
                self.anchor.release();
                Err(StorageError::OutcomeUnknown)
            }
        }
    }
    /// Loads retained immutable traffic evidence for frozen route validation/replay.
    /// # Errors
    /// Missing or corrupted required artifacts fail closed.
    pub fn traffic_artifact(&mut self, digest: &str) -> Result<Value, StorageError> {
        let row = self
            .client
            .query_opt(
                "SELECT body,sha256 FROM rr_traffic WHERE digest=$1",
                &[&digest],
            )?
            .ok_or(StorageError::Corrupt)?;
        let body: Value = row.get(0);
        if self::digest(&body) != row.get::<_, String>(1) {
            return Err(StorageError::Corrupt);
        }
        Ok(body)
    }
}
fn gate(
    tx: &mut Transaction<'_>,
    namespace: &str,
    fence: i64,
    head: &DurableHead,
) -> Result<(), StorageError> {
    let row = tx.query_one(
        "SELECT fence,storage_revision,content_sha256 FROM rr_worlds WHERE namespace=$1 FOR UPDATE",
        &[&namespace],
    )?;
    if row.get::<_, i64>(0) != fence
        || row.get::<_, i64>(1) != head.revision
        || row.get::<_, String>(2) != head.digest
    {
        return Err(StorageError::Fenced);
    }
    Ok(())
}
fn connect(url: &str) -> Result<Client, StorageError> {
    let mut c = Client::connect(url, NoTls)?;
    c.batch_execute(
        "SET synchronous_commit=on; SET lock_timeout='5s'; SET statement_timeout='30s'; SET client_connection_check_interval='100ms'",
    )?;
    let fsync: String = c.query_one("SHOW fsync", &[])?.get(0);
    if fsync != "on" {
        return Err(StorageError::Binding);
    }
    Ok(c)
}
fn installation(c: &mut Client) -> Result<(String, i64), StorageError> {
    let row=c.query_one("SELECT (pg_control_system()).system_identifier::text,(SELECT oid::bigint FROM pg_database WHERE datname=current_database())",&[])?;
    Ok((row.get(0), row.get(1)))
}
fn verify_installation(c: &mut Client) -> Result<(), StorageError> {
    let (cluster, oid) = installation(c)?;
    let row=c.query_one("SELECT cluster_id,database_oid,schema_version,migration_sha256,authority_table_oid FROM rr_installation WHERE singleton",&[])?;
    if row.get::<_, String>(0) != cluster
        || row.get::<_, i64>(1) != oid
        || row.get::<_, i32>(2) != 1
        || row.get::<_, String>(3) != digest_bytes(MIGRATION.as_bytes())
        || row.get::<_, i64>(4)
            != c.query_one("SELECT 'rr_worlds'::regclass::oid::bigint", &[])?
                .get::<_, i64>(0)
    {
        return Err(StorageError::Binding);
    }
    Ok(())
}
fn lock_key(key: &str) -> i64 {
    let hash = Sha256::digest(format!("roadrunner.writer/v1/{key}").as_bytes());
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&hash[..8]);
    i64::from_be_bytes(bytes)
}
pub(crate) fn digest(value: &Value) -> String {
    let mut value = value.clone();
    canonical_zero(&mut value);
    digest_bytes(value.to_string().as_bytes())
}
fn canonical_zero(v: &mut Value) {
    match v {
        Value::Number(n) if n.is_f64() && n.as_f64() == Some(0.0) => *v = json!(0.0),
        Value::Object(o) => o.values_mut().for_each(canonical_zero),
        Value::Array(a) => a.iter_mut().for_each(canonical_zero),
        _ => {}
    }
}
fn digest_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn verify_successor(before: &DurableImage, after: &DurableImage) -> Result<(), StorageError> {
    let a = &before.operational["state"];
    let b = &after.operational["state"];
    for key in ["history", "publications"] {
        let a = a[key].as_array().ok_or(StorageError::Corrupt)?;
        let b = b[key].as_array().ok_or(StorageError::Corrupt)?;
        if !b.starts_with(a) {
            return Err(StorageError::Corrupt);
        }
    }
    for key in ["orders", "profiles", "accepted"] {
        for (id, original) in a["world"]["data"][key]
            .as_object()
            .ok_or(StorageError::Corrupt)?
        {
            if &b["world"]["data"][key][id] != original {
                return Err(StorageError::Corrupt);
            }
        }
    }
    for key in [
        "next_action",
        "next_attempt",
        "next_effect",
        "next_order",
        "next_rider",
    ] {
        if b[key].as_u64().ok_or(StorageError::Corrupt)?
            < a[key].as_u64().ok_or(StorageError::Corrupt)?
        {
            return Err(StorageError::Corrupt);
        }
    }
    if b["revision"]["number"]
        .as_u64()
        .ok_or(StorageError::Corrupt)?
        < a["revision"]["number"]
            .as_u64()
            .ok_or(StorageError::Corrupt)?
    {
        return Err(StorageError::Corrupt);
    }
    Ok(())
}

fn facts_delta(before: &DurableImage, after: &DurableImage) -> Result<Value, StorageError> {
    let mut changes = Vec::new();
    let old = &before.operational["state"];
    let new = &after.operational["state"];
    for field in [
        "orders",
        "readiness",
        "fulfillment",
        "profiles",
        "riders",
        "assignments",
        "accepted",
        "plans",
    ] {
        let a = old["world"]["data"][field]
            .as_object()
            .ok_or(StorageError::Corrupt)?;
        let b = new["world"]["data"][field]
            .as_object()
            .ok_or(StorageError::Corrupt)?;
        let keys: std::collections::BTreeSet<_> = a.keys().chain(b.keys()).collect();
        for id in keys {
            if a.get(id) != b.get(id) {
                changes.push(json!({"category":"world","field":field,"id":id,"before":a.get(id),"after":b.get(id)}));
            }
        }
    }
    for field in ["execution", "contexts", "effects"] {
        let a = old[field].as_object().ok_or(StorageError::Corrupt)?;
        let b = new[field].as_object().ok_or(StorageError::Corrupt)?;
        let keys: std::collections::BTreeSet<_> = a.keys().chain(b.keys()).collect();
        for id in keys {
            if a.get(id) != b.get(id) {
                changes
                    .push(json!({"category":field,"id":id,"before":a.get(id),"after":b.get(id)}));
            }
        }
    }
    Ok(json!(changes))
}

fn verify_watermark(
    tx: &mut Transaction<'_>,
    ns: &str,
    current: &DurableHead,
    expected: &DurableHead,
) -> Result<(), StorageError> {
    if expected.revision < 0 || current.revision < expected.revision {
        return Err(StorageError::Binding);
    }
    let rows=tx.query("SELECT revision,content_sha256,previous_sha256 FROM rr_commits WHERE namespace=$1 AND revision>=$2 AND revision<=$3 ORDER BY revision",&[&ns,&expected.revision,&current.revision])?;
    let mut revision = expected.revision;
    let mut hash = expected.digest.clone();
    for (i, row) in rows.iter().enumerate() {
        let r: i64 = row.get(0);
        let h: String = row.get(1);
        if r != revision
            || (i == 0 && h != hash)
            || (i > 0 && row.get::<_, Option<String>>(2).as_deref() != Some(hash.as_str()))
        {
            return Err(StorageError::Binding);
        }
        hash = h;
        revision = revision.checked_add(1).ok_or(StorageError::Binding)?;
    }
    if revision
        != current
            .revision
            .checked_add(1)
            .ok_or(StorageError::Binding)?
        || hash != current.digest
    {
        return Err(StorageError::Binding);
    }
    Ok(())
}
fn installation_fingerprint(c: &mut Client, url: &str) -> Result<String, StorageError> {
    let (cluster, oid) = installation(c)?;
    let table: i64 = c
        .query_one("SELECT 'rr_worlds'::regclass::oid::bigint", &[])?
        .get(0);
    let config: url_config::Configuration = url.parse().map_err(|_| StorageError::Binding)?;
    Ok(digest(
        &json!({"cluster":cluster,"database_oid":oid,"authority_table_oid":table,"hosts":format!("{:?}",config.get_hosts()),"ports":config.get_ports()}),
    ))
}
mod url_config {
    pub type Configuration = postgres::Config;
}
