//! Actual frozen dataset and offline result integrity; no regenerated dataset goldens.
use roadrunner_dataset::{baseline::*, *};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
fn ok<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|e| panic!("{e:?}"))
}
fn dataset() -> DatasetBundle {
    ok(load_dataset(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ml/data/phase23-representative-v1"),
    ))
}
fn code() -> Reproduction {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = BTreeMap::new();
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "crates/roadrunner-dataset/src/lib.rs",
        "crates/roadrunner-dataset/src/baseline/metrics.rs",
        "crates/roadrunner-dataset/src/baseline/report.rs",
        "crates/roadrunner-dataset/src/baseline/artifact.rs",
    ] {
        files.insert(
            name.into(),
            RetainedBytes::new(ok(std::fs::read_to_string(root.join(name)))),
        );
    }
    Reproduction {
        base_revision: "controlled-test/v1; actual file bytes".into(),
        files,
        toolchain: build_toolchain(),
        patch: RetainedBytes::new(String::new()),
    }
}
#[test]
fn immutable_roundtrip_and_corruption_detection() {
    let artifact = ok(build_baseline_artifact(dataset(), code()));
    let destination = std::env::temp_dir().join(format!("b0-artifact-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&destination);
    ok(publish_baseline_artifact(&artifact, &destination));
    assert_eq!(ok(load_baseline_artifact(&destination)), artifact);
    assert!(publish_baseline_artifact(&artifact, &destination).is_err());
    ok(std::fs::write(destination.join("evaluation.json"), "{}"));
    assert!(load_baseline_artifact(&destination).is_err());
    ok(std::fs::remove_dir_all(destination));
}
#[test]
fn metrics_coverage_segments_and_pairing_tampering_fail_verification() {
    let artifact = ok(build_baseline_artifact(dataset(), code()));
    let mut changed = artifact.clone();
    changed
        .report
        .descriptive_all_included
        .metrics
        .as_mut()
        .unwrap_or_else(|| panic!("metrics"))
        .mae_seconds = 0.0;
    assert!(verify_baseline_artifact(&changed).is_err());
    let mut changed = artifact.clone();
    changed
        .report
        .descriptive_all_included
        .coverage
        .committed_starts = 0;
    assert!(verify_baseline_artifact(&changed).is_err());
    let mut changed = artifact.clone();
    changed.report.actions.remove(0);
    assert!(verify_baseline_artifact(&changed).is_err());
    let mut changed = artifact.clone();
    changed.report.segments.clear();
    assert!(verify_baseline_artifact(&changed).is_err());
    let mut changed = artifact;
    changed.report.protocol.version = "changed-after-evaluation".into();
    assert!(verify_baseline_artifact(&changed).is_err());
}
#[test]
fn source_corruption_and_missing_evaluator_bytes_block_publication() {
    let mut source = dataset();
    source.sources[0].result.utf8.push(' ');
    assert!(build_baseline_artifact(source, code()).is_err());
    let mut provenance = code();
    provenance.files.clear();
    assert!(build_baseline_artifact(dataset(), provenance).is_err());
    let mut artifact = ok(build_baseline_artifact(dataset(), code()));
    artifact.evaluator.patch.utf8 = "unrecorded patch".into();
    assert!(verify_baseline_artifact(&artifact).is_err());
}
#[test]
fn same_frozen_source_contract_and_code_yield_identical_semantic_results() {
    let first = ok(build_baseline_artifact(dataset(), code()));
    let second = ok(build_baseline_artifact(dataset(), code()));
    assert_eq!(ok(canonical(&first)), ok(canonical(&second)));
}

#[allow(dead_code)]
#[path = "../../roadrunner-simulation/tests/support/mod.rs"]
mod support;

#[test]
fn verified_zero_start_source_retains_declared_group_and_unavailable_metrics() {
    let mut scenario = support::scenario();
    scenario.orders.clear();
    let source = ok(SourceCapture::simulate(
        &support::graph(),
        &scenario,
        "empty-source",
        SyntheticLineage {
            generator: "controlled-test/v1".into(),
            family: "empty-family".into(),
            base_world: "empty-world".into(),
            realization: "fixed".into(),
            policy: "preparation-aware".into(),
            dependencies: BTreeSet::new(),
        },
    ));
    let split = SplitRequest {
        claim: "descriptive-synthetic/v1".into(),
        assignments: [("empty-source".into(), Partition::Descriptive)]
            .into_iter()
            .collect(),
        excluded_reasons: BTreeMap::new(),
    };
    let source_dataset = ok(build_dataset(vec![source], "empty/v1", code(), split));
    let report = ok(evaluate_b0(&source_dataset));
    assert_eq!(report.descriptive_all_included.coverage.committed_starts, 0);
    assert_eq!(report.descriptive_all_included.metrics, None);
    assert_eq!(
        report
            .descriptive_all_included
            .coverage
            .prediction_coverage
            .ratio,
        None
    );
    assert_eq!(report.declared_source_groups, 1);
    assert_eq!(
        report.segments["source"]["empty-source"]
            .coverage
            .committed_starts,
        0
    );
}
