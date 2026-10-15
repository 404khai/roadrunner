//! Simple offline Phase 23 composition; source files are captured from this checkout.
use roadrunner_dataset::{
    Reproduction, RetainedBytes, SourceCapture, SplitRequest, SyntheticLineage, build_dataset,
    load_dataset, publish_dataset,
};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path, process::Command};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    dataset_version: String,
    sources: Vec<Source>,
    splits: SplitRequest,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    scenario: String,
    run: String,
    lineage: SyntheticLineage,
}
fn command(program: &str, args: &[&str]) -> Result<String, String> {
    let result = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).into_owned());
    }
    String::from_utf8(result.stdout).map_err(|e| e.to_string())
}
fn reproduction() -> Result<Reproduction, String> {
    let root = command("git", &["rev-parse", "--show-toplevel"])?;
    let names = command(
        "git",
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "crates",
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
        ],
    )?;
    let mut files = BTreeMap::new();
    for name in names.lines() {
        if Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("rs") || ext.eq_ignore_ascii_case("sql"))
            || name.ends_with("Cargo.toml")
            || name == "Cargo.lock"
            || name == "rust-toolchain.toml"
        {
            let path = Path::new(root.trim()).join(name);
            let bytes = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            files.insert(name.to_owned(), RetainedBytes::new(bytes));
        }
    }
    Ok(Reproduction {
        base_revision: command("git", &["rev-parse", "HEAD"])?.trim().into(),
        files,
        toolchain: format!(
            "{}; Numeric: Rust IEEE-754 binary64; serde_json float_roundtrip; no fast-math; Cargo.lock retained",
            roadrunner_dataset::build_toolchain()
        ),
        patch: RetainedBytes::new(command(
            "git",
            &[
                "diff",
                "HEAD",
                "--",
                "crates",
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain.toml",
            ],
        )?),
    })
}
pub(super) fn build(request_path: &str, output: &str) -> Result<(), String> {
    let request: Request =
        serde_json::from_slice(&std::fs::read(request_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let parent = Path::new(request_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let mut sources = Vec::new();
    for source in request.sources {
        let scenario_path = parent.join(&source.scenario);
        let (graph, scenario) = crate::simulation_command::load(
            scenario_path.to_str().ok_or("non UTF-8 scenario path")?,
        )?;
        sources.push(
            SourceCapture::simulate(&graph, &scenario, &source.run, source.lineage)
                .map_err(|e| e.to_string())?,
        );
    }
    let bundle = build_dataset(
        sources,
        &request.dataset_version,
        reproduction()?,
        request.splits,
    )
    .map_err(|e| e.to_string())?;
    let manifest = publish_dataset(&bundle, Path::new(output)).map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({"manifest": manifest, "quality":bundle.quality, "evaluation":bundle.evaluation})).map_err(|e| e.to_string())?);
    Ok(())
}
pub(super) fn verify(directory: &str) -> Result<(), String> {
    let bundle = load_dataset(Path::new(directory)).map_err(|e| e.to_string())?;
    println!(
        "verified {}: {} committed starts, {} exact synthetic labels, {} dependency groups; physical evaluation unavailable",
        bundle.dataset_version,
        bundle.quality.committed_starts,
        bundle.quality.exact_labels,
        bundle.quality.dependency_groups
    );
    Ok(())
}

pub(super) fn run(args: &[String]) -> Result<(), String> {
    match args {
        [operation, request, output] if operation == "build" => build(request, output),
        [operation, directory] if operation == "verify" => verify(directory),
        _ => Err("usage: roadrunner dataset build <request.json> <new-directory> | dataset verify <directory>".into()),
    }
}
