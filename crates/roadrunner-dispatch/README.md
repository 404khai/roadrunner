# roadrunner-dispatch

Authoritative delivery-domain state, rider lookup, whole-plan timing, assignment,
service protections and atomic transitions. Dispatch consumes pinned core routing
providers and explicit logical time; it never reads a simulator queue or host clock.

See [project setup](../../README.md#setup). All commands run from the **repository root**.

## Domain and planning

| Interface | Responsibility |
| --- | --- |
| `Order`, `OrderReadiness`, `FulfillmentState`, `RiderProfile`, `RiderState` | Request/capability facts; predictions distinct from actual readiness; custody distinct from responsibility |
| `WorldData`, `World`, `CommittedAssignment`, `RiderPlan`, `Stop` | Validated ownership, remaining work, acceptance records and whole-world optimistic version |
| `validate_world`, `validate_plan`, `onboard_load` | Exact remaining stops, custody/owner consistency and capacity at every prefix |
| `spatial` rider locators | Linear baseline and immutable indexed nearest-rider queries |
| `RouteProvider`, `CoreRouteProvider`, `RoutingAnchors`, `DispatchSnapshot` | Immutable graph/traffic/profile, caller-supplied node projections and coherent logical evaluation context |
| `basic_dispatch`, `preparation_aware_dispatch`, `strategy_dispatch` | Legacy idle-rider assignment workflows and four comparison strategies |
| `World::commit`, `observe_ready`, `pickup`, `deliver` | Shared validated assignment and fulfillment transitions |
| `project_execution`, `evaluate_whole_plan`, `insert_order` | Phase 17 projected frozen-prefix evaluation and exhaustive single-order insertion |
| `OrderPolicy`, `AcceptedTerms`, `PoolingInputs`, `PoolingContext`, `World::commit_insertion` | Explicit policies, immutable authentic acceptance, exact proposal binding and atomic publication |

Existing responsibility is mandatory work. Pending orders do not consume onboard
capacity until pickup. A picked-up order has exactly one owner-bound remaining dropoff.
Dropoff releases only that order; general valid plans can contain multiple orders.

Legacy Phase 13–16 assignment requires an idle rider, observes deadlines softly and
uses declared historical readiness/zero-service semantics. Phase 17 considers all
eligible riders, preserving a started leg and its destination wait/service, then
exhaustively inserts one new order into each editable suffix. Hard deadlines and optional
cumulative completion-delay bounds are feasibility rules. Ranking uses signed incremental
road-travel seconds, road distance and canonical identity. Waiting/service/customer
impacts remain explicit metrics.

Publication requires complete search and sufficient input. Missing required forecasts
are typed evaluation failures; budget exhaustion is SearchIncomplete with no commit.
Proposals bind whole-world version and exact external policy/prediction/routing/execution/
clock context. Rebuild the **current** context before committing; never echo the old
proposal context as a substitute for checking authoritative sources. Phase 17/18 do not
reassign committed work. Phase 19 uses separate recovery APIs for unstarted work;
mid-leg diversion and acceptance-baseline reset remain unsupported.
See [dispatch](../../docs/dispatch.md), [multi-order API](../../docs/multi-order.md),
[normative audit](../../docs/adr/0016-multi-order-normative-audit.md) and
[Phase 17 completion](../../docs/phase-17-completion.md).

## Runnable examples

```bash
cargo +1.99.0 run -p roadrunner-dispatch --example basic_dispatch --locked
cargo +1.99.0 run -p roadrunner-dispatch --example preparation_aware_dispatch --locked
```

`basic_dispatch` constructs a synthetic road network, compares rider road ETAs,
commits the winner and executes shared readiness/pickup/delivery transitions.
`preparation_aware_dispatch` emits JSON comparing baseline and preparation-aware timing
for long preparation, already-ready stock and underestimated actual preparation.
Both need no external data or services.

For pooled execution use the simulation-backed CLI:

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-17/successful-pooling.json --json
```

## Build, tests and docs

```bash
cargo +1.99.0 build -p roadrunner-dispatch --locked
cargo +1.99.0 test -p roadrunner-dispatch --all-features --locked
cargo +1.99.0 test -p roadrunner-dispatch --test entry_gate --locked
cargo +1.99.0 test -p roadrunner-dispatch --test preparation_aware --locked
cargo +1.99.0 test -p roadrunner-dispatch --test rider_lookup --locked
cargo +1.99.0 test -p roadrunner-dispatch --test pooling --locked
cargo +1.99.0 doc -p roadrunner-dispatch --no-deps --locked
```

API docs: `target/doc/roadrunner_dispatch/index.html`. Pooling tests contain hand-worked
timing/load/cumulative fixtures and an independent tiny insertion oracle. No direct
standalone dispatch binary exists; the two `--example` programs are the executable entries.

## Rider lookup benchmark

```bash
cargo +1.99.0 bench -p roadrunner-dispatch --bench rider_lookup --locked -- --sample-size 30 --measurement-time 1
mkdir -p target/readme-results
RUSTUP_TOOLCHAIN=1.99.0 python3 scripts/collect_rider_lookup_benchmark.py --output target/readme-results/rider-lookup.json
```

Run the collector after a **measured** Criterion benchmark; `-- --test` smoke runs do
not supply statistical samples. The target compares indexed lookup with the linear
baseline on seeded 100–100K-rider datasets. Index construction is outside timed queries.
The collector reads `rustc --version`; the explicit `RUSTUP_TOOLCHAIN` keeps compiler
metadata aligned with the benchmark. See [rider index methodology](../../docs/rider-spatial-index.md).
For assignment process measurements use the root [dispatch/pooling collectors](../../README.md#benchmarks-and-replay-evidence).

## Dependencies

Depends on `roadrunner-core`, Serde and typed errors. Execution scheduling belongs to
[simulation](../roadrunner-simulation/README.md); raw OSM ingestion belongs to
[OSM](../roadrunner-osm/README.md). The package is a workspace path dependency and is
currently marked `publish = false`.

## Phase 18 fleet batches

Schema 3 `fleet_batch` jointly allocates current new work and resequences healthy editable
suffixes, preserving committed owners, accepted terms and active execution. Read
[fleet contracts](../../docs/fleet-optimization.md) for exact API, isolation and heuristic
coverage. Run the pooled fleet example from the repository root:

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-18/greedy-trap.json --json
```

## Phase 19 dynamic recovery

Schema 4 `dynamic` runs committed recovery before a separate Phase 18 admission
publication. Configure `RecoveryPolicy` version 1 with explicit reroute/stability
penalties, minimum improvement and cooldown. Active execution and custody stay
pinned; unstarted pickup/dropoff pairs may change owner without resetting terms.
Availability, forecast, cancellation and observed road-delay inputs use logical
timestamps; traffic, creation, actual readiness and stop completion also trigger
recovery. Refused cancellation and incomplete recovery preserve mandatory work.

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-19/offline-recovery.json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-19/budget-exhaustion.json --json
cargo +1.99.0 test -p roadrunner-dispatch --test recovery --locked -- --nocapture
cargo +1.99.0 test -p roadrunner-simulation --test recovery --locked
```

Dispatch APIs: `RecoveryPolicy`, `RecoveryContext`, `recover_fleet`,
`RecoveryDecision`, `RecoveryEvidence`, `RecoveryTermination`, `evaluate_recovery_plan`,
`World::commit_recovery`, `World::cancel_order`, `CancellationRefusal` and
`FulfillmentState::Cancelled`. Simulation adds `DynamicEvent`, `DynamicChange`,
`DispatchPolicy::Dynamic` and `SimulationRecoveryRecord`.
See [full recovery contracts](../../docs/dynamic-redispatch.md),
[scenario fixtures](../../data/fixtures/phase-19/README.md) and
[completion evidence](../../docs/phase-19-completion.md).
