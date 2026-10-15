//! Verified action pairing and frozen descriptive segmentation.
use super::metrics::{ErrorMetrics, count, metrics};
use crate::{
    ActionKey, DatasetBundle, DatasetError, EvaluationAvailability, Lifecycle, MeasurementSupport,
    Partition, SourceCapture, canonical, digest, verify_dataset,
};
use roadrunner_core::geo::Seconds;
use roadrunner_dispatch::{DispatchInstant, OperationalRevision, RiderId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Explicit fraction with an unavailable ratio for an empty denominator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageFraction {
    /// Supported observations/predictions in the denominator population.
    pub numerator: usize,
    /// Exact population for this fraction.
    pub denominator: usize,
    /// None for zero denominator, never a fabricated zero coverage.
    pub ratio: Option<f64>,
}
fn fraction(numerator: usize, denominator: usize) -> Result<CoverageFraction, DatasetError> {
    Ok(CoverageFraction {
        numerator,
        denominator,
        ratio: if denominator == 0 {
            None
        } else {
            Some(count(numerator)? / count(denominator)?)
        },
    })
}
/// Reconciled coverage over census rows, separately from exact point-error metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaselineCoverage {
    /// All committed starts in this slice, including excluded/unlabeled work.
    pub committed_starts: usize,
    /// Starts explicitly assigned Excluded in the frozen split plan.
    pub excluded_starts: usize,
    /// Five mutually exclusive census outcomes, including zero counts.
    pub outcomes: BTreeMap<Lifecycle, usize>,
    /// Overlapping eligibility/prediction exclusion reasons.
    pub reasons: BTreeMap<String, usize>,
    /// All exact labels divided by committed starts (before split exclusions).
    pub exact_label_coverage: CoverageFraction,
    /// Retained original predictions divided by committed starts, not just arrivals.
    pub prediction_coverage: CoverageFraction,
    /// Eligible exact labels divided by included starts.
    pub eligible_label_coverage: CoverageFraction,
    /// Paired predictions divided by eligible exact labels.
    pub prediction_on_eligible_coverage: CoverageFraction,
    /// Dependency groups represented by census rows (not independent sample size).
    pub census_groups: usize,
    /// Dependency groups with at least one eligible exact prediction-label pair.
    pub paired_groups: usize,
}
/// One descriptive slice; an empty exact population is unavailable, not zero error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaselineSlice {
    /// Complete selected-population/label/prediction denominator evidence.
    pub coverage: BaselineCoverage,
    /// Paired-population point-error metrics, if there are exact pairs.
    pub metrics: Option<ErrorMetrics>,
    /// Explicit reason when paired metrics cannot be computed.
    pub metrics_unavailable_reason: Option<String>,
    /// Full eligible-population availability; never impute a missing B0 or silent fallback.
    pub full_eligible_population: EvaluationAvailability,
}
/// Exact pairing lineage and diagnostic segment membership for one canonical start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaselineAction {
    /// Run/namespace/ActionId; generations/effects never multiply observations.
    pub key: ActionKey,
    /// Verified committed owner.
    pub rider: RiderId,
    /// Immutable retained source capture reference.
    pub source_content: String,
    /// Dependency group from the frozen split plan.
    pub dependency_group: String,
    /// Exact frozen assignment; excluded rows remain present.
    pub partition: Partition,
    /// Qualified measurement cohort; never pooled with unsupported physical data.
    pub measurement: MeasurementSupport,
    /// Original historical start availability witness.
    pub start_revision: OperationalRevision,
    /// Declared prediction reference, not physical departure.
    pub start_effective_at: DispatchInstant,
    /// Immutable original directed route content digest.
    pub original_route_sha256: String,
    /// Original B0; Option permits explicit missing-prediction kernel diagnostics.
    pub b0_seconds: Option<Seconds>,
    /// Admissible exact observed label only; censor bounds are never substituted.
    pub observed_seconds: Option<Seconds>,
    /// Primary lifecycle state, separate from split/measurement eligibility.
    pub outcome: Lifecycle,
    /// Eligible exact label before prediction coverage is considered.
    pub label_eligible: bool,
    /// Label minus original B0, only for eligible exact pairs.
    pub residual_seconds: Option<f64>,
    /// Overlapping reasons, independent of the lifecycle partition.
    pub reasons: Vec<String>,
    /// Frozen segment assignments; intervention is retrospective diagnostic metadata only.
    pub segments: BTreeMap<String, String>,
}
/// Metric/eligibility/segmentation semantics frozen independently of result values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineProtocol {
    /// Versioned evaluator and arithmetic contract.
    pub version: String,
    /// Explicit residual/metric/missingness/segmentation rules.
    pub rules: Vec<String>,
}
/// Reproducible original-B0 evaluation; no training or inference artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaselineReport {
    /// Independent report schema.
    pub schema: String,
    /// Frozen Phase 24 definitions, chosen before running results.
    pub protocol: BaselineProtocol,
    /// Exact canonical verified dataset identity (including retained evidence).
    pub dataset_semantic_sha256: String,
    /// Preserved dataset version, not a replacement source version.
    pub dataset_version: String,
    /// Original Phase 23 frozen evaluation-package content identity.
    pub frozen_package_sha256: String,
    /// Actual original split/group assignments.
    pub split_plan_sha256: String,
    /// Permitted scientific interpretation.
    pub claim: String,
    /// All included eligible actions; descriptive, never an independent holdout claim.
    pub descriptive_all_included: BaselineSlice,
    /// Predeclared slices, including separate source/cohort/group/partition reports.
    pub segments: BTreeMap<String, BTreeMap<String, BaselineSlice>>,
    /// Lossless canonical per-action pairing evidence, including incomplete/excluded starts.
    pub actions: Vec<BaselineAction>,
    /// Source groups, including sources with no started rows.
    pub declared_source_groups: usize,
    /// Availability inherited from the frozen split plan.
    pub independent_holdout: EvaluationAvailability,
    /// No physical labels/generalization in the supported source path.
    pub physical_generalization: EvaluationAvailability,
    /// Sampling uncertainty is not inferred from replay rows or a group-count threshold.
    pub metric_uncertainty: EvaluationAvailability,
    /// B0 is a point estimate, not an interval or certified physical bound.
    pub prediction_uncertainty: EvaluationAvailability,
    /// Predictive metrics do not establish dispatch policy effects.
    pub causal_policy_effect: EvaluationAvailability,
    /// Unsupported optional segment attributes are explicit, not current-state backfill.
    pub unavailable_segments: BTreeMap<String, String>,
}
fn protocol() -> BaselineProtocol {
    BaselineProtocol { version: "b0-evaluation/v1".into(), rules: [
        "target=whole started road leg; prediction reference=start_effective_at; B0=original frozen route elapsed seconds",
        "residual=eligible label minus B0; equal weight per eligible logical action; positive means underprediction",
        "MAE primary; RMSE; signed mean; signed midpoint median; absolute and strictly-positive residual nearest-rank p50/p95/p99",
        "underprediction frequency=count(residual>0)/paired actions; severity=mean strictly positive residual; absent positive tail/severity unavailable",
        "Observed+admissible exact SyntheticControlled label+not Excluded; zero valid; pending/missing/censored/disputed never imputed",
        "missing original B0 counted; no replacement or fallback authorized; full eligible-population performance unavailable if prediction coverage incomplete",
        "distance meters: [0,1000), [1000,5000), [5000,infinity); B0 seconds: [0,60), [60,300), [300,infinity)",
        "cohort/profile/graph/traffic/family/generator/policy/source/dependency_group/partition; retrospective RoadDelay presence diagnostic only",
        "all-included aggregate descriptive; exact partitions preserved; no fitting, row randomization, candidate comparisons or MAPE",
        "canonical action order; finite binary64 scaled arithmetic; empty populations unavailable; no confidence intervals without a justified sampling design",
    ].into_iter().map(str::to_owned).collect() }
}
fn tag<T: Serialize>(value: &T) -> Result<String, DatasetError> {
    String::from_utf8(canonical(value)?).map_err(|e| DatasetError::Source(e.to_string()))
}
fn action(
    row: &crate::DatasetRow,
    source: &SourceCapture,
    delayed: bool,
) -> Result<BaselineAction, DatasetError> {
    let mut segments = BTreeMap::new();
    let distance = row.features.distance_meters.value();
    let estimate = row.features.b0_seconds.value();
    for (dimension, value) in [
        (
            "distance_band",
            if distance < 1000.0 {
                "under_1km"
            } else if distance < 5000.0 {
                "1_to_5km"
            } else {
                "at_least_5km"
            }
            .into(),
        ),
        (
            "estimate_band",
            if estimate < 60.0 {
                "under_60s"
            } else if estimate < 300.0 {
                "60_to_300s"
            } else {
                "at_least_300s"
            }
            .into(),
        ),
        ("cohort", tag(&row.label.evidence.measurement)?),
        (
            "profile",
            row.features.original_route.routing.profile.clone(),
        ),
        (
            "graph",
            row.features.original_route.routing.graph_digest.clone(),
        ),
        (
            "traffic",
            tag(&row.features.original_route.routing.traffic)?,
        ),
        ("family", source.lineage.family.clone()),
        ("generator", source.lineage.generator.clone()),
        ("policy", source.lineage.policy.clone()),
        ("source", source.run.clone()),
        ("dependency_group", row.dependency_group.clone()),
        ("partition", tag(&row.partition)?),
        (
            "intervention_diagnostic",
            if delayed {
                "RoadDelayObserved"
            } else {
                "NoRoadDelayObserved"
            }
            .into(),
        ),
    ] {
        segments.insert(dimension.into(), value);
    }
    let eligible = row.partition != Partition::Excluded
        && row.label.outcome == Lifecycle::Observed
        && row.label.evidence.measurement == MeasurementSupport::SyntheticControlled
        && row.label.duration_seconds.is_some();
    let mut reasons = row.label.reasons.clone();
    if row.partition == Partition::Excluded {
        reasons.push("FrozenSplitExcluded".into());
    }
    let residual = if eligible {
        row.label.duration_seconds.map(|v| v.value() - estimate)
    } else {
        None
    };
    Ok(BaselineAction {
        key: row.key.clone(),
        rider: row.rider,
        source_content: row.source_content.clone(),
        dependency_group: row.dependency_group.clone(),
        partition: row.partition,
        measurement: row.label.evidence.measurement,
        start_revision: row.features.witness.start_revision,
        start_effective_at: row.label.evidence.start_effective_at,
        original_route_sha256: digest(&canonical(&row.features.original_route)?),
        b0_seconds: Some(row.features.b0_seconds),
        observed_seconds: row.label.duration_seconds,
        outcome: row.label.outcome,
        label_eligible: eligible,
        residual_seconds: residual,
        reasons,
        segments,
    })
}
/// Evaluates only a complete, source-verified bundle with immutable B0 and frozen splits.
/// # Errors
/// Rejects corrupt/unsupported datasets and numeric/identity/label contract failures.
pub fn evaluate_b0(dataset: &DatasetBundle) -> Result<BaselineReport, DatasetError> {
    verify_dataset(dataset)?;
    let mut actions = Vec::new();
    for source in &dataset.sources {
        let raw: serde_json::Value = serde_json::from_str(&source.result.utf8)?;
        let trace: roadrunner_simulation::SyntheticTrace =
            serde_json::from_value(raw["trace"].clone())?;
        for row in dataset.rows.iter().filter(|r| r.key.run == source.run) {
            actions.push(action(
                row,
                source,
                trace
                    .schedules
                    .iter()
                    .any(|s| s.execution.action_id == row.key.action),
            )?);
        }
    }
    // Dataset verification seals exact census order; no subset join or independent prediction sorting.
    let mut segments: BTreeMap<String, BTreeMap<String, Vec<&BaselineAction>>> = BTreeMap::new();
    for item in &actions {
        for (dimension, value) in &item.segments {
            segments
                .entry(dimension.clone())
                .or_default()
                .entry(value.clone())
                .or_default()
                .push(item);
        }
    }
    for (dimension, values) in [
        (
            "distance_band",
            vec!["under_1km", "1_to_5km", "at_least_5km"],
        ),
        (
            "estimate_band",
            vec!["under_60s", "60_to_300s", "at_least_300s"],
        ),
        (
            "partition",
            vec![
                "\"Descriptive\"",
                "\"Train\"",
                "\"Validation\"",
                "\"Test\"",
                "\"Excluded\"",
            ],
        ),
    ] {
        for value in values {
            segments
                .entry(dimension.into())
                .or_default()
                .entry(value.into())
                .or_default();
        }
    }
    for source in &dataset.sources {
        for (dimension, value) in [
            ("source", &source.run),
            ("family", &source.lineage.family),
            ("generator", &source.lineage.generator),
            ("policy", &source.lineage.policy),
            ("dependency_group", &dataset.splits.groups[&source.run]),
        ] {
            segments
                .entry(dimension.into())
                .or_default()
                .entry(value.clone())
                .or_default();
        }
    }
    let reports = segments
        .into_iter()
        .map(|(dimension, values)| {
            Ok((
                dimension,
                values
                    .into_iter()
                    .map(|(value, items)| Ok((value, score(&items)?)))
                    .collect::<Result<_, DatasetError>>()?,
            ))
        })
        .collect::<Result<_, DatasetError>>()?;
    Ok(BaselineReport { schema:"eta-b0-report/v1".into(),protocol:protocol(),dataset_semantic_sha256:digest(&canonical(dataset)?),dataset_version:dataset.dataset_version.clone(),
        frozen_package_sha256:digest(&canonical(&dataset.evaluation)?),split_plan_sha256:digest(&canonical(&dataset.splits)?),
        claim:"Descriptive synthetic estimator error / simulator consistency; scripted interventions are not physical measurements or causal policy effects".into(),
        descriptive_all_included:score(&actions.iter().collect::<Vec<_>>())?,segments:reports,actions,
        declared_source_groups:dataset.splits.groups.values().collect::<BTreeSet<_>>().len(),independent_holdout:dataset.splits.evaluation.clone(),
        physical_generalization:dataset.evaluation.physical.clone(),metric_uncertainty:EvaluationAvailability::EvaluationUnavailable("NoJustifiedSamplingDesignOrInferentialMethod; dependency/replay rows are not independent".into()),
        prediction_uncertainty:EvaluationAvailability::EvaluationUnavailable("B0PointEstimateOnly; no prediction interval or physical guarantee".into()),
        causal_policy_effect:EvaluationAvailability::EvaluationUnavailable("NoCounterfactualOrCausalIdentificationEvidence".into()),
        unavailable_segments:[("road_class".into(),"UnsupportedOriginalGraphSemantics".into()),("civil_hour_or_weekday".into(),"NoCivilEpochOrTimezoneForSimulationSeconds".into())].into_iter().collect() })
}
pub(super) fn score(actions: &[&BaselineAction]) -> Result<BaselineSlice, DatasetError> {
    let mut outcomes: BTreeMap<_, usize> = [
        Lifecycle::Observed,
        Lifecycle::Pending,
        Lifecycle::RightCensored,
        Lifecycle::MissingArrival,
        Lifecycle::DisputedOrInvalid,
    ]
    .into_iter()
    .map(|state| (state, 0))
    .collect();
    let mut reasons = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut groups = BTreeSet::new();
    let mut paired_groups = BTreeSet::new();
    let (mut excluded, mut exact, mut available, mut eligible) = (0, 0, 0, 0);
    let mut residuals = Vec::new();
    let cohorts: BTreeSet<_> = actions
        .iter()
        .map(|a| tag(&a.measurement))
        .collect::<Result<_, _>>()?;
    if cohorts.len() > 1 {
        return Err(DatasetError::Source(
            "cannot pool distinct measurement cohorts".into(),
        ));
    }
    for item in actions {
        if !seen.insert(canonical(&item.key)?) {
            return Err(DatasetError::Integrity(
                "duplicate canonical evaluation action".into(),
            ));
        }
        *outcomes.entry(item.outcome).or_default() += 1;
        groups.insert(&item.dependency_group);
        for reason in &item.reasons {
            *reasons.entry(reason.clone()).or_default() += 1;
        }
        excluded += usize::from(item.partition == Partition::Excluded);
        exact += usize::from(item.observed_seconds.is_some());
        available += usize::from(item.b0_seconds.is_some());
        if item.label_eligible {
            if item.partition == Partition::Excluded
                || item.outcome != Lifecycle::Observed
                || item.observed_seconds.is_none()
            {
                return Err(DatasetError::Integrity(
                    "inconsistent exact-label eligibility".into(),
                ));
            }
            eligible += 1;
            if let Some((label, prediction)) = item.observed_seconds.zip(item.b0_seconds) {
                residuals.push(label.value() - prediction.value());
                paired_groups.insert(&item.dependency_group);
            } else {
                *reasons
                    .entry("MissingOriginalB0; no authorized fallback".into())
                    .or_default() += 1;
            }
        }
    }
    let paired = residuals.len();
    Ok(BaselineSlice {
        coverage: BaselineCoverage {
            committed_starts: actions.len(),
            excluded_starts: excluded,
            outcomes,
            reasons,
            exact_label_coverage: fraction(exact, actions.len())?,
            prediction_coverage: fraction(available, actions.len())?,
            eligible_label_coverage: fraction(eligible, actions.len() - excluded)?,
            prediction_on_eligible_coverage: fraction(paired, eligible)?,
            census_groups: groups.len(),
            paired_groups: paired_groups.len(),
        },
        metrics: metrics(residuals)?,
        metrics_unavailable_reason: (paired == 0)
            .then(|| "NoEligibleExactPredictionLabelPairs".into()),
        full_eligible_population: if eligible > 0 && paired == eligible {
            EvaluationAvailability::Available
        } else {
            EvaluationAvailability::EvaluationUnavailable(
                if eligible == 0 {
                    "NoEligibleExactLabels"
                } else {
                    "MissingPredictionsAndNoAuthorizedFallback"
                }
                .into(),
            )
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> DatasetBundle {
        crate::load_dataset(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../ml/data/phase23-representative-v1"),
        )
        .unwrap_or_else(|e| panic!("fixture: {e}"))
    }
    fn ok<T>(value: Result<T, DatasetError>) -> T {
        value.unwrap_or_else(|e| panic!("{e}"))
    }
    fn row() -> BaselineAction {
        ok(evaluate_b0(&input())).actions.remove(0)
    }
    fn seconds(value: f64) -> Seconds {
        ok(Seconds::new(value).map_err(|e| DatasetError::Source(e.to_string())))
    }
    #[test]
    fn actual_census_pairs_original_b0_and_preserves_frozen_package() {
        let dataset = input();
        let before = ok(canonical(&dataset));
        let report = ok(evaluate_b0(&dataset));
        let coverage = &report.descriptive_all_included.coverage;
        assert_eq!(coverage.committed_starts, 5);
        assert_eq!(coverage.exact_label_coverage.numerator, 4);
        assert_eq!(coverage.prediction_coverage.numerator, 5);
        assert_eq!(coverage.prediction_on_eligible_coverage.numerator, 4);
        assert_eq!(coverage.prediction_on_eligible_coverage.denominator, 4);
        assert_eq!(coverage.outcomes[&Lifecycle::RightCensored], 1);
        assert_eq!(coverage.paired_groups, 1);
        for (action, row) in report.actions.iter().zip(&dataset.rows) {
            assert_eq!(action.key, row.key);
            assert_eq!(action.rider, row.rider);
            assert_eq!(action.b0_seconds, Some(row.features.original_route.travel));
            assert_eq!(action.observed_seconds, row.label.duration_seconds);
        }
        assert!(matches!(
            report.independent_holdout,
            EvaluationAvailability::EvaluationUnavailable(_)
        ));
        assert!(matches!(
            report.physical_generalization,
            EvaluationAvailability::EvaluationUnavailable(_)
        ));
        assert!(matches!(
            report.metric_uncertainty,
            EvaluationAvailability::EvaluationUnavailable(_)
        ));
        assert_eq!(before, ok(canonical(&dataset)));
        assert_eq!(report, ok(evaluate_b0(&dataset)));
    }
    #[test]
    fn exact_zero_labels_and_signed_residuals_enter_metrics() {
        let mut action = row();
        action.observed_seconds = Some(Seconds::ZERO);
        action.b0_seconds = Some(seconds(4.0));
        action.label_eligible = true;
        let result = ok(score(&[&action]));
        let metric = result.metrics.unwrap_or_else(|| panic!("metrics"));
        assert_eq!(metric.samples, 1);
        assert_eq!(metric.mae_seconds.to_bits(), 4.0_f64.to_bits());
        assert_eq!(
            metric.mean_signed_residual_seconds.to_bits(),
            (-4.0_f64).to_bits()
        );
        action.b0_seconds = Some(Seconds::ZERO);
        assert_eq!(
            ok(score(&[&action]))
                .metrics
                .unwrap_or_else(|| panic!("metrics"))
                .mae_seconds
                .to_bits(),
            0.0_f64.to_bits()
        );
    }
    #[test]
    fn missing_predictions_keep_label_denominator_and_cannot_fabricate_fallback_performance() {
        let first = row();
        let mut missing = first.clone();
        missing.key.action = roadrunner_dispatch::ActionId::new(99);
        missing.b0_seconds = None;
        let result = ok(score(&[&first, &missing]));
        let coverage = result.coverage;
        assert_eq!(coverage.eligible_label_coverage.numerator, 2);
        assert_eq!(coverage.prediction_on_eligible_coverage.numerator, 1);
        assert_eq!(coverage.prediction_on_eligible_coverage.denominator, 2);
        assert_eq!(coverage.prediction_on_eligible_coverage.ratio, Some(0.5));
        assert_eq!(
            result
                .metrics
                .unwrap_or_else(|| panic!("paired metrics"))
                .samples,
            1
        );
        assert!(matches!(
            result.full_eligible_population,
            EvaluationAvailability::EvaluationUnavailable(_)
        ));
        let all_missing = ok(score(&[&missing]));
        assert_eq!(all_missing.metrics, None);
        assert_eq!(
            all_missing.coverage.prediction_on_eligible_coverage.ratio,
            Some(0.0)
        );
        assert!(
            all_missing
                .coverage
                .reasons
                .contains_key("MissingOriginalB0; no authorized fallback")
        );
    }
    #[test]
    fn incomplete_and_disputed_labels_never_use_bounds_or_predictions_as_exact_labels() {
        for state in [
            Lifecycle::Pending,
            Lifecycle::RightCensored,
            Lifecycle::MissingArrival,
            Lifecycle::DisputedOrInvalid,
        ] {
            let mut action = row();
            action.outcome = state;
            action.observed_seconds = None;
            action.label_eligible = false;
            action.residual_seconds = None;
            let result = ok(score(&[&action]));
            assert_eq!(result.metrics, None);
            assert_eq!(result.coverage.outcomes[&state], 1);
            assert_eq!(result.coverage.prediction_coverage.ratio, Some(1.0));
            assert_eq!(result.coverage.prediction_on_eligible_coverage.ratio, None);
        }
    }
    #[test]
    fn duplicate_actions_and_cohort_pooling_are_contract_failures() {
        let first = row();
        assert!(score(&[&first, &first]).is_err());
        let mut other = first.clone();
        other.key.action = roadrunner_dispatch::ActionId::new(99);
        other.measurement = MeasurementSupport::OperationalReportInterval;
        assert!(score(&[&first, &other]).is_err());
    }
    #[test]
    fn unmatched_prediction_label_populations_and_substituted_b0_cannot_evaluate() {
        let mut dataset = input();
        dataset.rows.remove(0);
        assert!(evaluate_b0(&dataset).is_err());
        let mut dataset = input();
        dataset.rows[0].features.b0_seconds = seconds(30.0);
        assert!(evaluate_b0(&dataset).is_err());
        let mut dataset = input();
        dataset.rows.swap(0, 1);
        assert!(evaluate_b0(&dataset).is_err());
        let mut dataset = input();
        dataset.rows[0].key.action = roadrunner_dispatch::ActionId::new(100);
        assert!(evaluate_b0(&dataset).is_err());
    }
    #[test]
    fn late_invalid_physical_or_corrected_labels_cannot_be_promoted_by_the_evaluator() {
        let mut dataset = input();
        dataset.rows[0].label.evidence.measurement =
            MeasurementSupport::PhysicalMeasurementSupported;
        assert!(evaluate_b0(&dataset).is_err());
        let mut dataset = input();
        dataset.rows[0].label.evidence.disputed = true;
        assert!(evaluate_b0(&dataset).is_err());
        let mut dataset = input();
        dataset.rows[0].label.duration_seconds = Some(seconds(999.0));
        assert!(evaluate_b0(&dataset).is_err());
    }
    #[test]
    fn frozen_splits_eligibility_and_metric_definitions_cannot_be_rewritten() {
        let mut dataset = input();
        dataset
            .splits
            .request
            .assignments
            .insert("phase23-delay-complete".into(), Partition::Test);
        assert!(evaluate_b0(&dataset).is_err());
        let mut dataset = input();
        dataset.evaluation.metrics.clear();
        assert!(evaluate_b0(&dataset).is_err());
    }
    #[test]
    fn excluded_sources_remain_in_census_and_cannot_improve_coverage_by_disappearing() {
        let dataset = input();
        let mut request = dataset.splits.request.clone();
        for (run, role) in &mut request.assignments {
            *role = Partition::Excluded;
            request
                .excluded_reasons
                .insert(run.clone(), "predeclared exclusion fixture".into());
        }
        let excluded = ok(crate::build_dataset(
            dataset.sources,
            "excluded/v1",
            dataset.reproduction,
            request,
        ));
        let report = ok(evaluate_b0(&excluded));
        assert_eq!(report.actions.len(), 5);
        assert_eq!(report.descriptive_all_included.coverage.excluded_starts, 5);
        assert_eq!(
            report
                .descriptive_all_included
                .coverage
                .exact_label_coverage
                .numerator,
            4
        );
        assert_eq!(report.descriptive_all_included.metrics, None);
        assert_eq!(
            report
                .descriptive_all_included
                .coverage
                .eligible_label_coverage
                .denominator,
            0
        );
    }
    #[test]
    fn sparse_predeclared_segments_are_visible_and_interventions_are_diagnostics() {
        let report = ok(evaluate_b0(&input()));
        assert_eq!(
            report.segments["distance_band"]["at_least_5km"].metrics,
            None
        );
        assert_eq!(report.segments["partition"]["\"Test\""].metrics, None);
        assert_eq!(
            report.segments["intervention_diagnostic"]["RoadDelayObserved"]
                .coverage
                .committed_starts,
            3
        );
        assert_eq!(report.segments["source"].len(), 3);
        assert_eq!(report.declared_source_groups, 1);
        assert!(report.unavailable_segments.contains_key("road_class"));
    }
    #[test]
    fn invalid_eligibility_does_not_silently_enter_metrics() {
        let mut action = row();
        action.label_eligible = true;
        action.observed_seconds = None;
        assert!(score(&[&action]).is_err());
        action.observed_seconds = Some(seconds(1.0));
        action.partition = Partition::Excluded;
        assert!(score(&[&action]).is_err());
        assert_eq!(ok(score(&[])).metrics, None);
    }
}
