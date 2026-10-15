# Roadrunner — Phase 24 ETA Prediction Baseline Completion Report

**PHASE 24 COMPLETE — VERIFIED FOR DECLARED SYNTHETIC DESCRIPTIVE SCOPE**

Baseline engineering: **PASS**. Supported descriptive synthetic evaluation: **PASS**.
Independent holdout, physical generalization, causal effects and uncertainty intervals:
**EvaluationUnavailable**. Phase 25 has not started.

## Authorization and reconciliation

User explicitly authorized Phase 24 and a PR after Phase 23 completion. PR #30 is merged;
feature branch `feat/phase-24-eta-baseline` starts at `e58b796` on `origin/main`, tree-equal
to verified Phase 23. Implementation uses the root checkout; no worktree was created.
Untracked `.DS_Store`, `.t3/` and `logo.png` remain untouched.
The signed [Q1–Q43 audit](data-ml-readiness-audit.md), especially sections 17–21 and its
Phase 24 proof obligations, controls the scope. [ADR 0021](adr/0021-original-b0-evaluation.md)
freezes the evaluation definitions before generating results. Original B0 is mandatory;
regression, boosted trees and fitted corrections are explicitly optional under that
contract, not automatic prerequisites for completing this phase.

## Implementation and boundaries

Added `roadrunner-dataset::baseline::{evaluate_b0, build_baseline_artifact,
publish_baseline_artifact, load_baseline_artifact, verify_baseline_artifact}` with
metric, coverage/pairing/report and immutable artifact modules. CLI commands are
`roadrunner baseline evaluate <dataset-directory> <new-result-directory>` and
`roadrunner baseline verify <result-directory>`.
The CLI reuses existing source/toolchain capture and separates namespace dispatch from
legacy command parsing. Existing routing, dispatch, simulation, API, storage and event
crates are unchanged. No model is fitted, no preprocessing is trained, and no learned
inference or operational influence is introduced.

## Original B0 and exact pairing

Every input bundle is fully source-verified and replay-reconciled before evaluation.
Prediction/label join uses run + namespace + ActionId, with RiderId and original
route/source lineage. Report retains original B0, start reference/revision, original
route digest, exact label, lifecycle, partition, dependency group and residual per action.
Delay-adjusted movement travel or modern routing cannot substitute B0. Effect aliases
and generations do not create observations. Missing/corrupt/substituted evidence and
changed census/split/metric definitions fail, rather than producing a misleading result.

## Frozen metrics and numerical semantics

Residual is eligible label minus original B0: positive means underprediction. MAE is
primary; RMSE, signed mean/midpoint median, nearest-rank absolute and strictly positive
residual p50/p95/p99, underprediction frequency and mean positive severity are required
supporting metrics. Equal weight per eligible action; valid zero durations remain.
No positive tail gives unavailable severity/tails, distinct from zero error. Empty
paired populations give unavailable metrics, not made-up zeros. MAPE is deferred.
Canonical ordering and scaled compensated binary64 arithmetic avoid sum/square overflow
on finite extremes. Invalid/nonfinite inputs reject. No epsilon changes residual sign;
tiny representational errors are retained. Exact metric and bin definitions are in the
[usage/contract guide](eta-baseline.md) and each artifact's `report.protocol`.

## Coverage, selection and fallback

All starts remain in census/coverage, including excluded or unfinished actions. Exact
metrics use only eligible Observed SyntheticControlled labels in included partitions.
Pending, missing, censored and disputed outcomes receive no fabricated point label.
Fractions preserve numerator and denominator; empty denominators are unavailable.
Eligible-label count and paired-prediction count remain distinct if a prediction is
missing. No fallback is authorized in B0 v1: incomplete full eligible-population
performance is unavailable even when paired-population metrics can be inspected.
The current source schema retains B0 on every verified start; missing-prediction behavior
is tested in the scoring kernel, not invented as missing source data in the example.
There is no arbitrary candidate-file API: unmatched/subset comparisons and unobserved
alternative outcomes cannot be admitted as model or policy improvements.

## Segmentation, grouping and uncertainty

Distance bands are <1km, [1,5)km and >=5km. Original-estimate bands are <60s, [60,300)s
and >=300s. Cohort, profile, graph, traffic, family, generator, policy, source, dependency
group and exact frozen partition each receive coverage and metrics. Retrospective
RoadDelay presence is diagnostic only. Empty known bands/partitions and zero-start
sources stay visible. Road class and civil-hour/weekday segments are explicitly
unavailable under original graph/run-clock semantics.
The all-included aggregate is descriptive; partition metrics are separate and no fitting
or holdout tuning occurs. The representative runs share one protected group; replay row
counts are not independent sample counts. Independent holdout availability is inherited
from the original split package. Metric uncertainty and per-prediction uncertainty are
separately unavailable; neither a group-count threshold nor repeatable timings justifies
confidence intervals. Point estimates are not physical arrival guarantees.

## Actual generated results

[Result manifest](../ml/results/phase24-b0-v1/manifest.json) binds the actual self-contained
artifact, original dataset and evaluator evidence. The original Phase 23 bundle remains
byte-for-byte unchanged.

| Population or metric | Measured result |
|---|---|
| Sources / protected groups | 3 / 1 |
| Committed starts | 5 |
| Exact observed labels / census | 4 / 5 |
| Original predictions / census | 5 / 5 |
| Paired predictions / eligible exact labels | 4 / 4 |
| Censored actions | 1; excluded from point-error metrics |
| MAE | approximately 10 seconds |
| RMSE | approximately 14.142135624 seconds |
| Signed mean / midpoint median | approximately +10 / +10 seconds |
| Absolute nearest-rank p50 | approximately 3.55e-15 seconds |
| Absolute p95 / p99 | 20 / 20 seconds |
| Positive p50 / p95 / p99 | 20 / 20 / 20 seconds |
| Underprediction frequency / severity | 2/4 = 0.5 / 20 seconds |

Raw roundtrip binary64 values are retained rather than replaced by these rounded display
values. Two 20-second residuals arise from the configured RoadDelay and its dependent
replay. Tiny negative residuals on otherwise formula-identical legs arise from timestamp
subtraction. These results show simulator mechanics/scripted intervention effects; they
do not identify physical variation or a learnable residual from allowed start features.
Percentiles on four dependent observations are descriptive, not population-tail estimates.

## Adversarial and independent proof

**22 new tests passed:** 16 metric/pairing unit cases, five artifact integration cases and
one actual CLI end-to-end case. They cover hand-known signs/medians/quantiles, zero and
empty populations, finite extremes, nonfinite refusal, missing predictions/no-fallback
denominators, lifecycle exclusions, cohort pooling refusal, duplicate/subset/reordered
identities, substituted B0, fabricated physical/corrected labels, frozen split/metric
changes, explicit excluded sources, empty segments/zero-start sources, deterministic
content, corruption/rehashed-report tampering and overwrite refusal.
[Independent Python/Decimal80 oracle](evidence/phase-24-baseline-oracle.json) reconstructs
pairs from original source starts and accepted arrival effects, not reported residuals.
It checks all five actions, all **24 slices**, all coverage fields, source/run/owner joins,
split/frozen-package identities, measurement limitations and **three hand-calculated
metric fixtures** shared with Rust tests. Arithmetic tolerance is relative 2e-14 / absolute
1e-12; identities, counts, eligibility and availability are checked exactly.

## Reproducibility and immutable evidence

The result retains the complete original semantic dataset (including recoverable raw
source bytes and frozen definitions), action/segment reports and actual evaluator
source/configuration/lockfile/toolchain/patch bytes. Semantic and packaging SHA-256 are
separate. Publication fsyncs staging/files and refuses an existing destination. Verification
rechecks original sources and recomputes every report field.
[Actual reconstruction](evidence/phase-24-reproduction.json) restored **149 files** in a
new source directory without Git/original paths, built the verifier **offline** with the
recorded Rust 1.96 toolchain and verified the self-contained result without the original
dataset directory. Cargo dependencies came from the local cache; third-party vendoring,
full test-fixture duplication and byte-identical binary reproduction are not claimed.

## Existing regression execution

Actual final [verification record](evidence/phase-24-verification.json):

```sh
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

All passed locally on Rust 1.96.0: **285 passed, 0 failed, 13 existing opt-in cases**.
No tests were disabled or newly ignored. The opt-in SQL/process suites were not rerun
for this offline phase: no runtime/storage/event crate changed, and historical Phase 23
SQL/process results are not claimed as new execution here.
[Legacy comparison](evidence/phase-24-legacy-behavior.json) actually reran all **38 Phase
15–19 fixtures** against the original audited Phase 22 binary: every preexisting output
field matches, current traces replay exactly and the independent started-action oracle
passes. Only the existing additive Phase 23 trace is removed for comparison; no routing,
assignment, timing, objective, accepted-term or metric golden is regenerated.
The [input-dataset oracle](evidence/phase-24-input-dataset-oracle.json) also passes.

## Scientific, privacy and performance limits

Only verified synthetic whole-leg measurements are admitted. No physical departure,
clock calibration, operational reporting quality, causal dispatch effect or unchosen
counterfactual is invented. Complete starts do not establish complete potential demand
or remove selected-policy/label selection. The one-group example supports descriptive
metrics but not independent generalization or uncertainty. No latency, memory, throughput
or scalability claim is made: these are estimator-error results, not a system-performance
benchmark. All published inputs are public synthetic fixtures; code capture excludes
credentials, environment, `.t3` and unrelated files. Future operational trajectories need
their own source/privacy/retention contract. Source removal narrows reproducibility.

## Phase 24 completion gates

| Signed audit completion gate | Actual result |
|---|---|
| Original B0 extraction and exact prediction-label pairing | PASS |
| Accepted metrics and signed residual semantics | PASS |
| Coverage, missing prediction and fallback treatment | PASS; no fallback authorized |
| Cohort, segment and dependency-group reporting | PASS |
| Evaluator adversarial tests and independent metric checks | PASS |
| Reproducible results and honest uncertainty/claim classification | PASS |

Baseline engineering and supported synthetic descriptive evaluation pass. Required
unsupported holdout/physical/uncertainty claims remain **EvaluationUnavailable**, never
marked passed. No ML superiority is required or claimed.

## Phase 25 recommendation and stop boundary

Ready for user review of this baseline evidence. A next research proposal must name its
whole-route target, scientific source cohort, genuinely start-available features,
residual hypothesis, leakage/maturity/split rules, failure criteria and reproducibility.
Baseline/data engineering evidence now exists; the current scripted one-group example
alone does not establish a learned opportunity or operational model readiness. Synthetic
research can be proposed with explicit limits; physical-generalization research requires
appropriate independent measurements. Phase 25, fitted experiments, shadow execution and
operational admission require their own applicable scope/authorization. None has started.
