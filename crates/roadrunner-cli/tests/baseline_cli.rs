//! End-to-end Phase 24 baseline evaluation, verification and immutable-output refusal.
use std::{path::Path, process::Command};
#[test]
fn baseline_evaluate_verify_and_refuse_overwrite_or_unknown_candidate_inputs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::env::temp_dir().join(format!("roadrunner-b0-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&output);
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_roadrunner"))
            .current_dir(&root)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("{e}"))
    };
    let destination = output.to_str().unwrap_or_else(|| panic!("path"));
    let evaluated = run(&[
        "baseline",
        "evaluate",
        "ml/data/phase23-representative-v1",
        destination,
    ]);
    assert!(
        evaluated.status.success(),
        "{}",
        String::from_utf8_lossy(&evaluated.stderr)
    );
    let summary: serde_json::Value =
        serde_json::from_slice(&evaluated.stdout).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        summary["report"]["descriptive_all_included"]["coverage"]["committed_starts"],
        5
    );
    assert_eq!(
        summary["report"]["descriptive_all_included"]["metrics"]["samples"],
        4
    );
    assert!(run(&["baseline", "verify", destination]).status.success());
    assert!(
        !run(&[
            "baseline",
            "evaluate",
            "ml/data/phase23-representative-v1",
            destination
        ])
        .status
        .success()
    );
    assert!(
        !run(&[
            "baseline",
            "evaluate",
            "ml/data/phase23-representative-v1",
            destination,
            "--candidate",
            "unmatched.json"
        ])
        .status
        .success()
    );
    std::fs::write(output.join("evaluation.json"), "{}").unwrap_or_else(|e| panic!("{e}"));
    assert!(!run(&["baseline", "verify", destination]).status.success());
    std::fs::remove_dir_all(output).unwrap_or_else(|e| panic!("{e}"));
}
