# Time-dependent routing (Phase 11)

Roadrunner can evaluate a directed edge at the time a rider reaches it. A `TrafficProfile` holds ordered `(departure_seconds, multiplier)` points for one snapshot-local directed edge. The multiplier is constant before the first point and after the last point, with linear interpolation between points. Edges without profiles use `1.0`. The multiplier applies to free-flow edge traversal time:

```text
edge_entry_time = request_departure_time + elapsed_travel_time_to_edge
edge_travel_time = free_flow_edge_time × multiplier(edge_entry_time)
```

Seconds are nonnegative logical time since a scenario epoch. Profiles do not repeat daily and have no timezone interpretation. Traffic is sampled when entering an edge and held for that traversal; a long traversal does not recalculate its speed mid-edge. Static access rules and compiled turn restrictions still govern whether a maneuver is permitted. Profiles are bound to a graph snapshot and have a deterministic digest independent of profile input order. A profile affects only its named directed edge.

## FIFO requirement

The earliest-arrival Dijkstra and A* searches keep the best arrival label per search state. This is correct when each edge is FIFO: entering it later cannot yield an earlier exit. For an edge with free-flow time `b`, consecutive profile points `(t0, m0)` and `(t1, m1)` must satisfy:

```text
b × (m0 - m1) <= t1 - t0
```

Increasing or flat multipliers always pass. For a falling multiplier, this rule ensures `arrival(t) = t + b × multiplier(t)` never decreases within the linear interval. Constant values outside the points preserve FIFO. The constructor rejects profiles violating the rule, as well as empty profiles, unordered or repeated times, unknown edges, and duplicate edge profiles. Under FIFO, waiting at a node cannot improve arrival time. Multipliers are at least `1.0`, so the free-flow travel-time Haversine heuristic remains a lower bound. Time-dependent alternatives are outside this phase; the existing alternatives search accepts static costs.

## Reproduce the pinned scenario

The [scenario](../data/fixtures/phase-11/lagos-marina-profile.json) uses the committed Lagos Marina PBF. Directed edge `91` has a severe factor until logical second `300`, then eases linearly to normal by second `600`. The scenario is synthetic.

```sh
cargo run -q -p roadrunner-cli -- osm extract \
  data/fixtures/phase-7/lagos-marina.osm.pbf \
  /tmp/phase11-marina.rr-osm --source-id phase11-lagos-marina
cargo run -q -p roadrunner-cli -- osm compile \
  /tmp/phase11-marina.rr-osm /tmp/phase11-marina-snapshot
cargo run -q -p roadrunner-cli -- route schedule \
  /tmp/phase11-marina-snapshot 5602610872 5594385916 \
  --scenario data/fixtures/phase-11/lagos-marina-profile.json --depart 0
cargo run -q -p roadrunner-cli -- route schedule \
  /tmp/phase11-marina-snapshot 5602610872 5594385916 \
  --scenario data/fixtures/phase-11/lagos-marina-profile.json --depart 600
```

At second `0`, the free-flow fastest route takes `112.904` s under the profile, while the chosen route takes `94.945` s. At second `600`, both searches choose the free-flow path and take `81.368` s. These are deterministic fixture calculations, not observed travel times or performance benchmarks.

The scenario JSON requires `schema_version: 1`, an exact `graph_snapshot_digest`, and a `profiles` array. Each profile has an `edge_id` and ordered `points`; each point has `departure_seconds` and a numeric `multiplier` at least `1.0`. The CLI reports both the free-flow fastest route and the scheduled fastest route. `free_flow_fastest.route.elapsed_travel_time` remains free-flow time; its `time_dependent_eta_seconds` reevaluates that route under the profile and propagates arrival times along it. The scheduled route's elapsed time is already adjusted.

Core tests cover edge-entry propagation, route changes with departure time, A* agreement with Dijkstra, malformed profiles, and FIFO rejection. A CLI test checks both departures against the pinned OSM scenario.
