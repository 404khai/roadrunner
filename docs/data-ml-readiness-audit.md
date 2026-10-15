# Roadrunner — Data/ML Readiness Architecture Audit

**Status:** Consolidated for explicit architectural confirmation.  
**Repository examined:** `acfd80900dcf02c4358b2f18d731f2e72020968e`.  
**Decision scope:** Q1–Q43, including accepted refinements.  
**Execution scope:** Read-only inspection. No repository edits, dataset extraction, baseline evaluation, model training, or inference integration.

## 1. Executive verdict

**Dataset readiness**

> **READY AFTER SPECIFIED DATA-FOUNDATION REMEDIATIONS**

Roadrunner has a sufficiently defined operational architecture to support bounded **Phase 23 — ETA Dataset** implementation after separate authorization. It cannot yet publish the canonical started-leg dataset under the accepted contracts.

The principal source-foundation gap is the simulator’s incomplete exported started-action history. Current completed-movement records cannot establish every committed start, its original prediction, and its unfinished status at the horizon. Phase 23 must remedy and verify that gap.

**ML readiness**

> **ML NOT READY — BASELINE/DATA EVIDENCE REQUIRED**

The initial research target is **started road-leg elapsed duration/calibration**, compared with the original frozen-route travel-time estimate. A verified dataset and reproducible baseline do not yet exist for this target.

| Capability | Current assessment |
|---|---|
| Synthetic dataset engineering | Supportable after required source-foundation work |
| Synthetic ML research | Potentially supportable after Phase 23/24 evidence and separate approval |
| Operational-report prediction research | Required label, clock, source and population evidence not sufficiently established |
| Physical travel-duration prediction | Independently supported physical labels not established |
| Shadow inference | Not authorized; integration and resource-isolation evidence required |
| Operational learned prediction | Not admitted |
| Learned dispatch policies | Deferred; current evidence does not identify counterfactual policy effects |

The central answer is therefore conditional: **Roadrunner can build trustworthy, explicitly scoped datasets from retained evidence, but it must not promote operational acceptance into physical measurement, simulator consistency into real-world validity, or candidate estimates into observed counterfactuals.**

## 2. Actual roadmap phases and boundaries

The authoritative sequence is:

```text
Phase 23 — ETA Dataset
    ↓ completion review and separate authorization
Phase 24 — ETA Prediction Baseline
    ↓ completion review and target-specific readiness decision
Phase 25 — ML ETA Engine
```

The exact definitions are in [AGENTS.md](/Users/admin/Developer/roadrunner/AGENTS.md:1859).

| Phase | Accepted responsibility | Does not establish automatically |
|---|---|---|
| **23 — ETA Dataset** | Complete declared population, provenance, labels, point-in-time features, dependency-aware evaluation contracts and reproducible artifacts | Learnability, physical measurement validity or model readiness |
| **24 — ETA Prediction Baseline** | Original B0 evaluation, aligned metrics, coverage, segmentation, uncertainty limitations and reproducible evidence | ML superiority or generalization to unsupported populations |
| **25 — ML ETA Engine** | Separately authorized, target-specific learned-predictor research | Shadow authorization, operational admission or causal dispatch improvement |

Two roadmap clarifications require later authorized documentation changes:

1. Phase 24 must establish the deterministic B0 baseline before optional fitted corrections, regression or boosted-tree experiments.
2. Phase 25’s proposed `predict(edge, context)` interface does not match the accepted whole-leg label granularity. Initial research should predict the duration of a pinned whole route.

These preserve the phase numbers and sequence.

## 3. Current repository data-producing capabilities

### Evidence references

The following references identify the principal evidence used throughout this audit.

| Key | Repository evidence |
|---|---|
| **E1** | [Roadmap](/Users/admin/Developer/roadrunner/AGENTS.md:1859), [specification](/Users/admin/Developer/roadrunner/docs/spec-v1.md), [architecture](/Users/admin/Developer/roadrunner/docs/architecture.md), [glossary](/Users/admin/Developer/roadrunner/docs/glossary.md) |
| **E2** | Graph/routing ADRs [0001](/Users/admin/Developer/roadrunner/docs/adr/0001-graph-lifecycle-and-snapshot-identity.md), [0002](/Users/admin/Developer/roadrunner/docs/adr/0002-road-segments-directed-edges-and-geometry.md), [0003](/Users/admin/Developer/roadrunner/docs/adr/0003-traversal-evaluation-and-search-capabilities.md), [0004](/Users/admin/Developer/roadrunner/docs/adr/0004-astar-heuristic-policy.md); [artifact compatibility implementation](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/graph/artifact.rs:133) |
| **E3** | Dispatch/planning ADRs [0011](/Users/admin/Developer/roadrunner/docs/adr/0011-dispatch-domain-and-logical-plans.md), [0012](/Users/admin/Developer/roadrunner/docs/adr/0012-dispatch-evaluation-decision-evidence-and-commit.md), [0014](/Users/admin/Developer/roadrunner/docs/adr/0014-preparation-aware-dispatch.md), [0016](/Users/admin/Developer/roadrunner/docs/adr/0016-multi-order-normative-audit.md); [accepted terms and prediction identities](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/pooling.rs:98) |
| **E4** | [Runtime-boundary audit](/Users/admin/Developer/roadrunner/docs/runtime-boundary-audit.md), [runtime boundary](/Users/admin/Developer/roadrunner/docs/runtime-boundary.md), ADRs [0017](/Users/admin/Developer/roadrunner/docs/adr/0017-operational-authority-and-publication.md), [0018](/Users/admin/Developer/roadrunner/docs/adr/0018-planning-time-and-historical-provenance.md) |
| **E5** | [Operational execution transitions](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/operational.rs:623), [delay/effect recognition](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/operational.rs:730), [cancellation](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/operational.rs:1217) |
| **E6** | [API command/observation schemas](/Users/admin/Developer/roadrunner/crates/roadrunner-api/src/model.rs), [runtime staging and observation processing](/Users/admin/Developer/roadrunner/crates/roadrunner-api/src/runtime.rs), [Phase 20 completion](/Users/admin/Developer/roadrunner/docs/phase-20-completion.md) |
| **E7** | [Temporal publication](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/temporal.rs:594), [publication evidence](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/temporal.rs:45), [fleet objectives](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/fleet.rs), [recovery](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/recovery.rs) |
| **E8** | [Persistence guide](/Users/admin/Developer/roadrunner/docs/persistence.md), [durable entities and original execution routes](/Users/admin/Developer/roadrunner/crates/roadrunner-storage/src/entities.rs:94), [operational migration](/Users/admin/Developer/roadrunner/crates/roadrunner-storage/migrations/0001_operational.sql), [storage implementation](/Users/admin/Developer/roadrunner/crates/roadrunner-storage/src/lib.rs) |
| **E9** | [Event contracts](/Users/admin/Developer/roadrunner/docs/events.md), ADR [0019](/Users/admin/Developer/roadrunner/docs/adr/0019-durable-authority-and-event-contracts.md), [event storage](/Users/admin/Developer/roadrunner/crates/roadrunner-storage/src/events.rs), [event migration](/Users/admin/Developer/roadrunner/crates/roadrunner-storage/migrations/0002_events.sql) |
| **E10** | [Simulation scheduling](/Users/admin/Developer/roadrunner/crates/roadrunner-simulation/src/engine.rs:1276), [result construction](/Users/admin/Developer/roadrunner/crates/roadrunner-simulation/src/engine.rs:1498), [movement event schema](/Users/admin/Developer/roadrunner/crates/roadrunner-simulation/src/event.rs:74), [policy comparison](/Users/admin/Developer/roadrunner/crates/roadrunner-simulation/src/comparison.rs) |
| **E11** | [Simulation metrics](/Users/admin/Developer/roadrunner/crates/roadrunner-simulation/src/metrics.rs:7), [realized protection outcomes](/Users/admin/Developer/roadrunner/crates/roadrunner-simulation/src/metrics.rs:326), [strategy benchmark interpretation](/Users/admin/Developer/roadrunner/docs/dispatch-strategy-benchmarks.md) |
| **E12** | [Phase 21 completion](/Users/admin/Developer/roadrunner/docs/phase-21-completion.md), [Phase 22 completion](/Users/admin/Developer/roadrunner/docs/phase-22-completion.md), [Phase 21 verification](/Users/admin/Developer/roadrunner/docs/evidence/phase-21-verification.json), [Phase 22 verification](/Users/admin/Developer/roadrunner/docs/evidence/phase-22-verification.json) |

### Capability matrix

“Verified” below means supported by inspected implementation and recorded repository evidence. It does not mean this grill reran the suites.

| Capability | Classification | Data-readiness consequence |
|---|---|---|
| HTTP commands, explicit execution reports, OpenAPI 3.1 and Swagger UI | Implemented and verified within Phase 20’s supported envelope | Provides typed operational boundaries; authentication is not physical attestation |
| Operational ownership, custody, capacity, frozen execution and accepted terms | Implemented and verified | Supplies authoritative domain identities and transitions |
| Deterministic insertion, fleet optimization and recovery | Implemented and verified | Supplies decision evidence and behavior-policy context; not counterfactual observations |
| PostgreSQL/PostGIS authority and restoration | Implemented and verified within the documented single-primary envelope | Durable operational evidence exists |
| Original command outcomes, observations, effects, route and graph/traffic artifacts | Implemented, with retention and compatibility boundaries | Potential historical extraction sources; completeness must be demonstrated for each dataset |
| PostgreSQL event transport, inbox/effects, local projections and processing replay | Implemented and verified | Integration history exists; projections/notifications are not physical measurements |
| Deterministic/scripted simulation | Implemented and verified | Supports synthetic engineering and simulator-consistency claims |
| Exported exhaustive started-action simulation census | Specified by this audit; not implemented | Mandatory Phase 23 remediation |
| Independently supported physical departure/arrival measurements | Measurement evidence not established | Physical-duration labels cannot be presumed |
| Historical source-quality/clock classification manifests | Approved but unimplemented dataset contract | Unknown historical provenance remains unknown |
| Point-in-time extraction, dataset manifests and dependency-aware splits | Approved but unimplemented | Phase 23 work |
| Started-leg B0 error evaluator and ML comparison pipeline | Approved but unimplemented | Phase 24 work |
| Learned artifacts, inference validity and admission machinery | Proposed future capability | Phase 25 and separately authorized integration |

### Phases 20–22 completion evidence

The code and completion records support their documented implementation scopes. Phase 22’s verification record reports:

- 234 passing workspace cases, with opt-in cases accounted for separately.
- Five PostgreSQL authority tests and seven event-storage tests.
- Seven authority process/fault cases and five event-runtime process histories.
- Formatting, clippy, build and benchmark checks.

These establish runtime, persistence and failure-boundary evidence. They do **not** establish a production observational dataset, physical timing accuracy, dataset completeness or ML readiness.

The supported envelope remains constrained: trusted-local operation, single-primary authority, documented retained semantics and acknowledgement anchors. Remote source attestation, production high availability and physical-device measurement are not delivered by those phases.

## 4. Confirmed decisions and remaining choices

All Q1–Q43 are accepted with the supplied refinements. The full register appears in **Appendix A**; the contracts below retain their qualifications.

There are no unresolved semantic decisions requiring another grilling round.

Remaining implementation choices include serialization, physical artifact layout, extractor modules, validation tooling, exact Rust interfaces, model architecture, inference runtime and deployment topology. They remain constrained by the accepted contracts.

Unestablished evidence remains an obligation, not an unresolved preference:

- Complete synthetic start/action/horizon evidence.
- Dataset extraction and publication verification.
- Admissible operational clock/measurement evidence.
- Independently supported physical labels.
- B0 evaluation.
- Target-specific ML experiment evidence.
- Future shadow and operational admission evidence.

## 5. Recommended initial ML use case

Select **started road-leg elapsed duration/calibration**.

For an eligible action:

```text
target interval =
    admissible arrival reference
    − authoritative action-start reference

B0 =
    original frozen-route elapsed travel estimate
```

The label’s scientific meaning follows its endpoint and measurement evidence. It may be a synthetic duration, a clock-supported operational-report interval, or a physically supported duration. These are separate cohorts.

This target is smaller and better aligned with current execution identities than total delivery completion ETA. It isolates road-leg prediction from readiness, pickup/dropoff service, assignment changes and order creation timing.

The research question is whether admissible labels contain predictable variation beyond B0 using genuinely action-start-available information.

A useful residual is not guaranteed. A valid outcome is that no supported real-world ML opportunity has yet been demonstrated.

## 6. Rejected or deferred initial use cases

| Use case | Initial disposition | Reason |
|---|---|---|
| Pickup readiness | Deferred | Requires its own preparation/reference events and independent labels |
| Pickup/dropoff service duration | Deferred | Different endpoints and measurement semantics |
| Delivery completion ETA | Deferred | Different prediction reference and composite lifecycle target |
| Delivery lateness risk | Deferred | Commitment-specific labels, censoring and calibration obligations |
| Demand forecasting | Deferred | Durable operational commands are not a complete potential-demand or ingress census |
| Learned rider/candidate ranking | Deferred | Unchosen real outcomes are unobserved; selection and support unresolved |
| End-to-end learned dispatch | Deferred | Requires a separate policy-evaluation and authority design |
| Edge-level supervised duration | Unsupported by current whole-leg labels | No observed per-edge decomposition |
| Learned A* heuristic or edge-cost replacement | Deferred | Requires separate algorithmic correctness and applicability evidence |

Predictive inputs may eventually feed the existing deterministic evaluator. Learned policies do not inherit that authorization.

## 7. Data-source matrix and Data Provenance Contract

### Contract A — Data provenance

Each relevant value retains four independently queryable dimensions:

| Dimension | Required meaning |
|---|---|
| **Producer origin** | Simulator, external reporter, controlled test, manual integration, or unknown |
| **Record meaning** | Report, committed domain fact, routing estimate, forecast, model prediction, derived value |
| **Measurement support** | Synthetic controlled, operational-report interval, physical measurement supported, insufficient evidence |
| **Derivation lineage** | Original evidence and versioned transformations producing the value |

Operational acceptance does not upgrade measurement support. Aggregation, subtraction, naming and successful validation cannot strengthen the evidence’s scientific status.

### Source and permitted-claim matrix

| Source | What it establishes | Permitted dataset role and limitation |
|---|---|---|
| Committed action-start transition | Runtime accepted a logical start | Census anchor; not independently measured physical departure |
| Accepted arrival observation/effect | Runtime accepted the source report | Possible endpoint; clock and measurement eligibility assessed separately |
| Orders, assignments, profiles, plans and accepted terms | Domain facts under runtime authority | Historically supported features/lineage; current values cannot backfill earlier knowledge |
| Frozen route and retained original execution route | Original routing output and provenance | B0 and route features where historical joins are valid |
| Graph/traffic artifacts | Retained context content under supported semantics | Historical attributes; integrity and semantic compatibility are separate |
| Readiness forecasts | Predictions with their own generation/validity | Features only if available and applicable at cutoff; not actual readiness |
| Dispatch/recovery evidence | Evaluated estimates, selected plan and policy context | Selection analysis; not unchosen physical outcomes |
| Event stream/projection/notifications | Integration processing and local effects | Supporting lineage; no promotion into physical observations |
| Simulation scenario/results | Outcomes of declared generator/runtime mechanics | Synthetic engineering and scoped synthetic evaluation |
| Unknown historical source | Unestablished provenance | Coverage analysis; no invented origin or measurement quality |

A versioned source manifest binds namespace/source identity, applicable configuration and interval, producer origin, capabilities, clock assumptions, evidence references, version and integrity identity. Current configuration cannot retroactively classify unsupported historical periods.

Retrospective reclassification requires retained evidence and a new classification/dataset version.

## 8. Dataset unit and identity

The population is:

> Every successfully committed logical road-action start within the declared source boundary.

Operational identity:

```text
OperationalWorldNamespace + ActionId
```

Synthetic identity additionally qualifies that domain identity:

```text
SimulationRunIdentity
    + OperationalWorldNamespace
    + ActionId
```

`RiderId` is a required ownership/consistency join.

One logical action produces:

- One canonical census identity.
- One original action-start baseline lineage.
- Zero or one eligible final duration label per dataset version.

Schedule generations, rescheduled arrivals, observation messages and effect aliases are related records, not additional traversals.

Failed or rolled-back starts belong to separate command/coverage analysis. Never fabricate an ActionId or zero-duration label for them.

## 9. Label definitions and lifecycle rules

### Timestamp semantics

Preserve the meanings actually supported:

| Reference | Meaning |
|---|---|
| `start_effective_at` | Server-defined start sampled during transition staging, before durable commit |
| `arrival_reported_at` | Source-asserted arrival occurrence |
| `arrival_processing_sample_at` | Server sample during mutation processing; not HTTP ingress |
| Acceptance/storage-gate timing | Available gate evidence; not automatically exact transaction commit time |

A clock-compatibility witness must establish the relationship needed for subtraction: clock identities, epoch/conversion, offset/drift assumptions, precision, uncertainty, restart behavior and validity where applicable.

Compatible clocks alone do not make a logical server start a physical departure.

### Primary lifecycle partition

Each action has exactly one primary outcome as of extraction:

```text
Observed
Pending
RightCensored
MissingArrival
DisputedOrInvalid
```

The partition reconciles to all committed starts. Measurement support and eligibility reasons are separate dimensions and may overlap.

An observed arrival is the travel endpoint. Service completion and successful delivery are not prerequisites.

Right-censoring requires explicit coverage, timing and non-arrival evidence. Reporting silence, `Travelling`, cancellation, restart or incomplete delivery is insufficient for physical censoring.

Missing endpoints receive no fabricated exact duration. Supported censoring bounds are separate from exact labels.

### Corrections and finality

Distinguish:

- Operational finality.
- Extraction finality as of a declared boundary.
- Scientific confidence.

Rejected corrections do not replace accepted timestamps. Credible disputes follow a defined evidence policy. New evidence may change eligibility in a new immutable dataset version; published artifacts are not overwritten.

No universal waiting period is imposed.

## 10. Point-in-time feature availability contract

### Contract B — Point-in-time dataset

For the initial static, departure-invariant task:

```text
Prediction reference: start_effective_at
Publication condition: start successfully committed
Extraction boundary: declared source revision/time capture
```

The prediction reference is neither exact route-computation time nor physical departure.

Each feature must establish:

```text
source and scoped entity/reference
semantic meaning
original input/artifact identity
event/observation time, where supported
source processing/adoption ordering
predictor information boundary
availability/applicability witness
transformation and interpretation version
availability or typed-unavailability outcome
```

Authoritative revision/adoption ordering takes precedence over rounded timestamp equality. Earlier physical occurrence does not establish earlier runtime knowledge.

Retrospective calculation from original available inputs is legitimate. Later information is not.

Initial feature scope is deliberately small:

- Original frozen route estimate and distance.
- Supported original graph/route attributes.
- Applicable traffic/profile context.
- Other supported attributes justified by actual availability.

Calendar features require epoch/timezone semantics. Simulation seconds are not automatically civil hours or weekdays.

IDs, seeds, scenario names, policy identities and hashes initially serve joins, provenance and grouping—not predictors.

Rolling aggregates are deferred. Any later aggregate must use admissible contribution knowledge, exclude the target and aliases, respect training-label maturity, and fit preprocessing only on authorized training data.

## 11. Leakage taxonomy and prevention

| Leakage class | Example | Prevention |
|---|---|---|
| Temporal/look-ahead | Traffic adopted after start | Availability witness and authoritative ordering |
| Target-derived | Arrival or realized delay embedded in feature | Restricted predictor-input interface |
| Post-decision state | Later rider location or plan revision | Historical source capture and explicit lineage |
| Prediction substitution | Delay-adjusted route replacing B0 | Immutable original baseline |
| Entity/identity proxy | Scenario ID encoding outcome pattern | Metadata-only IDs initially; claim-specific holdouts |
| Aggregate contamination | Rolling mean includes target/future labels | As-of contribution rules and training maturity |
| Evaluation fitting | Normalization fitted on test rows | Training-only fitted artifacts |
| Dependency overlap | Paired policies or shared episodes split across partitions | Dependency grouping and closure |
| Historical reinterpretation | Modern graph/routing semantics backfill missing past | Retained original semantics or typed unavailability |
| Simulator oracle | Scheduled future RoadDelay used as input | Separate latent generator knowledge from predictor information |

Feature construction must operate through a constrained historical-input boundary, separate from label/outcome interfaces. Removing columns afterward is insufficient proof.

Required noninterference: adding later traffic, readiness, arrival, plans, fulfillment, disputes or adoptions cannot inject hindsight into original valid features or B0. A new version may downgrade eligibility when evidence reveals an earlier provenance error; it cannot silently rewrite history.

## 12. Synthetic versus observational limitations

The simulator schedules arrival from route elapsed travel time. Scripted `RoadDelay` can add a synthetic residual.

```text
Original estimate: 300 seconds
Scripted delay:      45 seconds
Synthetic outcome: 345 seconds
```

The residual is synthetic intervention evidence. It is not independently measured traffic variation.

Completed `RiderMoved.leg.travel` may incorporate the delay. It cannot replace the original 300-second B0.

Four claims remain distinct:

1. Dataset pipeline correctness.
2. Simulator consistency.
3. Synthetic generalization under declared mechanisms/families.
4. Real-world predictive validity.

Different seeds do not establish different mechanisms. Current seeded readiness variation does not imply independently varied graphs, demand, road-delay scripts or traffic.

Preserve hierarchical lineage: generator/runtime/configuration, scenario family, base world, graph/demand/fleet, exogenous realization, run/seed and policy variant.

Paired-policy comparisons require evidence of actual exogenous comparability. Same seed alone is insufficient.

Phase 23 does not require new noise or a richer generator. Zero residuals are valid simulator-consistency evidence.

### Mandatory trace remediation

Current exports omit a complete start ledger, movement ActionIds and comprehensive final active execution/pending-event evidence. Fixture namespaces can repeat across runs.

Phase 23 must preserve an auditable all-started trace, original route/B0, start and ownership lineage, arrivals/generations and horizon status.

Instrumented re-simulation is a **new derived synthetic source**, supported by retained scenario/configuration/artifacts and replay-equivalence evidence. It cannot masquerade as fields extracted from old outputs.

## 13. Selection bias and counterfactual limitations

The all-started census is complete only within committed starts.

Keep distinct units for orders, decision episodes, candidate placements, assignment transitions, started actions, observations/effects and eligible labels.

The command ledger is not a complete ingress or potential-demand census. Authentication, malformed requests and pre-reservation saturation may be absent. Unsupported exposure is **unknown**, not zero.

Labels form another selected subset. Report differential availability using supported pre-start characteristics and source groups. Coverage reveals selection; it does not remove it.

Preserve distinctions such as missing prediction, incomplete search, infeasible insertion, cooldown exclusion and stale proposal. They do not mean the same thing.

Permitted claim categories:

```text
EstimatorObjectiveComparison
SimulatorPolicyComparison
ObservedPredictivePerformance
IdentifiedCausalPolicyEffect
```

Only the corresponding evidence supports each category. Exhaustive candidate prediction is not exhaustive physical observation.

Current deterministic histories do not establish support, overlap, confounding control or fleet-interference assumptions for causal off-policy evaluation. Propensity logging alone would not resolve them. No production experimentation is authorized.

## 14. Dataset schema, versioning and reproducibility

Use five **logical components**, without prescribing five files or separate services:

| Component | Required content |
|---|---|
| Source capture/census | Declared boundaries and every canonical committed start |
| Features/B0 | Original prediction, reference, availability, transformation and unavailability evidence |
| Labels | Endpoints, clocks, cohort, primary outcome, duration/bound, eligibility, dispute/finality |
| Lineage/dependencies | Artifacts, actions, aliases, interventions, runs, policy/world/episode relationships and split groups |
| Manifest/quality | Versions, inclusion rules, exact assignments, counts, checks and integrity identities |

Preserve lossless scoped IDs, typed units and declared precision. Nulls need reasons. Distinguish not applicable from not retained or unsupported.

The manifest binds:

- Dataset/schema identities and canonicalization.
- Namespace/source boundaries and schema versions.
- Immutable evidence and graph/context references.
- Extractor revision; clean source identity or retained patch bytes.
- Lockfiles, toolchain/runtime and numeric semantics.
- Feature/label/provenance/eligibility versions.
- Scenario/generator/configuration/seed/policy/coupling evidence.
- Split protocol, dependency relationships and actual assignments.
- Row, group, missing, censored, excluded and cohort counts.
- Semantic content digest and separate packaging identity.

A durable head fingerprint is not a full historical snapshot. Source capture needs a consistent supported read boundary and retained evidence. Multiple namespaces use explicit per-source boundaries unless genuine cross-source atomicity is established.

A digest without recoverable bytes is not an archive.

## 15. Train/validation/test strategy

Splits test a declared generalization claim.

### Synthetic sources

Initially keep shared base worlds, scenario descendants, replays, paired policies, exogenous realizations and coupled trajectories together where required by the claim. Apply connected dependency closure.

One usable dependency group may support descriptive evaluation but not a manufactured independent holdout.

Distinguish within-mechanism, scenario-family, graph and mechanism-shift claims. Shared generator semantics do not invalidate every split; they limit its interpretation.

### Operational sources

Prefer chronological or rolling-origin designs appropriate to the target. Specify:

- Feature cutoff.
- Training cutoff and label maturity.
- Fitting population.
- Evaluation boundary.
- Crossing order/journey/episode handling.
- Relevant group protection.

Do not group every rider or graph forever unless unseen-rider or unseen-graph generalization is the intended claim.

No arbitrary split percentages or embargo durations are imposed.

Validation supports development. Final holdouts remain protected. Repeated tuning against them changes their role and invalidates an independent final-test claim.

Retain excluded assignments and reasons, not just the final partition labels.

## 16. Quality, eligibility, privacy and publication rules

| Condition | Required consequence |
|---|---|
| Expected pending/missing/censored outcome | Valid census record; explicit classification |
| Unsupported exact timestamp interval | Exact label ineligible; source evidence retained |
| Missing optional feature | Declared unavailable/null; no modern backfill |
| Missing required B0 evidence | Affected evaluation ineligible; action retained |
| Credible dispute | Explicit status under evidence policy; no silent replacement |
| Corrupt immutable artifact or ambiguous action identity | Block affected integrity/publication claim |
| Unverified complete source census | Source cannot claim completeness |
| Future feature or false provenance | Block affected dataset contract |
| Prohibited split overlap | Block affected evaluation/publication contract |
| Sparse or shifted distribution | Diagnostic unless a justified contract makes it disqualifying |

An independently verified source may publish under explicitly narrowed scope. Unverified sources cannot disappear silently or be removed because their outcomes are inconvenient.

Privacy/access controls cover locations, trajectories, stable IDs and timing. Use controlled raw evidence, minimized exports and justified pseudonymous linkage sufficient for joins, corrections and grouping. Do not export credentials or unrelated personal data.

Retention follows source/data-owner requirements; no universal duration is imposed. Authorized removal may narrow source reconstruction, inference reproduction or evaluation capabilities. Report the remaining capability precisely.

## 17. Non-ML baseline specification

### Contract C — Baseline-to-ML promotion

Mandatory B0:

```text
B0(action) = original frozen-route elapsed travel estimate
```

B0 must retain action, route, graph, traffic/profile/context and prediction-reference lineage. Missing historical B0 cannot be repaired using a modern route and still called B0.

Optional comparisons include fixed predeclared corrections and historically fitted corrections. A fitted correction is a statistical artifact with training-only data, mature labels, protected splits, fitting version, parameters, missingness and fallback.

No regression, boosted-tree or neural technique is mandatory merely because the roadmap lists it.

A perfect synthetic B0 establishes simulator consistency. No nonzero residual requirement applies.

## 18. Evaluation metrics, segmentation and uncertainty

For each eligible exact label:

```text
residual = eligible_duration − predicted_duration
```

Positive means underprediction; zero is exact.

**Primary:** MAE in seconds, equally weighted per eligible logical action.

**Required supporting evidence:** RMSE, mean/median signed residual, declared absolute-error and positive-residual-tail quantiles, underprediction frequency/severity, exact prediction/label coverage, cohort and dependency-group counts.

Freeze metric versions, quantile probabilities/calculation, weights, eligibility, segments and missing-prediction treatment before confirmatory interpretation.

Legitimate zero-duration labels remain valid for MAE/RMSE. Negative or impossible durations are classified, not clamped. MAPE is deferred; no denominator floor is invented.

Pending, missing, disputed and censored outcomes do not enter exact point-error metrics. Their coverage remains visible.

If a candidate predicts fewer rows, report:

1. Prediction coverage and failure reasons.
2. Matched-population comparison.
3. Full eligible-population performance under the approved fallback policy, where applicable.

Predeclare supported route-distance/estimate bands, cohorts, measurement support, profile, graph/traffic, family and policy segments. Retrospective intervention segments are diagnostics, not predictor inputs.

Uncertainty methods must respect dependence and sampling assumptions. Replay timings are not predictive confidence intervals. With inadequate independent groups, report descriptive results and uncertainty unavailability.

Metric uncertainty and per-prediction uncertainty are separate.

## 19. Phase 23 entry and completion gate

**Entry**

- Explicit confirmation of this audit.
- Separate Phase 23 implementation authorization.
- Approved initial source and started-leg scope.

Required synthetic trace remediation may be implemented **within Phase 23**; it is not a prerequisite to authorizing all Phase 23 work.

**Completion/publication**

- Complete declared committed-start census.
- Run-scoped synthetic identities and trace/horizon evidence.
- Original route/B0 and ownership joins.
- Provenance, clocks, label eligibility and lifecycle semantics.
- Restricted point-in-time predictor-input construction.
- Historical artifact interpretation and typed missingness.
- Lineage, dependency grouping and valid split treatment.
- Immutable retained evidence and reproducibility manifest.
- Source-appropriate access/retention controls.
- Quality, adversarial and independent reconciliation evidence.
- Explicit scientific limitations.

A correctly scoped synthetic-only dataset may pass. Missing physical labels do not require fabricated values or invalidate that engineering milestone.

## 20. Phase 24 entry and completion gate

**Entry**

Accepted Phase 23 completion evidence, frozen evaluation package and separate authorization.

**Completion**

- Original B0 extraction and exact prediction-label pairing.
- Accepted metric and residual semantics.
- Coverage reconciliation and missing-prediction treatment.
- Cohort/segment/group reporting.
- Evaluator adversarial tests and independent metric checks.
- Reproducible results and honest uncertainty/claim classification.

These statuses may coexist:

```text
Baseline engineering: PASS
Supported synthetic evaluation: PASS
Physical generalization: EvaluationUnavailable
```

An unavailable required holdout is never marked as passed. Phase 24 need not demonstrate ML superiority.

## 21. Phase 25 research entry and readiness

Require:

- Completed relevant Phase 23/24 gates.
- Target-specific proposal and separate authorization.
- Labels appropriate to the proposed scientific claim.
- Leakage-safe features, mature training labels and protected evaluation.
- B0 evidence and a defensible residual hypothesis.
- Metrics, failure criteria, reproducibility and resource scope.

No fixed row count or percentage improvement substitutes for evidence.

Research need not guarantee success. Residuals must still be interpreted: scripted delay, reporting behavior, clock error and physical variation are different explanations.

Current status remains **ML NOT READY — BASELINE/DATA EVIDENCE REQUIRED**.

Offline research, shadow evaluation and operational influence have separate authorization levels. Model availability, lower MAE and successful shadow execution do not automatically admit operational influence.

## 22. Future inference, provenance and fallback boundary

Initial conceptual interface:

```text
Pinned directed route
+ declared departure/evaluation reference
+ supported routing/context
+ feature snapshot
+ model identity
    ↓
Whole-route duration prediction and evidence
```

Started-route accuracy does not automatically validate unchosen candidate routes, future departures, path selection, edge costs or A* heuristics.

Retain exact effective values consumed after numerical conversion, with model/artifact/adoption identity, target, feature/preprocessing versions, recoverable inputs, context, applicability, runtime semantics and fallback lineage.

Separate:

- Historical decision inspection/replay.
- Inference reproduction.
- Training reproduction.

Recorded output may support decision inspection when the model is unavailable, provided compatible decision/evaluator evidence remains. It does not establish inference or retraining reproduction.

Model rollout and rollback are authoritative adoption transitions, including `M2 → M3 → M2`. Prediction applicability and whole-proposal freshness remain separate checks.

Initially, a required learned-prediction failure invalidates the learned evaluation and invokes a coherent deterministic reevaluation where supported. Do not silently rank mixed-source candidates. Fresh authority validation remains mandatory.

Historical B0 is immutable; runtime fallback is the deterministic estimate for the current pinned planning context.

Expected duration, quantile, interval, conservative allowance and certified physical upper bound are distinct. Neither a mean nor calibrated quantile guarantees physical arrival.

Existing hard domain invariants, accepted terms, objective ordering and atomic publication remain authoritative.

Inference operates on captured inputs outside authoritative publication, followed by revalidation. Measure loading, cold/warm latency, CPU/memory, concurrency, batching, queueing and failures before selecting runtime or budgets. Shadow execution must demonstrate isolation from authoritative behavior.

## 23. Adversarial tests and independent evidence

### Existing evidence

Operational, storage and event tests cover identities, duplicates/aliases, frozen execution, stale contexts, expiry, atomicity, restart and lost acknowledgements. Simulation tests cover deterministic progression and supported interventions.

They provide supporting foundations. They do not satisfy the new dataset/evaluator/inference proof obligations automatically.

### Mandatory proof matrix

| # | Adversarial case | Expected result | Owner |
|---|---|---|---|
| 1 | Feature available only after cutoff | Reject predictor feature; retain census action | 23 |
| 2 | Later readiness replaces original forecast | Reject substitution; preserve original evidence | 23 |
| 3 | Label directly generated from route cost | Synthetic-only evidence | 23/24 |
| 4 | Duplicate observation | One action, at most one final eligible label | 23 |
| 5 | Attempted timestamp correction | Preserve accepted original; explicit dispute handling | 23 |
| 6 | Never-assigned/never-started order | No fabricated leg or counterfactual label | 23 |
| 7 | Cancellation before delivery | Preserve starts and independent arrival status; no zero label | 23 |
| 8 | Multiple predictions | Original B0 retained; separate later lineage | 23/24 |
| 9 | Reassignment | Actual committed-start owner | 23 |
| 10 | Frozen leg survives replacement | Same action and original baseline | 23 |
| 11 | Obsolete graph | Supported original interpretation or typed unavailability | 23 |
| 12 | Scenario ancestry crosses protected splits | Reject overlap/regroup | 23 |
| 13 | Protected optimization episode crosses splits | Apply declared grouping/boundary rule | 23 |
| 14 | Shared generator in train/test | Only supported within-mechanism claim | 23/24 |
| 15 | Behavior-policy change | Preserve selected-population and policy lineage | 23/24 |
| 16 | Future actual traffic used | Reject leakage | 23 |
| 17 | Different baseline/candidate populations | Reject unmatched comparison | 24/25 |
| 18 | Many synthetic rows, absent physical labels | Physical evaluation unavailable | 23/24 |
| 19 | Unobserved alternative dispatch outcomes | Unsupported causal claim | 24/25 |
| 20 | Expired/unsupported prediction | Reject effective inference or approved fallback | 25/admission |
| 21 | Incompatible clocks or absent departure evidence | Insufficient measurement evidence; no invented duration | 23 |
| 22 | Same-second timestamps, different adoption order | Use authoritative order; reject unsupported availability | 23 |
| 23 | Arrival accepted after extraction cutoff | Pending/missing as applicable; later version may observe | 23 |
| 24 | Reported arrival before start | Domain/measurement classification; never clamp | 23 |
| 25 | Active action/pending arrival at horizon | Included with source-supported incomplete/censor status | 23 |
| 26 | Restart during active road action | Preserve logical identity and supported lineage | 23 |
| 27 | Reused namespace across runs | Distinct run-qualified identities | 23 |
| 28 | Stale generation or several effect aliases | Respect rejection/recognition; no duplicate label | 23 |
| 29 | Valid zero or invalid negative duration | Eligible zero where supported; invalid negative classified | 23/24 |
| 30 | Target/future aggregate or held-out fitting | Reject contamination | 23/24 |
| 31 | Archival loss versus predictor-time absence | Different unavailability; no archival-loss predictor proxy | 23 |
| 32 | Later records added | Original valid feature/B0 noninterference | 23 |
| 33 | Missing/corrupt artifact, wrong schema/units, NaN/infinity | Typed failure; no silent model/clamping substitution | 25 offline |
| 34 | Model/context change, including adoption ABA | Applicability and proposal freshness checked separately | Shadow/operational |
| 35 | One candidate fails, fallback becomes stale | Coherent fresh reevaluation; no partial publication | Operational |
| 36 | Batch identity mismatch, timeout, saturation, abandoned inference | Bound results correctly; supported failure/isolation | Relevant admission |
| 37 | Numeric conversion changes feasibility/rank | Exact effective value retained; decision-sensitive validation | Operational |
| 38 | Old model missing, output retained | Inspection may remain; inference reproduction unavailable | Relevant admission |
| 39 | Shadow overload or failure | No unauthorized authoritative outcome change | Shadow |
| 40 | Model attempts to weaken domain protections | Reject; no partial authoritative effect | Operational |

Fixtures need independently inspectable source evidence and oracles, rather than expected outputs generated solely by the extractor under test.

Mandatory properties:

```text
Committed starts
    = Observed + Pending + RightCensored
      + MissingArrival + DisputedOrInvalid
```

Each canonical start appears exactly once. Overlapping eligibility reasons are reported separately.

Phase 24 additionally requires independently checked metric fixtures for sign, pairing, zero labels, quantiles, cohort separation, missing predictions and fallback denominators.

Future inference tests belong to the capability introducing them; they are not hidden Phase 23 implementation requirements.

## 24. Required ADR and specification amendments

Recommend later authorized amendments covering:

1. **Dataset source/provenance contract:** four provenance dimensions, timestamp/clock evidence and permitted claims.
2. **Started-leg dataset contract:** population, scoped identity, original B0, lifecycle/finality, restricted historical inputs, missingness and dependency-aware evaluation.
3. **Synthetic trace contract:** complete starts, generations, arrivals, horizon evidence, run identity and instrumented re-simulation lineage.
4. **Baseline/evaluation contract:** residual sign, metrics, coverage, splits, uncertainty and claim classification.
5. **Phase promotion contract:** engineering completion, scientific evidence and authorization kept separate.
6. **Future prediction/inference contract:** whole-route interface, artifacts, exact effective outputs, adoption, validity, fallback and reproduction guarantees.
7. **Operational admission contract:** prediction-to-constraint semantics, resource isolation, rollback and accepted-term preservation.

Amend roadmap/specification terminology to distinguish started-leg duration from end-to-end ETA, observed reports from physical measurement, and predictive results from policy effects.

Review stale deferral/status headers against Phase 20–22 completion evidence. Preserve historical context while making current status unambiguous.

No amendments were applied.

## 25. Risk register

Severity describes the consequence if a violation is permitted, not a measured incident rate.

| Risk | Severity | Evidence | Mitigation and ownership |
|---|---|---|---|
| False physical measurement claim | Critical | Logical server start and source-reported arrival lack established physical/clock evidence | Cohorts, clock/endpoint witnesses; 23 |
| Temporal/target leakage | Critical | Mutable current contexts and later outcomes exist | Restricted historical inputs, availability proof, noninterference; 23 |
| Incomplete started census | High | Completed-only simulation export gaps | Complete trace/horizon evidence; 23 |
| Synthetic generator overfitting | High | Arrivals follow route estimates/scripted delays | Explicit mechanism claims and lineage; 23–25 |
| Survivorship/reporting selection | High | Labels require accepted arrivals and measurement eligibility | Full census, distinct denominators, differential coverage; 23/24 |
| Unsupported causal claim | Critical | Unchosen physical outcomes absent | Claim taxonomy; separate causal design |
| Scenario/episode split leakage | High | Shared worlds, policies and trajectories | Dependency closure and retained assignments; 23 |
| Historical artifact incompatibility | High | Inspection-only versus operational semantics | Supported original interpretation; typed unavailability; 23 |
| Unreproducible artifact | High | Head/digest alone is not recoverable source content | Bounded source bundle, retained bytes, canonical manifests; 23 |
| Privacy/excessive retention | High | Locations, trajectories and stable identities | Controlled raw access, minimized exports, owner-defined retention; 23+ |
| Misleading baseline metric | High | Existing ETA/duration distributions have different origins | Exact pairing and frozen metrics; 24 |
| False uncertainty precision | High | Many dependent rows/replays | Group counts and justified inference; 24 |
| Mixed estimator ranking | High | Partial learned availability can alter comparisons | Coherent fallback reevaluation; future admission |
| Stale model/context inference | Critical | Runtime authority can change during computation | Pinned adoption and publication validation; future admission |
| Deadline underprediction | Critical | Point estimates are not physical bounds | Separate prediction-to-constraint policy; future admission |
| Unsafe operational ML authority | Critical | No learned admission machinery/evidence yet | Separate offline/shadow/operational gates |
| Shadow resource interference | High | Inference resource behavior unmeasured | Representative measurement and isolation; shadow admission |
| Numerical decision instability | High | Small changes can affect feasibility/ties | Effective-value retention and numerical contract; future admission |

## 26. Explicitly deferred scope

Deferred unless separately justified and authorized:

- Feature-store or analytics services.
- Richer stochastic travel generators.
- Physical departure/edge instrumentation.
- Rolling historical features and synthetic augmentation experiments.
- Readiness, service-duration, lateness and demand models.
- Edge supervision and learned routing heuristics.
- Learned candidate ranking and end-to-end dispatch.
- Causal/off-policy policy-effect estimation.
- Prospective production experiments.
- Distributed model-serving infrastructure.
- Automatic model admission or phase promotion.
- Universal real-world ETA-performance claims.

These are not hidden prerequisites for an accurately scoped synthetic dataset milestone.

## 27. Recommended implementation order and confirmation boundary

1. **Confirm this consolidated architecture audit.**
2. Separately authorize bounded Phase 23 implementation.
3. Establish complete source traces/capture and scoped identities.
4. Implement census, original B0 lineage, provenance and lifecycle contracts.
5. Implement restricted point-in-time features and historical artifact handling.
6. Implement dependency/split, manifest, quality, access and reproducibility contracts.
7. Verify adversarial properties and review Phase 23 completion evidence.
8. Separately authorize Phase 24; implement and evaluate B0 on the frozen package.
9. Review supported baseline claims, residuals, coverage and uncertainty.
10. Make a target-specific ML-readiness decision and separately authorize any Phase 25 research.
11. Consider shadow and operational admission only through their separate gates.

Architecture confirmation does not authorize repository changes, dataset extraction, baseline execution, training or inference integration.

# Appendix A — Full Q1–Q43 decision register

The detailed contracts above are normative qualifications of these entries. Evidence keys refer to Section 3. “Owner” identifies responsibility, not authorization already granted.

| ID / problem | Evidence | Accepted decision and required invariant | Rationale; rejected/deferred alternatives | Consequence / owner |
|---|---|---|---|---|
| **Q1 — Prediction or learned policy?** | E3–E7 | Prediction-first; deterministic evaluator retains feasibility, protection, objective and publication authority | Existing decision logic is substantial; defer ranking/direct policies and unsupported causal claims | Predictive, synthetic and causal results separate; 25/future admission |
| **Q2 — Initial target?** | E5, E10 | One started road-leg interval versus original route estimate; one final eligible label/action/version | Smaller aligned target; whole-leg observations are not edge labels; defer composite ETA and other targets | Measurement-qualified labels; 23–25 |
| **Q3 — Synthetic scope and promotion?** | E1, E10–E11 | Synthetic-only dataset engineering permitted; B0 precedes learned experiments; separate Phase 25 decision | Formula reproduction is not physical validity; reject arbitrary row/improvement gates | Evidence-scoped progression; 23–25 |
| **Q4 — Admissible subtraction?** | E5–E6, E8 | Preserve actual endpoint meanings; require clock relationship and additional physical-event evidence for physical claims | Acceptance does not calibrate clocks or prove departure | Unsupported intervals explicitly ineligible; 23 |
| **Q5 — Provenance taxonomy?** | E6, E8–E10 | Independent origin, meaning, measurement and derivation; evidence-backed versioned classification | One source category loses distinctions; names/authentication do not establish measurement | Unknown stays unknown; derivations cannot upgrade evidence; 23 |
| **Q6 — Baseline/label owner?** | E5, E8 | Namespace+ActionId, Rider consistency and original route; generations/aliases separate | Latest remaining estimate leaks; no fabricated preparation timestamp or missing joins | One canonical action and immutable B0; 23 |
| **Q7 — Finality/corrections?** | E5–E6, E8 | Immutable as-of datasets; accepted original retained; rejected corrections do not amend labels | Operational finality differs from scientific certainty; no arbitrary waiting period | Credible disputes affect new versions; 23 |
| **Q8 — Historical reconstruction?** | E7–E10 | Original evidence/versioned derivation only; current state checks integrity, not missing historical values | Modern routes/forecasts cannot prove original knowledge | Typed missing artifacts and reconciled coverage; 23 |
| **Q9 — Population criterion?** | E5, E10 | All committed starts; arrival endpoint; fulfillment success not required | Completed-only selection hides difficult journeys; failed starts are not trips | Separate census inclusion from label eligibility; 23 |
| **Q10 — Censoring versus silence?** | E5–E6, E10 | Five primary outcomes; censoring requires coverage/time/non-arrival witness | Runtime silence cannot prove physical non-arrival | No zero/predicted/extraction-time labels; 23 |
| **Q11 — Feature cutoff?** | E6–E7 | `start_effective_at` reference under supported static semantics plus source/applicability/publication witnesses | Compute time, physical start and knowledge time differ; timestamps alone insufficient | Future time-dependent models need new applicability proof; 23/25 |
| **Q12 — Interventions and late reports?** | E5, E8, E10 | Original action/B0 persists; later interventions are outcome lineage; acceptance cutoff governs late reports | Replacing B0 or dropping delayed journeys biases evaluation | Frozen actions/aliases do not multiply rows; 23 |
| **Q13 — Complete synthetic source?** | E10 | Mandatory all-started trace, run scope, original route and horizon status; supported rerun is newly derived | Old completed traces omit denominator; reject fabricated ActionIds | Phase 23 foundation task, not reopening 20–22 |
| **Q14 — Availability witness?** | E6–E10 | Bind each feature to predictor knowledge and original capture/adoption evidence | Occurrence/archive access is not historical knowledge; simulator latent future is not a feature | No invented `available_at`; 23 |
| **Q15 — Historical graph semantics?** | E2, E8 | Original qualified artifacts and supported interpretation; integrity differs from semantic verification | New compilation/routing cannot repair historical provenance | Optional attributes may be unavailable without deleting action; 23 |
| **Q16 — Initial feature scope?** | E1–E2, E10 | Minimal justified allowlist; IDs/seed/policy metadata initially not features; calendar needs time semantics | Prevent memorization and unsupported meaning | Broader entity/calendar features need separate justification; 23 |
| **Q17 — Aggregates/fitting?** | E6, E8, E10 | Initially defer rolling aggregates; later enforce as-of contributions, target exclusion, label maturity and train-only fitting | Final labels and current aggregates leak; later disputes cannot backdate earlier knowledge | Fitted corrections and rolling features are distinct; 23/24 |
| **Q18 — Pipeline proof?** | E6–E10 | Restricted predictor inputs, full lineage, missingness distinction and post-cutoff noninterference | Dropping columns afterward does not prove isolation; archival loss is not predictor absence | Independent adversarial proof; no feature-store requirement; 23 |
| **Q19 — Richer generator?** | E10 | Keep deterministic/scripted mechanics; no required residual/noise; original B0 survives delays | Synthetic delay is not discovered physical dynamics | New generators require separate purpose/version/research scope; 23+ |
| **Q20 — Synthetic independence?** | E10–E11 | Hierarchical ancestry/coupling retained; distinguish replay, realization, family and mechanism claims | Seeds/names/hashes/policy variants do not establish independence | Row and group counts; verified paired exogenous comparability; 23/24 |
| **Q21 — Population generalization?** | E3, E6–E10 | Started completeness is not demand representativeness; distinct denominators and policy/missingness coverage | Unknown ingress is not zero; unchosen candidates lack observed outcomes | Coverage does not correct selection; no mandatory ingress platform; 23/24 |
| **Q22 — Policy improvement claim?** | E3, E7, E10–E11 | Estimator and simulator comparisons permitted precisely; real causal superiority unsupported | Exhaustive estimates are not exhaustive observations; deterministic zero support unresolved | Off-policy/prospective work separately designed and authorized |
| **Q23 — Pool cohorts?** | E6, E10–E11 | Separate synthetic, report-interval, physical and insufficient-evidence cohorts; shared infrastructure allowed | Seconds/schema equivalence is not target equivalence | Cohort metrics; augmentation separate; no blended physical claim; 23–25 |
| **Q24 — Dataset components?** | E5–E10 | Five logical components; canonical census, features, labels, lineage and manifest/quality | Physical layout need not mirror semantics; null/precision/reference meanings must survive | Lossless identities, resolvable evidence and reconciled partition; 23 |
| **Q25 — Split contract?** | E3, E7, E10 | Claim-specific dependency closure; chronological maturity where applicable; actual assignments retained | Random rows and renamed worlds fabricate independence; no universal entity grouping | `EvaluationUnavailable` valid when no holdout exists; 23/24 |
| **Q26 — Publication failures?** | E2, E8–E10 | Separate label ineligibility from contract failure; independently verified narrowed scope explicit | Missing endpoints are expected; corrupt census/provenance/leakage cannot be warnings | No silent difficult-source removal or arbitrary thresholds; 23 |
| **Q27 — Reproducibility/privacy?** | E8–E12 | Bounded immutable source package, code/patch bytes, semantics, splits and integrity; controlled/minimized access | Head/hash alone not archive; no fake cross-source atomicity or universal retention | Retention loss narrows claims explicitly; 23+ |
| **Q28 — Freeze evaluation?** | E10–E11 | Freeze target, eligibility, cohorts, splits, metrics and fitting before confirmation; preserve bugs/corrections as new versions | Performance-driven population changes invalidate comparison | Matched coverage plus approved full-system fallback; 23/24 |
| **Q29 — Mandatory baseline?** | E1, E5, E8, E10 | B0 original route first; fitted comparisons conditional and training-only | Listed ML techniques are examples, not substitutes; exact synthetic match legitimate | Version fitting, labels, parameters and fallback; 24 |
| **Q30 — Error metrics?** | E11 | MAE primary; label−prediction sign; secondary tails/bias/RMSE/coverage; fixed definitions | Unpaired distributions, imputed censoring and silent prediction dropout mislead | Zero supported labels valid; negative invalid; defer MAPE; 24 |
| **Q31 — Uncertainty/segments?** | E10–E12 | Respect dependencies; count groups, preserve sparse segments, predeclare confirmatory analysis | Replay rows cannot manufacture confidence intervals | Descriptive-only limits explicit; metric/prediction uncertainty distinct; 24 |
| **Q32 — Phase status?** | E1, E10–E12 | Engineering, supported scientific evaluation and next-phase authorization separate | Physical evidence absence need not block synthetic engineering; no circular ML-success gate | Unavailable never PASS; 23–25 |
| **Q33 — Research entry?** | E1, E10–E11 | Precise target, valid evidence/design, B0, plausible residual hypothesis and failure/reproducibility criteria | Research need not guarantee success; residual may be measurement/script artifact | Separate synthetic/report/physical research and operational admission; 25 |
| **Q34 — Inference granularity?** | E1–E2, E5, E10 | Whole-route experimental prediction; exact directed graph/context binding | No edge supervision; selected-start accuracy does not validate alternatives, future departures or A* | Explicit roadmap reconciliation; broader integration deferred; 25 |
| **Q35 — Reproduction guarantees?** | E3, E7–E9 | Separate decision/inference/training reproduction; retain exact effective consumed values | Approximate outputs can change ranking/feasibility; current model cannot replace old evidence | Missing artifacts narrow reproduction capabilities; 25/admission |
| **Q36 — Validity/fallback?** | E4, E7–E9 | Pin coherent identities/adoption; separate applicability/proposal freshness; initially coherent fallback reevaluation | Confidence/TTL/static proof do not establish learned validity; identity equality hides ABA | Fresh authority checks; typed failures; historical B0 unchanged; admission |
| **Q37 — Hard feasibility?** | E3–E7, E11 | Mean/quantile not physical guarantee; initial offline/shadow; separate prediction-to-constraint policy | Modeled constraints differ from physical outcomes; `max(B0, mean)` not a certificate | Preserve hard invariants and accepted terms; admission |
| **Q38 — Admission/runtime?** | E6–E7, E12 | Separate offline, shadow and operational authorization; measured resources and isolated inference | Loading/MAE/shadow success insufficient; infrastructure cannot be selected from speculation | Captured inputs outside publication, revalidation, rollback/batching tests; 25+ |
| **Q39 — Dataset adversarial proof?** | E2, E5–E11 | Evidence-backed suite, independent oracles, census conservation and noninterference | Matching final values can hide invalid provenance; bad labels differ from contract failure | Full Section 23 suite and scoped publication consequences; 23/24 |
| **Q40 — Inference failure proof?** | E6–E9 | Staged artifact/numeric/context/resource/fallback/history/publication tests | Component success does not authorize transitions; no partial/mixed substitution | Test ownership follows offline/shadow/operational capability |
| **Q41 — Final gates/verdicts?** | E1, E8–E12 | Conditional dataset readiness; ML not ready; separate implementation/publication/scientific/admission gates | Trace remediation may occur inside 23; no fabricated physical labels or circular success requirement | Accepted Section 19–21 gates; no authorization implied |
| **Q42 — Blockers versus choices?** | E1–E12 | Semantics/evidence mandatory; physical layout/tooling flexible; trace remediation in 23 | Neither premature infrastructure nor silent semantic deferral is justified | Explicit deferred scope and ordered work; 23–25 |
| **Q43 — Consolidation/confirmation?** | Entire inspected repository and accepted rounds | Full audit/register/contracts/evidence/tests/risks presented for confirmation | Future contracts must not be described as implemented; agreement is not implementation permission | This document; separate later authorization required |

**Please explicitly confirm this consolidated audit, or identify the sections requiring amendment.** No implementation or repository mutation is authorized by that confirmation.
