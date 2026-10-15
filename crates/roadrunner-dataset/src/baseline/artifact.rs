//! Immutable self-contained result publication with independent semantic/packaging identities.
use super::{BaselineReport, evaluate_b0};
use crate::{DatasetBundle, DatasetError, Reproduction, canonical, digest};
use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};

/// Recoverable input and evaluator evidence plus the exact report, not just result numbers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaselineArtifact {
    /// Independently versioned supported result envelope.
    pub schema: String,
    /// Complete original semantic dataset, preserving all raw source bytes and frozen splits.
    pub dataset: DatasetBundle,
    /// Actual evaluator source/configuration/toolchain/patch bytes at evaluation.
    pub evaluator: Reproduction,
    /// Deterministic action-paired B0 results and availability boundaries.
    pub report: BaselineReport,
}
/// Immutable result artifact's integrity and semantic content identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineManifest {
    /// Supported manifest contract.
    pub schema: String,
    /// Fixed artifact basename; arbitrary paths are refused.
    pub artifact_file: String,
    /// SHA-256 over canonical complete result/input/evaluator content.
    pub semantic_sha256: String,
    /// SHA-256 over actual pretty JSON file bytes.
    pub packaging_sha256: String,
}
fn validate_code(code: &Reproduction) -> Result<(), DatasetError> {
    if code.base_revision.is_empty() || code.toolchain.is_empty() {
        return Err(DatasetError::Source(
            "missing evaluator code/toolchain identity".into(),
        ));
    }
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "crates/roadrunner-dataset/src/baseline/metrics.rs",
        "crates/roadrunner-dataset/src/baseline/report.rs",
        "crates/roadrunner-dataset/src/baseline/artifact.rs",
    ] {
        if !code.files.contains_key(path) {
            return Err(DatasetError::Source(format!(
                "missing evaluator source bytes: {path}"
            )));
        }
    }
    for bytes in code.files.values() {
        bytes.verify()?;
    }
    code.patch.verify()
}
/// Constructs a fully verified immutable evaluation artifact with actual evaluator provenance.
/// # Errors
/// Rejects missing/corrupt evaluator bytes and unsupported or inconsistent source datasets.
pub fn build_baseline_artifact(
    dataset: DatasetBundle,
    evaluator: Reproduction,
) -> Result<BaselineArtifact, DatasetError> {
    validate_code(&evaluator)?;
    let report = evaluate_b0(&dataset)?;
    Ok(BaselineArtifact {
        schema: "eta-b0-artifact/v1".into(),
        dataset,
        evaluator,
        report,
    })
}
/// Rechecks original source, frozen populations and every metric/report field.
/// # Errors
/// Altered input, report, evaluator byte identity or unsupported schema fails closed.
pub fn verify_baseline_artifact(artifact: &BaselineArtifact) -> Result<(), DatasetError> {
    validate_code(&artifact.evaluator)?;
    if artifact.schema != "eta-b0-artifact/v1"
        || canonical(&artifact.report)? != canonical(&evaluate_b0(&artifact.dataset)?)?
    {
        return Err(DatasetError::Integrity(
            "B0 report differs from verified source or supported contract".into(),
        ));
    }
    Ok(())
}
/// Publishes a new fsynced result directory; never overwrites an existing version.
/// # Errors
/// Rejects invalid artifacts, existing destination and filesystem failures.
pub fn publish_baseline_artifact(
    artifact: &BaselineArtifact,
    destination: &Path,
) -> Result<BaselineManifest, DatasetError> {
    verify_baseline_artifact(artifact)?;
    if destination.exists() {
        return Err(DatasetError::Integrity(
            "immutable baseline destination already exists".into(),
        ));
    }
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let semantic = digest(&canonical(artifact)?);
    let stage = parent.join(format!(".b0-stage-{}-{semantic}", std::process::id()));
    std::fs::create_dir(&stage)?;
    let result = (|| {
        let bytes = serde_json::to_vec_pretty(artifact)?;
        let manifest = BaselineManifest {
            schema: "eta-b0-publication/v1".into(),
            artifact_file: "evaluation.json".into(),
            semantic_sha256: semantic,
            packaging_sha256: digest(&bytes),
        };
        write_sync(&stage.join("evaluation.json"), &bytes)?;
        write_sync(
            &stage.join("manifest.json"),
            &serde_json::to_vec_pretty(&manifest)?,
        )?;
        std::fs::File::open(&stage)?.sync_all()?;
        std::fs::rename(&stage, destination)?;
        std::fs::File::open(parent)?.sync_all()?;
        Ok(manifest)
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(stage);
    }
    result
}
fn write_sync(path: &Path, bytes: &[u8]) -> Result<(), DatasetError> {
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
/// Loads and verifies both byte packaging and semantic results, including source replay.
/// # Errors
/// Rejects corrupt bytes, changed paths/versions, dataset evidence or metrics.
pub fn load_baseline_artifact(directory: &Path) -> Result<BaselineArtifact, DatasetError> {
    let manifest: BaselineManifest =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    if manifest.schema != "eta-b0-publication/v1" || manifest.artifact_file != "evaluation.json" {
        return Err(DatasetError::Integrity(
            "unsupported baseline manifest schema/path".into(),
        ));
    }
    let bytes = std::fs::read(directory.join("evaluation.json"))?;
    if digest(&bytes) != manifest.packaging_sha256 {
        return Err(DatasetError::Integrity("baseline packaging digest".into()));
    }
    let artifact: BaselineArtifact = serde_json::from_slice(&bytes)?;
    if digest(&canonical(&artifact)?) != manifest.semantic_sha256 {
        return Err(DatasetError::Integrity("baseline semantic digest".into()));
    }
    verify_baseline_artifact(&artifact)?;
    Ok(artifact)
}
