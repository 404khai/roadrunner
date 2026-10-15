new thread 
# Roadrunner — Data/ML Architecture Sign-Off and Phase 23 Implementation Authorization

## 1. Architecture sign-off

I formally **approve and sign off the consolidated Roadrunner — Data/ML Readiness Architecture Audit**, covering Q1–Q43 and all accepted refinements.

The architecture grill is closed.

The consolidated audit is now the approved architectural baseline for Roadrunner's data, baseline, and ML roadmap.

The accepted readiness verdicts remain:

**Dataset readiness:**
`READY AFTER SPECIFIED DATA-FOUNDATION REMEDIATIONS`

**ML readiness:**
`ML NOT READY — BASELINE/DATA EVIDENCE REQUIRED`

There are no outstanding architecture questions requiring another grilling round.

Do not reopen settled decisions unless concrete repository evidence reveals a material contradiction.

---

## 2. Implementation authorization

I am now explicitly authorizing you to **implement Phase 23 — ETA Dataset**.

This authorization includes:

- Production-quality Rust implementation within Phase 23's scope.
- Necessary simulation instrumentation and trace remediation.
- Offline historical data extraction and dataset construction.
- Provenance, lifecycle, labeling, and feature contracts.
- Dataset manifests, dependency groups, and quality validation.
- Required automated and adversarial tests.
- Supporting fixtures and reproducibility tooling.
- Phase 23-specific ADRs, specifications, and documentation updates.
- Completion evidence and a formal Phase 23 completion report.

You may make the necessary repository changes.

**Do not stop at planning, pseudocode, scaffolding, or documentation. Implement and verify the phase.**

Work autonomously through the approved implementation sequence, using the audit as the normative contract.

Do not request permission after every individual task.

Where implementation choices remain open, use sound engineering judgment, examine existing repository conventions, choose the simplest architecture satisfying the accepted contracts, and document significant decisions.

If actual repository evidence contradicts an accepted semantic decision, stop only that affected work and explain the conflict. Do not silently weaken the contract.

### Explicitly unauthorized

This delegation does **not** authorize:

- Phase 24 baseline evaluation implementation.
- Phase 25 ML engine implementation.
- Model training.
- Regression or boosted-tree experiments.
- Learned inference integration.
- Shadow model execution.
- Operational learned-prediction admission.
- Learned dispatch/ranking policies.
- Physical instrumentation projects outside Phase 23.
- New distributed analytics or feature-store infrastructure.
- Unjustified stochastic travel-generator changes.

Preserve the exact roadmap sequence:

```text
Phase 23 — ETA Dataset
    ↓
Completion review
    ↓
Separate Phase 24 authorization
    ↓
Phase 24 — ETA Prediction Baseline
    ↓
Completion review
    ↓
Separate Phase 25 authorization
    ↓
Phase 25 — ML ETA Engine
```

Do not automatically advance to Phase 24.

---

# 3. Repository reconciliation before implementation

Before editing, inspect the actual current repository state.

Read and follow:

- `AGENTS.md`
- Applicable repository development instructions.
- The consolidated Data/ML Readiness Architecture Audit.
- Relevant runtime, persistence, event, dispatch, graph, and simulation ADRs.
- Phase 20–22 completion evidence.
- Existing source implementations and tests.

The audit examined repository revision:

`acfd80900dcf02c4358b2f18d731f2e72020968e`

Determine whether the current repository differs materially from that inspected revision.

Identify:

1. Current branch, HEAD, and worktree state.
2. Existing changes that must be preserved.
3. Any implementation already covering Phase 23 requirements.
4. Exact source evidence available for dataset extraction.
5. Actual simulation trace gaps.
6. Relevant existing test and fixture conventions.
7. Required modifications and their dependencies.

Do not overwrite unrelated work.

Do not assume an audit finding still applies if subsequent code changes have already resolved it.

Preserve the approved semantics while reconciling the implementation against the real repository.

---

# 4. Phase 23 implementation sequence

Work through the following workstreams in dependency order.

You may adjust internal sequencing where necessary, but all applicable completion gates must pass.

## Workstream A — Complete synthetic started-action traces

This is the first major blocker.

Current simulation exports do not establish the full population of committed road-action starts.

Implement a complete, versioned synthetic trace contract.

It must preserve every successfully committed started road action, including actions that have not arrived by the simulation horizon.

Retain or immutably reference:

```text
SimulationRunIdentity
OperationalWorldNamespace
ActionId
RiderId

Original frozen route
Originating graph/context
Original route travel estimate
Authoritative logical start reference

Schedule generations
Arrival observations/effects
Relevant intervention lineage
Final active execution status
Simulation horizon
```

Do not fabricate ActionIds from completed-movement ordering.

Do not interpret completed `RiderMoved` records as the full started-action population.

Qualify synthetic action identity by simulation run when fixture namespaces repeat.

Preserve existing simulation behavior.

Instrumentation must not alter the simulator's intended routing, dispatch, event ordering, or travel-time mechanics.

Where an existing scenario is replayed to generate newly instrumented evidence, classify the resulting trace as newly derived synthetic evidence.

Require deterministic replay-equivalence tests where supported.

Include fixtures with:

- Started and completed legs.
- Started but unfinished legs.
- Pending arrivals at the horizon.
- RoadDelay interventions.
- Schedule-generation changes.
- Frozen execution through plan replacement.
- Reused namespaces across independent runs.
- Multiple actions belonging to one order/rider lifecycle.

**Gate A:** The simulation can provide an independently verifiable census of every committed started road action in a declared run.

---

## Workstream B — Dataset source capture and canonical census

Implement the initial offline source-capture and census machinery.

The dataset population is:

**Every successfully committed logical road-action start within the declared source boundary.**

Operational identity:

```text
OperationalWorldNamespace + ActionId
```

Synthetic identity:

```text
SimulationRunIdentity
+ OperationalWorldNamespace
+ ActionId
```

Preserve RiderId as a required ownership/consistency check.

The source capture must establish consistent, explicit boundaries.

Do not treat a durable operational head as a complete historical snapshot.

Do not assume a global cross-source instant unless the source mechanism actually supports one.

Every committed started action must appear exactly once in the census.

Keep failed or rolled-back start attempts outside this population, while preserving separate upstream coverage where evidence supports it.

**Gate B:** The declared started-action census is complete, losslessly identified, deduplicated, and independently reconcilable against original source evidence.

---

## Workstream C — Provenance and label assessment

Implement the approved multidimensional provenance contract:

```text
ProducerOrigin
RecordMeaning
MeasurementSupport
DerivationLineage
```

Classifications must follow retained evidence.

Do not infer physical measurement quality from authentication, source names, successful API acceptance, or simulation output.

For started-leg labels, preserve:

```text
start_effective_at
arrival_reported_at
arrival_processing_sample_at
available acceptance/storage evidence
clock and measurement semantics
```

Do not invent physical departure, HTTP ingress, calibrated clock, or exact COMMIT timestamps.

Derive an exact duration only when the declared evidence contract supports the subtraction.

Implement the five primary lifecycle outcomes:

```text
Observed
Pending
RightCensored
MissingArrival
DisputedOrInvalid
```

Right-censoring requires its own non-arrival and observation-coverage evidence.

An absent arrival report must not become a zero-duration label or an unsupported physical censoring bound.

Preserve immutable as-of label assessments.

Rejected corrections cannot silently overwrite accepted observations.

Require exactly one primary lifecycle status per canonical started action.

Keep measurement eligibility and overlapping exclusion reasons separate.

**Gate C:** Every committed start has a scientifically honest outcome assessment, with explicit measurement provenance, evidence quality, and label eligibility.

---

## Workstream D — Original B0 lineage and point-in-time features

Phase 23 must preserve the baseline inputs required by Phase 24.

The historical reference prediction is:

```text
B0 = original frozen-route elapsed travel estimate
```

Do not substitute:

- Latest rescheduled ETA.
- Delay-adjusted completed movement.
- Recomputed current route.
- Newly adopted traffic.
- Eventual observed duration.

This phase stores and validates B0.

It does **not** implement Phase 24's MAE/RMSE evaluation.

Implement a restricted, versioned historical predictor-input boundary.

Only expose inputs supported as available to the declared action-start predictor.

The initial feature allowlist should follow the audit:

- Original frozen-route estimate.
- Original route distance.
- Supported historical graph/route characteristics.
- Pinned applicable traffic attributes.
- Supported routing/vehicle properties.
- Justified temporal attributes where clock semantics permit them.

Use exact original graph/traffic artifacts and supported historical interpretation.

Distinguish graph-byte integrity from semantic graph verification.

Legacy inspection-only artifacts cannot silently acquire newer verified graph guarantees.

Preserve availability witnesses using authoritative ordering, captured references, and historical evidence.

Do not fabricate universal `available_at` timestamps.

Exclude:

- Future traffic/readiness.
- Post-start delays and rescheduling.
- Actual arrival/completion values.
- Latest operational state.
- Synthetic latent future facts.
- Scenario seeds and IDs as default predictor inputs.
- Unsupported rolling aggregates.

Distinguish predictor-time missingness from archival loss.

**Gate D:** Historical features are point-in-time valid, retain original evidence lineage, and are insulated from later operational information.

---

## Workstream E — Dataset evidence bundle, manifests, and dependency-aware splits

Implement the five logical dataset components approved in Q24:

```text
Dataset Evidence Bundle
    ├── Source Capture / Census
    ├── Features and Original B0
    ├── Labels and Eligibility
    ├── Lineage and Dependencies
    └── Manifest and Quality Report
```

The exact physical serialization is an implementation choice.

Prefer simple, maintainable, offline tooling.

Do not introduce a feature-store service unless existing repository constraints genuinely require it.

The dataset manifest must preserve:

- Dataset/schema versions.
- Source boundaries and identities.
- Original immutable evidence.
- Graph/traffic artifact references.
- Feature and label transformation versions.
- Source provenance contracts.
- Extractor source revision and relevant patch content.
- Toolchain/dependency semantics.
- Simulation scenario/generator/seed lineage.
- Dependency-group relationships.
- Exact split assignments.
- Cohort and outcome counts.
- Eligibility and exclusion counts.
- Integrity identities.

Implement claim-specific dependency-aware splitting.

Synthetic sources must preserve shared scenario ancestry, base worlds, exogenous realizations, policy variants, and applicable coupled episodes.

Do not split closely related synthetic records into apparently independent train/test populations.

Where insufficient independent groups exist, publish an explicit `EvaluationUnavailable` assessment.

Do not invent independent worlds.

Prepare frozen Phase 24 evaluation packages identifying:

```text
Target and endpoint semantics
Prediction reference
Scientific cohort
Original B0 lineage
Eligibility and missingness rules
Label/finality policy
Dependency and split contract
Declared metrics
Coverage
Evaluation availability
```

These packages are Phase 23 artifacts for subsequent evaluation.

They do not authorize computing baseline error metrics now.

**Gate E:** The dataset is an immutable, reproducible, auditable evidence bundle, not merely a labeled CSV.

---

## Workstream F — Validation and adversarial testing

Implement the Phase 23-owned tests from the audit's Section 23 proof matrix.

Do not attempt to implement Phase 25 model-admission tests during this phase.

Mandatory coverage must include:

### Population correctness

- Every committed start represented.
- Exactly one canonical record per action.
- Unfinished actions represented.
- Reused synthetic namespaces correctly scoped.
- Duplicate observations do not create duplicate actions.
- Frozen action preservation.
- Reassignment attribution.
- Failed starts excluded from the started population.

### Label correctness

- Valid arrival without service completion.
- Missing arrival.
- Pending outcome.
- Evidence-supported right-censoring.
- Unsupported physical censoring.
- Incompatible timestamps/clocks.
- Late observations.
- Rejected corrections.
- Disputed reports.
- Valid zero-duration label.
- Invalid negative duration.
- At most one final eligible label per action/version.

### Point-in-time correctness

- Future traffic rejected.
- Later readiness cannot replace an earlier forecast.
- Arrival/completion outcomes inaccessible to the predictor.
- Post-start rescheduling cannot replace B0.
- Original graph identity enforced.
- Legacy unsupported semantics handled explicitly.
- Later-created archival metadata accepted only with valid underlying availability witnesses.
- Archival loss distinguished from predictor-time missingness.
- Precomputed synthetic future state excluded.
- Unsupported aggregates and target self-inclusion rejected.

### Split correctness

- Related synthetic worlds grouped.
- Paired policy variants retain dependencies.
- Protected episode leakage rejected.
- Repeated deterministic replay not counted as independent sources.
- Unsupported holdouts produce explicit unavailability.
- Exact split assignments reproducible.

### Reproducibility and integrity

- Same immutable source and contract yield identical semantic dataset content.
- Source captures and census counts reconcile.
- Corrupt source evidence cannot publish a false completeness claim.
- Missing artifacts produce declared unavailability.
- Different extraction boundaries produce appropriately versioned results.
- Feature values remain stable under future-data noninterference tests.

Use independently inspectable test oracles.

Do not validate extraction correctness only by comparing results generated by the extractor against itself.

**Gate F:** All applicable Phase 23 adversarial publication checks pass, and every declared dataset claim is supported by inspectable evidence.

---

# 5. Scope the initial published dataset correctly

Start with the verified synthetic evidence path.

Support historical operational extraction only to the extent existing authoritative evidence actually permits it.

Do not manufacture operational observations or physical durations merely to complete the phase.

It is acceptable for the final Phase 23 report to state:

```text
Synthetic dataset engineering:
    PASS

Synthetic started-action census:
    VERIFIED

Original B0 lineage:
    VERIFIED

Point-in-time feature validity:
    VERIFIED

Independent physical duration labels:
    NOT ESTABLISHED

Physical predictive generalization:
    EVALUATION UNAVAILABLE
```

Those statuses are illustrative and must reflect actual test evidence.

A fully valid synthetic dataset is a legitimate Phase 23 milestone.

Do not generate arbitrary noise or change the simulator to create nonzero prediction residuals.

---

# 6. Documentation and architectural records

Create or update the Phase 23-specific documentation necessary to make the implementation understandable and maintainable.

At minimum, preserve:

- The approved Q1–Q43 audit as architectural provenance.
- Data provenance and source contracts.
- Synthetic trace and all-started census semantics.
- Label eligibility and censoring.
- Point-in-time feature availability.
- Dataset schema and versioning.
- Source capture and reproducibility.
- Dependency-aware split rules.
- Data quality/publication requirements.
- Scientific-claim limitations.
- Phase 23 usage and verification instructions.

Follow the repository's ADR numbering and documentation conventions.

Do not rewrite historical architecture records inaccurately.

Where the roadmap mentions the future Phase 25 edge-level predictor, document the accepted whole-route research direction and defer detailed implementation until Phase 25 authorization.

Only make roadmap/specification changes necessary to accurately describe the approved contracts and current Phase 23 status.

---

# 7. Engineering execution and verification

Implement complete functionality, not merely abstractions.

Prefer existing crates, types, identity contracts, and source/provenance infrastructure.

Do not introduce unnecessary dependencies or architectural layers.

Preserve existing runtime boundaries.

In particular, do not weaken:

```text
Graph semantic identities
OperationalWorldNamespace
ActionId
ScheduleGeneration
AppliedExecutionEffectId
Frozen execution
Accepted terms
Operational revisions
Context adoption revisions
Atomic publication
Event recognition
Deterministic replay
```

Add focused unit tests, integration tests, and end-to-end dataset-export verification.

Run the applicable repository checks, including:

```text
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Run relevant opt-in PostgreSQL/event/storage tests when the environment supports them.

If some checks require unavailable external infrastructure, document the exact limitation rather than marking them passed.

Verify that changes do not invalidate existing Phase 15–22 behavior.

Do not blindly regenerate benchmark baselines or golden fixtures to make regressions disappear.

Before declaring completion, generate at least one small, representative, reproducible Phase 23 dataset artifact from supported source evidence and independently validate its manifest, census, lineage, and quality report.

This is dataset construction, not Phase 24 predictive evaluation.

---

# 8. Autonomous delegation expectations

You are authorized to carry the Phase 23 implementation through completion within the accepted scope.

Organize the work internally into milestones and maintain an implementation checklist.

Do not stop after creating a plan.

Do not request approval for ordinary implementation choices.

Do not repeatedly ask me to confirm already accepted Q1–Q43 decisions.

You may create supporting tests, fixtures, manifests, schemas, source instrumentation, and documentation as needed to complete the phase.

If a significant new architectural contradiction is discovered:

1. Identify exact repository evidence.
2. Explain why the approved contract cannot be implemented as stated.
3. Present the smallest viable alternatives.
4. Recommend one.
5. Stop only the affected work pending an actual decision.

Do not silently relax population, measurement, provenance, leakage, reproducibility, or publication requirements to make implementation easier.

Do not push changes or create a pull request without separate instruction.

---

# 9. Completion report

At the end of Phase 23, produce:

# Roadrunner — Phase 23 ETA Dataset Completion Report

Include:

1. Executive implementation summary.
2. Original audit requirements addressed.
3. Files/crates/modules added or modified.
4. Synthetic trace changes.
5. Source capture and census implementation.
6. Canonical dataset identities.
7. Original B0 lineage.
8. Provenance and measurement classifications.
9. Label/finality/censoring implementation.
10. Point-in-time feature extraction and availability proofs.
11. Dataset artifact schema and examples.
12. Dependency-aware split implementation.
13. Manifest and reproducibility guarantees.
14. Quality and coverage results.
15. Adversarial test evidence.
16. Existing regression-suite results.
17. Source/evaluation limitations.
18. Privacy and retention considerations.
19. ADR/specification updates.
20. Remaining issues and explicitly deferred scope.
21. Exact Phase 23 gate checklist.
22. Recommendation on readiness for Phase 24.

Every claim must distinguish actual test execution from architectural expectations.

The final verdict must be one of:

```text
PHASE 23 COMPLETE — VERIFIED FOR DECLARED SCOPE

PHASE 23 INCOMPLETE — BLOCKERS REMAIN

PHASE 23 BLOCKED — ARCHITECTURAL DECISION REQUIRED
```

If complete, the completion report must identify the scientific scope precisely.

For example:

```text
Dataset engineering:
    PASS

Synthetic source provenance:
    PASS

Canonical started-action census:
    PASS

Historical feature availability:
    PASS

Reproducible dataset publication:
    PASS

Physical label evidence:
    NOT ESTABLISHED

Physical generalization:
    EVALUATION UNAVAILABLE
```

Do not claim physical predictive validity without admissible measurements.

---

# 10. Final stop condition

**Stop after implementing, verifying, and reporting Phase 23.**

Do not proceed into Phase 24 automatically, even if Phase 23 passes all completion gates.

Do not compute or publish Phase 24 baseline-error results as part of Phase 23.

Do not begin ML experiments.

Present the implementation evidence and completion report for my review.

I will separately authorize Phase 24 after reviewing the Phase 23 result.

---

**The Q1–Q43 Data/ML Readiness Architecture Audit is signed off.**

**Phase 23 — ETA Dataset implementation is authorized.**

**You may now modify the repository, implement the approved requirements, add tests and documentation, and work autonomously through the Phase 23 completion gates.**

**Begin with repository reconciliation, then the complete synthetic started-action trace, and continue through the remaining Phase 23 workstreams. open a PR when done**
