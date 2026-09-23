# Rider spatial lookup (Phase 12)

`roadrunner-core::spatial` provides `LinearRiderLocator` and `IndexedRiderLocator`. Both accept a snapshot of **available** riders and a positive, configured speed. Both expose `nearest_riders(location, radius, limit)` through `RiderLookup`. Results are sorted by increasing great-circle distance, then rider ID, and include the rider ID, distance in meters, and estimated arrival in seconds. A zero limit returns no riders; the radius boundary is inclusive.

```text
estimated_arrival_seconds = haversine_distance_meters × 3.6 / configured_speed_kph
```

The ETA is a straight-line screening estimate. It does not account for roads, traffic, maneuvers, or rider availability changes. Dispatch can use routing later to score the short list. To reflect new locations or availability, build a new locator from current rider positions. IDs must be unique within a snapshot.

## Index and correctness

The linear implementation scans all riders and is the correctness baseline. The indexed implementation uses a balanced median-split tree over 3D unit-sphere coordinates. Each subtree stores a 3D bounding box. A search keeps at most `limit` candidates and prunes boxes farther than the current worst candidate or the requested radius. Exact inclusion and ordering still use the same Haversine calculation as the scan. The sphere representation avoids longitude wrap and polar special cases in pruning. Both implementations produce the same deterministic ordering, including equal-distance ties.

The index is immutable. Building it takes extra time and memory, so it benefits workloads with multiple queries per location snapshot. Phase 12 does not yet establish a rebuild frequency or a real-time update strategy.

## Benchmark

The [benchmark definition](../crates/roadrunner-core/benches/rider_lookup.rs) compares query time at 100, 1K, 10K, and 100K riders. It uses seeded synthetic points in a small Lagos-area rectangle, one fixed origin, a 5 km radius, a 10-rider result limit, and 30 Criterion samples per case. The benchmark checks that results match before measuring and excludes construction time. Run the exact command in [benchmarks/README.md](../benchmarks/README.md). Results are specific to this dataset and query radius; broad radii may visit most of the index.

The [recorded Apple M3 run](../benchmarks/results/2026-09-23-apple-m3-rider-lookup.json) measured these median query times:

| Riders | Linear scan | Spatial index |
| ---: | ---: | ---: |
| 100 | 2.84 µs | 1.14 µs |
| 1,000 | 30.97 µs | 2.60 µs |
| 10,000 | 366.69 µs | 3.88 µs |
| 100,000 | 3.33 ms | 4.72 µs |

The result file also records p95, p99, hardware, sample count, and configuration. These are repeated queries against one static synthetic snapshot, not end-to-end dispatch latency.

Tests compare the index to the scan on seeded global queries and explicit antimeridian, pole, empty, zero-radius, tie, and configuration cases. No road ETA or live-location performance claim follows from these tests.
