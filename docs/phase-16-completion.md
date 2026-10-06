# Phase 16 completion

Date: 2026-10-06
Status: COMPLETE — Dispatch Strategy Benchmarking

Phase 16 adds deterministic nearest-feasible-rider, lowest-pickup-ETA,
readiness-aware lowest-completion-time, and preparation-aware assignment strategies.
They share candidate coverage, road evaluation, hard feasibility, atomic assignment,
and execution. `compare_strategies` runs four fresh worlds over identical exogenous
facts. `roadrunner benchmark dispatch <scenario.json> [--idle-penalty-weight <weight>]
[--json]` exposes readable summaries and complete deterministic JSON evidence.
See [methodology and contracts](dispatch-strategy-benchmarks.md).

## Requirement review

| Requirement | Evidence |
| --- | --- |
| Nearest rider baseline | Haversine distance among shared road-feasible candidates, meter-valued score evidence |
| Lowest pickup ETA | Traffic-aware pickup travel ranking with objective-specific explanation |
| Lowest total completion time | Readiness-aware elapsed delivery completion, waiting penalty zero |
| Preparation-aware assignment | Completion plus configurable non-negative predicted waiting penalty |
| Average / median / p95 delivery time | Observed creation-to-dropoff distribution with sample count and p99 |
| Rider idle time | Available-unassigned rider seconds plus separate observed pickup waiting distribution |
| Distance traveled | Sum of completed road-leg distances; partial travel not fabricated |
| Late orders | Completed deadline misses and outstanding past-deadline populations |
| Rider utilization | Clipped responsibility intervals / full available rider-seconds |
| Automatically generated results | Paired CLI/library artifacts and external replay-checking benchmark collector |
| Reproducible measurements | Hardware, dataset/hash, graph size/digest, algorithms/configuration, compiler/binary hash, warmup/runs, raw timing samples, median/p95/p99 |

## Measured synthetic comparisons

The values below are derived from the committed collector artifacts, rounded for display.
Each policy has one excluded warmup and eleven measured byte-identical standalone runs.
The collector also verifies paired replay and standalone equivalence. Measurements ran
without simultaneous workspace validation. Timing includes process/loading/JSON overhead.

| Dataset | Policy | Delivered | Mean delivery s | Median s | p95 s | Mean pickup wait s | Distance m | Late | Utilization |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Slow pickup road | nearest_rider | 12 | 242.482 | 223.189 | 452.768 | 1.128 | 8562.021 | 12 | 76.117% |
| Slow pickup road | lowest_pickup_eta | 12 | 207.024 | 184.506 | 414.064 | 5.800 | 8339.631 | 12 | 71.819% |
| Slow pickup road | lowest_completion_time | 12 | 242.482 | 223.189 | 452.768 | 1.128 | 8562.021 | 12 | 76.117% |
| Slow pickup road | preparation_aware | 12 | 242.482 | 223.189 | 452.768 | 1.128 | 8562.021 | 12 | 76.117% |
| Preparation delay + traffic | nearest_rider | 4 | 90.919 | 92.874 | 122.330 | 10.008 | 2335.097 | 1 | 26.131% |
| Preparation delay + traffic | lowest_pickup_eta | 4 | 90.919 | 92.874 | 122.330 | 10.008 | 2335.097 | 1 | 26.131% |
| Preparation delay + traffic | lowest_completion_time | 4 | 90.919 | 92.874 | 122.330 | 10.008 | 2335.097 | 1 | 26.131% |
| Preparation delay + traffic | preparation_aware | 4 | 89.799 | 92.874 | 120.091 | 9.448 | 2335.097 | 1 | 25.944% |

The slow-road scenario contains five nodes, six directed edges, two riders, fourteen
scheduled orders and a 900-second window. All policies deliver twelve orders; one
created order has a disconnected pickup and one order is created after the horizon.
Lowest pickup ETA has lower completed delivery durations in this scenario. A/C/D have
identical observed fleet outcomes, including first-order rider selection. A distance
objective and a waiting penalty can both favor the geographically closer rider when
that rider arrives later over a slower road.

The preparation/traffic scenario reuses the Phase 15 fixture: five nodes, six edges,
two riders, six scheduled orders, and a 600-second horizon. All policies deliver four,
retain one capacity-infeasible order, and retain one uncreated order. Preparation-aware
assignment chooses a different first rider and reduces mean pickup waiting and delivery
duration in this specific replay. All policies have one late completed delivery.

Neither scenario establishes a universally best strategy or production scalability.
Responsibility utilization includes waiting; completed-duration percentiles exclude
unfinished work. These populations are explicit in the artifacts.

Artifacts:

- [Slow-road comparison](../benchmarks/results/2026-10-06-phase-16-dispatch-strategies.json)
- [Preparation/traffic comparison](../benchmarks/results/2026-10-06-phase-16-preparation-strategies.json)

Reproduce:

```bash
cargo +1.99.0 build -p roadrunner-cli --release --locked
python3 scripts/collect_dispatch_strategy_benchmark.py --output /tmp/phase16-slow-road.json
python3 scripts/collect_dispatch_strategy_benchmark.py --scenario data/fixtures/phase-15/seeded-deliveries.json --output /tmp/phase16-preparation.json
```

## Correctness and validation

The new comparison tests verify traffic-induced proximity/ETA disagreement, readiness
completion ties, waiting-penalty selection, meter/time score evidence, objective-specific
explanations, shared standalone equivalence, canonical exact replay, equal realized
readiness across policies, zero-penalty equivalence, empty fleets/populations, capacity
rejections, unfinished horizons, outstanding lateness, invalid weights/missing forecasts,
and immutable caller inputs. CLI tests verify A/B/C/D ordering, configured weight,
byte-identical JSON, readable metrics, invalid-argument errors without success output,
and disconnected/future populations in the Phase 16 fixture.

| Validation | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo +1.99.0 clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS |
| `cargo +1.99.0 test --workspace --all-features --locked` | PASS — 138 tests and doc tests |
| Release CLI build | PASS |
| Both benchmark collectors | PASS — eleven replay-checked runs per strategy per dataset |
| Documentation links / `git diff --check` | PASS |

No unsafe Rust or new dependencies are introduced. Multi-order insertion, VRP,
dynamic re-dispatch, live inputs, UI, HTTP, persistence and ML remain later work.
