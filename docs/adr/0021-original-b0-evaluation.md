# ADR 0021: Original B0 evaluation and evidence-scoped baseline results

Status: Accepted implementation of signed audit Q28–Q32 and Phase 24, authorized by
“implement phase 24, open pr when done” after merged Phase 23 PR #30.

## Decision, frozen before evaluating results

Extend the offline dataset crate with a versioned original-B0 evaluator. Consume only
fully verified Phase 23 bundles and their frozen endpoint, eligibility and split package.
No routing/runtime/simulator/storage changes, fitting, regression, boosted trees, neural
models, live inference or Phase 25 work are needed for this mandatory baseline.

`b0-evaluation/v1` pairs original frozen-route travel seconds with the eligible accepted
whole-leg duration by full run/namespace/ActionId and owner. Positive signed residual is
label minus B0. Equal weight per eligible logical action; legitimate zero labels remain.
Pending, censored, missing, disputed and excluded rows stay in coverage but not exact
metrics. Missing original B0 cannot be repaired from a current route, a delayed movement
or an observed label. Corrupt source, incomplete census, changed splits and substituted
B0 block evaluation. No external candidate predictions or fitted corrections are admitted
by this version; unmatched comparisons and invented fallback performance are unsupported.

MAE is primary. Also compute RMSE, signed mean and midpoint median, nearest-rank absolute
error and strictly positive residual p50/p95/p99, underprediction count/frequency and mean
positive residual severity. No positive residuals means unavailable severity/tails, not
an invented zero severity. Empty paired populations have unavailable metrics. MAPE is
not computed. Fractions retain explicit numerators/denominators; zero denominator is
unavailable. Overflow-safe scaled arithmetic and canonical ordering preserve finite
binary64 values; metric overflow or invalid inputs fail, never clamp.

Freeze distance bands <1km, [1,5)km, >=5km and B0 estimate bands <60s, [60,300)s, >=300s.
Also report cohort, profile, graph, traffic, family, generator, policy, source, dependency
group and exact partition. All-included aggregate is descriptive only, even if protected
partitions exist; each partition is separately visible. Retrospective road-delay presence
is diagnostic only. Unsupported road-class/civil-calendar segments remain unavailable.
No outcome-dependent rebinning, fitting or data-derived preprocessing occurs.

Report census, excluded, exact-label, eligible, prediction and paired coverage, lifecycle
counts, overlapping reasons and labeled-group counts for every slice. B0 is always present
in the currently supported verified source schema; the scoring kernel still refuses to
impute absent predictions and distinguishes eligible-label and paired denominators.
No fallback is authorized: incomplete full-population performance must be unavailable,
while paired-population metrics and coverage remain inspectable. No candidate comparison
can claim superiority by dropping inconvenient rows.

This version supports descriptive synthetic estimator error/simulator consistency only.
Independent holdout availability is inherited from the frozen split plan, never inferred
from row counts or distinct seeds. Confidence intervals remain EvaluationUnavailable:
no justified sampling design or dependency-aware inferential method is implemented.
Per-prediction uncertainty is separately unavailable for B0's point estimates. Physical
and causal policy claims remain unavailable. A nonzero scripted residual is not a
real-world learned opportunity, and an exact synthetic baseline is not a failure.

An immutable self-contained result artifact retains the complete original dataset,
its semantic digest, exact source/split/evaluation-package identities, action-level
pairing evidence, metric contract, reports and evaluator source/lockfile/toolchain/patch
bytes. Canonical semantic and actual packaging digests are separate. Publication refuses
an existing destination; verification rechecks source semantics and recomputes every
reported component. The old Phase 23 artifact and frozen definitions are not overwritten.
