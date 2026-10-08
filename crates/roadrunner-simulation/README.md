# roadrunner-simulation

Seeded discrete-event delivery simulation over Roadrunner graphs and shared dispatch
transitions. The simulator owns logical time, event ordering, forecasts and actual
observations, execution progression and metrics. Dispatch owns authoritative planning,
custody, fulfillment and atomic publication.

See [project setup](../../README.md#setup). All commands run from the **repository root**.
No wall-clock sleeping, live traffic, database, broker or external delivery service is
required. This crate is a library; use the CLI to load scenario files.

## Public interfaces

| API / type | Role |
| --- | --- |
| `simulate(&FrozenGraph, &SimulationScenario)` | Run a fresh validated scenario through its inclusive logical horizon |
| `compare_strategies` | Replay schema 1 inputs through four single-order policies in fresh worlds |
| `SimulationScenario`, `RiderInput`, `OrderInput`, `DispatchPolicy` | Versioned clock, fleet, requests and policy inputs |
| `ActualReadiness`, `TrafficChange`, `TrafficOverride` | Fixed/seeded actual readiness and complete static-overlay replacements |
| `SimulationEvent`, `RecordedEvent`, `ExecutedLeg` | Exact logical event trace and departed-leg routing provenance |
| `SimulationResult`, `SimulationSummary`, `OrderOutcome`, `RiderMetrics` | Structured results, populations, observed milestones and occupancy |
| `SimulationInsertionRecord`, `SimulationPredictionFailure`, `RealizedProtectionOutcome` | Phase 17 planning/publication evidence, blocked-input attempts and actual policy comparisons |

The library receives an already-built graph and a scenario. The CLI wraps them in a
JSON document with `graph` (inline synthetic nodes/roads or a core artifact path) and
`scenario`. Artifact paths resolve relative to that JSON file and require an explicit
graph digest. Use [simulation schema documentation](../../docs/simulation.md) and committed
fixtures as the source of field names; Rust APIs are available via generated docs.

## Run scenarios

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-15/seeded-deliveries.json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-15/seeded-deliveries.json --json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-17/successful-pooling.json --json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-17/hard-deadline.json --json
```

| Scenario contract | Semantics |
| --- | --- |
| Schema 1 | Historical single-order policies; soft observed deadlines and zero service |
| Schema 2 / `multi_order` | Named/versioned scenario, explicit per-order admission/service policy, forecast validity and deterministic insertion work budget |
| Schema 3 / `fleet_batch` | Joint batch/suffix heuristic with named algorithm, work budget and explicit isolation |

The Phase 15 fixture includes seeded readiness, changing traffic, infeasible and future
orders. The Phase 17 fixture admits B while A is active and correctly executes both.
[Phase 17 fixtures](../../data/fixtures/phase-17/README.md) also cover rejected placements,
incomplete search, frozen travel/wait/service and actual readiness errors.

Prediction and observation stay separate. A required missing prediction blocks new
admission with typed incomplete-input evidence while existing work continues; a later
observation can permit a fresh attempt. Hard predicted admission is not guaranteed
realized fulfillment. Actual hard misses are recorded separately. Execution finishes
each active stop unchanged and selects the next stop from the **current** committed plan;
only active actions are queued. Each dropoff releases one order and busy time is the
union of responsibility, never the sum of overlapping delivery intervals.

## Compare single-order strategies

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- benchmark dispatch data/fixtures/phase-16/paired-strategies.json
cargo +1.99.0 run -p roadrunner-cli --locked -- benchmark dispatch data/fixtures/phase-16/paired-strategies.json --idle-penalty-weight 1 --json
```

The comparison runs nearest feasible rider, lowest pickup ETA, lowest readiness-aware
completion time, and preparation-aware assignment on the same exogenous inputs. Use a
schema 1 document for this workflow. It does not benchmark fleet VRP or compare pooling
against a fleet optimizer. See [comparison populations/methodology](../../docs/dispatch-strategy-benchmarks.md).

## Replay and external timing collectors

```bash
cargo +1.99.0 build -p roadrunner-cli --release --locked
mkdir -p target/readme-results
./target/release/roadrunner simulate data/fixtures/phase-17/successful-pooling.json --json > target/readme-results/replay-a.json
./target/release/roadrunner simulate data/fixtures/phase-17/successful-pooling.json --json > target/readme-results/replay-b.json
cmp target/readme-results/replay-a.json target/readme-results/replay-b.json
python3 scripts/collect_simulation_benchmark.py --output target/readme-results/simulation.json
python3 scripts/collect_dispatch_strategy_benchmark.py --output target/readme-results/dispatch.json
python3 scripts/collect_multi_order_benchmark.py --output target/readme-results/multi-order.json
```

`cmp` succeeds with no output for identical files. Collectors default to eleven measured
runs plus an excluded warmup, accept `--binary` and `--runs`, and verify exact replay.
Use `--scenario` on the first two collectors or `--fixtures` on the pooling collector
to choose inputs. Process timing includes launch/loading/serialization overhead; it is
not isolated routing latency. Machine timings stay outside semantic simulation artifacts.

The fixed seed and canonical order IDs determine readiness generation. Events order by
logical instant then insertion sequence. Results retain unfinished work and report
sample populations: completed durations do not hide uncreated, unassigned or unfinished
orders. Utilization uses the configured measurement window and initially available
riders. See [metrics and limitations](../../docs/simulation.md).

## Build, tests and docs

```bash
cargo +1.99.0 build -p roadrunner-simulation --locked
cargo +1.99.0 test -p roadrunner-simulation --all-features --locked
cargo +1.99.0 test -p roadrunner-simulation --test simulation --locked
cargo +1.99.0 test -p roadrunner-simulation --test comparison --locked
cargo +1.99.0 test -p roadrunner-simulation --test pooling --locked
cargo +1.99.0 doc -p roadrunner-simulation --no-deps --locked
```

API docs: `target/doc/roadrunner_simulation/index.html`. This crate has no Criterion target;
its external collectors exercise the CLI/library workflow. It depends on core, dispatch,
Serde and typed errors and is currently marked `publish = false`.

## Phase 18 fleet batches

Schema 3 `fleet_batch` jointly allocates current new work and resequences healthy editable
suffixes, preserving committed owners, accepted terms and active execution. Read
[fleet contracts](../../docs/fleet-optimization.md) for exact API, isolation and heuristic
coverage. Run the pooled fleet example from the repository root:

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-18/greedy-trap.json --json
```
