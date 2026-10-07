# Phase 17 completion report

Date: 2026-10-07
Status: **COMPLETE — Multiple Orders per Rider**

Phase 18 has not begun. This report is the stopping point; batch allocation, editable
suffix resequencing and fleet optimization require explicit user confirmation.

## Implemented scope and architecture

The confirmed audit is persisted as normative [ADR 0016](adr/0016-multi-order-normative-audit.md),
including all deferred Phase 18 contracts. Implementation follows one new unassigned
order → every eligible rider → explicit frozen/post-prefix projection → all declared
order-preserving pairs → whole-plan road/readiness/service/load evaluation → hard
feasibility and accepted protection → signed incremental road objective → complete-search
proposal → atomic assignment/plan/terms → committed-plan-driven execution.

Dispatch owns authoritative timing, validation, service protections, exhaustive search,
evidence and atomic transitions. Core routing/geography/cost types are reused unchanged.
Simulation supplies logical clock, forecasts and actual observations, projects its
active execution explicitly, and invokes shared World pickup/delivery transitions.
CLI loads versioned scenarios and displays readable/JSON evidence. No optimizer framework,
external routing algorithm, ownership transfer, handoff or Phase 19 abstraction was added.

New types/APIs: `OrderPolicy`, `AdmissionDeadline`, `ReadinessRule`, `ReadinessForecast`,
`AcceptedTerms`/`AcceptedReadiness`, `PredictionIdentity`, `FrozenPrefix`,
`ExecutionProjection`, `PoolingInputs`/`PoolingContext`, `WholePlanEvaluation`,
`CompletionImpact`, `InsertionEvidence`/`InsertionDecision`/`InsertionProposal`,
`InsertionTermination`, `InsertionRejection`, `PoolingError`, `project_execution`,
`effective_readiness`, `evaluate_whole_plan`, `insert_order`, `World::commit_insertion`.
`WorldData` adds authoritative acceptance records. Existing `RiderPlan`, custody,
load/plan validation, routing providers/anchors and shared transitions are reused.

Simulation adds schema 2 named scenarios, explicit per-order policies, MultiOrder dispatch,
active execution identities, insertion/publication records, typed prediction-failure
records and realized protection outcomes. Only already-started road/destination actions
are queued; no editable future stop or vector cursor is queued. Existing Phase 13–16
APIs and schema 1 fixtures preserve historical declared semantics.

[Architecture/API details](multi-order.md), [API index](api.md),
[dispatch documentation](dispatch.md), [simulation documentation](simulation.md).

## Coverage, feasibility and publication semantics

Every operationally available rider with the pinned supported routing profile is covered;
other riders have explicit structural exclusions. Baseline health/input evaluation occurs
before placement budgets. An accepted hard baseline breach blocks admission on that rider
without changing its commitments. Unknown required readiness is an evaluation failure,
never silently omitted and never called infeasible or healthy.

For suffix n, all (n+1)(n+2)/2 pairs preserve existing relative order. One submission to
whole-plan evaluation consumes one work unit, even if rejected early. Baseline-hard-breach
policy exclusions have explicit reasons/counts; there is no objective/route pruning.
Evidence separates input, rider and placement coverage. Exhaustion returns SearchIncomplete
without a proposal even after finding feasible placements. Complete sufficient search
returns best feasible insertion or eligible-fleet NoFeasibleInsertion. Optimality is scoped
to the declared insertion neighborhood, not arbitrary resequencing.

Feasible candidates compare incremental remaining road seconds, road meters and canonical
rider/semantic stop sequence. Frozen road contribution is excluded identically from both
baseline and candidate. Customer waiting/service effects, raw marginal/cumulative signed
deltas, consumed delay and slack are explicit metrics. Time-dependent routing supports
negative signed road deltas. All accepted/current hard protections apply independently of
objective; stricter current protection cannot weaken immutable acceptance terms.

World identity/version and complete external context equality bind clock/epoch, graph,
traffic, routing profile, forecasts/readiness source, policy/config identities, anchors,
active execution and post-prefix projection. Source/prior plan/assignment mismatch rejects
stale with zero semantic mutation. Successful publication validates a cloned replacement
and atomically publishes assignment + plan + accepted terms + new world version.

Missing predictions produce typed blocked admission records with incomplete input/rider/
search coverage, zero placement work and no commit in simulation. Existing execution
continues; a later observation may allow a fresh complete admission. Structural corruption,
unsupported policy and impossible contexts remain explicit errors. No readiness forecast
is ever treated as observed readiness or used to authorize actual pickup.

## Tests, oracle and scenario results

**159 workspace tests pass**, including existing regression suites and doc tests.

The dispatch fixture suite independently hand-calculates all intermediate travel,
arrival, readiness wait, service, departure, load and completion. It checks immutable
acceptance, hard deadlines, cumulative bounds, marginal/cumulative/slack/remaining-allowance
metrics, zero consumption after improvements (no banking), repeated insertion against
original references, prefix overload, custody/stop corruption, missing/stale inputs,
unsupported versions/migration, frozen pickup/dropoff projections, per-order release,
exact stale world/plan/traffic/readiness/policy/clock/execution context and rejected state
nonmutation. A FIFO traffic fixture demonstrates propagated departures with **−0.4s**
incremental road travel; the hand calculation agrees with the evaluator.

The independent tiny oracle fills two new slots in a final stop vector, preserving other
slots from the old suffix. It shares only the authoritative evaluator, not production
candidate generation, exclusions/pruning or ranking. **32 deterministic seeds × three
admissions = 96 oracle comparisons**, with no rider, plan, objective or work-count mismatch.
Service/demand vary; accepted terms and validated ownership/load persist across insertions.
Equivalent reversed map construction yields identical semantic decisions and evidence.

Simulation tests cover active travel, readiness waiting and pickup service survival,
current next-stop execution, stale execution action nonmutation, independent colocated
stops, per-order release and one union busy interval, predicted-valid actual deadline
misses, unknown forecasts that preserve committed execution, and mixed hard/soft policies.
**24 seeded actual-readiness cases**, including fractional service times and reversed
external order/rider construction, replay identically.

All eleven [CLI fixtures](../data/fixtures/phase-17/README.md) execute and replay exactly:

| Scenario | Observed result |
| --- | --- |
| successful-pooling | A assigned at 0s, B inserted at 1s during active A travel; pickups 27/29s; deliveries 42.007557/45.007557s; one busy interval 45.007557s |
| frozen-wait / frozen-service | B admitted during A wait/service; A pickup remains at 27s |
| capacity-prefix | Overloaded placements rejected; sequential capacity-feasible placement admits and delivers both |
| complete-infeasibility | B exceeds capacity alone; complete rejection, only A delivered, no B terms/assignment leak |
| hard-deadline | B's impossible hard deadline rejected; only A delivered |
| cumulative-repeat | Three orders delivered through feasible placements; original A reference preserved and delayed placements rejected |
| unavailable-forecast | Two typed input-failure attempts, no publication; later observation enables fresh admission and both deliver |
| budget-exhaustion | Incomplete attempts with feasible candidates have no publication; later complete idle-horizon attempts can admit |
| baseline-predicted-breach | Six placement pairs excluded by accepted hard baseline breach; existing execution preserved; later healthy admission completes B |
| realized-readiness-violation | Hard admission prediction is valid; late actual readiness causes a separately recorded realized hard deadline miss |

Custody corruption is a direct domain fixture rather than a fresh-world scenario: runtime
scenario construction deliberately never accepts corrupted initial responsibility.

## Measured benchmark evidence

[External benchmark artifact](../benchmarks/results/2026-10-07-phase-17-multi-order.json)
links all eleven [semantic replays](../benchmarks/results/phase-17-replays/successful-pooling.json).
Each dataset has one excluded warmup and **11 measured byte-identical runs**. The collector
independently checks pair formula, work counts, complete coverage and no incomplete commit.
Measurements ran after validation/build, without concurrent workspace validation.

Hardware: Apple M3, arm64, 16 GiB RAM, macOS 27.0. Compiler: rustc 1.99.0
(b940084d7, 2026-09-28). Release binary/configuration, binary hash, scenario/dataset hashes,
graph digest, raw samples and run count are recorded. Each graph has two nodes/two directed
edges and one rider; scenarios have one to three orders. Memory is explicitly unmeasured.
Timing includes process launch, graph loading/validation, simulation, serialization and
output capture, with linearly interpolated sample percentiles. No scalability claim or
unmeasured latency/quality gate is asserted.

| Scenario | Candidate work | Policy-excluded pairs | Median ms | p95 ms | p99 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| baseline-predicted-breach | 2 | 6 | 4.554 | 8.415 | 10.387 |
| budget-exhaustion | 4 | 0 | 3.142 | 3.499 | 3.539 |
| capacity-prefix | 4 | 0 | 2.994 | 3.471 | 3.482 |
| complete-infeasibility | 8 | 0 | 2.906 | 3.373 | 3.404 |
| cumulative-repeat | 14 | 0 | 3.259 | 3.632 | 3.681 |
| frozen-service | 4 | 0 | 2.826 | 3.185 | 3.197 |
| frozen-wait | 4 | 0 | 3.080 | 3.421 | 3.522 |
| hard-deadline | 8 | 0 | 2.634 | 2.923 | 2.944 |
| realized-readiness-violation | 1 | 0 | 2.709 | 3.319 | 3.473 |
| successful-pooling | 4 | 0 | 2.643 | 3.361 | 3.583 |
| unavailable-forecast | 4 | 0 | 2.843 | 3.522 | 3.563 |

Reproduction commands (all executed successfully):

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.99.0 test --workspace --all-features --locked
cargo +1.99.0 build -p roadrunner-cli --release --locked
python3 scripts/collect_multi_order_benchmark.py --output benchmarks/results/2026-10-07-phase-17-multi-order.json
```

`git diff --check` and documentation-link validation also pass.

## Completion gate — every item

| Gate | Result | Evidence |
| --- | --- | --- |
| Whole-plan timing | PASS | Every editable leg/stop and all existing completion predictions, hand calculations + FIFO test |
| Readiness + service propagation | PASS | Explicit forecast validity/fallback, observed precedence, propagated departures and independent colocated service |
| Capacity after every stop | PASS | Shared full-prefix validation and post-stop load timelines; prefix overload fixture |
| Custody / fulfillment | PASS | Fixed owners, exact remaining multiplicity/precedence, picked-up dropoff-only plans and corruption tests |
| Acceptance terms | PASS | Successful commit only; authentic policy/reference/readiness provenance retained through rewrites and execution |
| Cumulative bound | PASS | Independent accepted/current hard bounds, repeated insertion original reference, improvements never bank allowance |
| Marginal / cumulative evidence | PASS | Raw signed deltas, consumption, current/accepted deadline slack and remaining cumulative allowance |
| All eligible riders | PASS | Canonical full fleet pass, explicit unavailable/profile exclusions; unevaluable eligible input prevents publication |
| Exhaustive declared placements | PASS | Pair formula/statistics and independent final-slot enumeration agree |
| NoFeasibleInsertion / prediction failure / incomplete semantics | PASS | Separate typed outcomes/coverage; incomplete feasible search cannot publish; prediction failures preserve execution |
| Exact context stale checking | PASS | World/plan/version/clock/epoch/graph/traffic/readiness/policy/projection/execution mismatches reject unchanged |
| Atomic commit | PASS | Assignment + replacement plan + accepted terms published together after cloned validation; stale/rejected states unchanged |
| Frozen preservation | PASS | Travel/wait/service and frozen pickup/dropoff survive plan replacement |
| Generation invalidation | PASS | No editable future events are queued; conditional generation requirement is vacuous. Active identities are separate, and obsolete identity actions cannot mutate state |
| Next-stop execution | PASS | Each completed active stop reads the current effective plan's first stop, without an old vector cursor |
| Per-order release | PASS | Dropoff removes only its order's assignment/custody/load and leaves other planned work |
| Multi-order occupancy | PASS | One union responsibility interval; successful CLI run 45.007557s, not summed overlapping order intervals |
| Independent tiny insertion oracle | PASS | 96 independently generated/ranked admission comparisons, zero mismatch |
| Hand-worked evaluator fixtures | PASS | Complete timing/load/service/completion, cumulative/slack and signed FIFO-road examples |
| Generated invariants | PASS | 32 demand/service seeds and 24 actual-readiness seeds; immutable terms, owner/load/plan and canonical construction |
| Deterministic replay | PASS | Seeded/reversed-input equivalence, CLI tests and 11×11 externally byte-identical measurements |
| Successful / rejected pooled CLI examples | PASS | Eleven runnable scenarios, readable coverage output and structured prediction/publication/actual evidence |
| Measured benchmark evidence | PASS | Hardware/build/hash/dataset/populations/raw/median/p95/p99/work; memory marked unmeasured |
| Updated architecture / API docs | PASS | Normative ADR, architecture, dispatch, simulation, API index, multi-order contract and fixture guide |

## Known limitations and deferrals

- Exhaustive insertion can be expensive: no shortlist, heuristic truncation, parallel
  search or arbitrary suffix resequencing. Budget exhaustion intentionally cannot publish.
- Baseline/input evaluations precede placement work budgeting. A failure is explicit;
  the work budget does not cap baseline routing. No wall-clock deadline is used.
- Simulation initially loads fresh riders; direct dispatch APIs/tests cover picked-up and
  active/frozen projected states. Importing initially busy runtime scenario state is deferred.
- Only active stop actions are queued. There is no editable queued event needing a plan
  generation. If future work is prequeued later, it must carry a plan generation/revision.
- Readiness fallback is a declared stock-ready-at-creation assumption, not learned readiness.
  No unconfigured service/detour defaults, robustness margins or fulfillment guarantee.
- Hard admission protects predictions under the pinned model. Actual readiness/traffic
  may produce realized misses; execution/terms are preserved and violations remain observable.
- Routing still uses caller-supplied node projections and existing supported core routing
  semantics. No live traffic/position sources, coordinate snapping, UI, persistence or HTTP.
- Only synthetic two-node CLI measurement datasets and tiny correctness oracles were measured.
  Memory/scaling and large fleet claims are deferred rather than fabricated.
- Phase 18 batch/fleet allocation, suffix resequencing, local search/VRP and its quality
  oracle are **deferred**. Phase 19 committed reassignment/recovery is also deferred.

**STOP: await user confirmation of Phase 17 before beginning Phase 18.**
