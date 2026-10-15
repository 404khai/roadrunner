# Roadrunner — Phase 23 ETA Dataset Completion Report

**PHASE 23 COMPLETE — VERIFIED FOR DECLARED SCOPE**

Declared scope: offline **verified synthetic started-road-leg dataset engineering**.
Physical labels are **NOT ESTABLISHED**; physical generalization and the representative
bundle's independent predictive holdout are **EvaluationUnavailable**. Phase 24 has
not started. All execution claims below refer to actual runs recorded in the
[verification record](evidence/phase-23-verification.json), not merely proposed tests.

## 1. Executive implementation summary

Implemented a complete versioned synthetic start trace, immutable source capture,
canonical census, four-dimensional value provenance, endpoint/lifecycle assessment,
original B0, restricted static start features, dependency closure, frozen exact splits,
quality manifests and immutable dataset publication/verification. Added an offline CLI,
a generated example and independent source/reconstruction evidence.

Implementation occurred on `feat/phase-23-eta-dataset` in the root checkout. No worktree
was created. Initial HEAD was the audited `acfd809`; fetched `origin/main` was merge
commit `bc4cad6` with exactly the same tree and became the feature-branch base.
Untracked `.DS_Store`, `.t3/` and `logo.png` were preserved and excluded from the PR.

## 2. Original audit requirements addressed

Read both complete source-thread items using itemId/textOffset pagination through
`nextTextOffset=null`: audit position 485 and authorization position 487 in thread
`67a08df0-4aab-4932-9af2-443c26d06e69`. The full [Q1–Q43 audit](data-ml-readiness-audit.md)
and [implementation authorization](phase-23-authorization.md) are preserved exactly,
plus a final newline, verified by direct text comparison. The later explicit PR and
root-branch instructions control delivery. Settled architecture was not reopened.
Workstreams A–F and Phase 23-owned proof obligations are implemented; see
[proof mapping](phase-23-proof-matrix.md). Future inference/admission obligations remain
with their explicitly authorized introducing phases.

## 3. Files, crates and modules added or modified

Added `roadrunner-dataset`: `source`, `features`, `labels`, `split`, `bundle`, public
contracts and build-toolchain capture. Added 28 dataset contract tests and a CLI
integration test. Modified simulator trace collection/result exports; dispatch exposes
read-only allocator/effect evidence without changing state transitions. Added CLI
build/verify commands, Phase 23 fixtures, two independent verification scripts,
`ml/data` evidence, ADR/spec/architecture/readme updates and this report.
Three preexisting documentation comments in storage needed backticks for Rust 1.96
Clippy. No storage/runtime behavior, lint levels or test exclusion policies changed.

## 4. Synthetic trace changes

`synthetic-started-actions/v1` retains every successful `start_action`, original route,
ActionId, owner, plan/revision, generation zero, original static traffic and logical
start. It retains successful delay generations, authority effects/history, allocator
count, complete final active execution and exact pending queue. Arrival evidence joins
ActionId/generation/owner to the original processed movement sequence and source/processing
references. It never infers identity from completed movement ordering.
Explicit archive-owned run tokens distinguish repeated fixture namespaces. Default
simulation uses a deterministic input-qualified replay token. Existing scenario schemas
and `shared-execution/v2` behavior remain unchanged; the new trace is additive.

## 5. Source capture and census implementation

Each `eta-source/v1` retains recoverable graph/scenario/result bytes with byte digests,
run lineage and its own inclusive horizon/final revision. Sources are newly instrumented
re-simulations, never fabricated fields on old output. Verification decodes the original
supported graph, exactly replays the retained scenario and compares every source field.
The authority's fresh-run ActionId allocator independently reconciles the committed-start
count. Effects, ownership, generations and unfinished pending work are checked separately.
No global cross-source atomic instant or operational historical snapshot is asserted.

## 6. Canonical dataset identities

One row per `(SimulationRunIdentity, OperationalWorldNamespace, ActionId)`, with RiderId
as a required consistency join. IDs retain lossless u64 representation; a max-u64 test
proves no float conversion. Generations and effect aliases do not add rows. Replays stay
dependent. Never-started/future orders, failed starts and unchosen alternatives do not
receive fabricated ActionIds or duration labels. The simulator regression explicitly
checks an invalid second start leaves both allocator and start ledger unchanged.

## 7. Original B0 lineage

B0 is the original frozen route's elapsed travel estimate. Each row retains the original
route, graph/profile/traffic identity and start availability witness. A hand-known fixture
checks a ten-second original estimate survives a twenty-second scripted delay. Updated
schedule travel, completed movement duration and later adoptions cannot replace B0.
This phase stores/checks B0; it does not evaluate MAE/RMSE or fit corrections.

## 8. Provenance and measurement classifications

Start, B0, feature and label categories each expose producer origin, record meaning,
measurement support and versioned derivation lineage. Published examples are
`Simulator` / `SyntheticControlled`. Derived values cannot upgrade the scientific cohort.
Original route estimates remain estimates; accepted synthetic events remain synthetic.
Operational-report and physical measurements are not admitted by this extractor.

## 9. Label, finality and censoring implementation

Every canonical start has exactly one primary outcome: Observed, Pending, RightCensored,
MissingArrival or DisputedOrInvalid. Exact duration requires supported endpoint subtraction.
Zero is valid when observed; negative/impossible intervals are classified, never clamped.
Observed arrival does not require service or delivery completion. Synthetic horizon bounds
require replay-verified exhaustive coverage plus active/pending movement evidence. Runtime
silence and unsupported physical/report clocks receive no fabricated duration or bound.
Rejected corrections cannot replace original accepted endpoints. Credible disputes are
explicit new immutable-as-of assessments, not edits to prior artifacts. The synthetic
driver has no physical correction/attestation adapter; classifier refusal paths are tested.

## 10. Point-in-time extraction and availability proofs

The feature function accepts only original `StartedAction` and optional original graph.
Scenario, future readiness/delay scripts, arrivals, final plans/state, schedules and labels
are absent from its input type. Static original inputs are captured synchronously at the
successful start transition, with authoritative revision ordering and transform version.
Future-data noninterference tests change traffic, readiness, delay and horizon while
preserving the first start's features and B0. Changed original traffic identity or later
generation is rejected. No universal `available_at` or calendar meaning is invented.
ArchivalLoss, PredictorTimeAbsent, UnsupportedSemantics and NotApplicable remain distinct.
No rolling aggregates or fitted preprocessing exist.

## 11. Dataset schema and actual example

The five logical components live in one `eta-dataset/v1` JSON bundle plus a packaging
manifest. Source/census and row identities join features/B0, labels, provenance and
lineage; splits, quality, reproduction and frozen evaluation rules are retained.
The actual [representative bundle](../ml/data/phase23-representative-v1/manifest.json)
contains three sources: complete scripted-delay run, horizon-truncated descendant and
replay. Its **5 starts = 4 Observed + 1 RightCensored**; Pending/MissingArrival/Disputed
counts are zero in this example, not invented to pad coverage. All five have original B0;
four have exact synthetic labels. Graph road class is unsupported; civil day-of-week is
not applicable to run-relative seconds. Whole-leg labels are not per-edge observations.

## 12. Dependency-aware splits

Connected closure protects shared graph, family, base world, realization, graph+scenario
replay and explicit episode/coupling dependencies. Exact assignments are required for
every source; exclusions retain reasons and census rows. Protected crossings fail.
All three representative sources share **one group**, so independent holdout evaluation
is explicitly unavailable. A positive test constructs genuinely separate graph/source
components with predeclared train/validation/test roles. Common generator mechanics do
not establish mechanism-shift generalization. No random-row split or arbitrary percentage
is used. No fitting or final-holdout tuning occurs in Phase 23.

## 13. Manifest and reproducibility guarantees

Manifest binds sorted JSON semantic content separately from pretty packaging bytes,
source boundaries/schema/lineage, versions, exact split assignments, counts and frozen
future evaluation definitions. Recoverable graph/context/scenario/result and actual
Rust/configuration/lockfile/toolchain/patch bytes are retained. Numeric semantics are
roundtrip binary64 and lossless u64. The actual build compiler is embedded at build time.
Publication fsyncs staging/files and renames into a new destination; overwrite is refused.
Readers validate both identities, source replay and every rebuilt derived component.
[Actual reconstruction evidence](evidence/phase-23-reproduction.json) restores **141**
retained files without Git/original source paths, builds the verifier with recorded Rust
1.99 and verifies the bundle. Third-party dependencies use Cargo cache/registry, not a
claimed vendor archive. Full historical test-fixture duplication and byte-identical
binaries are not claimed. Code at export is retained even after later test/doc-only work.

## 14. Quality and coverage results

The independent [Python dataset oracle](evidence/phase-23-dataset-oracle.json) checks
source byte identities, allocator/census/action/owner joins, original B0/route, accepted
endpoints, movement sequence joins, horizon pending evidence and quality reconciliation.
Actual example coverage: complete 2/2 exact, horizon 0/1 exact, replay 2/2 exact; no rows
excluded. One shared policy/source cohort and one dependency group are reported.
All routes are in the predeclared under-1km band. Optional road-class/day-of-week values
are explicitly unavailable on all five rows. Coverage does not eliminate selection bias.

## 15. Adversarial test evidence

**28 focused dataset tests and one actual CLI end-to-end test passed.** Cases cover
population conservation, horizons, pre-service arrivals, run scoping, duplicates/effect
aliases, owner mismatch, cancellation, reassignment/frozen actions, original B0 under delay,
wrong/legacy/missing graph, source corruption even after rehash, all lifecycle outcomes,
clock refusal, late endpoints, disputes, zero/negative durations, archival missingness,
noninterference, generation/adoption mismatch, dependency crossings/closure/exclusions,
positive protected assignments, immutable publication, max-u64 identity and invalid numeric
inputs. The CLI test exports, verifies, refuses overwrite and detects corrupted packaging.
Small graph/time oracles assert known endpoints independently of extractor output.

## 16. Existing regression-suite results

Actual required commands passed on local default Rust **1.96.0**:

```sh
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Workspace: **263 passed, 0 failed, 13 existing opt-in cases**. Twelve database cases
were separately executed; one existing operational measurement benchmark remains opt-in.
No tests were disabled or newly ignored. Real PostgreSQL authority **5/5** and event
storage **7/7** passed using Rust 1.99 and a dedicated PostgreSQL 18.4/PostGIS 3.6.4 database.
Actual authority process histories **7/7** and event-runtime histories **5/5** passed.
Their original binary/graph identities are retained in the individual evidence records.
[Behavior comparison](evidence/phase-23-behavior.json) built audited Phase 22 source and
compared all **38 Phase 15–19 fixtures**: every preexisting output field is identical,
new traces replay exactly, and an independent source oracle reconciles all starts.
Only the additive top-level trace is removed for the old/new comparison; no goldens,
timings, assignments, costs, work counts, accepted terms or existing identities changed.

One initial authority process attempt during concurrent compilation returned HTTP 503
at dispatch with `storage_unavailable` / unsupported-or-corrupt staged checkpoint and
`published:false`. The exact failure is retained in the verification record. Root cause
was not independently isolated; compiler load was observed, and staging expiry is only
a possible explanation. The fresh controlled rerun passed all seven cases with **no
runtime code, protection or expiry-budget change**. Initial Clippy documentation issues
and a replay-key omission found by the positive split test were corrected before final
passing verification. CI results are separate from these local execution claims.

## 17. Source and evaluation limitations

Synthetic elapsed times follow frozen route mechanics plus scripted interventions.
They establish dataset engineering and simulator consistency, not learned opportunity,
real-world predictive accuracy, causal policy effects or physical clock validity.
The all-started census is not all potential demand, ingress or candidate outcomes.
Unknown exposure remains unknown. The representative one-group package cannot establish
independent uncertainty or a final holdout. Operational extraction needs a separately
supported historical boundary/source/clock/measurement contract. No such adapter is
invented here. Source replay is deliberately offline and unoptimized; scalability,
CPU/memory overhead and throughput are unmeasured.

## 18. Privacy and retention

Only public synthetic fixtures are published. CLI source capture restricts itself to
relevant Rust/SQL/configuration/lockfile bytes and excludes credentials, environment,
`.t3`, unrelated images and user data. Retained raw synthetic trajectories are explicit.
Future operational locations, IDs and timings need source-owner access controls and
minimized exports. No universal retention duration is imposed. Removing source/code
bytes must narrow reconstruction/reproduction claims; a digest alone is not an archive.

## 19. ADR and specification updates

Added [ADR 0020](adr/0020-eta-dataset-evidence-and-synthetic-traces.md),
[usage/schema guide](eta-dataset.md), proof matrix, source authorization and full audit.
Updated architecture/spec/readmes and AGENTS' approved dataset boundary. Historical
ADRs remain historical; current amendments link the implemented phase. Initial future
research granularity is pinned whole-route duration, superseding an unqualified
edge-prediction interpretation. Phase 24 must start with original B0 and separate approval.

## 20. Remaining issues and deferred scope

No declared Phase 23 engineering gate remains open. Explicitly deferred: operational
historical adapters/clock conversion, physical instrumentation, rolling aggregates,
additional stochastic generators, Phase 24 evaluators/results, fitting/training, learned
inference, shadow/admission, causal dispatch evaluation and distributed data services.
Broader within-graph split protocols and supported real independent source cohorts need
purpose-specific evidence; the implemented conservative protocol does not pretend to
supply them. No claimed performance optimization requires a new benchmark conclusion.

## 21. Exact Phase 23 gate checklist

| Gate | Actual result | Evidence |
|---|---|---|
| A: complete synthetic starts | PASS | Allocator/trace tests, independent source oracle on 38 histories, pending/action joins |
| B: bounded capture/canonical census | PASS | Recoverable bytes, source replay, run/action/owner joins, integrity/refusal tests |
| C: provenance/lifecycle/measurement | PASS within synthetic scope | Five-state tests, clock refusal, exact/bound separation and full census partition |
| D: original B0/point-in-time features | PASS | Restricted input boundary, immutable route/traffic, intervention/noninterference tests |
| E: immutable evidence/splits/manifests | PASS | Exact dependency assignments, honest unavailability, publication tests and actual source reconstruction |
| F: adversarial/independent verification | PASS | 28 dataset + CLI test, independent Python/time/source oracles and full regressions |

Dataset engineering, synthetic provenance, canonical census, historical feature
availability and reproducible publication: **PASS**. Physical label evidence:
**NOT ESTABLISHED**. Physical generalization and representative independent holdout:
**EvaluationUnavailable**, never marked passed.

## 22. Phase 24 readiness recommendation

Ready for user review of Phase 23 completion and a **separate authorization** to implement
Phase 24 B0 evaluator engineering and descriptive synthetic evaluation on this frozen
package. Preserve the original action-paired B0, frozen eligibility/split/metric rules
and unavailable holdout/physical/uncertainty statuses. An independent predictive or
physical-generalization claim needs additional admissible evidence and protected groups.
ML readiness remains **ML NOT READY — BASELINE/DATA EVIDENCE REQUIRED**. No Phase 24
metric evaluator, baseline-error result, fitting or model experiment has been started.
