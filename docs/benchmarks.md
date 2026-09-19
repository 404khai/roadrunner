# Routing Benchmarks

> Historical note: the measurements below describe the Phase 6 mutable graph and
> graph-scanning A* implementation. They are preserved for comparison and are not
> evidence of Phase 7 or OSM-scale performance. The revised-core baseline is
> governed by [ADR 0010](adr/0010-benchmarking-and-phase-7-readiness.md).

Roadrunner records benchmark methodology and machine-readable results so algorithm comparisons
can be reproduced rather than inferred from isolated timing claims.

## Revised pre-Phase-7 baseline

The frozen-core baseline uses one deterministic `FrozenGraph` per size and a five-query corpus
covering local, medium, long, unreachable, and source-equals-destination requests. Dijkstra and A*
receive the same snapshot, corpus, distance objective, and routing context. The corpus seed is
`0x5EED`. Each reported routing sample is one complete five-query corpus iteration.

The lifecycle benchmark measures mutable builder construction, deterministic finalization,
serialization, and decode plus mandatory validation separately. Criterion uses 20 samples, a
one-second warmup, and a two-second target measurement window. Exact hardware, software,
per-iteration median/p95/p99, artifact sizes, and expanded-state totals are stored in
[`../benchmarks/results/2026-09-19-apple-m3-revised-core.json`](../benchmarks/results/2026-09-19-apple-m3-revised-core.json).

This deliberately simple forward-banded graph is useful for regression and asymptotic signals,
not realism. On this topology A* and Dijkstra expand the same number of states, so A*'s additional
Haversine work makes it slower. That is a property of this fixture, not a general algorithm
conclusion. Loaded heap memory, allocation counts, and peak build memory remain unmeasured until a
controlled measurement method is added; the JSON artifact size is measured and is currently large.

Reproduce the baseline with:

```bash
cargo bench -p roadrunner-core --bench graph_lifecycle -- --noplot --warm-up-time 1 --measurement-time 2 --sample-size 20
cargo bench -p roadrunner-core --bench dijkstra -- --noplot --warm-up-time 1 --measurement-time 2 --sample-size 20
cargo bench -p roadrunner-core --bench astar -- --noplot --warm-up-time 1 --measurement-time 2 --sample-size 20
cargo bench -p roadrunner-core --bench graph_traversal -- --noplot --warm-up-time 1 --measurement-time 2 --sample-size 20
```

## Historical Phase 6: A* versus Dijkstra

The implementation that produced this section has been superseded. The results remain immutable
historical evidence; the current `astar` benchmark exercises the revised frozen core described
above.

The deterministic `geographic_backbone_with_northern_dead_ends` dataset places 10% of its nodes
on the only route from source to destination. The remaining 90% are reachable dead ends whose
distance from the source is less than the complete route cost. Dijkstra therefore finalizes every
node, while A* can use geographic direction to avoid finalizing the dead ends.

For the selected cost model and routing context, A* prepares the lower bound:

```text
cost_per_meter = min(edge_cost / Haversine(edge.from, edge.to))
heuristic(node) = cost_per_meter * Haversine(node, destination)
```

Every edge cost is therefore at least the scaled straight-line distance between its endpoints.
Together with the Haversine triangle inequality, this makes the heuristic admissible and
consistent. For travel-time cost, the scale is seconds per meter—the reciprocal of the maximum
observed traversable geographic speed. If the graph has no positive geographic span, the scale is
zero and A* safely uses Dijkstra's search order.

Both algorithms use `DistanceCost`, `BinaryHeap`, `HashMap` best-cost storage, and predecessor
tracking. Graph construction is outside the timed region. A* timing includes the full heuristic
preparation scan. Before measurement, the benchmark asserts that both algorithms return the same
path and optimal cost.

The initial measurement was collected on an Apple M3 MacBook Pro with 16 GB RAM using the
Criterion bench profile and 20 samples per case:

| Nodes | Algorithm | Finalized nodes | Median | p95 | p99 |
| ---: | --- | ---: | ---: | ---: | ---: |
| 1,000 | Dijkstra | 1,000 | 0.121 ms | 0.123 ms | 0.140 ms |
| 1,000 | A* | 100 | 0.174 ms | 0.176 ms | 0.177 ms |
| 10,000 | Dijkstra | 10,000 | 1.242 ms | 1.262 ms | 1.297 ms |
| 10,000 | A* | 1,000 | 1.749 ms | 1.775 ms | 1.784 ms |
| 100,000 | Dijkstra | 100,000 | 17.776 ms | 18.465 ms | 18.971 ms |
| 100,000 | A* | 10,000 | 21.988 ms | 23.876 ms | 26.653 ms |

A* finalized 90% fewer nodes in every case, demonstrating the expected search-space reduction
when geography points toward the destination. Its end-to-end median remained 44.3%, 40.8%, and
23.7% slower at 1K, 10K, and 100K nodes respectively. On this dataset, the search savings did not
offset the current graph-wide lower-bound preparation and per-node Haversine evaluation. That is a
measured optimization opportunity, not evidence that the heuristic is ineffective.

The exact configuration and unrounded percentiles are in
[`../benchmarks/results/2026-09-04-apple-m3-astar-comparison.json`](../benchmarks/results/2026-09-04-apple-m3-astar-comparison.json).

These synthetic results isolate search behavior; they do not establish performance on real road
networks. Memory remains unreported until a controlled allocator or profiler configuration is
available.
