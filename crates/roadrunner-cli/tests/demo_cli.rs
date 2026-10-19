//! Shared fleet presets exercised through the real executable.
use serde_json::Value;
use std::{path::Path, process::Command};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("CLI execution: {e}"))
}

#[test]
fn real_fleet_scenarios_complete_and_replay_identically() {
    for (riders, orders, working) in [("12", 16, 8), ("24", 28, 14)] {
        let args = ["demo", "--riders", riders, "--json"];
        let first = run(&args);
        assert!(first.status.success(), "{:?}", first.stderr);
        assert_eq!(first.stderr, Vec::<u8>::new());
        let result: Value =
            serde_json::from_slice(&first.stdout).unwrap_or_else(|e| panic!("machine JSON: {e}"));
        assert_eq!(
            result["configured_riders"],
            riders
                .parse::<usize>()
                .unwrap_or_else(|e| panic!("riders: {e}"))
        );
        assert_eq!(result["riders_receiving_work"], working);
        assert_eq!(result["simulation"]["summary"]["scheduled_orders"], orders);
        assert_eq!(result["simulation"]["summary"]["delivered_orders"], orders);
        assert_eq!(result["simulation"]["summary"]["outstanding_orders"], 0);
        assert!(
            result["alternatives"]["routes"]
                .as_array()
                .unwrap_or_else(|| panic!("routes missing"))
                .len()
                > 1
        );
        let replay = run(&args);
        assert!(replay.status.success());
        assert_eq!(first.stdout, replay.stdout);
    }
}

#[test]
fn refuses_unsupported_population_and_graph() {
    for args in [
        vec!["demo", "--riders", "100", "--json"],
        vec!["demo", "--graph", "not-a-graph", "--json"],
        vec!["demo", "--riders", "--json"],
    ] {
        let output = run(&args);
        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
        let _: Value = serde_json::from_slice(&output.stderr)
            .unwrap_or_else(|e| panic!("machine error JSON: {e}"));
    }
}
