# Phase 24 original-B0 baseline evaluation

Phase 24 measures original frozen-route B0 against verified whole-leg synthetic labels.
The [signed audit](data-ml-readiness-audit.md) and
[ADR 0021](adr/0021-original-b0-evaluation.md) define the contract. This evaluates the
frozen Phase 23 package; it does not change that package, fit a correction, train an ML
model or influence operational routing/dispatch.

```sh
cargo build --workspace
./target/debug/roadrunner baseline evaluate ml/data/phase23-representative-v1 /tmp/b0-v1
./target/debug/roadrunner baseline verify /tmp/b0-v1
python3 scripts/verify_eta_baseline.py /tmp/b0-v1 \
  --dataset ml/data/phase23-representative-v1
```

`evaluate` requires a Git checkout, from the repository root, to capture actual evaluator
source/configuration/lockfile/patch bytes. Build-time compiler/Cargo identity is retained.
`verify` needs only the self-contained result artifact and compatible evaluator code;
no Git, original file paths, live services, credentials or database are consulted.
An existing output destination is refused. Create a new version/directory for a changed
source, definition or corrected implementation; never overwrite published evidence.

## Pairing and populations

The evaluator first verifies the complete Phase 23 source bundle, source replay, original
B0, ownership, frozen split plan and every derived component. It joins original B0 and
eligible exact label by run + namespace + ActionId, preserving RiderId and route/source
lineage. Delay generations/effect aliases do not multiply rows or replace B0. It does
not accept separately sorted predictions/labels or arbitrary candidate files.

Each census action remains in the report, including incomplete and excluded starts.
Only Observed, admissible SyntheticControlled exact labels in non-Excluded partitions
enter metrics. Zero duration is legitimate. Pending, censored, missing and disputed
outcomes are never imputed from B0, a censor bound or extraction time. Unsupported
physical/report cohorts cannot enter this synthetic evaluator.

Coverage preserves both counts and fractions:

- Original prediction / all committed starts.
- Exact labels / all committed starts, before split exclusions.
- Eligible exact labels / included starts.
- Paired original predictions / eligible exact labels.

Zero denominator yields an unavailable ratio. Missing B0 has an explicit reason and
never shrinks the eligible-label denominator. Paired-population errors may remain
inspectable, but full eligible-population performance is unavailable without complete
predictions. This version authorizes no fallback or candidate comparison; it cannot
claim an improved model by silently dropping rows. Current verified synthetic sources
retain B0 for every start. Missing-prediction and no-fallback behavior is tested in the
scoring kernel, not fabricated as missing observations in the published example.

## Frozen metric definitions

For each exact action pair, residual = label seconds − original B0 seconds. Positive
means underprediction. Every eligible logical action has equal weight.

| Metric | Version-1 definition |
|---|---|
| MAE (primary) | Mean absolute residual, seconds |
| RMSE | Square root of mean squared residual, seconds |
| Signed bias | Mean residual and midpoint median residual |
| Absolute error tails | Nearest-rank p50/p95/p99 on absolute residuals |
| Positive residual tails | Same quantiles on strictly positive residuals only |
| Underprediction frequency | Count(residual > 0) / exact paired actions |
| Underprediction severity | Mean strictly positive residual, unavailable if none |

For n observations and probability p, nearest rank is ceil(n×p), indexed from 1.
The even signed median is the midpoint of the two central signed observations; it is
not the nearest-rank p50. Empty exact populations have unavailable metrics. No positive
residuals means unavailable positive tails/severity; zero MAE is not unavailability.
MAPE and an arbitrary denominator floor are not introduced.

Scaled compensated binary64 arithmetic avoids overflow of sums/squares on finite
extremes. Canonical ordering makes calculations reproducible. Invalid/nonfinite inputs
fail; no clamping or epsilon threshold changes residual sign. Tiny floating residuals
from timestamp subtraction remain visible. Coverage and identities are exact integers;
independent Decimal80 checks allow declared numerical comparison tolerance.

## Segments, splits and claims

Definitions are frozen in ADR 0021 and `report.protocol` before evaluation:

- Distance [0,1km), [1,5km), >=5km; original estimate [0,60s), [60,300s), >=300s.
- Cohort, profile, graph, traffic, family, generator, policy, source, dependency group
  and each exact frozen partition.
- Retrospective successful RoadDelay presence, explicitly diagnostic, never a predictor.

Known empty bands/partitions and zero-start source metadata remain visible. Road class
is unavailable under the original graph schema; civil-hour/weekday segmentation is not
applicable to run-relative seconds. No latest-state backfill, outcome-selected bins,
fitted preprocessing, random-row holdout or altered assignment is introduced.

The all-included metric aggregate is descriptive, including when train/validation/test
partitions exist; each partition is separately reported and no fitting occurs here.
The representative package has one shared dependency group. Independent holdout,
physical generalization, causal policy effects, metric confidence intervals and B0
per-prediction intervals are explicitly EvaluationUnavailable. Even three group names
would not establish a sampling design or justify confidence intervals automatically.
A point estimate is not a certified physical arrival bound.

## Results and reproducibility

[Actual results](../ml/results/phase24-b0-v1/manifest.json) retain `evaluation.json`:
complete original semantic dataset/source bytes, evaluator code/toolchain/patch bytes,
fixed protocol, exact per-action pairs, segment/coverage reports and availability.
`manifest.json` separately binds semantic and actual packaging SHA-256. Verification
replays source evidence and recomputes every report field, rejecting rehashed-but-altered
metrics, subset populations, changed B0 or changed split definitions.

The independent [oracle](../scripts/verify_eta_baseline.py) reconstructs pairings from
raw committed starts/arrival effects and computes all slices using Python Decimal80.
It also checks the [hand-calculated fixtures](../data/fixtures/phase-24/metric-oracles.json)
used by Rust unit tests. Relative tolerance 2e-14 / absolute 1e-12 applies to arithmetic
comparisons; scoped identities, counts, eligibility and availability are exact.

Restore `artifact.evaluator.files[path].utf8` into a new directory after checking hashes
and rejecting absolute/escaping paths, then build the recorded Rust toolchain's
`cargo build --offline --locked -p roadrunner-cli --bin roadrunner` and run
`baseline verify <result-directory>`. Actual reconstruction evidence is recorded in
[phase-24-reproduction.json](evidence/phase-24-reproduction.json). Cached/registry Cargo
dependencies are required; no dependency-vendor archive or bit-identical binary claim
is made. The entire external test-fixture tree is not duplicated per result package.

See [completion and actual verification](phase-24-completion.md). These synthetic
results establish baseline engineering and scoped simulator evidence. Scripted future
delays do not establish learnable real-world variation. Phase 25 requires a separate
target-specific research proposal/authorization; this phase does not start it.
