//! End-to-end scenario composition and CLI replay on the versioned synthetic fixture.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use roadrunner_core::graph::encode_graph_artifact;

#[path = "../../roadrunner-simulation/tests/support/mod.rs"]
mod support;
use support::ok;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures/phase-15/seeded-deliveries.json")
}

#[test]
fn simulation_cli_is_reproducible_in_json_and_readable_with_horizon_populations() {
    let fixture_path = fixture();
    let args = [
        "simulate",
        ok(fixture_path.to_str().ok_or("path")),
        "--json",
    ];
    let first = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args(args)
        .output());
    let second = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args(args)
        .output());
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
    let json: serde_json::Value = ok(serde_json::from_slice(&first.stdout));
    assert_eq!(json["summary"]["scheduled_orders"], 6);
    assert_eq!(json["summary"]["created_orders"], 5);
    assert_eq!(json["summary"]["delivered_orders"], 4);
    assert_eq!(json["summary"]["unassigned_orders"], 1);
    assert_eq!(json["summary"]["uncreated_orders"], 1);
    let readable = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args(["simulate", ok(fixture().to_str().ok_or("path"))])
        .output());
    assert!(readable.status.success());
    let text = ok(String::from_utf8(readable.stdout));
    assert!(text.contains("Delivered: 4"));
    assert!(text.contains("Median predicted ETA:"));
    assert!(text.contains("Rider utilization:"));
}

#[test]
fn malformed_scenario_is_an_error_without_success_json() {
    let directory = temporary_directory();
    let path = directory.join("invalid.json");
    ok(std::fs::write(
        &path,
        "{\"graph\":{},\"scenario\":{},\"unknown\":true}",
    ));
    let output = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args(["simulate", ok(path.to_str().ok_or("path")), "--json"])
        .output());
    assert!(!output.status.success());
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("error:"));
    ok(std::fs::remove_dir_all(directory));
}

#[test]
fn graph_artifact_path_resolves_relative_to_scenario_and_digest_is_required() {
    let directory = temporary_directory();
    let graph = support::graph();
    ok(std::fs::write(
        directory.join("roads.rr-graph"),
        ok(encode_graph_artifact(&graph)),
    ));
    let mut scenario = support::scenario();
    scenario.graph_snapshot_digest = Some(graph.metadata().snapshot_digest().to_owned());
    let path = directory.join("scenario.json");
    let write = |scenario: &roadrunner_simulation::SimulationScenario| {
        ok(std::fs::write(
            &path,
            ok(serde_json::to_vec(&serde_json::json!({
                "graph": {"kind": "artifact", "path": "roads.rr-graph"}, "scenario": scenario,
            }))),
        ));
    };
    write(&scenario);
    let output = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args(["simulate", ok(path.to_str().ok_or("path")), "--json"])
        .output());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = ok(serde_json::from_slice(&output.stdout));
    assert_eq!(
        result["graph_snapshot_digest"],
        graph.metadata().snapshot_digest()
    );
    support::close(
        ok(result["orders"][0]["delivered_at"]
            .as_f64()
            .ok_or("delivery")),
        55.0,
    );
    scenario.graph_snapshot_digest = None;
    write(&scenario);
    let output = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args(["simulate", ok(path.to_str().ok_or("path")), "--json"])
        .output());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("graph_snapshot_digest"));
    ok(std::fs::remove_dir_all(directory));
}

fn temporary_directory() -> std::path::PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "roadrunner-simulation-cli-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    ok(std::fs::create_dir_all(&directory));
    directory
}

#[test]
fn inline_coordinates_and_geometry_are_validated_at_the_json_boundary() {
    let directory = temporary_directory();
    let path = directory.join("invalid-coordinates.json");
    let valid: serde_json::Value = ok(serde_json::from_slice(&ok(std::fs::read(fixture()))));
    let mut bad_node = valid.clone();
    bad_node["graph"]["nodes"][0]["coordinate"]["latitude_e7"] = serde_json::json!(900_000_001);
    let mut bad_geometry = valid.clone();
    bad_geometry["graph"]["roads"][0]["geometry"] = serde_json::json!([
        {"latitude_e7": 0, "longitude_e7": 0},
        {"latitude_e7": 0, "longitude_e7": 1_800_000_001},
        {"latitude_e7": 0, "longitude_e7": 10_000},
    ]);
    let mut unknown = valid;
    unknown["graph"]["nodes"][0]["coordinate"]["typo"] = serde_json::json!(0);
    for input in [bad_node, bad_geometry, unknown] {
        ok(std::fs::write(&path, ok(serde_json::to_vec(&input))));
        let output = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
            .args(["simulate", ok(path.to_str().ok_or("path")), "--json"])
            .output());
        assert!(!output.status.success());
        assert_eq!(output.stdout, [] as [u8; 0]);
    }
    ok(std::fs::remove_dir_all(directory));
}

#[test]
fn comparison_cli_replays_four_policies_and_exposes_metrics_and_configuration() {
    let path = fixture();
    let args = [
        "benchmark",
        "dispatch",
        ok(path.to_str().ok_or("path")),
        "--idle-penalty-weight",
        "2",
        "--json",
    ];
    let first = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args(args)
        .output());
    let second = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args(args)
        .output());
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
    let result: serde_json::Value = ok(serde_json::from_slice(&first.stdout));
    assert_eq!(result["schema_version"], 1);
    let runs = ok(result["runs"].as_array().ok_or("runs"));
    assert_eq!(runs.len(), 4);
    assert_eq!(runs[0]["dispatch"]["kind"], "nearest_rider");
    assert_eq!(runs[1]["dispatch"]["kind"], "lowest_pickup_eta");
    assert_eq!(runs[2]["dispatch"]["kind"], "lowest_completion_time");
    assert_eq!(runs[3]["dispatch"]["idle_penalty_weight"], 2.0);
    for run in runs {
        assert_eq!(run["seed"], 15);
        assert_eq!(run["summary"]["created_orders"], 5);
        assert_eq!(run["summary"]["uncreated_orders"], 1);
        assert_eq!(
            run["orders"][0]["realized_ready_at"],
            runs[0]["orders"][0]["realized_ready_at"]
        );
        assert!(run["summary"]["delivered_duration"]["mean_seconds"].is_number());
    }
    let text = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args(["benchmark", "dispatch", ok(path.to_str().ok_or("path"))])
        .output());
    assert!(text.status.success());
    let text = ok(String::from_utf8(text.stdout));
    assert_eq!(text.matches("Strategy:").count(), 4);
    assert!(text.contains("Mean delivered duration:"));
    assert!(text.contains("Mean rider idle time:"));
    assert!(text.contains("Unassigned:"));
}

#[test]
fn invalid_comparison_arguments_do_not_emit_success_output() {
    let path = fixture();
    for rest in [
        vec!["--idle-penalty-weight", "-1"],
        vec!["--idle-penalty-weight", "NaN"],
        vec!["--idle-penalty-weight"],
        vec!["--unknown"],
    ] {
        let mut args = vec!["benchmark", "dispatch", ok(path.to_str().ok_or("path"))];
        args.extend(rest);
        let output = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
            .args(args)
            .output());
        assert!(!output.status.success());
        assert_eq!(output.stdout, [] as [u8; 0]);
    }
}

#[test]
fn phase16_fixture_preserves_disconnected_and_future_orders_across_policies() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures/phase-16/paired-strategies.json");
    let output = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args([
            "benchmark",
            "dispatch",
            ok(path.to_str().ok_or("path")),
            "--json",
        ])
        .output());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = ok(serde_json::from_slice(&output.stdout));
    let runs = ok(result["runs"].as_array().ok_or("runs"));
    for run in runs {
        assert_eq!(run["summary"]["scheduled_orders"], 14);
        assert_eq!(run["summary"]["delivered_orders"], 12);
        assert_eq!(run["summary"]["unassigned_orders"], 1);
        assert_eq!(run["summary"]["uncreated_orders"], 1);
        assert!(run["orders"][12]["rider"].is_null());
        assert!(run["orders"][13]["created_at"].is_null());
    }
    assert_eq!(runs[0]["orders"][0]["rider"], 1);
    assert_eq!(runs[1]["orders"][0]["rider"], 2);
}

#[test]
fn phase17_pooled_cli_replays_success_rejections_and_prediction_failure() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/fixtures/phase-17");
    for name in [
        "successful-pooling",
        "capacity-prefix",
        "complete-infeasibility",
        "hard-deadline",
        "cumulative-repeat",
        "budget-exhaustion",
        "frozen-wait",
        "frozen-service",
        "realized-readiness-violation",
        "baseline-predicted-breach",
    ] {
        let path = root.join(format!("{name}.json"));
        let args = ["simulate", ok(path.to_str().ok_or("path")), "--json"];
        let first = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
            .args(args)
            .output());
        let second = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
            .args(args)
            .output());
        assert!(
            first.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&first.stderr)
        );
        assert_eq!(first.stdout, second.stdout);
        let json: serde_json::Value = ok(serde_json::from_slice(&first.stdout));
        assert_eq!(json["schema_version"], 2);
        assert!(json["insertions"].is_array());
        if name == "successful-pooling" {
            assert_eq!(json["summary"]["delivered_orders"], 2);
            assert_eq!(json["insertions"][1]["committed"], true);
        }
        if name == "hard-deadline" || name == "complete-infeasibility" {
            assert_eq!(json["summary"]["delivered_orders"], 1);
            assert_eq!(json["insertions"][1]["committed"], false);
        }
        if name == "budget-exhaustion" {
            assert_eq!(
                json["insertions"][1]["decision"]["evidence"]["termination"],
                "SearchIncomplete"
            );
        }
    }
    let path = root.join("unavailable-forecast.json");
    let output = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args(["simulate", ok(path.to_str().ok_or("path")), "--json"])
        .output());
    assert!(output.status.success());
    let json: serde_json::Value = ok(serde_json::from_slice(&output.stdout));
    assert!(
        json["prediction_failures"]
            .as_array()
            .is_some_and(|f| !f.is_empty())
    );
    assert_eq!(json["prediction_failures"][0]["committed"], false);
    assert_eq!(
        json["prediction_failures"][0]["coverage"]["input_complete"],
        false
    );
    let path = root.join("successful-pooling.json");
    let output = ok(Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .args(["simulate", ok(path.to_str().ok_or("path"))])
        .output());
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("incremental road"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("search_complete=true"));
}
