//! Requires a disposable PostgreSQL/PostGIS database. CI runs these explicitly.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use postgres::{Client, NoTls};
use roadrunner_core::graph::decode_graph_artifact;
use roadrunner_dispatch::{
    Availability, CapacityUnits, OperationalState, OperationalWorldNamespace, RiderId,
    RiderProfile, RiderState, World, WorldData,
};
use roadrunner_storage::{DurableImage, PgStore, StorageError};
use serde_json::json;
use std::{
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
fn url() -> String {
    std::env::var("ROADRUNNER_TEST_DATABASE_URL")
        .expect("set disposable ROADRUNNER_TEST_DATABASE_URL")
}
fn key() -> String {
    format!(
        "storage-test-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::SeqCst)
    )
}
fn anchor(key: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("roadrunner-phase21-{key}.head"))
}
fn graph() -> roadrunner_core::graph::FrozenGraph {
    decode_graph_artifact(include_bytes!(
        "../../../data/fixtures/phase-10/snapshot.semantic-v2/graph.rr-graph"
    ))
    .unwrap()
}
fn image() -> DurableImage {
    let state = OperationalState::new(
        OperationalWorldNamespace::fresh().unwrap(),
        World::new(1, WorldData::default()).unwrap(),
    )
    .unwrap();
    DurableImage {
        operational: state.checkpoint().unwrap(),
        adapter: json!({"schema_version":1,"commands":[],"observations":[]}),
        time_domain: "test-unix/v1".into(),
        clock_floor: now(),
    }
}
fn add_rider(before: &DurableImage) -> DurableImage {
    let mut state = OperationalState::restore_checkpoint(&before.operational).unwrap();
    let g = graph();
    let profile = RiderProfile {
        id: RiderId::new(0),
        routing_profile: g.metadata().routing_profile().into(),
        max_capacity: CapacityUnits::new(1),
    };
    state
        .register_rider(
            state.revision(),
            profile,
            RiderState {
                coordinate: g.nodes()[0].coordinate(),
                availability: Availability::Available,
            },
        )
        .unwrap();
    let mut next = before.clone();
    next.operational = state.checkpoint().unwrap();
    next.clock_floor = now();
    next
}
#[test]
#[ignore = "requires disposable ROADRUNNER_TEST_DATABASE_URL; CI explicitly runs this suite"]
fn atomic_rollback_history_spatial_facts_and_restart_restore() {
    let url = url();
    PgStore::migrate(&url).unwrap();
    let key = key();
    let bootstrap = image();
    let (mut store, before) = PgStore::open(
        &url,
        &key,
        graph(),
        "test-config/v1",
        bootstrap,
        &anchor(&key),
        None,
    )
    .unwrap();
    let staged = add_rider(&before);
    let original = store.head();
    let aborted = store.commit(
        &before,
        &staged,
        || {
            Err(StorageError::Staging(
                "injected after relational writes, before COMMIT".into(),
            ))
        },
        &[json!({"kind":"rider.registered"})],
    );
    assert!(aborted.is_err());
    assert_eq!(store.head(), original);
    let mut db = Client::connect(&url, NoTls).unwrap();
    let ns = store.namespace().to_owned();
    assert_eq!(
        db.query_one("SELECT count(*) FROM rr_riders WHERE namespace=$1", &[&ns])
            .unwrap()
            .get::<_, i64>(0),
        0
    );
    assert_eq!(
        db.query_one("SELECT count(*) FROM rr_outbox WHERE namespace=$1", &[&ns])
            .unwrap()
            .get::<_, i64>(0),
        0
    );
    let committed = store
        .commit(
            &before,
            &staged,
            || Ok(staged.clone()),
            &[json!({"kind":"rider.registered"})],
        )
        .unwrap();
    let row=db.query_one("SELECT ST_SRID(location::geometry),ST_X(location::geometry),ST_Y(location::geometry) FROM rr_riders WHERE namespace=$1",&[&ns]).unwrap();
    assert_eq!(row.get::<_, i32>(0), 4326);
    assert!((row.get::<_, f64>(1) - 3.380_130_5).abs() < 1e-10);
    let head = store.head();
    drop(store);
    let (reopened, restored) = PgStore::open(
        &url,
        &key,
        graph(),
        "test-config/v1",
        image(),
        &anchor(&key),
        Some(&head),
    )
    .unwrap();
    assert_eq!(restored, committed);
    assert_eq!(reopened.namespace(), ns);
    assert_eq!(reopened.head(), head);
    assert_eq!(
        db.query_one("SELECT count(*) FROM rr_history WHERE namespace=$1", &[&ns])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    assert_eq!(
        db.query_one("SELECT count(*) FROM rr_outbox WHERE namespace=$1", &[&ns])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    assert!(
        db.execute("UPDATE rr_outbox SET body='{}' WHERE namespace=$1", &[&ns])
            .is_err()
    );
}
#[test]
#[ignore = "requires disposable ROADRUNNER_TEST_DATABASE_URL; CI explicitly runs this suite"]
fn lease_exclusion_obsolete_writer_and_offline_migration_gate() {
    let url = url();
    PgStore::migrate(&url).unwrap();
    let key = key();
    let (mut first, before) = PgStore::open(
        &url,
        &key,
        graph(),
        "test-config/v1",
        image(),
        &anchor(&key),
        None,
    )
    .unwrap();
    assert!(matches!(
        PgStore::open(
            &url,
            &key,
            graph(),
            "test-config/v1",
            image(),
            &anchor(&format!("{key}-other-writer")),
            None
        ),
        Err(StorageError::Fenced)
    ));
    assert!(matches!(PgStore::migrate(&url), Err(StorageError::Fenced)));
    let mut db = Client::connect(&url, NoTls).unwrap();
    let name = format!("roadrunner/{key}");
    let pid: i32 = db
        .query_one(
            "SELECT pid FROM pg_stat_activity WHERE application_name=$1",
            &[&name],
        )
        .unwrap()
        .get(0);
    db.query_one("SELECT pg_terminate_backend($1)", &[&pid])
        .unwrap();
    let staged = add_rider(&before);
    assert!(
        first
            .commit(&before, &staged, || Ok(staged.clone()), &[])
            .is_err()
    );
    let (replacement, restored) = PgStore::open(
        &url,
        &key,
        graph(),
        "test-config/v1",
        image(),
        &anchor(&key),
        None,
    )
    .unwrap();
    assert_eq!(restored, before);
    assert!(first.check().is_err());
    drop(replacement);
    drop(first);
    PgStore::migrate(&url).unwrap();
}
#[test]
#[ignore = "requires disposable ROADRUNNER_TEST_DATABASE_URL; CI explicitly runs this suite"]
fn changed_policy_watermark_and_corrupt_relations_fail_closed() {
    let url = url();
    PgStore::migrate(&url).unwrap();
    let key = key();
    let (mut store, before) = PgStore::open(
        &url,
        &key,
        graph(),
        "test-config/v1",
        image(),
        &anchor(&key),
        None,
    )
    .unwrap();
    let obsolete = store.head();
    let staged = add_rider(&before);
    store
        .commit(&before, &staged, || Ok(staged.clone()), &[])
        .unwrap();
    let ns = store.namespace().to_owned();
    drop(store);
    assert!(matches!(
        PgStore::open(
            &url,
            &key,
            graph(),
            "wrong-config",
            image(),
            &anchor(&key),
            None
        ),
        Err(StorageError::Binding)
    ));
    assert!(matches!(
        PgStore::open(
            &url,
            &key,
            graph(),
            "test-config/v1",
            image(),
            &anchor(&key),
            Some(&roadrunner_storage::DurableHead {
                revision: obsolete.revision,
                digest: "wrong-ack-digest".into()
            })
        ),
        Err(StorageError::Binding)
    ));
    let mut db = Client::connect(&url, NoTls).unwrap();
    db.execute("UPDATE rr_riders SET state=jsonb_set(state,'{availability}','\"Unavailable\"') WHERE namespace=$1",&[&ns]).unwrap();
    assert!(matches!(
        PgStore::open(
            &url,
            &key,
            graph(),
            "test-config/v1",
            image(),
            &anchor(&key),
            None
        ),
        Err(StorageError::Corrupt)
    ));
}
#[test]
fn checkpoint_rejects_allocator_reuse_and_preserves_large_lossless_id_domain() {
    let before = image();
    let good = add_rider(&before);
    let restored = OperationalState::restore_checkpoint(&good.operational).unwrap();
    assert_eq!(restored.checkpoint().unwrap(), good.operational);
    let mut bad = good.operational;
    bad["state"]["next_rider"] = json!(1);
    assert!(OperationalState::restore_checkpoint(&bad).is_err());
    let mut high = before.operational;
    high["state"]["next_order"] = json!(9_007_199_254_740_995_u64);
    let restored = OperationalState::restore_checkpoint(&high).unwrap();
    assert_eq!(
        restored.checkpoint().unwrap()["state"]["next_order"],
        json!(9_007_199_254_740_995_u64)
    );
}

#[test]
#[ignore = "requires disposable ROADRUNNER_TEST_DATABASE_URL; CI explicitly runs this suite"]
fn independent_acknowledgement_anchor_rejects_rollback_and_missing_watermark() {
    let url = url();
    PgStore::migrate(&url).unwrap();
    let key = key();
    let path = anchor(&key);
    let (mut store, before) =
        PgStore::open(&url, &key, graph(), "test-config/v1", image(), &path, None).unwrap();
    let original = store.head();
    let ns = store.namespace().to_owned();
    let mut db = Client::connect(&url, NoTls).unwrap();
    let saved = db
        .query_one(
            "SELECT metadata,adapter,clock_floor FROM rr_worlds WHERE namespace=$1",
            &[&ns],
        )
        .unwrap();
    let staged = add_rider(&before);
    store
        .commit(&before, &staged, || Ok(staged.clone()), &[])
        .unwrap();
    drop(store);
    let anchor_bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(matches!(
        PgStore::open(&url, &key, graph(), "test-config/v1", image(), &path, None),
        Err(StorageError::Binding)
    ));
    std::fs::write(&path, anchor_bytes).unwrap();
    // Administrative rollback of the database head cannot roll back the independent acknowledgement.
    let metadata: serde_json::Value = saved.get(0);
    let adapter: serde_json::Value = saved.get(1);
    let floor: f64 = saved.get(2);
    db.execute("UPDATE rr_worlds SET storage_revision=0,operational_revision=0,content_sha256=$2,metadata=$3,adapter=$4,clock_floor=$5,certified_until=NULL WHERE namespace=$1",&[&ns,&original.digest,&metadata,&adapter,&floor]).unwrap();
    assert!(matches!(
        PgStore::open(&url, &key, graph(), "test-config/v1", image(), &path, None),
        Err(StorageError::Binding)
    ));
}

#[test]
#[ignore = "requires disposable ROADRUNNER_TEST_DATABASE_URL; CI explicitly runs this suite"]
fn acknowledgement_failure_after_real_commit_recovers_without_repeating_effects() {
    let url = url();
    PgStore::migrate(&url).unwrap();
    let key = key();
    let directory = std::env::temp_dir().join(format!("roadrunner-ack-fault-{key}"));
    let saved = directory.with_extension("saved");
    let path = directory.join("world.head");
    let (mut store, before) =
        PgStore::open(&url, &key, graph(), "test-config/v1", image(), &path, None).unwrap();
    let ns = store.namespace().to_owned();
    let staged = add_rider(&before);
    let error = store
        .commit(
            &before,
            &staged,
            || {
                // The subsequent PostgreSQL COMMIT succeeds, but independent acknowledgement fsync cannot.
                std::fs::rename(&directory, &saved).unwrap();
                std::fs::write(&directory, b"not-a-directory").unwrap();
                Ok(staged.clone())
            },
            &[json!({"kind":"rider.registered"})],
        )
        .unwrap_err();
    assert!(matches!(error, StorageError::OutcomeUnknown));
    assert!(matches!(store.check(), Err(StorageError::OutcomeUnknown)));
    let mut db = Client::connect(&url, NoTls).unwrap();
    assert_eq!(
        db.query_one("SELECT count(*) FROM rr_riders WHERE namespace=$1", &[&ns])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    std::fs::remove_file(&directory).unwrap();
    std::fs::rename(&saved, &directory).unwrap();
    drop(store);
    // The acknowledged parent watermark is older than the confirmed DB head; verified receipts reconcile it.
    let (reopened, restored) =
        PgStore::open(&url, &key, graph(), "test-config/v1", image(), &path, None).unwrap();
    assert_eq!(restored, staged);
    assert_eq!(reopened.head().revision, 1);
    assert_eq!(
        db.query_one("SELECT count(*) FROM rr_outbox WHERE namespace=$1", &[&ns])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    drop(reopened);
    std::fs::remove_dir_all(directory).unwrap();
}
