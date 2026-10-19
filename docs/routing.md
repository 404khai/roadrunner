# Routing

Routing consumes a verified graph snapshot, graph-local directed anchors, a supported
vehicle/access evaluator, an objective, and explicit traffic/departure context.
Graph-local IDs are not portable across snapshots. A geometry match alone does not
establish graph or evaluation identity.

```text
verified graph + request context
             |
      traversal evaluator
             |
      Dijkstra / A* / alternatives
             |
      directed reconstruction
             |
      route evidence + geometry
```

The graph uses dense indexing, contiguous adjacency and shared physical-segment
geometry. Directed traversal and incoming-edge maneuver restrictions must survive
reconstruction. Unsupported OSM semantics remain diagnosable, rather than being
claimed as fully enforced. See [OSM ingestion](osm-ingestion.md) and
[turn restriction completion](phase-7.5-completion.md).

[Algorithms](algorithms.md) explains search, complexity and admissibility.
[Traffic](traffic.md) separates free flow, static factors and supported FIFO
[time-dependent routing](time-dependent-routing.md). Time-dependent evaluation
propagates arrival time through each leg; it does not evaluate every edge at the
original request departure. FIFO support is a model assumption, not a claim about
arbitrary real traffic.

Use [CLI routing workflows](../crates/roadrunner-cli/README.md) or the
[HTTP API](api.md). [Visualization contracts](visualization-contract.md) preserve
route/evaluation identity and distinguish objective cost, elapsed time and geometry.
[Alternative routes](alternative-routes.md) exposes diversity and incomplete-search
termination explicitly. No-route, unsupported context, invalid input and incomplete
search are different outcomes.

## Performance and reuse

Search currently incurs graph-sized per-query scratch allocation even when little
of the graph is explored. Concurrent requests can multiply that memory. Immutable
shared graph storage does not make query scratch shared. Evaluate allocation and
worker admission along with latency; do not infer capacity from a tiny fixture.

Reuse must preserve graph identity, directed anchors, profile/evaluator, objective,
algorithm and tie-breaking, heuristic, traffic snapshot, departure and applicable
search options. A cache hit cannot report original expansions as fresh request work
or bypass current assignment publication checks. There is no blanket shared-route
cache guarantee. [Performance evidence](phase-33-completion.md) distinguishes measured
changes from remaining hypotheses, and [benchmarks](benchmarks.md) supplies reproduction.
