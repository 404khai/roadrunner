# Offline learned whole-route ETA research

Status: accepted for the separately authorized Phase 25 implementation, 2026-10-16.
Authority: signed [audit](../data-ml-readiness-audit.md), sections 17–23,
completed Phases 23/24, and the user's explicit Phase 25 implementation request.
The earlier audit's not-yet-authorized status records its original date.

Target: exact synthetic logical elapsed seconds of a committed started whole road
leg, referenced to its original start. This does not supervise edges, unchosen
routes, future departures, path selection, physical travel or dispatch policy.

Use a regularized linear residual model before considering more complex models.
No neural architecture is mandatory in the signed contract. The Rust engine
continues without Python or ML dependencies. Only the offline CLI exports
source-verified, pinned original route requests. Numeric predictors are original
B0 seconds and original distance meters. IDs, scenario, seed, policy, family,
interventions, labels, calendar and future traffic never enter the vector.

Before results: fit one ridge correction (lambda=1, including intercept penalty)
to eligible, mature, exact Train residuals. Standardize using Train mean and
population standard deviation only; constant columns use scale 1. No tuning,
validation checkpoint selection or test-guided fitting. Exact source dependency
components and assignments are inherited, never row shuffled. Require eligible
Train/Validation/Test labels and the verified unseen-components split. Otherwise
publish EvaluationUnavailable with no fitted artifact. Excluded and incomplete
starts remain in all census/coverage reports, never fitted or zero filled.

Hypothesis: observable static features may explain a systematic residual where
the source supports one. The existing delayed dataset has no independent holdout;
its scripted future delays are not permissible features. The new controlled
no-delay graphs are an engineering null experiment: route-derived labels give
no reason to expect a useful residual. Distinct geometry/topology and declared
ancestry support a narrow within-generator component comparison, not a sampled
population or evidence of physical generalization. No new noise generator or
nonzero residual is required or fabricated.

Frozen metrics follow b0-evaluation/v1 (label minus prediction; action weighting;
MAE primary; RMSE, signed mean/midpoint median, nearest-rank absolute and positive
p50/p95/p99; underprediction frequency/severity). MAPE is deferred. Report full
coverage, exact partitions, dependency groups, all existing diagnostic segments,
matched candidate/B0 pairs, and full eligible population under original B0
fallback. Unsupported numeric values, schema, units, context, model digest,
inputs outside Train feature ranges, or negative/nonfinite predictions fail
explicitly. No clamping. A batch with any failure uses coherent all-B0 fallback
for its entire slice; matched candidate performance remains separately visible.

Candidate evidence requires complete protected-test coverage and MAE improvement
greater than 1e-9 seconds with no RMSE deterioration. This numerical margin is a
predeclared comparison guard, not a claim of scientific significance. A negative
experiment passes engineering but rejects promotion. Even a positive result is
offline evidence only: operational admission remains unavailable without separate
authorization, representativeness, uncertainty, runtime/resource, freshness,
atomicity and whole-proposal fallback gates. No routing integration or shadow.

Keep exact binary64 effective inputs, coefficients, preprocessing, source and
split digests, model identity, requests, outputs/failures and fallback lineage.
Immutable publications contain original dataset evidence plus recoverable Python
source and Rust source in the dataset reproduction package. Verification reruns
source validation, training, inference and metrics. Retained output inspection,
inference reproduction and training reproduction are distinct capabilities.
No confidence intervals, prediction bounds, production latency claims or causal
claims are authorized by these controlled experiments. Resource scope is bounded
offline, single process, pure standard-library Python, fixed two-feature model.
