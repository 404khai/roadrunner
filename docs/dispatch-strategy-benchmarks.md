# Dispatch strategy benchmarking

Phase 16 compares fleet outcomes by replaying identical exogenous inputs through four
independent simulation worlds. Run:

```bash
cargo run -p roadrunner-cli -- benchmark dispatch data/fixtures/phase-16/paired-strategies.json
cargo run -p roadrunner-cli -- benchmark dispatch data/fixtures/phase-16/paired-strategies.json --json
cargo run -p roadrunner-cli -- benchmark dispatch data/fixtures/phase-16/paired-strategies.json --idle-penalty-weight 2 --json
```

The comparison accepts the same inline/artifact scenario envelope as `simulate`.
`compare_strategies(graph, scenario, idle_penalty_weight)` is the library entry point.
The source `dispatch` field is replaced for each run; graph, rider profiles/positions,
orders, readiness forecasts, realized seeded readiness, traffic changes, start, horizon,
and routing epoch remain identical. Each run starts with a fresh world. A failure in any
run fails the whole comparison, without printing partial success output.

## Objectives and shared constraints

| Strategy | JSON policy | Minimized objective |
| --- | --- | --- |
| A: nearest rider | `nearest_rider` | Haversine rider-to-pickup distance in meters |
| B: lowest pickup ETA | `lowest_pickup_eta` | Traffic-aware rider-to-pickup road travel in seconds |
| C: lowest total completion time | `lowest_completion_time` | Readiness-aware evaluation-to-delivery duration in seconds |
| D: preparation-aware | `preparation_aware` | C plus `idle_penalty_weight × predicted pickup waiting` |

All policies consider the complete eligible idle fleet and reject capacity, profile,
custody, invalid plan, and NoRoute candidates through the shared dispatch evaluator.
Nearest rider means nearest **feasible** rider, so an unreachable rider does not block
reachable delivery. Both road legs use Roadrunner Dijkstra. Exact objective ties choose
the lowest RiderId. Deadlines remain soft observations, not ranking constraints.

A and B retain the baseline's zero-wait forecast, while execution still waits for
actual readiness. C uses the preparation evaluator with weight zero; it is distinct from
Phase 13 `basic`, which minimizes pickup plus delivery road travel. D defaults to weight
one, configured independently of the source scenario. C and D require a forecast when
readiness will only be observed later. No policy sees realized readiness before the
actual event. Weight zero makes C and D produce identical physical outcomes; their
strategy identities remain distinct.

For a single order with a static overlay, B and the Phase 13 basic objective commonly
select the same rider because pickup-to-dropoff travel is independent of the rider.
C can tie multiple riders arriving before readiness; D can then prefer a later arrival
to reduce pickup waiting. Equal routes or fleet outcomes are valid measured results.
There is no requirement that each policy beat another policy.

Every decision records its versioned strategy, candidate coverage, rejection/evaluation,
objective contributions, selected proposal, and exact tie reason. Nearest decisions
record `score.nearest_distance` in meters as their ranking objective; `score.total`
retains informational road-travel seconds for that policy. Other policies rank by
`score.total` seconds. Completion/preparation decisions retain readiness source and
waiting breakdown. Candidate evaluation is boxed in memory; its JSON shape is unchanged.

## Comparison populations

JSON schema 1 has `runs` in A/B/C/D order, each a complete simulation artifact. Readable
output includes each policy's mean/median/p95 observed delivery duration, mean pickup
waiting, mean rider idle seconds, completed distance, late deliveries, utilization,
and created/unassigned/unfinished populations. JSON also records p99 and sample counts.
These are computed from execution, rather than strategy predictions.

Use [simulation metric definitions](simulation.md#results-and-metric-populations) when
interpreting comparisons. Delivery durations run from creation to delivered dropoff;
unassigned or unfinished orders have no fabricated duration and remain explicit.
Pickup waiting covers completed pickups; partial horizon waits remain per-order facts.
Distance includes only arrived whole legs. Rider utilization is responsibility occupancy
(including pickup waiting), divided by the full initially available rider-seconds window.
Higher occupancy alone is not evidence of a better strategy. Compare delivered counts
and outstanding lateness alongside completed-duration percentiles to avoid censoring bias.

## Reproducible measurement

```bash
cargo +1.99.0 build -p roadrunner-cli --release --locked
python3 scripts/collect_dispatch_strategy_benchmark.py --runs 11 --output /tmp/phase16.json
```

The collector first verifies byte-identical paired comparisons. It then runs each policy
independently, verifies equivalence to its comparison result, and checks byte-identical
replay for one excluded warmup and every measured run. Relative graph artifact paths
are bound to the source directory before temporary variant documents are written.

The generated artifact records hardware/OS, compiler and binary hash, source dataset/hash,
graph digest/size, generator/seed, routing/coverage/tie configuration, penalty weight,
observation window, number of runs, replay hashes, raw timings and median/p95/p99,
computed summaries, and canonical order/rider outcomes. Timing includes process startup,
loading/validation, simulation, JSON serialization, and capture. Timing percentiles use
linear interpolation; simulated outcome percentiles use nearest rank. Memory is unmeasured.
Measurements should run without simultaneous workspace checks. The recorded build command
assumes the release binary was built using the command above.

The [fixture](../data/fixtures/phase-16/paired-strategies.json) is deliberately small and
synthetic. See [Phase 16 completion](phase-16-completion.md) for measured evidence. It
establishes reproducibility and different objective behavior, not production scalability
or a universal strategy ranking. Multi-order routing, VRP and re-dispatch remain later phases.
