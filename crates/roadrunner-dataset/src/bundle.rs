//! Five logical components and immutable offline publication.
use crate::{
    ClockWitness, DatasetError, EvaluationAvailability, LabelAssessment, LabelEvidence, Lifecycle,
    MeasurementSupport, Partition, ProducerOrigin, Provenance, RecordMeaning, RetainedBytes,
    SourceCapture, SplitPlan, SplitRequest, StartFeatures, assess_label, canonical, digest,
    extract_features, plan_splits,
};
use roadrunner_dispatch::{
    ActionId, AppliedExecutionEffectId, ExecutionEffect, OperationalWorldNamespace, RiderId,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Lossless scoped action key; u64 stays an integer in JSON (never JS-coerced).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionKey {
    /// Independent run scope above repeated fixture namespace.
    pub run: String,
    /// Operational authority namespace.
    pub namespace: OperationalWorldNamespace,
    /// Original authority-allocated logical action.
    pub action: ActionId,
}
/// Canonical census row plus feature/label/lineage components joined one-to-one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetRow {
    /// Canonical census identity; no fabricated counterfactual rows.
    pub key: ActionKey,
    /// Required source ownership consistency join.
    pub rider: RiderId,
    /// Original static predictor boundary and immutable B0.
    pub features: StartFeatures,
    /// Evidence-qualified lifecycle and exact-label eligibility.
    pub label: LabelAssessment,
    /// Separate provenance by value category (start, B0, features, label).
    pub provenance: BTreeMap<String, Provenance>,
    /// Source-capture content identity (recoverable in this bundle).
    pub source_content: String,
    /// Recognized arrival effect aliases; not additional labels.
    pub arrival_effects: Vec<AppliedExecutionEffectId>,
    /// Protected dependency component, not a predictor.
    pub dependency_group: String,
    /// Frozen exact assignment, including excluded rows.
    pub partition: Partition,
}
/// Reproducibility source package, not merely a git HEAD fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reproduction {
    /// Original base commit from git, qualified by retained working file bytes.
    pub base_revision: String,
    /// Relevant actual working source/configuration bytes, including Cargo.lock/toolchain.
    pub files: BTreeMap<String, RetainedBytes>,
    /// Actual rustc/cargo host/version/numeric semantics.
    pub toolchain: String,
    /// Exact retained diff from the base (new files are additionally in files).
    pub patch: RetainedBytes,
}
/// Census conservation and differential availability; no Phase 24 error metrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityReport {
    /// All committed starts, including incomplete/unlabeled/excluded actions.
    pub committed_starts: usize,
    /// Mutually exclusive outcomes sum exactly to committed starts.
    pub outcomes: BTreeMap<Lifecycle, usize>,
    /// Exact eligible endpoint labels (including valid zero).
    pub exact_labels: usize,
    /// Rows assigned to excluded sources remain in the census.
    pub excluded_rows: usize,
    /// Optional historical feature availability reasons (not predictor columns).
    pub unavailable_features: BTreeMap<String, usize>,
    /// Whole-source behavior-policy lineage coverage (total/exact), not causal evidence.
    pub policy_coverage: BTreeMap<String, (usize, usize)>,
    /// Original B0 predictions retained independently of labels.
    pub b0_predictions: usize,
    /// Connected protected source groups, not row count as effective sample size.
    pub dependency_groups: usize,
    /// Overlapping exact-label exclusion reasons.
    pub reasons: BTreeMap<String, usize>,
    /// Supported pre-start distance bands: <1km, 1–5km, >=5km; total/exact counts.
    pub distance_band_coverage: BTreeMap<String, (usize, usize)>,
    /// Source/policy selected-population coverage (metadata only).
    pub source_coverage: BTreeMap<String, (usize, usize)>,
    /// Current extraction only establishes the synthetic cohort.
    pub cohort: MeasurementSupport,
}
/// Frozen Phase 24 specification without executing any prediction-error evaluator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationPackage {
    /// Whole-leg interval and prediction reference.
    pub target: String,
    /// Exact B0, eligibility, finality and missingness contracts.
    pub rules: Vec<String>,
    /// Declared metrics for future separately authorized Phase 24.
    pub metrics: Vec<String>,
    /// Synthetic holdout availability under exact frozen assignments.
    pub synthetic: EvaluationAvailability,
    /// Physical measurement/generalization is not supported by simulation.
    pub physical: EvaluationAvailability,
}
/// Immutable dataset evidence bundle; five logical components, simple JSON layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetBundle {
    /// Versioned schema/canonicalization/precision contract.
    pub schema: String,
    /// Explicit immutable dataset version chosen before construction.
    pub dataset_version: String,
    /// All declared bounded original sources, no silent filtering.
    pub sources: Vec<SourceCapture>,
    /// Full actual extractor/source package.
    pub reproduction: Reproduction,
    /// Canonical source census joined with features, labels and value provenance.
    pub rows: Vec<DatasetRow>,
    /// Exact dependency closure and assignments.
    pub splits: SplitPlan,
    /// Reconciled quality report.
    pub quality: QualityReport,
    /// Frozen future evaluation contract; metrics are not computed here.
    pub evaluation: EvaluationPackage,
}
/// Packaging identity is separate from semantic dataset identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationManifest {
    /// Format and byte digest algorithm.
    pub schema: String,
    /// Semantic digest of canonical dataset component JSON.
    pub semantic_sha256: String,
    /// SHA-256 of pretty-printed bundle bytes as packaged.
    pub packaging_sha256: String,
    /// Recoverable package path, fixed to avoid traversal.
    pub bundle_file: String,
}

/// Constructs all logical components after source replay/authority reconciliation.
/// # Errors
/// Rejects source, graph, split, provenance or reproducibility contract failures.
pub fn build_dataset(
    mut sources: Vec<SourceCapture>,
    version: &str,
    reproduction: Reproduction,
    request: SplitRequest,
) -> Result<DatasetBundle, DatasetError> {
    if sources.is_empty() || version.is_empty() {
        return Err(DatasetError::Source(
            "empty dataset/source population".into(),
        ));
    }
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "crates/roadrunner-dataset/src/lib.rs",
    ] {
        if !reproduction.files.contains_key(name) {
            return Err(DatasetError::Source(format!(
                "missing reproduction bytes: {name}"
            )));
        }
    }
    if reproduction.base_revision.is_empty() || reproduction.toolchain.is_empty() {
        return Err(DatasetError::Source(
            "missing code/toolchain identity".into(),
        ));
    }
    for bytes in reproduction.files.values() {
        bytes.verify()?;
    }
    reproduction.patch.verify()?;
    sources.sort_by(|a, b| a.run.cmp(&b.run));
    let splits = plan_splits(&sources, request)?;
    let mut rows = Vec::new();
    for source in &sources {
        rows.extend(source_rows(source, &splits)?);
    }
    let quality = quality_report(&rows, &sources, &splits)?;
    let evaluation = evaluation_package(&splits);
    Ok(DatasetBundle {
        schema: "eta-dataset/v1; sorted-json-keys; binary64-roundtrip; lossless-u64".into(),
        dataset_version: version.into(),
        sources,
        reproduction,
        rows,
        splits,
        quality,
        evaluation,
    })
}

fn source_rows(
    source: &SourceCapture,
    splits: &SplitPlan,
) -> Result<Vec<DatasetRow>, DatasetError> {
    let mut rows = Vec::new();
    let (graph, trace) = source.verify()?;
    let content = source.content_id()?;
    for start in &trace.starts {
        let effects: Vec<_> = trace
            .effects
            .iter()
            .filter(|e| {
                e.action == start.original.action_id && e.effect == ExecutionEffect::Arrival
            })
            .collect();
        let endpoint = effects.first().map(|e| e.at);
        let label = assess_label(LabelEvidence {
            start_effective_at: start.original.route.departed_at,
            arrival_reported_at: endpoint,
            arrival_processing_sample_at: trace
                .arrivals
                .iter()
                .find(|a| a.action == start.original.action_id)
                .map(|a| a.processing_sample_at),
            boundary: trace.horizon,
            non_arrival_through: endpoint.is_none().then_some(trace.horizon),
            source_closed: true,
            disputed: false,
            clock: ClockWitness::SyntheticLogicalV1,
            measurement: MeasurementSupport::SyntheticControlled,
        });
        let features = extract_features(start, Some(&graph))?;
        let mut provenance = BTreeMap::new();
        for (name, meaning) in [
            ("start", RecordMeaning::CommittedDomainFact),
            ("b0", RecordMeaning::RoutingEstimate),
            ("features", RecordMeaning::DerivedValue),
            ("label", RecordMeaning::DerivedValue),
        ] {
            provenance.insert(
                name.into(),
                Provenance {
                    producer_origin: ProducerOrigin::Simulator,
                    record_meaning: meaning,
                    measurement_support: MeasurementSupport::SyntheticControlled,
                    derivation_lineage: vec![
                        format!("source:{content}"),
                        format!("action:{}", start.original.action_id.value()),
                        format!("start-revision:{}", start.committed_revision.number),
                        format!("{name}/v1"),
                    ],
                },
            );
        }
        rows.push(DatasetRow {
            key: ActionKey {
                run: source.run.clone(),
                namespace: trace.final_revision.namespace,
                action: start.original.action_id,
            },
            rider: start.rider,
            features,
            label,
            provenance,
            source_content: content.clone(),
            arrival_effects: effects.iter().map(|e| e.id).collect(),
            dependency_group: splits.groups[&source.run].clone(),
            partition: splits.request.assignments[&source.run],
        });
    }

    Ok(rows)
}
fn quality_report(
    rows: &[DatasetRow],
    sources: &[SourceCapture],
    splits: &SplitPlan,
) -> Result<QualityReport, DatasetError> {
    let mut quality = QualityReport {
        committed_starts: rows.len(),
        outcomes: [
            Lifecycle::Observed,
            Lifecycle::Pending,
            Lifecycle::RightCensored,
            Lifecycle::MissingArrival,
            Lifecycle::DisputedOrInvalid,
        ]
        .into_iter()
        .map(|s| (s, 0))
        .collect(),
        exact_labels: 0,
        excluded_rows: 0,
        unavailable_features: BTreeMap::new(),
        policy_coverage: BTreeMap::new(),
        b0_predictions: rows.len(),
        dependency_groups: splits.groups.values().collect::<BTreeSet<_>>().len(),
        reasons: BTreeMap::new(),
        distance_band_coverage: BTreeMap::new(),
        source_coverage: BTreeMap::new(),
        cohort: MeasurementSupport::SyntheticControlled,
    };
    for row in rows {
        quality.excluded_rows += usize::from(row.partition == Partition::Excluded);
        for field in [
            "road_class:UnsupportedSemantics",
            "day_of_week:NotApplicable",
        ] {
            *quality
                .unavailable_features
                .entry(field.into())
                .or_default() += 1;
        }
        let policy = sources
            .iter()
            .find(|s| s.run == row.key.run)
            .map(|s| s.lineage.policy.clone())
            .ok_or_else(|| DatasetError::Integrity("source lineage join".into()))?;
        let counts = quality.policy_coverage.entry(policy).or_default();
        counts.0 += 1;
        counts.1 += usize::from(row.label.duration_seconds.is_some());
        *quality.outcomes.entry(row.label.outcome).or_default() += 1;
        let eligible = usize::from(row.label.duration_seconds.is_some());
        quality.exact_labels += eligible;
        for reason in &row.label.reasons {
            *quality.reasons.entry(reason.clone()).or_default() += 1;
        }
        let distance = row.features.distance_meters.value();
        let band = if distance < 1000.0 {
            "under_1km"
        } else if distance < 5000.0 {
            "1_to_5km"
        } else {
            "at_least_5km"
        };
        let counts = quality
            .distance_band_coverage
            .entry(band.into())
            .or_default();
        counts.0 += 1;
        counts.1 += eligible;
        let counts = quality
            .source_coverage
            .entry(row.key.run.clone())
            .or_default();
        counts.0 += 1;
        counts.1 += eligible;
    }
    Ok(quality)
}
fn evaluation_package(splits: &SplitPlan) -> EvaluationPackage {
    EvaluationPackage {
        target: "started whole-road-leg: synthetic accepted arrival minus logical start; predictor reference=start_effective_at".into(),
        rules: vec!["B0=original frozen route travel seconds; never updated".into(), "immutable-as-of/v1; no corrections overwrite prior versions".into(),
            "equal weight per eligible action; only exact observed labels; missing/censored/disputed excluded from point-error metrics, retained in coverage".into(),
            "fitting only on training, mature labels; no IDs/scenario/seed/policy or outcome predictors; no rolling aggregates".into(),
            "source boundaries inclusive; synthetic logical seconds have no civil epoch; binary64 roundtrip, IDs lossless u64".into(),
            "signed residual=label-prediction; zero valid; MAPE deferred; quantiles nearest-rank p50/p95/p99; median midpoint; uncertainty unavailable without defensible independent groups".into()],
        metrics: vec!["MAE seconds (primary)", "RMSE seconds", "mean/median signed residual seconds", "absolute-error and positive-residual nearest-rank p50/p95/p99", "underprediction frequency=count(residual>0)/eligible exact actions; severity=mean positive residual, unavailable if none", "exact prediction/label coverage by cohort/group/distance band/source/policy"].into_iter().map(str::to_owned).collect(),
        synthetic: splits.evaluation.clone(), physical: EvaluationAvailability::EvaluationUnavailable("NoPhysicalDepartureOrArrivalMeasurements".into()),
    }
}

/// Rebuilds the dataset from retained sources and compares every derived component.
/// # Errors
/// Rejects corrupted source, altered features/labels/quality/splits or missing reproduction bytes.
pub fn verify_dataset(bundle: &DatasetBundle) -> Result<(), DatasetError> {
    let rebuilt = build_dataset(
        bundle.sources.clone(),
        &bundle.dataset_version,
        bundle.reproduction.clone(),
        bundle.splits.request.clone(),
    )?;
    if canonical(bundle)? != canonical(&rebuilt)? {
        return Err(DatasetError::Integrity(
            "derived dataset differs from retained evidence".into(),
        ));
    }
    Ok(())
}
/// Writes a new immutable directory via staging+rename; existing versions are never overwritten.
/// # Errors
/// Rejects existing destination, validation failures and filesystem errors.
pub fn publish_dataset(
    bundle: &DatasetBundle,
    destination: &Path,
) -> Result<PublicationManifest, DatasetError> {
    verify_dataset(bundle)?;
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    if destination.exists() {
        return Err(DatasetError::Integrity(
            "immutable dataset destination already exists".into(),
        ));
    }
    let stage = parent.join(format!(
        ".eta-stage-{}-{}",
        std::process::id(),
        digest(&canonical(bundle)?)
    ));
    std::fs::create_dir(&stage)?;
    let result = (|| {
        let bytes = serde_json::to_vec_pretty(bundle)?;
        let manifest = PublicationManifest {
            schema: "eta-publication/v1".into(),
            semantic_sha256: digest(&canonical(bundle)?),
            packaging_sha256: digest(&bytes),
            bundle_file: "bundle.json".into(),
        };
        write_sync(&stage.join("bundle.json"), &bytes)?;
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
        let _ = std::fs::remove_dir_all(&stage);
    }
    result
}
fn write_sync(path: &Path, bytes: &[u8]) -> Result<(), DatasetError> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
/// Loads and verifies both packaging and semantic identities, then independently rebuilds.
/// # Errors
/// Rejects malformed manifests, corrupt bytes, source replay or derived content mismatch.
pub fn load_dataset(path: &Path) -> Result<DatasetBundle, DatasetError> {
    let manifest: PublicationManifest =
        serde_json::from_slice(&std::fs::read(path.join("manifest.json"))?)?;
    if manifest.schema != "eta-publication/v1" || manifest.bundle_file != "bundle.json" {
        return Err(DatasetError::Integrity("publication schema/path".into()));
    }
    let bytes = std::fs::read(path.join("bundle.json"))?;
    if digest(&bytes) != manifest.packaging_sha256 {
        return Err(DatasetError::Integrity("packaging digest".into()));
    }
    let bundle: DatasetBundle = serde_json::from_slice(&bytes)?;
    if digest(&canonical(&bundle)?) != manifest.semantic_sha256 {
        return Err(DatasetError::Integrity("semantic digest".into()));
    }
    verify_dataset(&bundle)?;
    Ok(bundle)
}
