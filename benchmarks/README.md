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
