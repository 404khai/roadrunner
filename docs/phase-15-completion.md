# Phase 15 completion

Date: 2026-10-04
Status: COMPLETE — Deterministic Dispatch Simulation Engine

`roadrunner-simulation` owns explicit logical time, a stable priority queue, versioned
seeded preparation generation, scenario orchestration, executed outcomes, and metric
populations. It depends on core and dispatch. The CLI composes its library API as
`roadrunner simulate scenario.json [--json]`, loading either a synthetic inline graph
or a validated core graph artifact. See [simulation documentation](simulation.md) and
[ADR 0015](adr/0015-deterministic-simulation-runtime.md).

## Requirement review

| Phase 15 requirement | Implementation / evidence |
| --- | --- |
| Model riders | Canonical profiles/states, node anchors, initial operational availability, scalar capacity, responsibility intervals |
| Model restaurants/stores and customers | Pickup/dropoff graph locations, expected readiness, separate actual ready events; no separate account/kitchen model |
| Model orders | Scheduled creation, scalar demand, optional deadlines, shared fulfillment/custody and remaining plans |
| Model road network | Frozen core graph; Roadrunner Dijkstra for both evaluated and executed road legs |
| Model traffic | Validated immutable static overlays; full replacement events apply at subsequent departures |
| Model time | Explicit start/horizon/epoch; checked DispatchInstant/Seconds arithmetic without host clocks or sleeping |
| Required eight events | OrderCreated, OrderReady, RiderMoved, RiderAssigned, RiderArrivedPickup, OrderPickedUp, OrderDelivered, TrafficChanged |
| Priority queue | BinaryHeap ordered by exact logical time then checked monotonic insertion sequence |
| Reproducible randomness | Explicit seed, versioned SplitMix64 upper-53-bit source, generation before dispatch in canonical OrderId order |
| Shared state transitions | Atomic World::register_order plus existing commit/movement/readiness/pickup/delivery methods |
| CLI and metrics | Readable and JSON output, population-aware predictions versus actual execution, explicit unfinished work |
| Faster than wall-clock execution | No sleeps; externally measured 600-second fixture replay, linked below |

Valid failed assignments also produce DispatchUnassigned trace entries and preserve
completed immutable decision evidence. Pending work retries on external changes and
completed delivery without a self-triggered unassigned retry loop. Assignment actions
serialize snapshot evaluation and commit, preventing duplicate simultaneous reservations.

## Deterministic correctness evidence

The simulation integration suite includes 15 fixtures covering event lifecycle/order,
actual readiness overriding baseline predictions, rider reuse, waiting and movement
at the horizon, created versus uncreated populations, completed and outstanding lateness,
traffic changes during travel, simultaneous traffic replacement ordering, exact seeded
JSON replay, seed independence from dispatch policy and order-input permutation,
simultaneous creations, empty/unavailable/capacity-limited fleets, preparation-aware
selection, already-ready stock, nonzero epochs, zero-time events, inclusive horizon,
disconnected routes, scenario validation, and typed evaluation failures without input mutation.

Two unit fixtures pin the generator's reference vector and check reversed clock scheduling
and sequence exhaustion without insertion. Four CLI fixtures cover byte-for-byte replay,
readable metrics, invalid scenario failure without success JSON, graph artifact loading,
relative path resolution, required digest binding, and checked node/geometry coordinates. A new shared World regression
checks registration, duplicate rejection, and version exhaustion with no partial publication.

The [versioned synthetic fixture](../data/fixtures/phase-15/seeded-deliveries.json) has
five nodes, six directed edges, two riders, six scheduled orders, and a 600-second horizon.
It yields five created orders, four deliveries, one capacity-infeasible pending order,
one later/uncreated order, one late completed delivery, and one outstanding order past
its deadline. These observations are generated and checked, not hardcoded conclusions
inside the engine. Predicted ETA, actual delivery duration, pickup waiting, and rider
responsibility utilization are separate measures.

## External timing artifact

The [measured result](../benchmarks/results/2026-10-04-phase-15-simulation.json) records
Apple M3 hardware, 16 GiB memory, macOS, graph/dataset identities, seed/generator,
algorithm/policy, compiler/build configuration, one excluded warmup, 11 measured runs,
raw elapsed samples, median/p95/p99, and output replay hash. The measured median process
time is below the fixture's 600-second logical window. Process startup, graph/scenario
loading and validation, simulation, JSON serialization, and captured output are included.
The run did not overlap workspace validation.

Reproduce with:

```bash
cargo +1.99.0 build -p roadrunner-cli --release --locked
python3 scripts/collect_simulation_benchmark.py --output /tmp/phase15-simulation.json
```

This tiny synthetic scenario demonstrates fast logical-clock execution. It establishes
no production scalability, memory usage, fleet efficiency, or strategy-comparison claim.

## Validation

| Command/check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo +1.99.0 clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS |
| `cargo +1.99.0 test --workspace --all-features --locked` | PASS — 130 tests; doc tests pass |
| `cargo +1.99.0 build -p roadrunner-cli --release --locked` | PASS |
| Simulation benchmark collector | PASS — 11 byte-identical measured CLI replays plus warmup |
| Relative documentation file links | PASS |
| `git diff --check` | PASS |

The existing Phase 13/14/index suites remain intact. New exact float assertions are
restricted to test fixtures, consistent with existing exact deterministic contracts.
The generator's production cast allowance is narrowly documented: its upper 53 bits
fit exactly in f64. No test/lint policy is relaxed and no unsafe Rust is introduced.

## Scope boundary

The engine starts with idle rider plans and assigns one active order per rider. Movement
is whole-leg endpoint execution; in-flight traffic never rewrites a departed leg. Initial
operational availability is fixed. Store/customer entities are supported as locations
and readiness/fulfillment inputs, without additional production entity models.

Phase 16 fleet strategy benchmarking, multi-order insertion, initial busy work, dynamic
availability, cancellation/re-dispatch, continuous/per-edge movement, kitchen queues,
UI/replay controls, HTTP, persistence, live data, and ML remain deferred.
