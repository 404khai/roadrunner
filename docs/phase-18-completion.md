# Phase 18 completion report

Status: **COMPLETE — all declared Phase 18 gates PASS**. Implementation authorized
2026-10-08 after Phase 17 completion. Phase 19 is not implemented or started.
Normative source: [ADR 0016](adr/0016-multi-order-normative-audit.md).

## Implemented scope and architecture

One coherent current unassigned batch is jointly allocated while healthy riders'
editable suffixes are resequenced. Existing owners, custody, accepted reference terms
and active execution remain fixed. Deterministic greedy, canonical multi-start and
best-improvement local search compare feasible candidates by admitted count, road seconds,
road distance and canonical fleet identity. No blended penalties or demand reward.
The declared local neighborhood includes unplaced insertion, editable stop relocation,
proposed pair relocation and one-request ejection/reinsertion. Publication requires
complete declared search and commits all assignments/plans/new acceptance terms together.

Breached riders remain unchanged. Required unavailable predictions or unroutable baselines
isolate riders, unavailable new-order forecasts isolate requests, and the healthy remainder
can optimize with explicitly incomplete INPUT coverage. Unsupported policy/migration or
structural corruption remains a typed error. Budget exhaustion never publishes an incumbent.

Dispatch adds `fleet.rs`; shared `pooling.rs` evaluation accepts multiple provisional new
orders without changing Phase 17 public behavior. Core is unchanged. Simulation forms
current equal-time batches through one queued decision and uses existing active-action
identities/current-plan progression. Schema 3 is explicit; schemas 1/2 preserve their
historical declared semantics. CLI provides readable and JSON evidence through `simulate`.

New APIs/types: `optimize_fleet`, `evaluate_batch_plan`, `World::commit_fleet`,
`FleetAlgorithm`, `FleetInputs`, `FleetContext`, `UnavailableExecution`, `FleetIsolation`,
`FleetObjective`, `FleetWork`, `FleetRiderEvidence`, `FleetEvidence`, `FleetTermination`,
`FleetProposal`, `FleetDecision`; simulation adds `DispatchPolicy::FleetBatch`,
`SimulationFleetRecord` and `SimulationEvent::FleetPlanned`. Construction of decisions/
proposals remains private; consumers use read-only getters. Exact context includes
algorithm, budget, routing graph/traffic/profile, anchors, forecasts, policy identities,
clock/epoch, projections and unavailable active identities, plus world identity/version.
No optimizer framework or distributed transaction machinery.

[Detailed algorithm/API contracts](fleet-optimization.md) describe horizon accounting,
publication, isolation and the exact neighborhood. [Implementation contract](phase-18-design.md)
records the bounded scope selected before implementation.

## Validation and oracle results

Formatting, strict workspace Clippy, workspace tests, release build, API documentation,
independent oracle emission, CLI replay and the measured collector all passed. **177 Rust
tests** pass, including all previous 159 and 18 new tests: 11 dispatch fleet tests,
6 simulation fleet tests and one all-fixture CLI test.

The independent exact oracle enumerates rider/unplaced ownership choices and full
precedence-valid stop permutations; it does not call production generation/pruning/ranking.
It shares the authoritative physical evaluator. Coverage includes:

- Greedy trap: greedy admits one; local ejection and multi-start admit two and match oracle.
- 16 seeded two-order/two-rider exact comparisons and canonical reversed input/replay.
- 24 three-order/two-rider seeds × three algorithms = 72 measured quality comparisons.
  All admit the oracle count on this dataset. Greedy has a maximum 20s road gap; local
  and multi-start have zero road gaps on these 24 seeds. This is dataset evidence only.
- A directed four-onboard-stop relocation trap: declared local optimum **32s**, independent
  global optimum **30s**, gap **2s**. All committed owners remain fixed. No proposal is
  published for the unchanged local optimum. The gap is retained in versioned evidence.
- 24 seeded actual-readiness simulations replay byte-identically with reversed external
  order/rider construction. Frozen travel/wait/service retain completion and active id.

Stale tests cover world identity/revision, clock/epoch, graph/traffic/profile, prediction,
service/optimizer policy, forecasts, projections, anchors, algorithm, work budget and
frozen execution identity. Atomic success publishes exactly evaluated plans/terms; stale,
incomplete or unplaced work leaks no ownership, stops or acceptance records. Tests cover
fixed picked-up custody, committed suffix resequencing, immutable cumulative references,
baseline breach/unknown isolation, explicit policy errors, current-plan execution,
per-order release and union occupancy. Existing hand-worked Phase 17 intermediate timing,
service/readiness, capacity and protection fixtures all pass through the reused evaluator.

## Runnable scenarios and coverage

[12 schema 3 CLI fixtures](../data/fixtures/phase-18/README.md) exercise joint pooling,
greedy trap, prefix capacity, hard deadline, repeated cumulative protection, baseline
breach, unknown input, frozen wait/service, complete scoped nonadmission, incomplete budget
and admission-valid realized readiness violation. Joint pooling admits both orders in
one fleet publication and executes both. The all-fixture CLI test checks JSON replay
and readable output. The collector independently verifies 36 scenario/algorithm
configurations and byte-identical outputs for all runs.

Coverage dimensions remain separate: all riders are classified; input completeness may
be false after explicit isolation; search completeness means the declared evaluable
heuristic search completed. `LocalOptimum` is a complete local-neighborhood result,
`ConstructionComplete` is greedy's scoped result, and `SearchIncomplete` has no proposal.
No `NoFeasibleInsertion` or global VRP infeasibility claim is reused for fleet heuristics.
The objective excludes isolated and frozen constants identically. Unknown fleet totals
are not fabricated. Customer marginal/cumulative impacts remain separate from road cost.

## Measured benchmark evidence

[Versioned benchmark record](../benchmarks/results/2026-10-08-phase-18-fleet.json) includes
source/configured dataset hashes, scenario identities, graph digests/populations,
algorithm/budget, compiler/build/binary hash, hardware, raw samples, median/p95/p99,
deterministic work, exact-oracle comparisons and the local-minimum gap. The 36 chosen
semantic artifacts are retained in [phase-18-replays](../benchmarks/results/phase-18-replays/)
with hashes, expected/proposed state, timing, protection impacts and publication results.

Hardware: Apple M3 arm64, 16 GiB system memory, macOS 27.0. Compiler:
`rustc 1.99.0 (b940084d7 2026-09-28)`. Release CLI, one excluded warmup and **11 measured
runs per configuration** (396 measured process runs). No concurrent workspace validation
ran during measurement. Process memory is **unmeasured**; system RAM is not a process
memory measurement. Fixtures are small synthetic graphs, not production workloads.
Timings include launch, graph load/validation, simulation and JSON capture.

| Fixture | Algorithm | First-batch admitted | Delivered | Fleet submissions (all attempts) | Median ms | p95 ms | p99 ms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| greedy-trap | Greedy | 1 | 1 | 11 | 2.323 | 2.456 | 2.483 |
| greedy-trap | LocalSearch | 2 | 2 | 48 | 2.438 | 2.568 | 2.569 |
| greedy-trap | MultiStartLocal | 2 | 2 | 32 | 2.422 | 2.639 | 2.710 |
| joint-pooling | Greedy | 2 | 2 | 7 | 2.293 | 2.429 | 2.435 |
| joint-pooling | LocalSearch | 2 | 2 | 25 | 2.361 | 2.473 | 2.486 |
| joint-pooling | MultiStartLocal | 2 | 2 | 32 | 2.410 | 2.560 | 2.566 |

No latency/quality threshold or production scalability claim is inferred. The 40.030s
road objective in the two-admission greedy trap is appropriately larger than greedy's
10.008s one-admission objective: admission count is the first objective, mandatory before
road minimization. Process timing is separate from those predicted road values.

## Completion gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Current batch jointly allocated | PASS | Joint pooling/greedy trap; multi-assignment proposal |
| Editable suffix resequencing | PASS | Committed-suffix fixture and declared stop neighborhood |
| Existing owners fixed; no handoff/recovery | PASS | Custody/resequencing tests; generators move only proposed ownership |
| Whole-plan timing | PASS | Shared evaluator and hand-worked Phase 17 fixtures |
| Readiness and service propagation | PASS | Shared evaluator, colocated stops, frozen service and realized-error tests |
| Capacity after every stop | PASS | Shared validation/evaluator, seeded varying demand/capacity, capacity CLI fixture |
| Custody and exact fulfillment/stops | PASS | Picked-up fixed-owner fixture and final world validation |
| Acceptance terms atomic/authentic/immutable | PASS | Multi-order commit and repeated-reference tests |
| Cumulative bound independent of deadline | PASS | Shared protection fixtures, cumulative repeat and baseline-breach scenarios |
| Raw signed marginal/cumulative evidence | PASS | Per-rider impacts and immutable reference assertions |
| Admitted-count / road / distance / canonical objective | PASS | Independent comparator/oracle and greedy trap |
| All riders classified | PASS | Canonical health pass with explicit exclusions/isolation |
| Baseline hard breach isolated unchanged | PASS | Dispatch and CLI breach fixtures |
| Unevaluable riders/orders isolated; incomplete input explicit | PASS | Unknown-active and unavailable-order tests; collector coverage |
| Structural/policy error distinct from ordinary rejection | PASS | Unsupported-policy/migration tests and shared world validation |
| Declared neighborhood/search coverage qualified | PASS | Work, rounds, starts, structural exclusion counts and local-minimum evidence |
| Budget exhaustion yields no commit | PASS | Found-incumbent budget tests and zero-budget CLI fixture |
| Exact context stale checks | PASS | Full-context mutation tests including traffic/world/frozen id |
| Whole-fleet all-or-nothing publication | PASS | One shared validated replacement; exact plan/terms and stale nonmutation tests |
| Frozen execution survives plan replacement | PASS | Travel/wait/service tests and current-plan simulation |
| Superseded editable execution cannot mutate | PASS | No editable future events queued; existing stale-active-identity tests pass |
| Current next-stop execution | PASS | Joint pooled execution and frozen rewrite tests |
| Per-order release | PASS | Shared transitions; both orders finish and final responsibility is empty |
| Multi-order union occupancy | PASS | Busy interval equals last completion, not overlapping sum |
| Independent tiny fleet oracle | PASS | Ownership/permutation enumeration, 16 seeded checks, 72 quality cases |
| Hand-worked evaluator fixtures retained | PASS | Previous intermediate timing/custody/load/protection suite passes |
| Generated invariants and deterministic replay | PASS | Seeded demand/capacity tests, 24 actual seeds, 36 measured CLI configurations |
| Greedy trap and local-optimum quality quantified | PASS | One versus two admissions; 32s versus 30s measured gap |
| Successful/rejected CLI examples | PASS | All 12 fixtures, JSON/readable CLI test |
| Measured benchmark evidence | PASS | 11 runs/config, hashes/hardware/raw percentiles/work; memory explicitly unmeasured |
| Architecture/API/scenario docs updated | PASS | Fleet contract, API/architecture guides, schema 3 and README additions |
| Required fmt/Clippy/tests/replay/benchmark commands | PASS | Commands below and versioned collector artifacts |

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.99.0 test --workspace --all-features --locked
cargo +1.99.0 doc --workspace --no-deps --locked
cargo +1.99.0 test -p roadrunner-dispatch --test fleet --locked -- --nocapture > target/phase18-oracle.log
cargo +1.99.0 build -p roadrunner-cli --release --locked
PYTHONDONTWRITEBYTECODE=1 python3 scripts/collect_fleet_benchmark.py --oracle-log target/phase18-oracle.log --output benchmarks/results/2026-10-08-phase-18-fleet.json
```

## Known limits and deferrals

The heuristic is not exhaustive arbitrary VRP. Cyclic starts and one-request ejection
can miss better basins; the committed-stop fixture demonstrates a local optimum gap.
Large batches can exhaust deterministic work and withhold publication. No robust
uncertainty margin, route cache, parallel search, spatial shortlist or larger-neighborhood
quality guarantee. No current production-scale benchmark or process-memory measurement.
Hard admission feasibility remains pinned-model feasibility, not realized guarantee.
Unknown or hard-breached obligations are preserved rather than recovered/reassigned.
Phase 19 recovery/reassignment, handoffs, mid-leg diversion, persistence and distributed
runtime remain deferred. **Stop after Phase 18 and await user direction.**
