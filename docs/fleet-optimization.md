# Phase 18 fleet batch allocation and editable suffix optimization

[ADR 0016](adr/0016-multi-order-normative-audit.md) is normative. Phase 18 was explicitly
authorized on 2026-10-08. Phase 19 reassignment/recovery remains deferred.

## Scope and authority

A decision receives one coherent world snapshot and a current unassigned batch. It may
allocate several new orders and resequence healthy riders' editable suffixes jointly.
Committed owners, onboard custody, authentic accepted terms, and active road/destination
wait/service are fixed. A new proposal is provisional until one successful fleet commit.
No handoff, mid-leg diversion, active-stop abandonment, recovery, or workload cap.

Core remains ignorant of orders/plans. Dispatch owns `fleet.rs`, domain validation,
whole-plan timing, search, immutable evidence and publication. Simulation owns equal-time
batch formation, active identities, logical events and actual outcomes. CLI loads schema
3 and exposes readable/JSON results. No optimizer trait, distributed transaction,
external solver or speculative later-phase service.

## API

```rust,ignore
let inputs: FleetInputs = /* explicit current policy/forecast/projection inputs */;
let decision = optimize_fleet(&snapshot, &current_batch, inputs)?;
// Rebuild from CURRENT sources, never echo the proposal's old context.
let current = FleetContext::new(&current_snapshot, current_inputs)?;
if decision.proposal().is_some() {
    world.commit_fleet(&decision, &current)?;
}
```

`FleetInputs` wraps existing `PoolingInputs`, a pinned `FleetAlgorithm`, and explicit
`UnavailableExecution` facts for active prefixes whose departure cannot be predicted.
The latter retains active id, order, road arrival and destination anchor; it does not
fabricate an editable origin. `FleetContext` binds them, the clock/epoch, routing graph /
traffic / profile, all anchors, policies, forecasts, budget and optimizer identity
`fleet-greedy-local/v1`. All are compared exactly at commit alongside world identity/version.
Policy errors, unsupported migration, invalid projection or structural corruption fail
the decision. An unavailable active prefix must have a required missing forecast or an
unobserved past forecast which cannot establish readiness at the current clock.

`evaluate_batch_plan` reuses the Phase 17 physical evaluator with multiple provisional
new assignments in one plan. `evaluate_whole_plan` remains compatible. Travel is routed
at every propagated departure, pickup includes readiness wait and pickup service,
dropoff completion includes service, and capacity is checked at every stop. Existing
accepted deadline/delay protections remain mandatory and current policies can strengthen
them. New acceptance references come from the FINAL joint evaluation. Rewrite, pickup,
and delivery never reset existing references. Legacy obligations retain explicit
compatibility and cannot acquire an invented historical reference.

## Objective and horizon

Compare feasible candidates lexicographically:

1. Most **newly admitted order count**.
2. Least remaining evaluable fleet road-travel seconds.
3. Least road distance over that same horizon.
4. Canonical `(rider, semantic stop sequence)` fleet identity.

Demand is feasibility only. Existing obligations are mandatory, never scored rewards.
Customer waiting/service/completion impacts are evidence, not blended road objectives.
Each rider starts at its explicit projected post-prefix instant/anchor. Frozen road
contributions and isolated rider contributions are excluded identically from all
comparisons. Unknown isolated travel is never presented as a measured fleet total.
`baseline_objective` and `selected_objective` therefore explicitly concern the evaluable
nonisolated remainder. Planning `admitted_orders` differs from realized `delivered_orders`.

## Baseline health and input coverage

Classify every rider canonically. Operationally unavailable/profile-unsupported riders
are structural exclusions. Accepted/current hard-breached riders stay unchanged: no new
admission, resequencing, or recovery. Soft misses alone do not isolate them. Missing
required readiness or an unroutable baseline isolates that rider as unknown, preserving
ownership/plan/terms/execution. Unknown new orders are isolated independently. Optimize
healthy evaluable riders and evaluable new work with explicit incomplete INPUT coverage.

`input_complete=false` can coexist with `riders_complete=true`, `search_complete=true`
and a publishable remainder. That is intentional Phase 18 behavior, different from
Phase 17's complete-fleet-input requirement. Unsupported policy versions/missing hard
data are errors, not unknown forecasts. Even isolated committed work's policies are
validated so input isolation cannot hide invalid protection migration.

## Declared deterministic heuristic

`Greedy`: canonical order sequence, exhaustive pickup/dropoff insertion into all healthy
plans, choosing best fleet objective at each addition. This is the comparison baseline.

`LocalSearch`: canonical greedy, then strict best-improvement rounds.

`MultiStartLocal`: greedy for every cyclic rotation of canonical order identities,
select the best completed construction, then the same local search. Cyclic starts do
not enumerate arbitrary order permutations and do not claim exhaustive VRP search.

The complete local neighborhood contains:

- Insert each unplaced request into every rider/position pair.
- Relocate each editable stop within its fixed owner's plan.
- Remove/reinsert each proposed new pickup/dropoff pair across all healthy riders.
- Eject one proposed new request, insert one unplaced request (including the replacement
  candidate), then reinsert the displaced request into every rider/position pair.

Frozen stops are never generated as editable positions. Committed pair ownership is
never moved. Single-stop relocations that put a pending dropoff before its pickup are
excluded by the structural precedence proof and counted separately. No route/objective
pruning. Every complete fleet submission consumes one unit even if rejected early;
per-rider physical evaluation stops at a rejection. Intermediate ejection feasibility
is not used to prune final joint plans, because changed departure times can change
feasibility. Duplicate submissions in different declared starts/neighborhoods are counted.

A strict objective improvement is required, including canonical ties, so finite-state
search cannot cycle. Stop after a fully covered round without improvement. Budgets
are deterministic evaluator-work limits, not host clocks. Exhaustion at any stage returns
`SearchIncomplete`, no proposal and no commit even if a feasible incumbent was found.
`ConstructionComplete` covers greedy's declared work; `LocalOptimum` covers the declared
local neighborhood only. An unchanged converged result has no proposal and does not
increment world version. Zero admissions never proves globally infeasible arbitrary VRP.

## Atomic publication and execution

`FleetDecision`/`FleetProposal` have private construction and expose read-only getters.
Evidence pins all prior plans, ownership and accepted terms. A successful proposal
contains new assignments/terms, all healthy replacement plans, exact timing and objective.
`commit_fleet` checks exact world/context, clones the entire next state, installs all
new assignments/terms/plans, validates it, then publishes one version increment. Any
stale input or invalid replacement rejects the entire fleet with zero mutation. No
partial salvage or acceptance leaks for isolated/unplaced work.

Schema 3 selects `fleet_batch` with explicit work budget, forecast duration and algorithm.
Equal-time external creation/ready events are processed before the queued fleet attempt;
one queued attempt sees the current pending batch. New traffic/readiness/completion
observations may queue a fresh attempt for pending work. Existing schema 1/2 behavior is
preserved. Simulation queues only started actions and retains their execution ids across
fleet rewrites. After active completion, the next stop comes from CURRENT effective
plans. It retains per-order release and union rider occupancy from Phase 17. Actual
readiness/traffic errors and realized protections remain separate from admission.

## Evidence, oracle and limits

`FleetEvidence` version 1 separates input/rider/search coverage, canonical batch,
isolated orders/riders, baseline/chosen per-rider timing/load, accepted/current/candidate
completion and raw signed marginal/cumulative effects, expected ownership/plans/terms,
objective boundary, construction/neighborhood work, precedence exclusions, rejection
counts, completed starts/rounds and qualified termination. Schema 3 results also record
publication/version, final state and actual execution. Chosen timing is reproducible;
full all-candidate route traces are not retained.

The independent tiny oracle enumerates ownership/unplaced Cartesian products and full
precedence-valid stop permutations, and independently compares the objective. It shares
only authoritative physical evaluation. Tests cover the greedy allocation trap, 16
seeded two-order exact comparisons, 24 three-order seeds × three algorithms, and an
explicit four-onboard-stop local minimum: 32s versus an oracle's 30s. This measured gap
is retained rather than described as global optimality. Tests additionally cover stale
contexts, immutable cumulative references, frozen execution, fixed custody, zero leaks,
input isolation, one-version publication and 24 seeded actual-outcome replays.

Large-batch search can exhaust its budget and intentionally withhold publication.
Multi-start and ejection work is expensive; caching, spatial shortlisting, parallel
search, broader neighborhoods and robust uncertainty policies are deferred. No
production-scale latency threshold is claimed. Phase 19 committed reassignment/recovery
is out of scope.

See [fixtures](../data/fixtures/phase-18/README.md) and [completion evidence](phase-18-completion.md).

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.99.0 test --workspace --all-features --locked
cargo +1.99.0 test -p roadrunner-dispatch --test fleet --locked -- --nocapture > target/phase18-oracle.log
cargo +1.99.0 build -p roadrunner-cli --release --locked
PYTHONDONTWRITEBYTECODE=1 python3 scripts/collect_fleet_benchmark.py \
  --oracle-log target/phase18-oracle.log \
  --output benchmarks/results/2026-10-08-phase-18-fleet.json
```

The collector verifies bytes and publication/isolation coverage for every scenario ×
algorithm, records source/configured dataset hashes, graph sizes, binary/compiler/build,
hardware, raw/median/p95/p99 samples and deterministic work. Memory is explicitly
unmeasured. Process timing includes launch, loading, simulation and serialization.
