# Roadrunner Benchmarks

Roadrunner stores reproducible benchmark definitions in crate `benches/` directories and
structured result snapshots in [`results/`](results/).

## Dijkstra baseline

Run the Phase 5 baseline with:

```bash
cargo bench -p roadrunner-core --bench dijkstra --locked
```

The benchmark builds deterministic directed ring-lattice graphs with three outgoing edges per
node. Edge offsets and distance costs are one, two, and three. Each search starts at node `0` and
targets the final node, forcing Dijkstra to finalize the full generated graph.

Cases cover 1K, 10K, and 100K nodes. The benchmark identifier includes both graph size and the
observed finalized-node count. Criterion measures search time only; graph construction happens
outside the timed closure.

The initial result is
[`2026-09-02-apple-m3-dijkstra.json`](results/2026-09-02-apple-m3-dijkstra.json).
Only compare results collected with equivalent hardware, dataset, build profile, dependency
versions, and Criterion configuration.

Memory is not reported until Roadrunner adopts a controlled allocator or profiler configuration.
An unavailable measurement is recorded explicitly rather than estimated.

## A* comparison

Run the Phase 6 comparison with:

```bash
cargo bench -p roadrunner-core --bench astar --locked
```

The benchmark builds deterministic geographic graphs with a directed eastbound backbone and
dead-end northern branches. Both algorithms return the same optimal backbone route. Dijkstra
finalizes the lower-cost dead ends before reaching the destination, while the scaled Haversine
heuristic lets A* exclude them from its search.

Cases cover 1K, 10K, and 100K nodes. Benchmark identifiers include graph size and observed
finalized-node count. Criterion measures the complete routing call, including A* heuristic
preparation; graph construction and correctness comparisons happen outside the timed closure.

See [`../docs/benchmarks.md`](../docs/benchmarks.md) for findings and the linked structured
result snapshot.

The initial result is
[`2026-09-04-apple-m3-astar-comparison.json`](results/2026-09-04-apple-m3-astar-comparison.json).

## Phase 7 OSM remediation baseline

Run the real-fixture ingestion benchmark with:

```bash
cargo bench -p roadrunner-osm --bench osm_pipeline --locked
```

It measures extraction, normalized validation, `delivery_motorcycle_v2` plus
`ng_v2` compilation, graph/provenance encoding and validation, deep
verification, and Dijkstra/A* routing. It runs both the small correctness input
and the larger pinned engineering input documented under
`data/fixtures/phase-7/`. Process-level `/usr/bin/time -l` measurements provide
peak resident memory for extraction, compile/publication, and load/verification.
Neither input supports an OSM-scale or bounded-memory claim.

The remediated result is
[`2026-09-21-apple-m3-phase-7-remediation.json`](results/2026-09-21-apple-m3-phase-7-remediation.json).
The 2026-09-19 result remains historical evidence for the superseded schema.

## Phase 8 reference validation

[`reference-comparison/`](reference-comparison/) contains a reproducible OSRM comparison
using the pinned Lagos Marina PBF and source-node route corpus. Its structured result
records reachability, distance, duration, route geometry differences, and failures.
This is a correctness diagnostic, not a routing latency benchmark.
