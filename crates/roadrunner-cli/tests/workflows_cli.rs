//! Phase 36 adapters exercised through the actual executable and pinned graph.
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_roadrunner"))
        .current_dir(root())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("run CLI: {e}"))
}
fn json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, Vec::<u8>::new());
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| panic!("JSON: {e}"))
}
const GRAPH: &str = "data/fixtures/phase-10/snapshot.semantic-v2";

#[test]
fn verified_stats_and_algorithm_oracle() {
    let stats = json(&run(&["graph", "stats", GRAPH, "--json"]));
    assert_eq!(stats["nodes"], 58);
    assert_eq!(stats["deep_verified"], true);
    assert_eq!(stats["provenance_available"], true);
    let compare = json(&run(&[
        "route",
        "compare",
        "--graph",
        GRAPH,
        "--from",
        "osm:5602610872",
        "--to",
        "osm:5594385916",
        "--json",
    ]));
    assert_eq!(compare["cost_delta_seconds"], 0.0);
    assert_eq!(
        compare["graph_snapshot_digest"],
        stats["graph_snapshot_digest"]
    );
    let ordinary = json(&run(&[
        "route",
        "--graph",
        GRAPH,
        "--from",
        "osm:5602610872",
        "--to",
        "osm:5594385916",
        "--algorithm",
        "astar",
        "--json",
    ]));
    assert_eq!(ordinary["route"], compare["astar"]);
    let alternatives = json(&run(&[
        "route",
        "--graph",
        GRAPH,
        "--from",
        "osm:5602610872",
        "--to",
        "osm:5594385916",
        "--alternatives",
        "3",
        "--json",
    ]));
    assert!(
        alternatives["alternatives"]["routes"]
            .as_array()
            .is_some_and(|a| !a.is_empty())
    );
    assert!(!alternatives["alternatives"]["termination"].is_null());
}

#[test]
fn coordinate_resolution_is_bounded_and_preserves_zero_length_route() {
    let bundle = roadrunner_osm::load_snapshot_bundle(root().join(GRAPH), true)
        .unwrap_or_else(|e| panic!("fixture: {e}"));
    let node = bundle.graph.nodes()[0];
    let coordinate = node.coordinate();
    let endpoint = format!("{},{}", coordinate.latitude(), coordinate.longitude());
    let result = json(&run(&[
        "route",
        "--graph",
        GRAPH,
        "--from",
        &endpoint,
        "--to",
        &endpoint,
        "--snap-radius-meters",
        "0",
        "--json",
    ]));
    assert_eq!(result["source"]["snap_distance_meters"], 0.0);
    assert_eq!(result["source"]["resolution"], "nearest_node");
    assert_eq!(result["route"]["total_cost"]["value"], 0.0);
    assert_eq!(result["route"]["expanded_states"], 1);
    let fail = run(&[
        "route", "--graph", GRAPH, "--from", "0,0", "--to", &endpoint, "--json",
    ]);
    assert!(!fail.status.success());
    assert_eq!(fail.stdout, Vec::<u8>::new());
    let error: Value =
        serde_json::from_slice(&fail.stderr).unwrap_or_else(|e| panic!("error JSON: {e}"));
    assert!(
        error["error"]
            .as_str()
            .is_some_and(|s| s.contains("snap radius"))
    );
}

#[test]
fn strict_options_and_machine_errors() {
    for extra in [
        vec!["--alternatives", "0"],
        vec!["--snap-radius-meters", "NaN"],
        vec!["--algorithm", "magic"],
        vec!["--graph", GRAPH],
        vec!["--wat"],
        vec!["--json"],
    ] {
        let mut args = vec![
            "route", "--graph", GRAPH, "--from", "node:0", "--to", "node:0", "--json",
        ];
        args.extend(extra);
        let output = run(&args);
        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
        let error: Value =
            serde_json::from_slice(&output.stderr).unwrap_or_else(|e| panic!("error JSON: {e}"));
        assert!(error["error"].is_string());
    }
    let one = json(&run(&[
        "route",
        "--graph",
        GRAPH,
        "--from",
        "node:0",
        "--to",
        "node:0",
        "--alternatives",
        "1",
        "--json",
    ]));
    assert_eq!(
        one["alternatives"]["routes"].as_array().map(Vec::len),
        Some(1)
    );
    let fail = run(&[
        "route",
        "--graph",
        GRAPH,
        "--from",
        "node:0",
        "--to",
        "node:0",
        "--alternatives",
        "1",
        "--algorithm",
        "astar",
        "--json",
    ]);
    assert!(!fail.status.success());
    let artifact = format!("{GRAPH}/graph.rr-graph");
    let stats = json(&run(&["graph", "stats", &artifact, "--json"]));
    assert_eq!(stats["provenance_available"], false);
    let fail = run(&[
        "route",
        "--graph",
        &artifact,
        "--from",
        "osm:5602610872",
        "--to",
        "node:0",
        "--json",
    ]);
    assert!(!fail.status.success());
}

#[test]
fn import_publishes_verified_bundle_and_never_overwrites() {
    let destination =
        std::env::temp_dir().join(format!("roadrunner-cli-phase36-{}", std::process::id()));
    assert!(!destination.exists());
    let dest = destination.to_str().unwrap_or_else(|| panic!("temp path"));
    let args = [
        "graph",
        "import",
        "data/fixtures/phase-7/lagos-marina.osm.pbf",
        dest,
        "--source-id",
        "cli-phase36-test",
        "--json",
    ];
    let imported = json(&run(&args));
    let stats = json(&run(&["graph", "stats", dest, "--json"]));
    assert_eq!(
        imported["manifest"]["graph_snapshot_digest"],
        stats["graph_snapshot_digest"]
    );
    let manifest = std::fs::read(destination.join("manifest.json"))
        .unwrap_or_else(|e| panic!("manifest: {e}"));
    let second = run(&args);
    assert!(!second.status.success());
    assert_eq!(
        std::fs::read(destination.join("manifest.json"))
            .unwrap_or_else(|e| panic!("manifest: {e}")),
        manifest
    );
    std::fs::remove_dir_all(destination).unwrap_or_else(|e| panic!("cleanup: {e}"));
}

#[test]
fn human_help_has_block_branding_and_human_routes_stay_readable() {
    let help = run(&["--help"]);
    assert!(help.status.success());
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(text.contains("████"));
    assert!(text.contains("graph import"));
    let human = run(&[
        "route", "--graph", GRAPH, "--from", "node:0", "--to", "node:0",
    ]);
    assert!(human.status.success());
    let text = String::from_utf8_lossy(&human.stdout);
    assert!(text.contains("0.000s travel"));
    assert!(!text.contains("████")); // Piped command output remains plain and compact.
}

#[cfg(unix)]
#[test]
fn serve_preserves_api_arguments_environment_and_exit_status() {
    use std::os::unix::fs::PermissionsExt;
    let directory =
        std::env::temp_dir().join(format!("roadrunner-cli-serve-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap_or_else(|e| panic!("temp directory: {e}"));
    let cli = directory.join("roadrunner");
    // Reuse executable bytes instead of duplicating the build for this launcher test.
    std::fs::hard_link(env!("CARGO_BIN_EXE_roadrunner"), &cli)
        .unwrap_or_else(|e| panic!("link: {e}"));
    let missing = Command::new(&cli)
        .arg("serve")
        .output()
        .unwrap_or_else(|e| panic!("run: {e}"));
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("API executable missing"));
    let api = directory.join("roadrunner-api");
    std::fs::write(
        &api,
        "#!/bin/sh\nprintf '%s\\n' \"$ROADRUNNER_LAUNCH_TEST\" \"$@\"\nexit 7\n",
    )
    .unwrap_or_else(|e| panic!("stub: {e}"));
    std::fs::set_permissions(&api, std::fs::Permissions::from_mode(0o700))
        .unwrap_or_else(|e| panic!("permissions: {e}"));
    let output = Command::new(&cli)
        .args(["serve", "--trusted-local"])
        .env("ROADRUNNER_LAUNCH_TEST", "inherited")
        .output()
        .unwrap_or_else(|e| panic!("serve: {e}"));
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(output.stdout, b"inherited\n--trusted-local\n");
    assert_eq!(output.stderr, Vec::<u8>::new());
    std::fs::remove_dir_all(directory).unwrap_or_else(|e| panic!("cleanup: {e}"));
}
