# Phase 19 completion report

Status: **COMPLETE — declared Phase 19 dynamic recovery v1 gates PASS**.
Implemented on `feat/phase-19-dynamic-redispatch` from merged Phase 18 main,
authorized by the user's explicit Phase 19 request. Phase 20 is not started.
[Phase 17/18 normative contracts](adr/0016-multi-order-normative-audit.md)
remain intact; their APIs continue to preserve committed ownership.

## Scope and architecture

Logical-time changes now invoke a separate committed-work recovery decision.
Unstarted awaiting-pickup pairs may move to available riders; picked-up custody
and active road/destination waiting/service remain pinned. Healthy recovery
requires positive road saving after explicit reroute and assignment-stability
penalties, minimum-improvement and cooldown policy. An existing hard breach or
movable work on an unavailable rider requires a fully feasible repair and bypasses
optional churn thresholds. Hard protections never become objective penalties.

The deterministic declared neighborhood relocates each movable pair to all
available riders and all precedence-preserving positions, and relocates editable
single stops within their current owner. Whole-fleet candidates pass authoritative
routing, readiness, service, exact work/custody and capacity evaluation. Complete
best-improvement rounds reach a strict local minimum; this is not a global VRP
optimum. Search exhaustion publishes nothing, including a feasible incumbent.

Dispatch adds `recovery.rs` and shares physical timing through the existing pooling
evaluator. Publication replaces owners/plans in one validated world-version update
and leaves authentic accepted terms unchanged. Core is unchanged. Simulation
schema 4 owns dynamic inputs, observed road-delay generations, union responsibility
intervals, actual outcomes and current-plan execution. Recovery and new-order
Phase 18 admission are explicitly separate transactions.

Triggers include order creation, traffic replacement, readiness observations/
forecast receipt, availability, cancellation, observed road delay and stop-completion
boundaries. A departed leg's delayed arrival supersedes its old generation without
rerouting or double movement. Cancellation removes only unstarted pre-pickup work;
active/onboard cancellation is a typed refusal. Historic terms survive cancellation.

## APIs and evidence

New dispatch types/APIs: `RecoveryPolicy`, `RecoveryContext`, `RecoveryDecision`,
`RecoveryProposal`, `RecoveryEvidence`, `RecoveryTermination`, `recover_fleet`,
`evaluate_recovery_plan`, `World::commit_recovery`, `World::cancel_order`,
`CancellationRefusal`, and terminal `FulfillmentState::Cancelled`.
New simulation types: `DispatchPolicy::Dynamic`, `DynamicEvent`, `DynamicChange`,
`SimulationRecoveryRecord`, cancellation outcomes/count and explicit
`fleet-window-responsibility/v1` utilization basis. CLI emits readable or JSON evidence.

The decision binds graph/traffic/profile, world identity/version and exact prior
state, clock/epoch/anchors, readiness/service/optimizer identities, policies,
forecasts, execution projections, trigger and last successful recovery time.
Evidence includes baseline evaluations/breaches, covered riders, movable/locked
orders, candidate work/feasibility/rejection counts, proven precedence exclusions,
complete rounds, termination (including BelowThreshold with signed saving),
resulting plans/owners, penalty/road accounting and
accepted/current/candidate completion with raw marginal/cumulative impacts/slack.
Failed publication is stale with zero state or version mutation. Predictions and
realized protection outcomes are separate.

## Tests, oracle and replay

**195 Rust tests pass**, including all prior Phase 17/18 tests and 18 new tests:
nine dispatch, seven simulation, one all-fixture CLI test and one compile-fail
API test proving evaluated publication evidence cannot be rebound by callers.

- Independent single-order oracle enumerates both complete owner placements and
  computes its own penalty/ranking across 32 seeds; all 32 match production output.
  It shares the authoritative evaluator, not production generation or ranking.
- Sixteen generated three-order cases validate exact pickup/dropoff multiplicity,
  ownership, prefix capacity/world consistency and equivalent reversed unordered
  input construction. Repeated runs return identical decisions/evidence.
- Atomic publication checks one version advance, exact evaluated plans/owners,
  unchanged acceptance, and stale publication with unchanged state/version.
- Twelve stale context variants cover clock, predictions, service, churn policy,
  trigger/history, forecast, projection, traffic, graph/profile and optimizer.
- Hard baseline repair and repeated recovery prove that original cumulative delay
  references remain immutable; when both riders breach, no repair publishes.
- Tests cover budget/prediction failures, cooldown/penalties, unavailable owner,
  frozen pickup/custody, safe cancellation and explicit active/onboard refusals.
- End-to-end recovery moves B while active A stays with its original rider; both
  execute, per-order release is correct and overlapping work counts one busy
  interval per rider. Forecast updates remain distinct from actual observations.
- Observed road delay supersedes the old arrival; movement is recorded once per
  actual completed leg. Typed unknown readiness preserves execution.
- The CLI test replays all 13 schema 4 fixtures byte-identically and checks readable
  successful recovery. Legacy schema behavior remains covered by the full suite.
- Existing hand-calculated whole-plan timing/readiness/service/load/protection
  fixtures continue to pass through the shared evaluator.

## Scenarios and measurements

[13 runnable fixtures](../data/fixtures/phase-19/README.md) cover offline and optional
recovery, churn suppression, frozen wait/service, cancellation, scoped no recovery,
observed road delay/cumulative realized breach, hard realized breach, unavailable
readiness, forecast update, traffic and budget exhaustion. The collector asserts
expected outcomes, no incomplete publication, exact successful version advance,
locked ownership and original accepted terms retained in final state.

[Measured record](../benchmarks/results/2026-10-08-phase-19-recovery.json) and
[semantic replays](../benchmarks/results/phase-19-replays/) retain hardware,
compiler/build/binary/source identity, dataset hashes, scenario IDs, graph sizes,
policy/configuration, raw samples and median/p95/p99 plus deterministic work.
Apple M3, arm64, 16 GiB, macOS 27.0; Rust 1.99.0 release build. Each configuration
has one warmup and **11 measured runs**, all byte-identical, with no concurrent
workspace validation. Memory is explicitly **unmeasured**. Measurement includes
process launch, graph loading/validation, simulation and JSON capture; it is not
isolated optimizer latency. Graphs are 2–3 nodes/2–4 directed edges, 2–4 riders,
and 1–2 orders. No production latency/quality/scalability gate is claimed.

| Scenario | Median ms | p95 ms | p99 ms | Recovery submissions / commits |
| --- | ---: | ---: | ---: | ---: |
| offline-recovery | 2.614 | 2.728 | 2.767 | 5 / 1 |
| optional-recovery | 2.780 | 3.285 | 3.435 | 9 / 1 |
| churn-suppression | 2.732 | 2.996 | 3.069 | 19 / 0 |
| frozen-wait | 2.641 | 2.694 | 2.695 | 5 / 1 |
| frozen-service | 2.761 | 2.958 | 2.998 | 11 / 1 |
| budget-exhaustion | 4.068 | 6.954 | 7.907 | 23 / 0 |

Percentiles use linear interpolation; full raw measurements for every scenario
are in the record. Submissions are summed across all recovery attempts within one
scenario, while the configured budget applies independently to each decision.

## Completion gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Dynamic triggers and logical context | PASS | Schema 4 events and creation/traffic/readiness/completion hooks |
| Committed reassignment / recovery | PASS | Offline and optional recovery execute both orders |
| Feasibility before optional objectives | PASS | Whole-plan evaluation; hard breach repair tests |
| Readiness, service and whole-plan timing | PASS | Shared evaluator and existing hand-worked tests |
| Exact work, custody and capacity every prefix | PASS | Validated replacement plus generated three-order invariants |
| Immutable acceptance / cumulative reference | PASS | Repeated recovery and final-state collector assertions |
| Marginal/cumulative/slack evidence | PASS | Recovery completion impacts and authentic references |
| Frozen road/wait/service preservation | PASS | Frozen fixtures and lock tests; no mid-leg route change |
| Superseded event invalidation | PASS | Delayed old arrival cannot mutate domain or distance |
| Current-plan next-stop progression | PASS | Recovery at boundaries then effective plan execution |
| Per-order release and union occupancy | PASS | Both-delivery tests and schema 4 exposure basis |
| Explicit cancellation / refusal | PASS | Terminal populations; no active/custody abandonment |
| Churn penalties, minimum gain, cooldown | PASS | Explicit versioned policy, oracle and suppression cases |
| Coverage and no global-optimum overclaim | PASS | All recipients in declared moves; scoped local termination |
| Unknown input / baseline health | PASS | Typed prediction/baseline failures; structural errors propagate |
| Budget exhaustion without incumbent commit | PASS | Dispatch test and runnable exhaustion fixture |
| Exact-context stale checks / atomic publication | PASS | Twelve context variants, world guard and one-version commit |
| Independent tiny oracle | PASS | 32/32 independent owner-placement comparisons |
| Generated invariants / deterministic replay | PASS | 16 multi-order cases and 13 × 11 measured exact replays |
| Successful and rejected CLI examples | PASS | All-fixture CLI test, readable recovery, refusals/incomplete fixtures |
| Measured benchmark evidence | PASS | Hardware/hash/build/raw percentiles/work; memory unmeasured |
| Architecture/API/crate READMEs | PASS | Recovery contracts and root/crate/docs updates |
| Required formatting, lint, tests, replay, build, benchmark | PASS | Commands below all completed successfully |

## Reproduction commands

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --locked -- -D warnings
cargo +1.99.0 test --workspace --locked
cargo +1.99.0 test -p roadrunner-dispatch --test recovery --locked -- --nocapture > target/phase19-oracle.log
cargo +1.99.0 build -p roadrunner-cli --release --locked
PYTHONDONTWRITEBYTECODE=1 python3 scripts/collect_recovery_benchmark.py --oracle-log target/phase19-oracle.log --output benchmarks/results/2026-10-08-phase-19-recovery.json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-19/offline-recovery.json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-19/budget-exhaustion.json --json
```

## Limitations and deferrals

Recovery is a bounded local heuristic. A baseline requiring multiple simultaneous
moves through infeasible intermediate states may report scoped `NoRecovery` even
when a larger global repair exists. No global impossibility or optimum is claimed.
Every required existing rider must be evaluable; unknown health blocks recovery
rather than silently excluding obligations. Candidate fleet clones are materialized
per neighborhood round; the work budget bounds evaluations, not peak generation
memory. These tiny measurements do not demonstrate large-scale recovery performance.

Availability means new-work eligibility, not physical immobilization: unchanged
mandatory work continues if recovery fails. Custody handoffs, returns/disposal,
active/onboard cancellation, mid-leg diversion, active-stop abandonment, stochastic
robust protection, distributed transactions and production feeds remain deferred.
Actual readiness is still materialized from scenario inputs; a forecast update
cannot rewrite a historical fact. Dynamic road-delay observations are supported
only for an uncompleted road leg; waiting/service obligations use their existing
actual readiness/deterministic service semantics. Schemas 1–3 retain historical
behavior; adding the terminal Rust enum variant requires exhaustive downstream
matches to handle cancellation. Phase 20 HTTP APIs are not begun.
