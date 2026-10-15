//! End-to-end Phase 23 export and immutable publication through the actual CLI.
use std::{path::Path, process::Command};

#[test]
fn dataset_build_verify_and_overwrite_refusal() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::env::temp_dir().join(format!("roadrunner-eta-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&output);
    let run = |arguments: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_roadrunner"))
            .current_dir(&root)
            .args(arguments)
            .output()
            .unwrap_or_else(|e| panic!("CLI process: {e}"))
    };
    let destination = output.to_str().unwrap_or_else(|| panic!("UTF-8 temp path"));
    let built = run(&[
        "dataset",
        "build",
        "data/fixtures/phase-23/request.json",
        destination,
    ]);
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let summary: serde_json::Value =
        serde_json::from_slice(&built.stdout).unwrap_or_else(|e| panic!("JSON: {e}"));
    assert_eq!(summary["quality"]["committed_starts"], 5);
    assert_eq!(summary["quality"]["exact_labels"], 4);
    assert_eq!(summary["quality"]["dependency_groups"], 1);
    assert!(run(&["dataset", "verify", destination]).status.success());
    assert!(
        !run(&[
            "dataset",
            "build",
            "data/fixtures/phase-23/request.json",
            destination
        ])
        .status
        .success()
    );
    std::fs::write(output.join("bundle.json"), b"{}")
        .unwrap_or_else(|e| panic!("corrupt fixture: {e}"));
    assert!(!run(&["dataset", "verify", destination]).status.success());
    std::fs::remove_dir_all(output).unwrap_or_else(|e| panic!("cleanup: {e}"));
}
