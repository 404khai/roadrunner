//! Verified whole-route interchange and rejection before offline fitting.
use std::{path::Path, process::Command};

#[test]
fn export_restricts_features_and_rejects_corrupt_evidence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_roadrunner"))
            .current_dir(&root)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("{e}"))
    };
    let result = run(&["learned", "export", "ml/data/phase23-representative-v1"]);
    assert!(result.status.success());
    let export: serde_json::Value =
        serde_json::from_slice(&result.stdout).unwrap_or_else(|e| panic!("{e}"));
    let inputs = export["inputs"]
        .as_array()
        .unwrap_or_else(|| panic!("inputs"));
    assert_eq!(inputs.len(), 5);
    for input in inputs {
        let object = input.as_object().unwrap_or_else(|| panic!("request"));
        assert_eq!(object.len(), 6);
        assert_eq!(input["values"][0], input["route"]["travel"]);
        assert_eq!(input["values"][1], input["route"]["distance"]);
        assert_eq!(input["reference_seconds"], input["route"]["departed_at"]);
        for forbidden in ["scenario", "seed", "arrival", "label", "policy", "delay"] {
            assert!(!object.contains_key(forbidden));
        }
    }
    let temporary = std::env::temp_dir().join(format!("eta-export-{}", std::process::id()));
    std::fs::create_dir(&temporary).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(temporary.join("bundle.json"), "{}").unwrap_or_else(|e| panic!("{e}"));
    let path = temporary.to_str().unwrap_or_else(|| panic!("path"));
    assert!(!run(&["learned", "export", path]).status.success());
    assert!(!run(&["learned", "export"]).status.success());
    std::fs::remove_dir_all(temporary).unwrap_or_else(|e| panic!("{e}"));
}
