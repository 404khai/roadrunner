//! At-least-once local transport. Current operational facts remain in `PgStore`.
//! Projection and notification consumers never turn integration events into observations.
use crate::{
    DurableHead, MIGRATION_LOCK, StorageError, anchor, connect, digest, installation_fingerprint,
    lock_key, verify_installation,
};
use postgres::{Client, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

/// Independently versioned consumer implementation semantics.
pub const CONSUMER_SEMANTICS: &str = "roadrunner.integration-consumer/v1";
/// Stable local transport envelope. Sequence orders this source, not domain revisions.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Delivery {
    /// Wire format version.
    pub schema_version: u32,
    /// Namespace-qualified source partition.
    pub source: String,
    /// Stable transport message identity (separate from domain/effect identities).
    pub message_id: String,
    /// Contiguous source sequence encoded losslessly.
    pub sequence: String,
    /// Original immutable outbox event.
    pub event: Value,
}
impl Delivery {
    /// Canonical identity/meaning fingerprint, excluding receipt time.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        digest(&json!(self))
    }
}
/// Consumer effects supported by this version; neither mutates operational authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsumerKind {
    /// Checked read model of committed world/execution/context/effect facts.
    Projection,
    /// Durable local notification obligations, with remote sending suppressed.
    Notification,
}
impl ConsumerKind {
    fn name(self) -> &'static str {
        match self {
            Self::Projection => "projection",
            Self::Notification => "notification",
        }
    }
}
/// Connection role. Each writer owns one session lease and cannot reconnect implicitly.
#[derive(Clone, Debug)]
pub enum Role {
    /// Serializes outbox -> transport publication for one source partition.
    Publisher,
    /// Binds a stable consumer name to its effect semantics.
    Consumer {
        /// Stable consumer identity.
        name: String,
        /// Immutable effect implementation selection.
        kind: ConsumerKind,
    },
}
/// Durable receipt; acknowledgement is permitted only for terminal outcomes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Receipt {
    /// Applied, `already_applied`, deferred, quarantined, skipped, or `identity_conflict`.
    pub state: String,
    /// Original immutable terminal result or diagnostic evidence.
    pub outcome: Value,
}
impl Receipt {
    /// Whether delivery may be acknowledged after durable receipt.
    #[must_use]
    pub fn terminal(&self) -> bool {
        matches!(
            self.state.as_str(),
            "applied" | "already_applied" | "skipped"
        )
    }
}
/// Synchronous, single-session event writer. Use a blocking worker in async runtimes.
pub struct EventStore {
    client: Client,
    namespace: String,
    role: Role,
    failed: bool,
    anchor: anchor::Anchor,
    head: DurableHead,
    installation: String,
    actor: String,
    world_key: String,
}
impl EventStore {
    /// Opens a bound publisher/consumer after offline migration. Active duplicates reject.
    /// # Errors
    /// Rejects missing worlds, incompatible semantics, exclusion or installation binding.
    pub fn open(
        url: &str,
        world_key: &str,
        role: Role,
        anchor_path: &Path,
    ) -> Result<Self, StorageError> {
        let anchor = anchor::Anchor::acquire(anchor_path)?;
        let watermark = anchor.read()?;
        let mut client = connect(url)?;
        verify_installation(&mut client)?;
        let namespace: String = client
            .query_one(
                "SELECT namespace FROM rr_worlds WHERE world_key=$1",
                &[&world_key],
            )?
            .get(0);
        let actor = match &role {
            Role::Publisher => "publisher".to_owned(),
            Role::Consumer { name, .. } => {
                validate_name(name)?;
                format!("consumer/{name}")
            }
        };
        let shared: bool = client
            .query_one("SELECT pg_try_advisory_lock_shared($1)", &[&MIGRATION_LOCK])?
            .get(0);
        let owned: bool = client
            .query_one(
                "SELECT pg_try_advisory_lock($1)",
                &[&lock_key(&format!("events/{namespace}/{actor}"))],
            )?
            .get(0);
        if !shared || !owned {
            return Err(StorageError::Fenced);
        }
        client.query_one(
            "SELECT set_config('application_name',$1,false)",
            &[&format!("roadrunner-events/{world_key}/{actor}")],
        )?;
        if let Role::Consumer { name, kind } = &role {
            client.execute("INSERT INTO rr_event_consumers(namespace,consumer,semantics,kind) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING", &[&namespace,name,&CONSUMER_SEMANTICS,&kind.name()])?;
            let row = client.query_one(
                "SELECT semantics,kind FROM rr_event_consumers WHERE namespace=$1 AND consumer=$2",
                &[&namespace, name],
            )?;
            if row.get::<_, String>(0) != CONSUMER_SEMANTICS
                || row.get::<_, String>(1) != kind.name()
            {
                return Err(StorageError::Binding);
            }
        }
        let installation = installation_fingerprint(&mut client, url)?;
        let binding = format!("{world_key}/{actor}");
        let mut tx = client.transaction()?;
        let current = event_snapshot(&mut tx, &namespace, &actor)?;
        let head = if let Some(row)=tx.query_opt("SELECT revision,fingerprint FROM rr_event_heads WHERE namespace=$1 AND actor=$2 FOR UPDATE", &[&namespace,&actor])? {
            let head=DurableHead{revision:row.get(0),digest:row.get(1)};
            let marker=watermark.as_ref().ok_or(StorageError::Binding)?;
            if marker.world_key!=binding || marker.namespace!=namespace || marker.installation!=installation || head.digest!=current {return Err(StorageError::Binding)}
            verify_event_watermark(&mut tx,&namespace,&actor,&head,&marker.head)?;
            head
        } else {
            if watermark.is_some(){return Err(StorageError::Binding)}
            tx.execute("INSERT INTO rr_event_heads VALUES($1,$2,0,$3)", &[&namespace,&actor,&current])?;
            tx.execute("INSERT INTO rr_event_receipts VALUES($1,$2,0,$3,NULL)", &[&namespace,&actor,&current])?;
            DurableHead{revision:0,digest:current}
        };
        tx.commit()?;
        anchor.write(&binding, &namespace, &installation, &head)?;
        Ok(Self {
            client,
            namespace,
            role,
            failed: false,
            anchor,
            head,
            installation,
            actor,
            world_key: binding,
        })
    }
    /// Source namespace. Delivery cannot cross this authority boundary.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
    fn consumer(&self) -> Result<(String, ConsumerKind), StorageError> {
        match &self.role {
            Role::Consumer { name, kind } => Ok((name.clone(), *kind)),
            Role::Publisher => Err(StorageError::Binding),
        }
    }
    fn live(&self) -> Result<(), StorageError> {
        if self.failed {
            Err(StorageError::OutcomeUnknown)
        } else {
            Ok(())
        }
    }
    /// Publishes the oldest outstanding obligation. Append and publisher ACK are separate.
    /// A crash after append reuses the same source position/body on redelivery.
    /// # Errors
    /// Rejects nonpublisher role, corrupt origin or unknown COMMIT; retires on uncertainty.
    pub fn publish(&mut self) -> Result<Option<Delivery>, StorageError> {
        self.live()?;
        if !matches!(self.role, Role::Publisher) {
            return Err(StorageError::Binding);
        }
        let mut tx = self.client.transaction()?;
        event_gate(&mut tx, &self.namespace, &self.actor, &self.head)?;
        let Some(row) = tx.query_opt("SELECT event_id,body FROM rr_outbox o WHERE namespace=$1 AND NOT EXISTS(SELECT 1 FROM rr_publish_ack a WHERE a.event_id=o.event_id) ORDER BY storage_revision,ordinal LIMIT 1", &[&self.namespace])? else {return Ok(None)};
        let id: String = row.get(0);
        let body: Value = row.get(1);
        let sequence = if let Some(row) = tx.query_opt(
            "SELECT sequence,body,fingerprint FROM rr_stream WHERE event_id=$1",
            &[&id],
        )? {
            if row.get::<_, Value>(1) != body || row.get::<_, String>(2) != digest(&body) {
                return Err(StorageError::Corrupt);
            }
            row.get::<_, i64>(0)
        } else {
            tx.execute(
                "INSERT INTO rr_stream_heads VALUES($1,0,NULL) ON CONFLICT DO NOTHING",
                &[&self.namespace],
            )?;
            let head = tx.query_one(
                "SELECT sequence,event_id FROM rr_stream_heads WHERE namespace=$1 FOR UPDATE",
                &[&self.namespace],
            )?;
            let seq = head
                .get::<_, i64>(0)
                .checked_add(1)
                .ok_or(StorageError::Corrupt)?;
            let predecessor: Option<String> = head.get(1);
            tx.execute(
                "INSERT INTO rr_stream VALUES($1,$2,$3,$4,$5,$6)",
                &[
                    &self.namespace,
                    &seq,
                    &id,
                    &predecessor,
                    &body,
                    &digest(&body),
                ],
            )?;
            tx.execute(
                "UPDATE rr_stream_heads SET sequence=$2,event_id=$3 WHERE namespace=$1",
                &[&self.namespace, &seq, &id],
            )?;
            seq
        };
        let delivery = delivery(&self.namespace, sequence, body);
        attempt(
            &mut tx,
            &self.namespace,
            "publisher",
            &delivery,
            "published",
        )?;
        finish_transaction(
            tx,
            &mut self.failed,
            &self.namespace,
            &self.actor,
            &mut self.head,
            &self.anchor,
            &self.world_key,
            &self.installation,
        )?;
        Ok(Some(delivery))
    }
    /// Records transport acceptance, not consumer application. Repeated ACK is identical.
    /// # Errors
    /// Rejects a changed/nonpublished event or failed writer.
    pub fn acknowledge_publish(&mut self, delivery: &Delivery) -> Result<(), StorageError> {
        self.live()?;
        if !matches!(self.role, Role::Publisher) {
            return Err(StorageError::Binding);
        }
        let mut tx = self.client.transaction()?;
        event_gate(&mut tx, &self.namespace, &self.actor, &self.head)?;
        validate_transport(&mut tx, &self.namespace, delivery, false)
            .map_err(|_| StorageError::Binding)?;
        let id = event_id(delivery)?;
        tx.execute(
            "INSERT INTO rr_publish_ack(event_id,fingerprint) VALUES($1,$2) ON CONFLICT DO NOTHING",
            &[&id, &digest(&delivery.event)],
        )?;
        attempt(
            &mut tx,
            &self.namespace,
            "publisher",
            delivery,
            "acknowledged",
        )?;
        finish_transaction(
            tx,
            &mut self.failed,
            &self.namespace,
            &self.actor,
            &mut self.head,
            &self.anchor,
            &self.world_key,
            &self.installation,
        )
    }
    /// Oldest unacknowledged delivery, including quarantined/deferred work. Does not skip poison.
    /// # Errors
    /// Rejects a failed writer or wrong role.
    pub fn next_delivery(&mut self) -> Result<Option<Delivery>, StorageError> {
        self.live()?;
        let (consumer, _) = self.consumer()?;
        let mut tx = self.client.transaction()?;
        event_gate(&mut tx, &self.namespace, &self.actor, &self.head)?;
        let row = tx.query_opt("SELECT sequence,body FROM rr_stream s WHERE namespace=$1 AND NOT EXISTS(SELECT 1 FROM rr_delivery_ack a WHERE a.namespace=s.namespace AND a.consumer=$2 AND a.message_id=s.event_id AND a.source=$3) ORDER BY sequence LIMIT 1", &[&self.namespace,&consumer,&source(&self.namespace)])?;
        Ok(row.map(|r| delivery(&self.namespace, r.get(0), r.get(1))))
    }
    /// Atomically recognizes message meaning, source order, business effect and original outcome.
    /// Deferred/quarantined inputs remain unacknowledged; conflicting retries preserve original.
    /// # Errors
    /// SQL rollback leaves neither effects nor success receipt; ambiguous COMMIT retires writer.
    #[allow(clippy::too_many_lines)] // Recognition, ordering, reduction and outcome share one transaction.
    pub fn receive(&mut self, delivery: &Delivery) -> Result<Receipt, StorageError> {
        self.live()?;
        let (consumer, kind) = self.consumer()?;
        let sequence = parse_sequence(delivery)?;
        validate_name(&delivery.message_id)?;
        let hash = delivery.fingerprint();
        let mut tx = self.client.transaction()?;
        event_gate(&mut tx, &self.namespace, &self.actor, &self.head)?;
        let cursor:i64=tx.query_one("SELECT sequence FROM rr_event_consumers WHERE namespace=$1 AND consumer=$2 FOR UPDATE", &[&self.namespace,&consumer])?.get(0);
        let prior=tx.query_opt("SELECT fingerprint,state,outcome FROM rr_inbox WHERE namespace=$1 AND consumer=$2 AND source=$3 AND message_id=$4", &[&self.namespace,&consumer,&delivery.source,&delivery.message_id])?;
        let mut receipt = if let Some(row) = &prior {
            if row.get::<_, String>(0) != hash {
                Some(Receipt {
                    state: "identity_conflict".into(),
                    outcome: json!({"reason":"same source/message identity has different canonical meaning"}),
                })
            } else if matches!(
                row.get::<_, String>(1).as_str(),
                "applied" | "already_applied" | "skipped"
            ) {
                Some(Receipt {
                    state: row.get(1),
                    outcome: row.get(2),
                })
            } else {
                None
            }
        } else {
            None
        };
        if receipt.is_none() {
            receipt = Some(
                if let Err(reason) = validate_transport(&mut tx, &self.namespace, delivery, true) {
                    Receipt {
                        state: "quarantined".into(),
                        outcome: json!({"reason":reason}),
                    }
                } else {
                    let (business_id, effect) = effect_input(kind, delivery)
                        .unwrap_or_else(|_| ("invalid".into(), json!({"kind":"invalid"})));
                    let old=tx.query_opt("SELECT fingerprint,effect FROM rr_consumer_effects WHERE namespace=$1 AND consumer=$2 AND event_id=$3", &[&self.namespace,&consumer,&business_id])?;
                    if sequence > cursor.checked_add(1).ok_or(StorageError::Corrupt)? {
                        Receipt {
                            state: "deferred".into(),
                            outcome: json!({"reason":"source dependency gap","expected_sequence":(cursor+1).to_string()}),
                        }
                    } else if let Some(old) = old {
                        if old.get::<_, String>(0) == digest(&effect) {
                            Receipt {
                                state: "already_applied".into(),
                                outcome: json!({"effect":business_id,"original_effect":old.get::<_,Value>(1)}),
                            }
                        } else {
                            Receipt {
                                state: "quarantined".into(),
                                outcome: json!({"reason":"conflicting business effect identity"}),
                            }
                        }
                    } else if sequence <= cursor {
                        Receipt {
                            state: "quarantined".into(),
                            outcome: json!({"reason":"superseded source position"}),
                        }
                    } else {
                        let mut model = load_model(&mut tx, &self.namespace, &consumer)?;
                        if let Err(reason) = reduce(kind, &mut model, &effect) {
                            Receipt {
                                state: "quarantined".into(),
                                outcome: json!({"reason":reason}),
                            }
                        } else {
                            tx.execute(
                                "INSERT INTO rr_consumer_effects VALUES($1,$2,$3,$4,$5,$6)",
                                &[
                                    &self.namespace,
                                    &consumer,
                                    &business_id,
                                    &sequence,
                                    &digest(&effect),
                                    &effect,
                                ],
                            )?;
                            if kind == ConsumerKind::Projection {
                                write_model(&mut tx, &self.namespace, &consumer, &model)?;
                            } else {
                                tx.execute(
                                    "INSERT INTO rr_notifications VALUES($1,$2,$3,$4)",
                                    &[&self.namespace, &consumer, &business_id, &effect],
                                )?;
                            }
                            Receipt {
                                state: "applied".into(),
                                outcome: json!({"effect":business_id,"projection_digest":digest(&json!(model))}),
                            }
                        }
                    }
                },
            );
        }
        let receipt = receipt.ok_or(StorageError::Corrupt)?;
        // Identity conflicts are retained as attempts; never replace the original input/outcome.
        if receipt.state != "identity_conflict" {
            let id = delivery.event["event_id"].as_str().unwrap_or("invalid");
            tx.execute("INSERT INTO rr_inbox(namespace,consumer,source,message_id,sequence,event_id,fingerprint,payload,state,outcome) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT(namespace,consumer,source,message_id) DO UPDATE SET state=excluded.state,outcome=excluded.outcome", &[&self.namespace,&consumer,&delivery.source,&delivery.message_id,&sequence,&id,&hash,&json!(delivery),&receipt.state,&receipt.outcome])?;
            if receipt.terminal() && sequence == cursor + 1 {
                tx.execute(
                    "UPDATE rr_event_consumers SET sequence=$3 WHERE namespace=$1 AND consumer=$2",
                    &[&self.namespace, &consumer, &sequence],
                )?;
            }
        }
        attempt(
            &mut tx,
            &self.namespace,
            &self.actor,
            delivery,
            &receipt.state,
        )?;
        finish_transaction(
            tx,
            &mut self.failed,
            &self.namespace,
            &self.actor,
            &mut self.head,
            &self.anchor,
            &self.world_key,
            &self.installation,
        )?;
        Ok(receipt)
    }
    /// Delivery acknowledgement after a durable terminal inbox result, never after receipt alone.
    /// # Errors
    /// Rejects nonterminal or conflicting meaning and uncertain COMMIT.
    pub fn acknowledge_delivery(&mut self, delivery: &Delivery) -> Result<(), StorageError> {
        self.live()?;
        let (consumer, _) = self.consumer()?;
        let mut tx = self.client.transaction()?;
        event_gate(&mut tx, &self.namespace, &self.actor, &self.head)?;
        let row=tx.query_one("SELECT fingerprint,state FROM rr_inbox WHERE namespace=$1 AND consumer=$2 AND source=$3 AND message_id=$4", &[&self.namespace,&consumer,&delivery.source,&delivery.message_id])?;
        if row.get::<_, String>(0) != delivery.fingerprint()
            || !matches!(
                row.get::<_, String>(1).as_str(),
                "applied" | "already_applied" | "skipped"
            )
        {
            return Err(StorageError::Binding);
        }
        tx.execute("INSERT INTO rr_delivery_ack(namespace,consumer,source,message_id,fingerprint) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING", &[&self.namespace,&consumer,&delivery.source,&delivery.message_id,&delivery.fingerprint()])?;
        attempt(
            &mut tx,
            &self.namespace,
            &self.actor,
            delivery,
            "delivery_acknowledged",
        )?;
        finish_transaction(
            tx,
            &mut self.failed,
            &self.namespace,
            &self.actor,
            &mut self.head,
            &self.anchor,
            &self.world_key,
            &self.installation,
        )
    }
    /// Explicit trusted operator reconciliation: skip the next quarantined source position.
    /// This records omission; it never fabricates a successful effect or rewrites input.
    /// # Errors
    /// Rejects stale fingerprints, nonquarantined/out-of-order inputs or missing principal/reason.
    pub fn skip(
        &mut self,
        message_id: &str,
        expected: &str,
        principal: &str,
        reason: &str,
    ) -> Result<Receipt, StorageError> {
        self.live()?;
        validate_name(principal)?;
        if reason.trim().is_empty() || reason.len() > 1024 {
            return Err(StorageError::Binding);
        }
        let (consumer, _) = self.consumer()?;
        let mut tx = self.client.transaction()?;
        event_gate(&mut tx, &self.namespace, &self.actor, &self.head)?;
        let cursor:i64=tx.query_one("SELECT sequence FROM rr_event_consumers WHERE namespace=$1 AND consumer=$2 FOR UPDATE", &[&self.namespace,&consumer])?.get(0);
        let row=tx.query_one("SELECT fingerprint,state,sequence,payload FROM rr_inbox WHERE namespace=$1 AND consumer=$2 AND source=$3 AND message_id=$4 FOR UPDATE", &[&self.namespace,&consumer,&source(&self.namespace),&message_id])?;
        let sequence: i64 = row.get(2);
        if row.get::<_, String>(0) != expected
            || row.get::<_, String>(1) != "quarantined"
            || sequence != cursor + 1
        {
            return Err(StorageError::Binding);
        }
        let receipt = Receipt {
            state: "skipped".into(),
            outcome: json!({"principal":principal,"reason":reason,"omitted_effect":true}),
        };
        tx.execute("INSERT INTO rr_reconciliations(namespace,consumer,source,message_id,principal,reason,expected_fingerprint,resolution) VALUES($1,$2,$3,$4,$5,$6,$7,'skip')", &[&self.namespace,&consumer,&source(&self.namespace),&message_id,&principal,&reason,&expected])?;
        tx.execute("UPDATE rr_inbox SET state='skipped',outcome=$5 WHERE namespace=$1 AND consumer=$2 AND source=$3 AND message_id=$4", &[&self.namespace,&consumer,&source(&self.namespace),&message_id,&receipt.outcome])?;
        tx.execute(
            "UPDATE rr_event_consumers SET sequence=$3 WHERE namespace=$1 AND consumer=$2",
            &[&self.namespace, &consumer, &sequence],
        )?;
        let delivery: Delivery =
            serde_json::from_value(row.get(3)).map_err(|_| StorageError::Corrupt)?;
        attempt(
            &mut tx,
            &self.namespace,
            &self.actor,
            &delivery,
            "operator_skipped",
        )?;
        finish_transaction(
            tx,
            &mut self.failed,
            &self.namespace,
            &self.actor,
            &mut self.head,
            &self.anchor,
            &self.world_key,
            &self.installation,
        )?;
        Ok(receipt)
    }
    /// Deterministic isolated processing replay of committed effects in recorded source order.
    /// Pure reduction produces no SQL mutations, notifications, acknowledgements or domain effects.
    /// # Errors
    /// Rejects incompatible semantics, corrupt input/effects or an inconsistent live projection.
    pub fn replay(&mut self) -> Result<Value, StorageError> {
        self.live()?;
        let (consumer, kind) = self.consumer()?;
        let mut tx = self.client.transaction()?;
        event_gate(&mut tx, &self.namespace, &self.actor, &self.head)?;
        let mut replay = ReplayState::default();
        let rows=tx.query("SELECT payload,result,fingerprint FROM rr_event_attempts WHERE namespace=$1 AND actor=$2 ORDER BY id", &[&self.namespace,&self.actor])?;
        let mut replayed = 0;
        for row in rows {
            let payload: Value = row.get(0);
            let d: Delivery = serde_json::from_value(payload).map_err(|_| StorageError::Corrupt)?;
            if d.fingerprint() != row.get::<_, String>(2) {
                return Err(StorageError::Corrupt);
            }
            let result: String = row.get(1);
            if result == "delivery_acknowledged" {
                continue;
            }
            let transport = validate_transport(&mut tx, &self.namespace, &d, true);
            let predicted = if result == "operator_skipped" {
                let prior = replay
                    .inbox
                    .get_mut(&(d.source.clone(), d.message_id.clone()))
                    .ok_or(StorageError::Corrupt)?;
                let row=tx.query_one("SELECT principal,reason,expected_fingerprint FROM rr_reconciliations WHERE namespace=$1 AND consumer=$2 AND source=$3 AND message_id=$4", &[&self.namespace,&consumer,&d.source,&d.message_id])?;
                if prior.1.state != "quarantined"
                    || row.get::<_, String>(2) != d.fingerprint()
                    || parse_sequence(&d)? != replay.cursor + 1
                {
                    return Err(StorageError::Corrupt);
                }
                prior.1 = Receipt {
                    state: "skipped".into(),
                    outcome: json!({"principal":row.get::<_,String>(0),"reason":row.get::<_,String>(1),"omitted_effect":true}),
                };
                replay.cursor += 1;
                "operator_skipped".into()
            } else {
                replay.apply(kind, &d, transport)?.state
            };
            if predicted != result {
                return Err(StorageError::Corrupt);
            }
            replayed += 1;
        }
        let mut ids = Vec::new();
        for row in tx.query("SELECT event_id,effect,fingerprint FROM rr_consumer_effects WHERE namespace=$1 AND consumer=$2 ORDER BY sequence,event_id", &[&self.namespace,&consumer])? {
            let id:String=row.get(0); let effect:Value=row.get(1);
            if digest(&effect)!=row.get::<_,String>(2) || replay.effects.get(&id)!=Some(&effect){return Err(StorageError::Corrupt)}
            ids.push(id);
        }
        if ids.len() != replay.effects.len() {
            return Err(StorageError::Corrupt);
        }
        let final_cursor: i64 = tx
            .query_one(
                "SELECT sequence FROM rr_event_consumers WHERE namespace=$1 AND consumer=$2",
                &[&self.namespace, &consumer],
            )?
            .get(0);
        let inbox_rows=tx.query("SELECT source,message_id,fingerprint,state,outcome FROM rr_inbox WHERE namespace=$1 AND consumer=$2", &[&self.namespace,&consumer])?;
        if final_cursor != replay.cursor || inbox_rows.len() != replay.inbox.len() {
            return Err(StorageError::Corrupt);
        }
        for row in inbox_rows {
            let key = (row.get::<_, String>(0), row.get::<_, String>(1));
            let expected = replay.inbox.get(&key).ok_or(StorageError::Corrupt)?;
            if expected.0 != row.get::<_, String>(2)
                || expected.1
                    != (Receipt {
                        state: row.get(3),
                        outcome: row.get(4),
                    })
            {
                return Err(StorageError::Corrupt);
            }
        }
        let model = replay.model;
        if kind == ConsumerKind::Projection
            && model != load_model(&mut tx, &self.namespace, &consumer)?
        {
            return Err(StorageError::Corrupt);
        }
        let omissions: i64 = tx
            .query_one(
                "SELECT count(*) FROM rr_reconciliations WHERE namespace=$1 AND consumer=$2",
                &[&self.namespace, &consumer],
            )?
            .get(0);
        Ok(
            json!({"schema_version":1,"semantics":CONSUMER_SEMANTICS,"namespace":self.namespace,"consumer":consumer,"kind":kind.name(),"isolated":true,"external_effects_suppressed":true,"replayed_attempts":replayed,"effect_ids":ids,"projection":model,"projection_digest":digest(&json!(model)),"reconciliation_omissions":omissions}),
        )
    }
    /// Read-only backlog inspection while runtime writers remain active. This is not authority restore.
    /// # Errors
    /// Rejects incompatible installation, unknown world or database errors.
    pub fn inspect(url: &str, world_key: &str) -> Result<Value, StorageError> {
        let mut client = connect(url)?;
        verify_installation(&mut client)?;
        let mut tx = client.transaction()?;
        tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")?;
        let ns: String = tx
            .query_one(
                "SELECT namespace FROM rr_worlds WHERE world_key=$1",
                &[&world_key],
            )?
            .get(0);
        read_status(&mut tx, &ns)
    }
    /// Counts backlog, outcomes and attempts for inspection. No customer payloads are logged.
    /// # Errors
    /// Rejects failed writer or database errors.
    pub fn status(&mut self) -> Result<Value, StorageError> {
        self.live()?;
        let mut tx = self.client.transaction()?;
        event_gate(&mut tx, &self.namespace, &self.actor, &self.head)?;
        read_status(&mut tx, &self.namespace)
    }
}
fn validate_name(value: &str) -> Result<(), StorageError> {
    if value.is_empty() || value.len() > 256 || !value.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(StorageError::Binding);
    }
    Ok(())
}
fn source(ns: &str) -> String {
    format!("{ns}:outbox/v1")
}
fn delivery(ns: &str, sequence: i64, event: Value) -> Delivery {
    Delivery {
        schema_version: 1,
        source: source(ns),
        message_id: event["event_id"].as_str().unwrap_or("invalid").into(),
        sequence: sequence.to_string(),
        event,
    }
}
fn event_id(d: &Delivery) -> Result<&str, StorageError> {
    d.event["event_id"].as_str().ok_or(StorageError::Corrupt)
}
fn parse_sequence(d: &Delivery) -> Result<i64, StorageError> {
    let n = d
        .sequence
        .parse::<i64>()
        .map_err(|_| StorageError::Binding)?;
    if n <= 0 || n.to_string() != d.sequence {
        return Err(StorageError::Binding);
    }
    Ok(n)
}
fn validate_transport(
    tx: &mut Transaction<'_>,
    ns: &str,
    d: &Delivery,
    check_semantics: bool,
) -> Result<(), String> {
    if d.schema_version != 1
        || d.source != source(ns)
        || (check_semantics && d.event["schema_version"] != 1)
    {
        return Err("unsupported schema or source binding".into());
    }
    let id = d.event["event_id"]
        .as_str()
        .ok_or("missing event identity")?;
    let seq = parse_sequence(d).map_err(|_| "invalid sequence")?;
    let row=tx.query_opt("SELECT s.body,s.fingerprint,o.body,s.predecessor FROM rr_stream s JOIN rr_outbox o USING(event_id) WHERE s.namespace=$1 AND s.sequence=$2 AND s.event_id=$3", &[&ns,&seq,&id]).map_err(|e|e.to_string())?.ok_or("missing original source position")?;
    if row.get::<_, Value>(0) != d.event
        || row.get::<_, Value>(2) != d.event
        || row.get::<_, String>(1) != digest(&d.event)
    {
        return Err("conflicting original event meaning".into());
    }
    if seq > 1 {
        let previous = tx
            .query_one(
                "SELECT event_id FROM rr_stream WHERE namespace=$1 AND sequence=$2",
                &[&ns, &(seq - 1)],
            )
            .map_err(|e| e.to_string())?;
        if row.get::<_, Option<String>>(3) != Some(previous.get(0)) {
            return Err("broken causal predecessor".into());
        }
    } else if row.get::<_, Option<String>>(3).is_some() {
        return Err("invalid initial predecessor".into());
    }
    if !check_semantics {
        return Ok(());
    }
    match d.event["payload"]["kind"].as_str() {
        Some("operational.transition" | "command.completed" | "commands.interrupted") => Ok(()),
        _ => Err("unsupported event semantics".into()),
    }
}
fn effect_input(kind: ConsumerKind, d: &Delivery) -> Result<(String, Value), StorageError> {
    if kind == ConsumerKind::Projection && d.event["payload"]["kind"] == "operational.transition" {
        let revision = d.event["storage_revision"]
            .as_i64()
            .ok_or(StorageError::Corrupt)?;
        let changes = d.event["payload"]["committed_facts"]
            .as_array()
            .ok_or(StorageError::Corrupt)?;
        // Multiple history records in one commit carry the same complete final delta.
        Ok((
            format!("{}:facts:{revision}", d.source),
            json!({"kind":"facts","changes":changes}),
        ))
    } else {
        Ok((
            event_id(d)?.into(),
            json!({"kind":"record","event":d.event}),
        ))
    }
}
fn reduce(
    kind: ConsumerKind,
    model: &mut BTreeMap<String, Value>,
    effect: &Value,
) -> Result<(), String> {
    if effect["kind"] == "invalid" {
        return Err("unsupported effect schema".into());
    }
    if kind != ConsumerKind::Projection || effect["kind"] != "facts" {
        return Ok(());
    }
    let changes = effect["changes"].as_array().ok_or("missing fact delta")?;
    for c in changes {
        let category = c["category"].as_str().ok_or("missing category")?;
        if !matches!(category, "world" | "execution" | "contexts" | "effects") {
            return Err("unsupported fact category".into());
        }
        let id = c["id"].as_str().ok_or("missing fact id")?;
        let field = if category == "world" {
            c["field"].as_str().ok_or("missing world field")?
        } else {
            ""
        };
        if category == "world"
            && !matches!(
                field,
                "orders"
                    | "readiness"
                    | "fulfillment"
                    | "profiles"
                    | "riders"
                    | "assignments"
                    | "accepted"
                    | "plans"
            )
        {
            return Err("unsupported world field".into());
        }
        if !c.is_object() || c.get("before").is_none() || c.get("after").is_none() {
            return Err("incomplete fact delta".into());
        }
        let path = format!("{category}/{field}/{id}");
        if category == "contexts" && !model.contains_key(&path) && !c["before"].is_null() {
            model.insert(path.clone(), c["before"].clone());
        }
        if model.get(&path).unwrap_or(&Value::Null) != &c["before"] {
            return Err(format!("projection dependency mismatch at {path}"));
        }
        if c["after"].is_null() {
            model.remove(&path);
        } else {
            model.insert(path, c["after"].clone());
        }
    }
    Ok(())
}
fn load_model(
    tx: &mut Transaction<'_>,
    ns: &str,
    consumer: &str,
) -> Result<BTreeMap<String, Value>, StorageError> {
    Ok(tx
        .query(
            "SELECT path,value FROM rr_projection WHERE namespace=$1 AND consumer=$2 ORDER BY path",
            &[&ns, &consumer],
        )?
        .into_iter()
        .map(|r| (r.get(0), r.get(1)))
        .collect())
}
fn write_model(
    tx: &mut Transaction<'_>,
    ns: &str,
    consumer: &str,
    model: &BTreeMap<String, Value>,
) -> Result<(), StorageError> {
    tx.execute(
        "DELETE FROM rr_projection WHERE namespace=$1 AND consumer=$2",
        &[&ns, &consumer],
    )?;
    for (path, value) in model {
        tx.execute(
            "INSERT INTO rr_projection VALUES($1,$2,$3,$4)",
            &[&ns, &consumer, path, value],
        )?;
    }
    Ok(())
}
fn attempt(
    tx: &mut Transaction<'_>,
    ns: &str,
    actor: &str,
    d: &Delivery,
    result: &str,
) -> Result<(), StorageError> {
    tx.execute("INSERT INTO rr_event_attempts(namespace,actor,message_id,fingerprint,payload,result) VALUES($1,$2,$3,$4,$5,$6)", &[&ns,&actor,&d.message_id,&d.fingerprint(),&json!(d),&result])?;
    Ok(())
}
#[allow(clippy::too_many_arguments)] // Explicit independent fields preserve the borrowed SQL transaction.
fn finish_transaction(
    tx: Transaction<'_>,
    failed: &mut bool,
    ns: &str,
    actor: &str,
    head: &mut DurableHead,
    anchor: &anchor::Anchor,
    binding: &str,
    installation: &str,
) -> Result<(), StorageError> {
    let mut tx = tx;
    let hash = event_snapshot(&mut tx, ns, actor)?;
    let revision = head.revision.checked_add(1).ok_or(StorageError::Corrupt)?;
    tx.execute(
        "UPDATE rr_event_heads SET revision=$3,fingerprint=$4 WHERE namespace=$1 AND actor=$2",
        &[&ns, &actor, &revision, &hash],
    )?;
    tx.execute(
        "INSERT INTO rr_event_receipts VALUES($1,$2,$3,$4,$5)",
        &[&ns, &actor, &revision, &hash, &head.digest],
    )?;
    match tx.commit() {
        Ok(()) => {
            let next = DurableHead {
                revision,
                digest: hash,
            };
            if anchor.write(binding, ns, installation, &next).is_err() {
                *failed = true;
                anchor.release();
                return Err(StorageError::OutcomeUnknown);
            }
            *head = next;
            Ok(())
        }
        Err(e) if e.as_db_error().is_some() => Err(StorageError::Database(e)),
        Err(_) => {
            *failed = true;
            anchor.release();
            Err(StorageError::OutcomeUnknown)
        }
    }
}
fn event_gate(
    tx: &mut Transaction<'_>,
    ns: &str,
    actor: &str,
    head: &DurableHead,
) -> Result<(), StorageError> {
    let row=tx.query_one("SELECT revision,fingerprint FROM rr_event_heads WHERE namespace=$1 AND actor=$2 FOR UPDATE", &[&ns,&actor])?;
    if row.get::<_, i64>(0) != head.revision
        || row.get::<_, String>(1) != head.digest
        || event_snapshot(tx, ns, actor)? != head.digest
    {
        return Err(StorageError::Fenced);
    }
    Ok(())
}
fn event_snapshot(tx: &mut Transaction<'_>, ns: &str, actor: &str) -> Result<String, StorageError> {
    let mut image = json!({});
    let consumer = actor.strip_prefix("consumer/");
    if let Some(consumer) = consumer {
        for table in [
            "rr_event_consumers",
            "rr_inbox",
            "rr_consumer_effects",
            "rr_projection",
            "rr_notifications",
            "rr_delivery_ack",
            "rr_reconciliations",
        ] {
            let sql = format!(
                "SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]') FROM {table} t WHERE namespace=$1 AND consumer=$2"
            );
            image[table] = tx.query_one(&sql, &[&ns, &consumer])?.get::<_, Value>(0);
        }
    } else {
        image["stream"]=tx.query_one("SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY sequence),'[]') FROM rr_stream t WHERE namespace=$1", &[&ns])?.get::<_,Value>(0);
        image["head"]=tx.query_one("SELECT COALESCE(jsonb_agg(to_jsonb(t)),'[]') FROM rr_stream_heads t WHERE namespace=$1", &[&ns])?.get::<_,Value>(0);
        image["ack"]=tx.query_one("SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY event_id),'[]') FROM rr_publish_ack t JOIN rr_stream s USING(event_id) WHERE s.namespace=$1", &[&ns])?.get::<_,Value>(0);
    }
    let attempt_actor = actor;
    image["attempts"]=tx.query_one("SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM rr_event_attempts t WHERE namespace=$1 AND actor=$2", &[&ns,&attempt_actor])?.get::<_,Value>(0);
    Ok(digest(&image))
}
fn verify_event_watermark(
    tx: &mut Transaction<'_>,
    ns: &str,
    actor: &str,
    head: &DurableHead,
    expected: &DurableHead,
) -> Result<(), StorageError> {
    if expected.revision < 0 || expected.revision > head.revision {
        return Err(StorageError::Binding);
    }
    let mut revision = expected.revision;
    let mut hash = expected.digest.clone();
    for (i,row) in tx.query("SELECT revision,fingerprint,previous FROM rr_event_receipts WHERE namespace=$1 AND actor=$2 AND revision>=$3 AND revision<=$4 ORDER BY revision", &[&ns,&actor,&expected.revision,&head.revision])?.into_iter().enumerate(){
        let actual:String=row.get(1);
        if row.get::<_,i64>(0)!=revision || (i==0 && actual!=hash) || (i>0 && row.get::<_,Option<String>>(2)!=Some(hash)){return Err(StorageError::Binding)}
        hash=actual;
        revision=revision.checked_add(1).ok_or(StorageError::Corrupt)?;
    }
    if revision != head.revision.checked_add(1).ok_or(StorageError::Corrupt)? || hash != head.digest
    {
        return Err(StorageError::Binding);
    }
    Ok(())
}

#[derive(Default)]
struct ReplayState {
    model: BTreeMap<String, Value>,
    inbox: BTreeMap<(String, String), (String, Receipt)>,
    effects: BTreeMap<String, Value>,
    cursor: i64,
}
impl ReplayState {
    #[allow(clippy::too_many_lines)] // Pure replay checks the full recorded message/ordering history.
    fn apply(
        &mut self,
        kind: ConsumerKind,
        d: &Delivery,
        transport: Result<(), String>,
    ) -> Result<Receipt, StorageError> {
        let key = (d.source.clone(), d.message_id.clone());
        let hash = d.fingerprint();
        let sequence = parse_sequence(d)?;
        if let Some((original, receipt)) = self.inbox.get(&key) {
            if *original != hash {
                return Ok(Receipt {
                    state: "identity_conflict".into(),
                    outcome: json!({"reason":"same source/message identity has different canonical meaning"}),
                });
            }
            if receipt.terminal() {
                return Ok(receipt.clone());
            }
        }
        let receipt = if let Err(reason) = transport {
            Receipt {
                state: "quarantined".into(),
                outcome: json!({"reason":reason}),
            }
        } else {
            let (id, effect) = effect_input(kind, d)
                .unwrap_or_else(|_| ("invalid".into(), json!({"kind":"invalid"})));
            if sequence > self.cursor.checked_add(1).ok_or(StorageError::Corrupt)? {
                Receipt {
                    state: "deferred".into(),
                    outcome: json!({"reason":"source dependency gap","expected_sequence":(self.cursor+1).to_string()}),
                }
            } else if let Some(old) = self.effects.get(&id) {
                if old == &effect {
                    Receipt {
                        state: "already_applied".into(),
                        outcome: json!({"effect":id,"original_effect":old}),
                    }
                } else {
                    Receipt {
                        state: "quarantined".into(),
                        outcome: json!({"reason":"conflicting business effect identity"}),
                    }
                }
            } else if sequence <= self.cursor {
                Receipt {
                    state: "quarantined".into(),
                    outcome: json!({"reason":"superseded source position"}),
                }
            } else {
                let mut model = self.model.clone();
                if let Err(reason) = reduce(kind, &mut model, &effect) {
                    Receipt {
                        state: "quarantined".into(),
                        outcome: json!({"reason":reason}),
                    }
                } else {
                    self.model = model;
                    self.effects.insert(id.clone(), effect);
                    Receipt {
                        state: "applied".into(),
                        outcome: json!({"effect":id,"projection_digest":digest(&json!(self.model))}),
                    }
                }
            }
        };
        if receipt.terminal() && sequence == self.cursor + 1 {
            self.cursor = sequence;
        }
        self.inbox.insert(key, (hash, receipt.clone()));
        Ok(receipt)
    }
}

fn read_status(tx: &mut Transaction<'_>, ns: &str) -> Result<Value, StorageError> {
    let mut status = json!({"namespace":ns});
    for (key, sql) in [
        (
            "outbox_pending",
            "SELECT count(*) FROM rr_outbox o WHERE namespace=$1 AND NOT EXISTS(SELECT 1 FROM rr_publish_ack a WHERE a.event_id=o.event_id)",
        ),
        (
            "stream_messages",
            "SELECT count(*) FROM rr_stream WHERE namespace=$1",
        ),
        (
            "effects",
            "SELECT count(*) FROM rr_consumer_effects WHERE namespace=$1",
        ),
        (
            "quarantined",
            "SELECT count(*) FROM rr_inbox WHERE namespace=$1 AND state='quarantined'",
        ),
        (
            "deferred",
            "SELECT count(*) FROM rr_inbox WHERE namespace=$1 AND state='deferred'",
        ),
        (
            "attempts",
            "SELECT count(*) FROM rr_event_attempts WHERE namespace=$1",
        ),
    ] {
        status[key] = json!(tx.query_one(sql, &[&ns])?.get::<_, i64>(0));
    }
    status["consumers"]=json!(tx.query("SELECT consumer,kind,sequence::text FROM rr_event_consumers WHERE namespace=$1 ORDER BY consumer", &[&ns])?.into_iter().map(|r|json!({"consumer":r.get::<_,String>(0),"kind":r.get::<_,String>(1),"source_sequence":r.get::<_,String>(2)})).collect::<Vec<_>>());
    Ok(status)
}
