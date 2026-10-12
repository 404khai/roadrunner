//! Controlled real PostgreSQL delivery/inbox/effect histories; CI opts in explicitly.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use postgres::{Client, NoTls};
use roadrunner_core::graph::{FrozenGraph, decode_graph_artifact};
use roadrunner_dispatch::{
    Availability, CapacityUnits, OperationalState, OperationalWorldNamespace, RiderId,
    RiderProfile, RiderState, World, WorldData,
};
use roadrunner_storage::{
    DurableImage, PgStore, StorageError,
    events::{ConsumerKind, Delivery, EventStore, Role},
};
use serde_json::json;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}
fn graph() -> FrozenGraph {
    decode_graph_artifact(include_bytes!(
        "../../../data/fixtures/phase-10/snapshot.semantic-v2/graph.rr-graph"
    ))
    .unwrap()
}
struct Fixture {
    url: String,
    key: String,
    ns: String,
    root: PathBuf,
    image: DurableImage,
}
impl Fixture {
    fn new(kinds: &[&str]) -> Self {
        let url = std::env::var("ROADRUNNER_TEST_DATABASE_URL")
            .expect("dedicated disposable database required");
        Self::new_with_url(url, kinds)
    }
    fn new_with_url(url: String, kinds: &[&str]) -> Self {
        PgStore::migrate(&url).unwrap();
        let key = format!(
            "event-test-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::SeqCst)
        );
        let root = std::env::temp_dir().join(&key);
        std::fs::create_dir_all(&root).unwrap();
        let state = OperationalState::new(
            OperationalWorldNamespace::fresh().unwrap(),
            World::new(1, WorldData::default()).unwrap(),
        )
        .unwrap();
        let image = DurableImage {
            operational: state.checkpoint().unwrap(),
            adapter: json!({"schema_version":1,"commands":[],"observations":[]}),
            time_domain: "unix-test/v1".into(),
            clock_floor: now(),
        };
        let (mut store, mut before) = PgStore::open(
            &url,
            &key,
            graph(),
            "test/v1",
            image,
            &root.join("world.head"),
            None,
        )
        .unwrap();
        let ns = store.namespace().into();
        for kind in kinds {
            let mut state = OperationalState::restore_checkpoint(&before.operational).unwrap();
            let g = graph();
            state
                .register_rider(
                    state.revision(),
                    RiderProfile {
                        id: RiderId::new(0),
                        routing_profile: g.metadata().routing_profile().into(),
                        max_capacity: CapacityUnits::new(1),
                    },
                    RiderState {
                        coordinate: g.nodes()[0].coordinate(),
                        availability: Availability::Available,
                    },
                )
                .unwrap();
            let next = DurableImage {
                operational: state.checkpoint().unwrap(),
                clock_floor: now(),
                ..before.clone()
            };
            let events = if *kind == "two.transitions" {
                vec![
                    json!({"kind":"operational.transition","record":state.history().last()}),
                    json!({"kind":"operational.transition","record":state.history().last()}),
                ]
            } else {
                vec![json!({"kind":kind,"record":state.history().last()})]
            };
            before = store
                .commit(&before, &next, || Ok(next.clone()), &events)
                .unwrap();
        }
        Self {
            url,
            key,
            ns,
            root,
            image: before,
        }
    }
    fn publisher(&self) -> EventStore {
        EventStore::open(
            &self.url,
            &self.key,
            Role::Publisher,
            &self.root.join("publisher.head"),
        )
        .unwrap()
    }
    fn consumer(&self, name: &str, kind: ConsumerKind) -> EventStore {
        EventStore::open(
            &self.url,
            &self.key,
            Role::Consumer {
                name: name.into(),
                kind,
            },
            &self.root.join(format!("{name}.head")),
        )
        .unwrap()
    }
    fn all(&self) -> Vec<Delivery> {
        let mut p = self.publisher();
        let mut all = Vec::new();
        while let Some(d) = p.publish().unwrap() {
            p.acknowledge_publish(&d).unwrap();
            all.push(d);
        }
        all
    }
    fn db(&self) -> Client {
        Client::connect(&self.url, NoTls).unwrap()
    }
    fn count(&self, table: &str, consumer: &str) -> i64 {
        self.db()
            .query_one(
                &format!("SELECT count(*) FROM {table} WHERE namespace=$1 AND consumer=$2"),
                &[&self.ns, &consumer],
            )
            .unwrap()
            .get(0)
    }
}
struct OwnedDatabase<'a> {
    admin: &'a mut Client,
    name: String,
}
impl Drop for OwnedDatabase<'_> {
    fn drop(&mut self) {
        let _ = self
            .admin
            .batch_execute(&format!("DROP DATABASE {} WITH (FORCE)", self.name));
    }
}
#[test]
#[ignore = "requires disposable ROADRUNNER_TEST_DATABASE_URL; CI explicitly runs"]
fn publisher_consumer_lost_ack_restart_and_distinct_message_same_effect() {
    let f = Fixture::new(&["operational.transition"]);
    let mut p = f.publisher();
    let d = p.publish().unwrap().unwrap();
    drop(p);
    let mut p = f.publisher();
    assert_eq!(p.publish().unwrap().unwrap(), d);
    p.acknowledge_publish(&d).unwrap();
    assert!(p.publish().unwrap().is_none());
    assert!(matches!(
        EventStore::open(
            &f.url,
            &f.key,
            Role::Publisher,
            &f.root.join("other-publisher.head")
        ),
        Err(StorageError::Fenced)
    ));
    let mut c = f.consumer("projection", ConsumerKind::Projection);
    let original = c.receive(&d).unwrap();
    assert!(original.terminal());
    drop(c);
    let mut c = f.consumer("projection", ConsumerKind::Projection);
    assert_eq!(c.receive(&d).unwrap(), original);
    let mut alias = d.clone();
    alias.message_id = "different-transport-message".into();
    assert_eq!(c.receive(&alias).unwrap().state, "already_applied");
    assert_eq!(f.count("rr_consumer_effects", "projection"), 1);
    let mut conflict = d.clone();
    conflict.event["committed_at"] = json!(0);
    assert_eq!(c.receive(&conflict).unwrap().state, "identity_conflict");
    assert_eq!(c.receive(&d).unwrap(), original);
    c.acknowledge_delivery(&d).unwrap();
    assert!(c.next_delivery().unwrap().is_none());
    let replay = c.replay().unwrap();
    assert_eq!(replay["effect_ids"].as_array().unwrap().len(), 1);
    assert_eq!(replay["projection"].as_object().unwrap().len(), 3);
    let mut notification = f.consumer("notifications", ConsumerKind::Notification);
    assert_eq!(notification.receive(&d).unwrap().state, "applied");
    assert_eq!(
        notification.receive(&alias).unwrap().state,
        "already_applied"
    );
    assert_eq!(f.count("rr_notifications", "notifications"), 1);
    assert!(
        f.db()
            .execute(
                "UPDATE rr_notifications SET body='{}' WHERE namespace=$1",
                &[&f.ns]
            )
            .is_err()
    );
}
#[test]
#[ignore = "requires disposable ROADRUNNER_TEST_DATABASE_URL; CI explicitly runs"]
fn gap_deferral_conflicts_and_checked_source_order() {
    let f = Fixture::new(&["operational.transition", "operational.transition"]);
    let d = f.all();
    let mut c = f.consumer("projection", ConsumerKind::Projection);
    assert_eq!(c.receive(&d[1]).unwrap().state, "deferred");
    assert_eq!(f.count("rr_consumer_effects", "projection"), 0);
    assert!(c.acknowledge_delivery(&d[1]).is_err());
    assert_eq!(c.receive(&d[0]).unwrap().state, "applied");
    assert_eq!(c.receive(&d[1]).unwrap().state, "applied");
    assert_eq!(f.count("rr_consumer_effects", "projection"), 2);
    let mut bad = d[0].clone();
    bad.message_id = "superseded-conflicting-event".into();
    bad.event["payload"]["kind"] = json!("bogus");
    assert_eq!(c.receive(&bad).unwrap().state, "quarantined");
    assert!(c.acknowledge_delivery(&bad).is_err());
    assert_eq!(
        c.replay().unwrap()["effect_ids"].as_array().unwrap().len(),
        2
    );
}
#[test]
#[ignore = "requires disposable ROADRUNNER_TEST_DATABASE_URL; CI explicitly runs"]
fn poison_quarantine_and_authorized_fingerprint_bound_skip() {
    let f = Fixture::new(&["unknown.semantic/v9", "operational.transition"]);
    let d = f.all();
    let mut c = f.consumer("notifications", ConsumerKind::Notification);
    assert_eq!(c.receive(&d[0]).unwrap().state, "quarantined");
    assert!(c.acknowledge_delivery(&d[0]).is_err());
    assert_eq!(c.receive(&d[1]).unwrap().state, "deferred");
    assert!(
        c.skip(
            &d[0].message_id,
            "wrong",
            "operator",
            "unsupported semantics"
        )
        .is_err()
    );
    assert!(
        c.skip(
            &d[0].message_id,
            &d[0].fingerprint(),
            "",
            "unsupported semantics"
        )
        .is_err()
    );
    assert_eq!(
        c.skip(
            &d[0].message_id,
            &d[0].fingerprint(),
            "operator",
            "approved omission of unsupported event semantics"
        )
        .unwrap()
        .state,
        "skipped"
    );
    c.acknowledge_delivery(&d[0]).unwrap();
    assert_eq!(c.receive(&d[1]).unwrap().state, "applied");
    assert_eq!(c.replay().unwrap()["reconciliation_omissions"], 1);
    assert_eq!(f.count("rr_notifications", "notifications"), 1);
}
#[test]
#[ignore = "requires disposable ROADRUNNER_TEST_DATABASE_URL; CI explicitly runs"]
fn sql_effect_failure_rolls_back_inbox_and_notification_together() {
    let f = Fixture::new(&["operational.transition"]);
    let d = f.all();
    let mut c = f.consumer("notifications", ConsumerKind::Notification);
    let mut db = f.db();
    db.batch_execute("CREATE FUNCTION rr_test_effect_abort() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected effect fault'; END $$; CREATE TRIGGER rr_test_effect_abort AFTER INSERT ON rr_consumer_effects FOR EACH ROW EXECUTE FUNCTION rr_test_effect_abort()").unwrap();
    let result = c.receive(&d[0]);
    db.batch_execute("DROP TRIGGER rr_test_effect_abort ON rr_consumer_effects; DROP FUNCTION rr_test_effect_abort()").unwrap();
    assert!(result.is_err());
    assert_eq!(f.count("rr_consumer_effects", "notifications"), 0);
    assert_eq!(f.count("rr_notifications", "notifications"), 0);
    assert_eq!(f.count("rr_inbox", "notifications"), 0);
    assert_eq!(c.receive(&d[0]).unwrap().state, "applied");
}
#[test]
#[ignore = "requires disposable ROADRUNNER_TEST_DATABASE_URL; CI explicitly runs"]
fn post_commit_anchor_failure_and_rollback_watermark_fail_closed() {
    let f = Fixture::new(&["operational.transition"]);
    let d = f.all();
    let mut c = f.consumer("notifications", ConsumerKind::Notification);
    let backup = f.root.with_extension("backup");
    std::fs::rename(&f.root, &backup).unwrap();
    assert!(matches!(
        c.receive(&d[0]),
        Err(StorageError::OutcomeUnknown)
    ));
    assert_eq!(f.count("rr_notifications", "notifications"), 1);
    assert!(matches!(
        c.receive(&d[0]),
        Err(StorageError::OutcomeUnknown)
    ));
    std::fs::rename(&backup, &f.root).unwrap();
    drop(c);
    let mut c = f.consumer("notifications", ConsumerKind::Notification);
    assert_eq!(c.receive(&d[0]).unwrap().state, "applied");
    assert_eq!(f.count("rr_notifications", "notifications"), 1);
    drop(c);
    f.db().execute("UPDATE rr_event_heads SET revision=0 WHERE namespace=$1 AND actor='consumer/notifications'",&[&f.ns]).unwrap();
    assert!(matches!(
        EventStore::open(
            &f.url,
            &f.key,
            Role::Consumer {
                name: "notifications".into(),
                kind: ConsumerKind::Notification
            },
            &f.root.join("notifications.head")
        ),
        Err(StorageError::Binding)
    ));
}

#[test]
#[ignore = "requires disposable ROADRUNNER_TEST_DATABASE_URL with CREATE DATABASE; CI explicitly runs"]
fn additive_upgrade_preserves_original_authority_and_anchor() {
    let url = std::env::var("ROADRUNNER_TEST_DATABASE_URL").unwrap();
    assert!(
        !url.starts_with("postgres://") && !url.starts_with("postgresql://"),
        "this owned-database upgrade fixture requires PostgreSQL keyword connection syntax"
    );
    let name = format!(
        "rr22_upgrade_{}_{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::SeqCst)
    );
    let mut admin = Client::connect(&url, NoTls).unwrap();
    admin
        .batch_execute(&format!("CREATE DATABASE {name}"))
        .unwrap();
    let _cleanup = OwnedDatabase {
        admin: &mut admin,
        name: name.clone(),
    };
    let f = Fixture::new_with_url(format!("{url} dbname={name}"), &["operational.transition"]);
    // Remove only the unused additive tables to reproduce the exact schema-1 installation.
    f.db().batch_execute("DROP TABLE rr_event_receipts,rr_event_heads,rr_reconciliations,rr_event_attempts,rr_delivery_ack,rr_notifications,rr_projection,rr_consumer_effects,rr_inbox,rr_event_consumers,rr_publish_ack,rr_stream,rr_stream_heads,rr_event_schema CASCADE; DROP FUNCTION rr_inbox_guard()").unwrap();
    let original = f.image.clone();
    let watermark = std::fs::read(f.root.join("world.head")).unwrap();
    PgStore::migrate(&f.url).unwrap();
    let (store, restored) = PgStore::open(
        &f.url,
        &f.key,
        graph(),
        "test/v1",
        original.clone(),
        &f.root.join("world.head"),
        None,
    )
    .unwrap();
    assert_eq!(original, restored);
    assert_eq!(store.namespace(), f.ns);
    assert_eq!(std::fs::read(f.root.join("world.head")).unwrap(), watermark);
    drop(store);
    let all = f.all();
    assert_eq!(all.len(), 1);
    let mut consumer = f.consumer("projection", ConsumerKind::Projection);
    assert_eq!(consumer.receive(&all[0]).unwrap().state, "applied");
}

#[test]
#[ignore = "requires disposable ROADRUNNER_TEST_DATABASE_URL; CI explicitly runs"]
fn same_commit_fanout_applies_complete_fact_delta_once() {
    let f = Fixture::new(&["two.transitions"]);
    let d = f.all();
    assert_eq!(d.len(), 2);
    let mut c = f.consumer("projection", ConsumerKind::Projection);
    assert_eq!(c.receive(&d[1]).unwrap().state, "deferred");
    assert_eq!(c.receive(&d[0]).unwrap().state, "applied");
    assert_eq!(c.receive(&d[1]).unwrap().state, "already_applied");
    assert_eq!(f.count("rr_consumer_effects", "projection"), 1);
    assert_eq!(
        c.replay().unwrap()["effect_ids"].as_array().unwrap().len(),
        1
    );
    let mut n = f.consumer("notifications", ConsumerKind::Notification);
    for event in &d {
        assert_eq!(n.receive(event).unwrap().state, "applied");
    }
    assert_eq!(f.count("rr_notifications", "notifications"), 2);
}
