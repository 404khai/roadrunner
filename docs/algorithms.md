# Algorithms and mathematical assumptions

Roadrunner implements its routing and dispatch algorithms in the workspace.
External route engines are comparison tools, not the implementation.

## Shortest paths

Dijkstra maintains tentative nonnegative path costs and a minimum-priority frontier
implemented using Rust's binary heap. An improved relaxation records a predecessor
and pushes a new entry; stale popped entries are skipped. Destination settlement
ends the search. Reconstruction follows directed predecessor traversals.

With V node states and E traversals, ordinary search has graph-sized scratch arrays;
heap work is conventionally O((V + E) log(V + E)) with duplicate entries. The
maneuver-aware search distinguishes incoming directed edges: a single node can have
several states, with approximately E incoming states plus the source. Work depends
on legal state transitions, not simply physical node count. These bounds are models,
not measured throughput. See [search implementation](../crates/roadrunner-core/src/routing/search.rs),
[Dijkstra](../crates/roadrunner-core/src/routing/dijkstra.rs) and
[A*](../crates/roadrunner-core/src/routing/astar.rs).

A* orders the frontier using `f = g + h`. Geographic distance is a lower bound only
under the supported distance semantics; travel-time heuristics additionally require
a valid maximum-speed bound and compatible evaluator/traffic assumptions. An arbitrary
heuristic is not admissible merely because it uses Haversine distance. See the
[heuristic implementation](../crates/roadrunner-core/src/routing/heuristic.rs).
Algorithm-specific deterministic tie-breaking remains part of the contract: equal
optimal cost does not authorize replacing one algorithm's selected traversal sequence
with another's. `expanded_states` counts non-stale pops, not unique physical nodes;
source-equals-destination retains the existing count-of-one convention.

Alternative routing performs bounded loopless deviation search with explicit search
termination and diversity selection. A requested count is not a guarantee of that
many distinct feasible routes. Partial selected alternatives follow their declared
termination semantics. See [alternative routing](alternative-routes.md).

## Dispatch timing and feasibility

For a pickup with arrival a and supported readiness r:

```text
service_start = max(a, r)
wait = service_start - a
next_departure = service_start + pickup_service
```

Each subsequent leg is routed at its propagated departure. Capacity is checked over
every prefix: `0 <= onboard_load <= capacity`, pickups precede their deliveries,
and accepted protections remain mandatory. Unknown readiness or geometry does not
become zero cost. [Multi-order evaluation](multi-order.md) defines the supported cases.

Fleet search compares feasible plans lexicographically: most newly admitted orders,
least remaining evaluable road-travel time, least corresponding road distance, then
canonical fleet identity. It does not blend these into a weighted score or promise
global VRP optimality. Submission budgets do not bound all nested routing or candidate
allocation. Incomplete search must not publish where the algorithm prohibits it.
See [optimization](optimization.md) and [fleet contract](fleet-optimization.md).

## Spatial lookup and simulation

Nearby-rider lookup compares an indexed implementation against linear scanning.
Candidate filtering and complete-world validation remain distinct from spatial query
cost. A prebuilt-index query benchmark excludes construction: report both before
claiming end-to-end improvement. See [spatial index](rider-spatial-index.md).

Simulation uses a logical clock and priority queue, not frame-by-frame authoritative
movement. Same-time ordering is deterministic under the scenario contract. Presentation
interpolates retained directed routes between accepted boundaries; a predicted arrival
cannot complete an action. See [simulation](simulation.md).
