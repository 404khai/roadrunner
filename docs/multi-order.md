# Phase 17: exhaustive multi-order admission

The normative contract is [ADR 0016](adr/0016-multi-order-normative-audit.md).
Phase 17 admits exactly one unassigned order per invocation. Committed owners remain
fixed. Phase 18 batch allocation/resequencing and Phase 19 recovery are deferred.

## Module and API contract

Core remains unaware of orders, custody and protections. Dispatch adds `pooling.rs`
using existing `World`, `WorldData`, `RiderPlan`, `Stop`, `DispatchSnapshot`, routing
anchors/provider and prefix-capacity validation. No optimizer trait or external solver.

```rust,ignore
let inputs: PoolingInputs = /* explicit policies, valid forecasts and projections */;
let decision = insert_order(&snapshot, new_order, inputs)?;
// Rebuild a context from CURRENT clock/provider/anchors/inputs before publication.
let current = PoolingContext::new(&current_snapshot, current_inputs)?;
world.commit_insertion(&decision, &current)?;
```

`project_execution` validates caller-owned frozen execution and produces explicit
post-prefix time, anchor, custody, load and suffix. `evaluate_whole_plan` is the shared
physical evaluator. A proposed new assignment must be unassigned and awaiting pickup.
It validates exact stop multiplicity, fixed custody and load at every prefix. Invalid
structural input is an error; scalar capacity excess and NoRoute are typed rejections.
Every required stop policy/readiness input is validated before ordinary route rejection.

Pickup arrival → readiness wait → pickup service → departure. Dropoff arrival → dropoff
service → completion. Each road leg uses the previous departure and pinned core routing
provenance; colocated pickups/dropoffs still perform distinct services and transitions.
`WholePlanEvaluation` records editable stops, road seconds/distance, post-stop load,
readiness sources and all dropoff predictions (including a frozen dropoff).

## Policy and acceptance

Every new admission requires explicit `OrderPolicy`: nonempty id, supported version 1,
`SoftObserved` or `Hard` deadline, optional typed cumulative completion delay (None is
valid), pickup/dropoff service, and readiness semantics. Hard requires deadline data.
Zero service is supported. New commitments cannot use `LegacyV1` readiness.

`ValidForecastV1` requires observed readiness or matching `ReadinessForecast` with
nonempty id, generated_at ≤ evaluation_at ≤ valid_until. Forecast expected_at matches
the coherent world's expectation. Validity is checked at evaluation, not at a future
stop. A past predicted-ready timestamp is still a prediction. `CreatedAtFallbackV1`
explicitly assumes stock ready at creation if a valid forecast is unavailable. Legacy
Phase 13–16 expectations are named `legacy-phase13-16/v1`, remain soft and zero-service,
and never acquire fabricated acceptance history.

`WorldData.accepted` stores `AcceptedTerms` only on successful `commit_insertion`:
policy, accepted_at, authentic completed-dropoff reference, pinned deadline, prediction/
routing/service/optimizer identities and exact readiness source/forecast validity inputs.
Existing terms survive every plan rewrite, pickup and delivery. Accepted and current hard
deadlines both apply. The tighter current/accepted cumulative allowance applies. Existing
legacy work cannot enable cumulative protection without an authentic reference.

Consumption = max(candidate completion − accepted reference, 0). Improvements do not
bank allowance. `CompletionImpact` preserves signed marginal and cumulative deltas,
consumption, current and accepted deadline slack, and remaining cumulative allowance.
These customer effects are not blended into the initial road objective. A baseline hard
breach blocks new admission on that rider and records its unchanged obligation. Soft
misses do not block; unknown readiness fails evaluation rather than claiming health.

## Search and publication evidence

All operationally available riders with the pinned supported profile are considered.
Unavailable/profile exclusions are explicit. For editable suffix n, enumerate
(n+1)(n+2)/2 pickup/dropoff pairs, preserving existing suffix relative order. A frozen
first stop stays first. The restriction belongs to insertion, not general plan validity.

Compare feasible candidates by signed whole-plan incremental road seconds, signed road
meters, rider identity and semantic resulting stop sequence. Frozen road accounting is
excluded from baseline and candidate identically. Time-dependent travel deltas can be
negative because stop service changes subsequent departures.

Baseline evaluations establish complete input/rider health before budgeted placements.
Each evaluator submission consumes one deterministic work unit, including early rejection.
Baseline-hard-breached riders exclude all placements with the explicit policy proof:
new admission is forbidden regardless of objective. There is no route/objective pruning.
The complete decision reports per-rider suffix size, pairs, evaluated/excluded/feasible
counts and counted rejections. `input_complete`, `riders_complete`, `search_complete`
are separate. Budget exhaustion returns `SearchIncomplete` without a proposal. Complete
search returns `BestInsertion` or eligible-fleet `NoFeasibleInsertion`. Missing forecasts,
unsupported policies, inconsistent projections or failed routing evaluation are errors,
not completed infeasibility. No arbitrary-resequencing global optimum is asserted.

`InsertionDecision` and proposal have private construction/mutation. Proposal records
expected plan/assignment, existing owner-bound assignments, exact replacement, new terms,
road deltas and per-order impacts. Full world identity/version and exact `PoolingContext`
(clock/epoch, graph/traffic/profile, projections/execution id, policies, forecasts, config
identities, budget and anchors) must still match at commit. Any mismatch is stale with
no semantic mutation. A cloned replacement is validated before a single publication of
assignment + plan + terms + incremented version. No partial salvage or rerouting at commit.
The caller must supply current context from its authoritative sources, not echo old evidence.

## Execution and CLI

Schema 1 keeps historical Phase 13–16 policies. Schema 2 requires a named/versioned
scenario_id, `multi_order` dispatch with work_budget and forecast_validity_seconds,
and each order's explicit admission policy. `roadrunner simulate` supports both JSON
and readable coverage/plan summaries. Mixed deadline and cumulative policies are per order.
Simulation forecasts are generated at creation and valid for the configured duration;
actual readiness is separate fixed/seeded input and never consulted as a future forecast.

The simulator queues only already-started road/destination wait/service actions. It does
not queue any editable future stop, so no obsolete plan cursor or generation-bound
editable event exists. Each active action has an independent execution identity; stale
identities are discarded before state/metrics mutation. That identity survives replacement.
After completing the active stop through shared World transitions, the simulator reads
the CURRENT plan's first stop and immediately starts it. No active stop is abandoned.

Pickup establishes only that order's custody. Dropoff releases only that order. Busy
occupancy starts once and ends only when the remaining effective plan is empty; pooled
responsibilities form one continuous interval. JSON includes each search, whether it
committed, publication version, final domain state/terms, actual outcomes and explicit
realized hard-deadline/cumulative comparisons. Prediction-feasible admission is not a
realized guarantee when readiness or traffic later differs.

## Verification and measurement

Dispatch fixtures independently hand-calculate travel/arrival/wait/service/departure,
loads and completion; verify cumulative references, prefix overload, custody, frozen
pickup/dropoff projection, exact context stale rejection and signed negative road deltas.
The tiny oracle fills two new positions in a final vector rather than using production
insertion generation, independently sorts the audited objective and compares selected
rider, resulting plan, road deltas and candidate counts across 32 seeds × three admissions.
Seeded actual-readiness replay covers 24 seeds and reversed external collection order.

[Runnable scenarios](../data/fixtures/phase-17/README.md) cover pooling, capacity,
deadline, repeated cumulative delay, forecast failure, budget exhaustion, frozen
travel/wait/service, baseline hard breach and prediction-valid realized violation.
Custody corruption is an independent domain test because runtime scenarios construct
validated fresh worlds, rather than accepting corrupted initial responsibility.

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.99.0 test --workspace --all-features --locked
cargo +1.99.0 build -p roadrunner-cli --release --locked
python3 scripts/collect_multi_order_benchmark.py --output benchmarks/results/2026-10-07-phase-17-multi-order.json
```

The collector independently checks replay bytes, work/pair accounting, complete coverage
and nonpublication of incomplete search. It records hardware, compiler/build/binary hash,
dataset/hash/scenario/graph populations, raw elapsed-process samples, median/p95/p99 and
explicit unmeasured memory. Timing includes loading/process/serialization overhead.
Small synthetic measurements make no production scaling or quality-gate claim.
