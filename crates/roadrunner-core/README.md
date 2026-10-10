# roadrunner-core

The source-independent road-network and routing library. It implements Roadrunner's
own algorithms and knows nothing about orders, rider custody, assignment policies,
simulation queues or OSM element identities.

See the [project README](../../README.md) for setup. All commands below run from the
**repository root**, using Rust 1.99.0 and the committed lockfile.

## Modules and public interfaces

| Module | Main interfaces | Purpose |
| --- | --- | --- |
| `geo` | `Coordinate`, `CanonicalCoordinate`, `BoundingBox`, `Meters`, `Seconds`, `KilometersPerHour`, `haversine_distance` | Validated coordinates/units and physical calculations |
| `graph` | `GraphBuilder`, `FrozenGraph`, node/segment/edge IDs, graph metadata and artifact encode/decode | Deterministic construction, directed traversal, geometry, identity and legal maneuvers |
| `cost` | `TraversalEvaluator`, `RoutingContext`, `DistanceCost`, `TravelTimeCost` | Separate objective cost from road topology and elapsed travel |
| `cost` | `TrafficSnapshot`, `TrafficAwareCost`, `TrafficProfile`, `TimeDependentTrafficSnapshot`, `TimeDependentCost` | Immutable static and FIFO time-dependent directed-edge overlays |
| `routing` | `dijkstra`, `astar`, Haversine/zero heuristics, `RouteResult` | Manual shortest-path search and validated reconstruction |
| `routing` | `alternatives`, `AlternativeRouteOptions`, `AlternativeTermination` | Static-cost loopless alternatives with diversity and explicit budgets |

Construct source facts with `GraphBuilder`, finalize to a `FrozenGraph`, choose a cost
model and `RoutingContext`, then call a routing function. OSM graphs are supplied by
[roadrunner-osm](../roadrunner-osm/README.md). Decode `graph.rr-graph` with
`decode_graph_artifact` to validate an existing artifact; a complete OSM snapshot also
needs the provenance/manifest verification provided by the OSM loader.

Builder IDs, snapshot-local graph IDs and source OSM IDs belong to different identity
domains. Route results carry graph identity and directed traversals; callers must not
reuse edge/node IDs across unrelated snapshots. Turn legality may depend on incoming
edge, so routing search tracks expanded maneuver state.

A* must use a heuristic admissible for its chosen objective. Traffic multipliers are
at least one. Time-dependent profiles must satisfy FIFO and propagate the actual edge
entry time; alternative-route enumeration currently supports static nonnegative costs.
See [architecture](../../docs/architecture.md), [alternatives](../../docs/alternative-routes.md),
[traffic](../../docs/traffic.md), and [time-dependent routing](../../docs/time-dependent-routing.md).

## Build, test and inspect API docs

```bash
cargo +1.99.0 build -p roadrunner-core --locked
cargo +1.99.0 test -p roadrunner-core --all-features --locked
cargo +1.99.0 test -p roadrunner-core --test routing_correctness --locked
cargo +1.99.0 test -p roadrunner-core --test traffic_routing --locked
cargo +1.99.0 doc -p roadrunner-core --no-deps --locked
```

Open `target/doc/roadrunner_core/index.html`. This crate is a library; it has no
standalone `cargo run` binary. To route committed OSM data, use the
[CLI snapshot/routing workflow](../roadrunner-cli/README.md).

## Benchmarks

```bash
cargo +1.99.0 bench -p roadrunner-core --bench graph_traversal --locked
cargo +1.99.0 bench -p roadrunner-core --bench graph_lifecycle --locked
cargo +1.99.0 bench -p roadrunner-core --bench dijkstra --locked
cargo +1.99.0 bench -p roadrunner-core --bench astar --locked
```

| Target | Measures |
| --- | --- |
| `graph_traversal` | Frozen adjacency traversal on generated graphs |
| `graph_lifecycle` | Builder construction, finalization, graph encoding and validated loading |
| `dijkstra` | Shortest-path baseline on deterministic ring-lattice graphs |
| `astar` | Dijkstra/A* comparison with geographic backbone and dead-end branches |

Add `-- --test` to one target for a smoke run. Full Criterion measurements write to
`target/criterion`; graph construction and measured regions are defined by each target.
See [benchmark discipline/results](../../docs/benchmarks.md) before comparing machines
or interpreting visited-node improvements as latency/scalability claims.

## Dependency boundary

Dependencies cover serialization, hashes, tracing and typed errors. No third-party
routing API implements the core search. Downstream workspace crates use this package
as a path dependency; it is currently marked `publish = false`.

## Pre-Phase-20 foundation

Graph artifact schema 4/compiler v4 verify canonical compiled semantic identities;
older identity claims remain historical. Dispatch OperationalState owns coherent
volatile World/execution, distinct plan/action/schedule/effect identities and atomic
publication. Simulation drives shared validated transitions and declares its
`shared-execution/v2` evidence contract. Named static-road temporal certification
supports delayed publication; time-dependent operational certification is unsupported.
HTTP, persistence and event transport remain separate future phases.

See [runtime boundary](../../docs/runtime-boundary.md) and
[prerequisite verification](../../docs/pre-phase-20-remediation-completion.md).

```bash
cargo +1.99.0 test -p roadrunner-core --test semantic_identity --locked
cargo +1.99.0 test -p roadrunner-dispatch --test operational --locked
```
