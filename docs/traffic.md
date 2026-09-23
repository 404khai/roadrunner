# Static traffic-aware routing (Phase 10)

Roadrunner applies traffic through an immutable, graph-bound `TrafficSnapshot`. A directed edge has a free-flow travel time from OSM compilation; the overlay supplies a factor for that edge:

```text
adjusted_edge_seconds = free_flow_edge_seconds × traffic_multiplier
```

The `TrafficAwareCost` evaluator uses adjusted seconds as both route objective and elapsed travel time. Dijkstra, A*, and the Phase 9 alternative-route search consume it through the existing traversal interface. Static access prohibitions still apply. Distance routing remains a separate objective. The base graph and its artifact are unchanged when a scenario changes; the traffic overlay has its own deterministic digest.

## Factors and validation

Unspecified edges use `1.0`. A `TrafficMultiplier` must be finite and at least `1.0`; invalid or overflowing adjusted times become typed errors. This lower bound preserves the free-flow travel-time Haversine heuristic's admissibility. Named levels are a convenience for deterministic scenarios:

| Level | Factor |
| --- | ---: |
| normal | 1.0 |
| moderate | 1.25 |
| heavy | 1.6 |
| severe | 2.5 |

An overlay is bound to a graph snapshot ID, semantic digest, and edge count. Unknown and duplicate directed-edge overrides fail validation. Input order does not affect the overlay digest. A forward edge's factor does not implicitly apply to its reverse traversal. This overlay contains static values for one calculation. [Phase 11 profiles](time-dependent-routing.md) provide deterministic departure-time costs; production traffic feeds remain outside the current scope.

## Reproduce the pinned fixture

The committed [scenario](../data/fixtures/phase-10/lagos-marina-severe.json) uses the Phase 7 Lagos Marina PBF and puts a severe factor on directed edge `91`. Its graph digest is tied to the declared source ID below.

```sh
cargo run -q -p roadrunner-cli -- osm extract \
  data/fixtures/phase-7/lagos-marina.osm.pbf \
  /tmp/phase10-marina.rr-osm --source-id phase10-lagos-marina
cargo run -q -p roadrunner-cli -- osm compile \
  /tmp/phase10-marina.rr-osm /tmp/phase10-marina-snapshot
cargo run -q -p roadrunner-cli -- route traffic \
  /tmp/phase10-marina-snapshot 5602610872 5594385916 \
  --scenario data/fixtures/phase-10/lagos-marina-severe.json
```

The command emits JSON for the shortest-distance route, the free-flow fastest route, and the traffic fastest route. It includes each route's adjusted ETA plus the graph and traffic digests. For this synthetic fixture, the shortest route is 452.043 m with a traffic-adjusted ETA of 112.904 s; the traffic-aware choice is 527.470 m with an ETA of 94.945 s. These are deterministic calculations on a tiny extract, not observed traffic or production ETA measurements.

Scenario JSON requires `schema_version: 1`, the exact `graph_snapshot_digest`, and an `overrides` array. Each entry has a snapshot-local `edge_id` and a `multiplier` given either as a named level or a numeric factor at least `1.0`. An empty array models free flow.

The CLI's `shortest_distance.route.elapsed_travel_time` is its original free-flow elapsed time, because distance routing uses `DistanceCost`. Compare `traffic_adjusted_eta_seconds` across all three output routes for this scenario. `traffic_fastest.route.elapsed_travel_time` is itself traffic-adjusted because that route uses `TrafficAwareCost`.

## Correctness checks

Tests prove shortest distance can differ from fastest traffic-adjusted travel time; Dijkstra and A* agree on optimal adjusted cost; alternatives can use the same evaluator; forward and reverse factors remain independent; invalid and duplicate inputs fail; overflow propagates as an error; and the committed OSM scenario changes the selected route through the CLI. No routing latency claim is made here.
