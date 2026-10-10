//! End-to-end CLI scenario on a pinned OSM extract.

use roadrunner_osm::load_snapshot_bundle;
use std::process::Command;

#[test]
fn pinned_traffic_scenario_selects_longer_faster_route() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let snapshot = fixture_snapshot(&root, 10);
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
            root.join("data/fixtures/phase-10/lagos-marina-severe.semantic-v2.json")
                .to_str()
                .unwrap_or_else(|| panic!("scenario path is not UTF-8")),
        ])
        .output()
        .unwrap_or_else(|error| panic!("run CLI: {error}"));
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
    let snapshot = fixture_snapshot(&root, 11);
    let scenario = root.join("data/fixtures/phase-11/lagos-marina-profile.semantic-v2.json");
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

// Load the exact published graph pinned by the overlay. Recompilation uses platform
// floating-point math and need not produce identical compiled distance bits.
fn fixture_snapshot(root: &std::path::Path, phase: u8) -> std::path::PathBuf {
    let snapshot = root.join(format!("data/fixtures/phase-{phase}/snapshot.semantic-v2"));
    let loaded = load_snapshot_bundle(&snapshot, true)
        .unwrap_or_else(|error| panic!("verify pinned snapshot: {error}"));
    let scenario_name = match phase {
        10 => "lagos-marina-severe.semantic-v2.json",
        11 => "lagos-marina-profile.semantic-v2.json",
        _ => panic!("unsupported fixture phase"),
    };
    let scenario: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join(format!("data/fixtures/phase-{phase}/{scenario_name}")))
            .unwrap_or_else(|error| panic!("read scenario: {error}")),
    )
    .unwrap_or_else(|error| panic!("decode scenario: {error}"));
    assert_eq!(
        scenario["graph_snapshot_digest"].as_str(),
        Some(loaded.graph.metadata().snapshot_digest()),
    );
    snapshot
}

#[test]
fn pinned_scenarios_reject_another_verified_snapshot() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let phase10 = fixture_snapshot(&root, 10);
    let phase11 = fixture_snapshot(&root, 11);
    for (mode, snapshot, scenario) in [
        (
            "traffic",
            &phase11,
            "phase-10/lagos-marina-severe.semantic-v2.json",
        ),
        (
            "schedule",
            &phase10,
            "phase-11/lagos-marina-profile.semantic-v2.json",
        ),
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_roadrunner"));
        command
            .args(["route", mode])
            .arg(snapshot)
            .args(["5602610872", "5594385916", "--scenario"])
            .arg(root.join("data/fixtures").join(scenario));
        if mode == "schedule" {
            command.args(["--depart", "0"]);
        }
        let output = command
            .output()
            .unwrap_or_else(|error| panic!("run mismatched scenario: {error}"));
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("different graph snapshot"),
            "{}",
            String::from_utf8_lossy(&output.stderr),
        );
        assert_eq!(output.stdout, [] as [u8; 0]);
    }
}
