# Roadrunner v0 Specification

Status: Accepted; preparation-aware dispatch implemented through Phase 14
Last updated: 2026-10-04

## 1. Purpose

Roadrunner v0 is a deterministic, explainable baseline for last-mile routing and
single-order rider assignment. It proves the core algorithms and their boundaries
with real road fixtures and deterministic FIFO traffic profiles already implemented.
Persistence, production traffic, and distributed runtime remain deferred.

Roadrunner is optimization infrastructure that can sit behind a mapping product.
It is not a geocoder, turn-by-turn navigation client, or Google Maps replacement.

The terms in [glossary.md](glossary.md) are normative for this specification.

## 2. Goals

The v0 implementation must:

- construct and validate a directed road-network graph;
- calculate route costs independently from graph topology;
- implement Dijkstra's algorithm and A* within Roadrunner;
- return the same optimal cost from Dijkstra and A* when the A* heuristic is
  admissible for the selected cost model;
- compare a primary route with reasonable alternative routes;
- route with deterministic graph-bound static and FIFO traffic profiles;
- assign one available rider to one order using a documented deterministic score;
- run reproducible, faster-than-wall-clock delivery simulations;
- expose routing through an HTTP API and a command-line interface;
- benchmark important algorithms without hard-coded performance claims; and
- explain route and assignment outcomes using structured result data.

## 3. Non-goals

The following are explicitly excluded from v0:

- machine-learning models or learned cost functions;
- production or continuously updated traffic feeds;
- non-FIFO and multi-label time-dependent routing;
- multi-order assignment, route insertion, or vehicle routing optimization;
- dynamic re-dispatch;
- real road-network ingestion beyond small, documented fixtures;
- multi-region or microservice deployment;
- Kafka, Redpanda, or another event broker;
- PostgreSQL, PostGIS, Redis, or durable application state;
- mobile applications;
- authentication, authorization, accounts, or billing; and
- a production maps or delivery-operations UI.

OSRM, GraphHopper, and similar systems may later validate results, but v0 does
not call them to perform Roadrunner's routing work.

## 4. Users and use cases

The initial users are Roadrunner developers and evaluators. They need to:

1. load a small graph fixture or construct a graph in code;
2. request the lowest-cost route between two graph nodes;
3. compare Dijkstra and A* for correctness and explored-node count;
4. request a bounded set of alternative routes and compare their costs;
5. submit an order and candidate riders to receive an explainable assignment;
6. run a seeded simulation and inspect aggregate delivery metrics; and
7. reproduce routing benchmarks on a named dataset and configuration.

Coordinate-to-road snapping, address lookup, navigation instructions, and live
rider tracking are outside these use cases.

## 5. Functional requirements

### 5.1 Graph construction

Roadrunner constructs a directed graph through `GraphBuilder` and publishes a
validated immutable `FrozenGraph`. It must support:

- adding nodes during construction and assigning deterministic dense `NodeId` values at finalization;
- adding directed edges between existing nodes;
- representing a two-way road as two directed edges;
- rejecting edges whose endpoints are absent;
- an explicit, documented policy for duplicate edges;
- builder-time mutation without live mutation of a published snapshot;
- retrieving outgoing neighbors;
- node and edge counts; and
- disconnected graphs and isolated nodes.

Parallel directed edges are allowed. `NodeId`, `RoadSegmentId`, and `EdgeId` are
snapshot-local; durable references carry the full `GraphSnapshotDigest`
(represented by the core digest string).

### 5.2 Geographic and unit types

Coordinates use WGS 84 latitude and longitude. Public/calculation boundaries use
validated decimal degrees; canonical source and graph geometry uses a
schema-declared fixed-point representation, initially E7 for supported standard
OSM extracts.

The core must provide domain types for at least:

- `Meters`;
- `Seconds`;
- `KilometersPerHour`; and
- `Coordinate`.

Haversine distance is the v0 straight-line distance calculation. Public domain
models must not use unlabelled `f64` values for distances or durations.

### 5.3 Cost models

Graph topology and traversal policy remain separate. A traversal evaluator
returns either `Forbidden` or a traversable result containing objective cost and
elapsed travel time. Evaluation failures remain errors. v0 includes:

- `DistanceCost`, measured in meters; and
- `TravelTimeCost`, measured in seconds.

Every traversable edge cost must be finite and non-negative. Dijkstra and A* must
reject an invalid cost instead of silently producing a route.

Search/evaluator capabilities explicitly distinguish `StaticNonNegative` from
`FifoEarliestArrival`. Unsupported expanded-state or multi-label problems are
rejected. Static evaluators remain deterministic and simple.

### 5.4 Shortest-path routing

Dijkstra and A* must be implemented manually using Roadrunner's graph and cost
model abstractions. Calling a third-party shortest-path implementation is not an
acceptable substitute.

Both algorithms accept a graph, source node, destination node, cost model, and
routing context. Both return a `RouteResult` containing:

- ordered node and edge identifiers;
- total distance;
- total selected cost;
- number of expanded search states; and
- algorithm identifier.

Required behavior:

- source equal to destination returns a zero-cost route containing that node;
- an unknown source or destination returns a typed error;
- an unreachable destination returns a typed `NoRoute` outcome;
- cycles do not cause non-termination; and
- every returned edge connects its adjacent nodes in order.

A* receives an explicit heuristic compatible with the traversal objective and
search capability. Haversine is admissible for distance when graph construction
proves the segment-distance invariant. A travel-time heuristic divides Haversine
distance by a documented maximum possible profile speed. Unknown combinations
use `ZeroHeuristic`; A* never derives a heuristic by scanning edge costs.

When multiple equal-cost paths exist, results must be deterministic. The router
breaks ties by stable node or edge identifier ordering, and tests document the
chosen path.

### 5.5 Alternative-route comparison

Roadrunner must return up to a caller-specified, bounded number of loopless routes.
Every returned alternative must:

- connect the requested source and destination;
- be valid under the same cost model and routing context;
- be ordered by total cost with deterministic tie-breaking;
- contain a unique edge sequence; and
- include a similarity value relative to the primary route.

Route similarity is the distance shared by both routes divided by the shorter
route's distance. The value is in `[0, 1]`. The exact candidate-generation
algorithm and initial maximum similarity threshold will be decided and documented
with Phase 9 tests; these details do not cross the routing module boundary.

### 5.6 Basic rider assignment

Basic Dispatch evaluates one new order against coherently idle riders. This is a
policy restriction, not a permanent limit on rider plans or responsibility.

```text
DispatchSnapshot -> world validation -> BasicDispatch eligibility
 -> candidate generation -> Pickup(A), Dropoff(A) candidate plan
 -> feasibility -> plan evaluation -> baseline ranking
 -> immutable AssignmentDecision -> explicit all-or-nothing commit
```

Eligible riders are operationally available, with no active committed responsibility,
no onboard custody, and an empty effective remaining plan. Contradictory assignment,
plan, or custody facts are invalid world state, not ordinary ineligibility.
Scalar order demand must fit rider capacity; unsupported routing profiles are rejected.
Exhaustive eligible-rider generation has Complete coverage. Spatial radius/limit
shortlisting is PotentiallyIncomplete: a winner is best among evaluated candidates,
and failure cannot claim fleet-wide infeasibility.

The baseline score is exactly pickup road travel plus delivery road travel, ordered
by finite exact score then lower RiderId, without epsilon. Deadline treatment is
SoftObserved: lateness uses completed dropoff, is recorded, and changes neither
feasibility nor score. Phase 13 uses zero waiting and service durations while retaining
the general leg -> arrival -> wait -> service -> departure timeline.

Completed decisions are Assigned or Unassigned, distinct from EvaluationError.
Structured evidence retains source world version, instant, graph/traffic/profile,
policy configurations, coverage, canonical candidate metrics or typed rejections,
and the exact proposal and tie reason. Routes and ETA are evaluation details.
Commit compares source version and exact expected assignment/plan, rechecks invariants,
and atomically applies responsibility and plan or rejects with zero mutation.

Phase 13 usage and verification are documented in [dispatch.md](dispatch.md) and
[the completion report](phase-13-completion.md).

#### Preparation-aware dispatch (Phase 14)

Preparation-aware dispatch uses the same idle eligibility, candidate coverage,
feasibility, immutable decisions, and exact assignment/plan commit. Effective readiness
is the observed ready instant, otherwise the expected instant from OrderReadiness.
Missing both is an evaluation error, not an implicit ready state. Pickup waiting is
`max(effective_ready_at - pickup_arrival, 0)`; the delivery leg departs after that waiting
with zero service, including when using FIFO time-dependent traffic.

The objective is elapsed completed-delivery time since evaluation plus weighted pickup
waiting. The finite non-negative idle penalty weight defaults to 1.0; zero minimizes
completion duration while preserving physical waiting. A positive weight may trade later
completion for less rider idle time. Exact score/RiderId ordering and SoftObserved
deadlines remain unchanged. Evidence records original readiness facts, effective source
and instant, weight, completion duration, waiting, and penalty contributions.

Readiness updates invalidate outstanding decisions through world versioning. Forecasts
never authorize actual pickup; shared transitions require recorded readiness. The Phase 13
zero-wait road-travel baseline remains independently callable. Deterministic comparisons
execute fresh equivalent worlds against identical external ready events and distinguish
predictions from recorded execution. See [dispatch.md](dispatch.md) and
[ADR 0014](adr/0014-preparation-aware-dispatch.md).

### 5.7 Simulation

The simulator is a deterministic, discrete-event system. It must:

- process events in simulation-time order using a priority queue;
- use a stable sequence number to order events with the same timestamp;
- accept an explicit random seed when randomness is used;
- run independently of wall-clock time; and
- produce machine-readable results.

The v0 event set is limited to order creation, rider assignment, rider arrival at
pickup, order pickup, and order delivery. Required summary metrics are total,
assigned, delivered, unassigned, and late orders; median and p95 delivery time;
total distance; and rider utilization. A metric without enough observations must
be reported as unavailable, not invented.

### 5.8 HTTP route API

The v0 server exposes:

```text
GET  /health
POST /v1/routes
```

`POST /v1/routes` accepts graph node identifiers, algorithm, cost model, and an
optional alternatives count. It returns the primary route, alternatives when
requested, cost breakdown, and routing metadata. Graph loading is process-level
configuration; the request does not upload an arbitrary graph.

The API uses JSON and stable error codes. It holds no durable order, rider, or
delivery state. Coordinate-based routing and GeoJSON geometry are deferred until
road ingestion and snapping semantics exist.

### 5.9 CLI

The CLI composes the same library APIs as the HTTP adapter. The initial command
surface is:

```text
roadrunner route
roadrunner route compare
roadrunner simulate
roadrunner benchmark
roadrunner serve
```

Commands that emit results must support a machine-readable JSON form.

### 5.10 Benchmarking

Benchmarks must use reproducible generated graphs or versioned fixtures. Each
recorded result includes:

- hardware and operating system;
- dataset name and provenance;
- graph node and edge counts;
- algorithm and configuration;
- number of runs; and
- median, p95, and p99 where applicable.

Initial routing benchmarks compare Dijkstra and A* on 1K, 10K, and 100K-node
generated graphs. Benchmarks never replace correctness tests, and documentation
must not claim improvements without stored results.

## 6. Domain model

### 6.1 Identity and units

`NodeId`, `EdgeId`, `OrderId`, `RiderId`, `DeliveryId`, and `RouteId` are distinct
opaque types. They are not interchangeable strings or integers. Distance and time
values use the domain types defined in section 5.2.

### 6.2 Node and edge

```rust
struct RoadSegment {
    id: RoadSegmentId,
    endpoints: (NodeId, NodeId),
    geometry: GeometryRange,
    distance: Meters,
}

struct DirectedEdge {
    id: EdgeId,
    segment: RoadSegmentId,
    orientation: Orientation,
    effective_free_flow_speed: KilometersPerHour,
    free_flow_travel_time: Seconds,
    access: AccessClass,
}
```

A `RoadSegment` is a physical corridor with canonical geometry stored once. A
`DirectedEdge` is a potentially permitted traversal. Directionality compiles into
edge existence; contextual access remains evaluator input. Segment distance is
derived by summing Haversine distance across canonical geometry. Free-flow travel
time is a deterministic profile estimate, not a legal limit or live ETA.

### 6.3 Order and fulfillment

`Order` owns request facts: OrderId, validated pickup/dropoff Coordinates,
DispatchInstant creation/deadline, and scalar CapacityUnits demand. Mutable readiness
estimates, actual readiness observations, fulfillment progress, and custody are
separate records keyed by OrderId. Request amendments, if later supported, are
explicit domain transitions; dispatch does not mutate request facts.

One CapacityUnit is one abstract normalized reference-parcel slot. Caller/scenario
inputs express all demand and maximum capacity in this same unit; an order can consume
several slots. These integers do not imply kilograms, volume, or physical package count.
Zero capacity/demand is representable; custody remains an explicit fulfillment fact.
Priority, nonzero service constraints, skills, and specialized cargo are outside the
initial supported request contract and are not silently stored or ignored.

### 6.4 Rider and remaining work

`RiderProfile` owns RiderId, routing profile, and maximum scalar capacity.
`RiderState` owns coordinate and operational availability. Responsibility is a
CommittedAssignment (OrderId -> RiderId); intended work is an ordered RiderPlan
of logical Pickup/Dropoff stops. Plans can contain multiple orders; Phase 13
eligibility restricts new assignment to an idle rider.

AwaitingPickup assigned work requires exactly one pickup before exactly one dropoff.
PickedUp custody requires no pending pickup and exactly one dropoff with the same
rider. Delivered work has no remaining stops or active responsibility. Current load
is derived from custody demand, never active-order count; plan validation checks
scalar load after each stop. No handoff model exists.

Stable coordinates are independent of node-backed RoutingAnchors. Each anchor binds
a location fact to the pinned graph digest and node; callers supply anchors until
snapping exists. Missing, mismatched, or absent-node anchors are evaluation errors;
valid anchors with no legal path yield candidate NoRoute infeasibility.

DispatchInstant is distinct from Seconds durations. The caller supplies now and a
scenario RoutingEpoch; checked conversion supplies each propagated leg departure.
Every leg uses the same graph, traffic snapshot, and supported profile, and returned
provenance is checked. No live stores or clocks enter DispatchSnapshot.

### 6.5 Route and trip

A `Route` is an immutable computed path and its calculation metadata. A `Trip` is
the planned or actual movement of a rider using zero or more route computations.
Recomputing a path creates a new route; it does not mutate historical route data.

```rust
struct Route {
    id: RouteId,
    algorithm: RoutingAlgorithm,
    nodes: Vec<NodeId>,
    edges: Vec<EdgeId>,
    total_distance: Meters,
    total_cost: RouteCost,
    expanded_states: usize,
}
```

For a non-empty route, `nodes.len() == edges.len() + 1`. A zero-length route has
one node and no edges.

### 6.6 Assignment and delivery

An immutable AssignmentDecision is evidence plus an exact proposal, not current
responsibility. CommittedAssignment and RiderPlan are updated together through shared
validated domain transitions. Pickup consumes the pending pickup, establishes custody;
delivery consumes the dropoff and clears active responsibility. Simulation later calls
these same operations. Delivery records summarize execution, not fundamental planning.

Simulation compares strategies with identical fixed exogenous inputs and measures
observed execution separately from predicted plan metrics. Future insertion evaluates
whole-plan deltas. Fleet planning, PlanId/PlanVersion, acceptance, handoffs, and churn
policy are deferred. See ADRs 0011–0013 for the normative evolution boundaries.

## 7. Quality requirements

### 7.1 Correctness

- Core algorithms have deterministic example and generated tests.
- An independent oracle and route validator cover optimality, connectivity,
  reconstructed totals, zero-cost behavior, forbidden edges, numeric failures,
  and A*/Dijkstra equivalence under compatible heuristics.
- Every fixed routing or dispatch bug receives a regression test.
- Unsafe Rust requires a documented need, tests, and benchmark evidence; v0 is
  expected to need none.

### 7.2 Explainability

Routing results identify the algorithm, traversal policy, total cost, distance,
elapsed time, and expanded states. Durable results include graph snapshot identity.
Dispatch results preserve candidate scores and rejection reasons.
Simulation results identify their scenario and seed.

### 7.3 Observability

Library crates do not choose a global subscriber. Binary crates configure
structured `tracing` output. Request and simulation identifiers accompany their
respective spans. Sensitive user data does not exist in v0 fixtures or logs.

### 7.4 Performance

v0 has no invented latency or throughput targets. Baselines are measured first.
An optimization is acceptable only after correctness tests pass and a reproducible
benchmark demonstrates its effect and trade-offs.

## 8. Acceptance criteria

Phase 0 is complete when:

- this specification, [architecture.md](architecture.md), and
  [glossary.md](glossary.md) agree on scope and terminology;
- data ownership and crate dependencies are unambiguous;
- the graph, routing, route, order, rider, assignment, delivery, and simulation
  models have stated invariants; and
- unresolved implementation choices are recorded without violating a module
  boundary or blocking Phase 1.

The overall v0 is complete only when all goals in section 2 are implemented,
tested, and benchmarked. Phase 1 must not add a routing algorithm; it establishes
the Rust workspace, linting, CI, and the `core` and `cli` crate foundations.

## 9. Deferred decisions

The following choices belong to later phases and do not block the workspace
foundation:

- exact physical encoding of the accepted dense frozen graph;
- the alternative-route candidate algorithm and calibrated diversity threshold;
- compact/binary graph artifact encoding, mmap, and zero-copy loading (the
  initial canonical JSON encoding and validating load boundary are implemented);
- PBF implementation details and the supported tag matrix;
- persistence schemas;
- non-FIFO and multi-label time-dependent routing; and
- frontend route geometry contracts.
