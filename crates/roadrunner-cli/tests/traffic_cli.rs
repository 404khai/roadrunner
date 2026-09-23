//! End-to-end CLI scenario on a pinned OSM extract.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use roadrunner_osm::{
    compile_motorcycle_graph, decode_dataset_artifact, encode_dataset_artifact, extract_pbf,
    write_snapshot_bundle_atomic,
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn pinned_traffic_scenario_selects_longer_faster_route() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dataset = extract_pbf(
        root.join("data/fixtures/phase-7/lagos-marina.osm.pbf"),
        "phase10-lagos-marina",
    )
    .unwrap_or_else(|error| panic!("extract fixture: {error}"));
    let decoded = decode_dataset_artifact(
        &encode_dataset_artifact(&dataset)
            .unwrap_or_else(|error| panic!("encode fixture: {error}")),
    )
    .unwrap_or_else(|error| panic!("decode fixture: {error}"));
    let compiled = compile_motorcycle_graph(&decoded)
        .unwrap_or_else(|error| panic!("compile fixture: {error}"));
    let snapshot = std::env::temp_dir().join(format!(
        "roadrunner-phase10-cli-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    write_snapshot_bundle_atomic(&snapshot, &compiled)
        .unwrap_or_else(|error| panic!("write snapshot: {error}"));
    let output = Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args([
            "route",
            "traffic",
            snapshot
                .to_str()
                .unwrap_or_else(|| panic!("snapshot path is not UTF-8")),
            "5602610872",
            "5594385916",
            "--scenario",
            root.join("data/fixtures/phase-10/lagos-marina-severe.json")
                .to_str()
                .unwrap_or_else(|| panic!("scenario path is not UTF-8")),
        ])
        .output()
        .unwrap_or_else(|error| panic!("run CLI: {error}"));
    std::fs::remove_dir_all(&snapshot)
        .unwrap_or_else(|error| panic!("remove temporary snapshot: {error}"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("parse CLI output: {error}"));
    let shortest = result["shortest_distance"]["route"]["total_distance"]
        .as_f64()
        .unwrap_or_else(|| panic!("shortest distance is absent"));
    let fastest = result["traffic_fastest"]["route"]["total_distance"]
        .as_f64()
        .unwrap_or_else(|| panic!("traffic distance is absent"));
    let short_eta = result["shortest_distance"]["traffic_adjusted_eta_seconds"]
        .as_f64()
        .unwrap_or_else(|| panic!("shortest traffic ETA is absent"));
    let fast_eta = result["traffic_fastest"]["traffic_adjusted_eta_seconds"]
        .as_f64()
        .unwrap_or_else(|| panic!("fastest traffic ETA is absent"));
    assert!(fastest > shortest);
    assert!(fast_eta < short_eta);
    assert_ne!(
        result["shortest_distance"]["route"]["edges"],
        result["traffic_fastest"]["route"]["edges"]
    );
}
