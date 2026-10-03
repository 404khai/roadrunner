# Pre-Phase-13 dispatch remediation report

Date: 2026-10-03
Status: PRE-PHASE-13 DISPATCH ENTRY GATE: PASS

## Summary and supported behavior

The accepted Q1–Q31 decisions and Pre-Phase-13 Dispatch Architecture Audit now govern
the normative specification, glossary, architecture, and implementation. The new
`roadrunner-dispatch` library implements the entry-gate contracts and their narrow
Basic Dispatch reference workflow:

```text
DispatchSnapshot -> world validation -> Basic Dispatch eligibility
 -> candidate generation -> candidate logical plan -> feasibility
 -> plan evaluation -> pure baseline ranking -> immutable AssignmentDecision
 -> explicit all-or-nothing World::commit
```

Basic Dispatch itself is implemented as a library operation: one new, unassigned,
awaiting-pickup order, one coherently idle available rider, and an evaluated
`Pickup(A) -> Dropoff(A)` proposal. Its objective is exactly pickup road travel plus
delivery road travel, with soft-observed deadlines and zero stop wait/service.
No subsequent roadmap phase was started.

## Normative documents and ADRs

Amended:

- [spec-v1.md](spec-v1.md): request/state ownership, logical plans, supported scalar
  capacity/profile constraints, routing anchors, time, candidate scope, exact proposals,
  commit, and the already-implemented deterministic FIFO routing scope.
- [glossary.md](glossary.md): assignment versus decision, request versus fulfillment,
  profile versus state, remaining work, capacity units, time, anchors, and coverage.
- [architecture.md](architecture.md): actual crate dependency, snapshot/evaluation/commit
  pipeline, pinned routing, shared transitions, and deferred evolution boundaries.
- [rider-spatial-index.md](rider-spatial-index.md): dispatch ownership, eligibility
  projection, incomplete shortlist guarantee, and migrated benchmark target.

Added accepted ADRs, following existing numbering and style:

- [0011 — Dispatch Domain and Logical Plans](adr/0011-dispatch-domain-and-logical-plans.md)
- [0012 — Dispatch Evaluation, Decision Evidence, and Commit](adr/0012-dispatch-evaluation-decision-evidence-and-commit.md)
- [0013 — Simulation and Planning Evolution](adr/0013-simulation-and-planning-evolution.md)

Single-order limits are eligibility and proposal policy, not domain cardinality limits.
Future simulation comparisons use fresh equivalent worlds, fixed exogenous inputs,
and observed execution metrics. Whole-plan insertion deltas and custody constraints
are documented; their algorithms remain deferred.

## Ownership and migration

`roadrunner-dispatch -> roadrunner-core` is the only new dependency direction.
Core exports no rider, order, plan, strategy, or fulfillment types.

The Phase 12 implementation, integration tests, and Criterion benchmark moved to
`crates/roadrunner-dispatch`. Canonical `RiderId` is defined once in dispatch;
`dispatch::spatial` re-exports that same type. There is no core compatibility identity.
`RiderLocation`, `RiderCandidate`, lookup implementations, radius/limit semantics,
ordering, and ties remain unchanged. The spatial tree remains rider-specific.

The migration exposed the core Earth-radius constant as a documented geographic
primitive. Checked `Seconds::new` replaces the old private unchecked ETA constructor;
pruning accepts the existing validated numeric radius threshold. The unused private
Seconds constructor was removed. Geographic calculations remain core-owned.

## Domain and transition contracts

[domain.rs](../crates/roadrunner-dispatch/src/domain.rs) defines opaque OrderId,
DecisionId, WorldVersion, and CapacityUnits, plus Order, OrderReadiness,
FulfillmentState, RiderProfile, RiderState, Availability, CommittedAssignment,
Stop, RiderPlan, and WorldData.

One capacity unit is an abstract normalized reference-parcel slot. All callers use
consistent demand/capacity normalization; units are neither kilograms nor active-order
count. Onboard load derives from custody. State-aware validation checks initial load
and every pickup/dropoff change, including multi-stop plans without implementing insertion.

Awaiting-pickup assigned work requires exactly one pending pickup before exactly one
dropoff. Picked-up work requires only its dropoff with the same responsible custodian.
Delivered work has neither active responsibility nor remaining stops. Completed pickup
and dropoff instants remain in fulfillment history. Key mismatches, orphan stops,
missing responsibility, contradictory custody, and invalid load are InvalidWorldState.

Order stores only supported request facts. Readiness estimates and actual observations
are separate. Forecasts and observations can precede request creation for already-ready
stock; actual pickup requires recorded readiness no later than pickup completion.
A deadline already expired at creation remains soft-observed. Priority, specialized
cargo, and nonzero service constraints are outside this initial request contract.

[world.rs](../crates/roadrunner-dispatch/src/world.rs) owns validated assignment commit,
rider state updates, readiness estimates/observations, pickup, and delivery. These
operations publish a validated replacement world and advance its version only after
all checks pass. Simulation later invokes these operations rather than duplicating rules.

## Snapshot, routing, time, and timeline

DispatchSnapshot borrows an immutable validated World, caller-supplied DispatchInstant,
RoutingEpoch, RouteProvider, and RoutingAnchors. No live stores, internal clocks, or
simulation dependency are present. Snapshot validation rejects future observed facts
and checks every supplied anchor's coordinate binding, graph digest, and node existence.
Missing anchors required for evaluation are typed errors, even when no road route
could be attempted.

[time.rs](../crates/roadrunner-dispatch/src/time.rs) distinguishes DispatchInstant
from core Seconds durations. RoutingEpoch centrally converts explicit instants to
scenario departure seconds. Reversed epoch conversion, non-finite values, overflow,
and a positive duration lost to floating-point precision are rejected without clamping.

[routing.rs](../crates/roadrunner-dispatch/src/routing.rs) defines the pinned graph,
TrafficIdentity, RoutingProvenance, and node-backed location projections. CoreRouteProvider
uses Roadrunner Dijkstra with free-flow, static traffic, or FIFO time-dependent traffic.
The initial adapter uses conservative default access permissions. Each leg receives
its propagated departure; returned graph/traffic/profile, departure, endpoints, and
travel-time objective are checked. Valid NoRoute is candidate infeasibility; engine
failure is DispatchEvaluationError::Routing. No snapping, matrix routing, or cache exists.

PlanEvaluation separates road movement, distance, pickup arrival/departure, dropoff
arrival, completed delivery, stop waiting/service, and lateness. StopTimeline implements
arrival -> wait -> service -> departure with checked arithmetic. Basic Dispatch supplies
zero wait/service, then routes onward from pickup departure. Deadline lateness uses
completed dropoff and affects neither feasibility nor the baseline objective.

## Candidates, feasibility, and strategy

CandidatePolicy::Exhaustive evaluates every Basic Dispatch eligible rider and records
Complete coverage. CandidatePolicy::Spatial builds the migrated immutable index from
the same world's authoritative eligible coordinates and records radius/limit plus
PotentiallyIncomplete coverage. Screening ETA never enters ranking.

Eligibility requires operational availability, no active committed responsibility,
no onboard custody, and an empty effective remaining plan. Coherent busy or unavailable
riders are ineligible; contradictory work/custody records invalidate the world.

CandidateRejection includes NoRoute, CapacityExceeded, InvalidPlan, CustodyViolation,
and UnsupportedCandidateProfile. Invalid request/world/anchors, provenance mismatch,
routing failure, invalid arithmetic, and generation errors fail decision evaluation.
There are no infinite scores or huge penalties for infeasibility.

BaselineStrategy receives feasible PlanEvaluation values only and has no routing,
spatial, clock, mutation, or commit dependency. Its structured ScoreContributions are
pickup travel, delivery travel, and their checked finite sum. Ranking uses exact score
then lower RiderId; evidence is independently canonicalized by RiderId. No epsilon or
randomness is used. No universal optimizer or fleet-planning interface was introduced.

## Decisions and commit

[decision.rs](../crates/roadrunner-dispatch/src/decision.rs) exposes immutable,
serializable AssignmentDecision through read-only accessors. Each completed decision
contains DecisionEvidence and an Assigned or Unassigned outcome. Errors use a separate
`Result<AssignmentDecision, DispatchEvaluationError>`.

Evidence records decision identity, world identity/version, explicit evaluation instant
and routing epoch, graph/traffic/profile, versioned strategy/eligibility/feasibility
identities, SoftObserved deadline policy, candidate configuration/coverage, canonical
candidate metrics and score contributions or typed rejections, and the selection/tie
reason. Assigned contains the exact expected/proposed responsibility and RiderPlan.
Unassigned distinguishes EligibleFleet from EvaluatedCandidates. Full hypothetical
route geometry is omitted; explanations render from structured evidence.

World::commit checks world identity/version, exact expected responsibility/plan,
profile compatibility, idle eligibility, transition shape, and resulting whole-world
invariants. It publishes responsibility and plan together or rejects without changing
any record/version. Version exhaustion also leaves all state unchanged. Commit does
not route, rescore, select another rider, or silently refresh stale decisions.

## Correctness fixtures and validation

[entry_gate.rs](../crates/roadrunner-dispatch/tests/entry_gate.rs) has 16 deterministic
integration fixtures covering exhaustive versus spatial scope, nearest versus routed
best, missing/wrong-graph/absent-node/wrong-coordinate anchors, NoRoute versus routing
failure, all provenance components and departure, scalar capacity, unsupported profile,
soft-observed deadlines (including already-expired deadlines), propagated departures,
FIFO traffic and nonzero epoch, exact ties/permutations and sub-epsilon real differences,
non-finite accumulation, contradictory world state, stale/repeated/wrong-world commit,
structured unassigned outcomes, shared custody transitions, general remaining-plan
validation, and checked time/stop arithmetic.

Two unit regressions cover wrong expected prior state, malformed proposals, capacity
rechecking at commit, and version overflow without partial mutation. Five rider lookup
tests preserve the existing Phase 12 suite and add a frozen migration regression for
ties, radius inclusion, limits, and exact zero-distance screening ETA.

Final validation succeeded:

| Command | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace` | PASS — 96 Rust tests, including 23 dispatch/index tests; doc tests pass |
| `python3 -m unittest discover -s benchmarks/reference-comparison -p 'test_*.py'` | PASS — 3 harness tests |
| `cargo bench -p roadrunner-dispatch --bench rider_lookup --locked -- --sample-size 30 --measurement-time 1` | PASS — all eight cases and equality assertions |
| `python3 scripts/collect_rider_lookup_benchmark.py --output benchmarks/results/2026-10-03-dispatch-index-migration.json --concurrent-validation` | PASS — measured sample artifact created |
| `git diff --check` | PASS |

No test or lint setting was weakened. The existing test-only exact float comparison
allowance is also used by the dispatch fixtures because exact ordering is normative.

## Benchmark migration status

The Criterion target is now `roadrunner-dispatch --bench rider_lookup`. The seed,
100/1K/10K/100K populations, location distribution, query, 5 km inclusive radius,
10-rider limit, construction exclusion, 30 samples, and equality checks are preserved.
All eight migrated benchmark cases ran successfully. Historical Phase 12 results and
their original command remain untouched.

The [migration run artifact](../benchmarks/results/2026-10-03-dispatch-index-migration.json)
records hardware, OS, dataset/configuration, algorithms, sample counts, median, p95,
and p99, collected by [the sample collector](../scripts/collect_rider_lookup_benchmark.py).
This verification run overlapped workspace validation, and its timings are substantially
higher than historical measurements. It is not a controlled before/after performance
comparison; no speedup or performance-equivalence claim follows from this migration.

## Entry gate

| # | Requirement | Result | Repository evidence |
| ---: | --- | --- | --- |
| 1 | Normative docs aligned | PASS | Four amended normative documents |
| 2 | ADRs accepted/added | PASS | Accepted ADRs 0011–0013 |
| 3 | roadrunner-dispatch created | PASS | Workspace member and dispatch Cargo.toml |
| 4 | Rider index migrated | PASS | dispatch spatial module, tests, benchmark |
| 5 | RiderId canonical ownership moved | PASS | dispatch identity.rs; same-type spatial re-export |
| 6 | Domain/request/state separation implemented | PASS | domain.rs and versioned World |
| 7 | RiderPlan implemented | PASS | General Vec<Stop>, state-aware validator |
| 8 | DispatchSnapshot implemented | PASS | Immutable borrowed world and pinned inputs |
| 9 | Typed time implemented | PASS | DispatchInstant, Seconds durations, RoutingEpoch |
| 10 | Routing anchors implemented | PASS | Bound coordinates/digest/node validation |
| 11 | Pinned routing boundary implemented | PASS | RouteProvider and CoreRouteProvider |
| 12 | Candidate-generation coverage implemented | PASS | Exhaustive/Spatial and coverage evidence |
| 13 | Feasibility/error distinction implemented | PASS | CandidateRejection versus DispatchEvaluationError |
| 14 | Baseline strategy implemented | PASS | Pure BaselineStrategy and score contributions |
| 15 | Deterministic tie-breaking implemented | PASS | Exact score/RiderId and canonical reporting |
| 16 | Assigned / Unassigned / EvaluationError contract implemented | PASS | Immutable completed decision versus typed Result error |
| 17 | Structured evidence implemented | PASS | Serializable DecisionEvidence plus exact outcome |
| 18 | Stale all-or-nothing commit implemented | PASS | World::commit and atomic publish boundary |
| 19 | Required correctness fixtures pass | PASS | 16 entry fixtures, 2 transition regressions, 5 index tests |
| 20 | Workspace fmt/clippy/tests pass | PASS | Final workspace commands above |

## Stop boundary

Coordinate snapping, batch/matrix routing, cross-decision caching, multidimensional
cargo, preparation-aware waiting/scoring, simulation queue, multi-order insertion,
VRP/FleetPlan, universal optimizers, PlanId/PlanVersion, acceptance, handoffs, churn
thresholds, persistence, distributed transactions, Kafka, HTTP, ML, and UI remain deferred.

**PRE-PHASE-13 DISPATCH ENTRY GATE: PASS**

Basic Dispatch itself is already implemented in the dispatch library with the exact
narrow behavior above. This report closes the remediation; it does not authorize or
claim completion of preparation-aware dispatch or any later roadmap phase.
