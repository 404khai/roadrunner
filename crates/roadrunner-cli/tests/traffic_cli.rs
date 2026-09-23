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
    let snapshot = fixture_snapshot(&root, "phase10-lagos-marina");
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

#[test]
fn pinned_time_profile_changes_route_with_departure_time() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let snapshot = fixture_snapshot(&root, "phase11-lagos-marina");
    let scenario = root.join("data/fixtures/phase-11/lagos-marina-profile.json");
    let mut routes = Vec::new();
    for departure in ["0", "600"] {
        let output = Command::new(env!("CARGO_BIN_EXE_roadrunner"))
            .args([
                "route",
                "schedule",
                snapshot
                    .to_str()
                    .unwrap_or_else(|| panic!("snapshot path is not UTF-8")),
                "5602610872",
                "5594385916",
                "--scenario",
                scenario
                    .to_str()
                    .unwrap_or_else(|| panic!("scenario path is not UTF-8")),
                "--depart",
                departure,
            ])
            .output()
            .unwrap_or_else(|error| panic!("run scheduled CLI: {error}"));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|error| panic!("parse scheduled output: {error}"));
        routes.push(result);
    }
    std::fs::remove_dir_all(&snapshot)
        .unwrap_or_else(|error| panic!("remove temporary snapshot: {error}"));
    assert_ne!(
        routes[0]["time_dependent_fastest"]["route"]["edges"],
        routes[1]["time_dependent_fastest"]["route"]["edges"]
    );
    assert_eq!(
        routes[1]["time_dependent_fastest"]["route"]["edges"],
        routes[1]["free_flow_fastest"]["route"]["edges"]
    );
    assert!(
        routes[0]["time_dependent_fastest"]["time_dependent_eta_seconds"]
            .as_f64()
            .unwrap_or_else(|| panic!("early ETA"))
            < routes[0]["free_flow_fastest"]["time_dependent_eta_seconds"]
                .as_f64()
                .unwrap_or_else(|| panic!("early baseline ETA"))
    );
}

fn fixture_snapshot(root: &std::path::Path, source_id: &str) -> std::path::PathBuf {
    let dataset = extract_pbf(
        root.join("data/fixtures/phase-7/lagos-marina.osm.pbf"),
        source_id,
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
        "roadrunner-traffic-cli-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    write_snapshot_bundle_atomic(&snapshot, &compiled)
        .unwrap_or_else(|error| panic!("write snapshot: {error}"));
    snapshot
}
