# Roadrunner Architecture

Status: Accepted through pre-Phase-7 review
Last updated: 2026-09-19

## 1. Architectural intent

Roadrunner begins as a Rust workspace and modular monolith. Its core algorithms
are pure, deterministic library code. Adapters at the boundary handle files,
HTTP, process configuration, clocks, and presentation.

This shape supports the v0 requirements in [spec-v1.md](spec-v1.md) without
committing the project to services, databases, or event infrastructure before
there is measured need.

The governing dependency direction is:

```text
                         +-----------------+
                         | roadrunner-core |
                         +-----------------+
                           ^             ^
                           |             |
                 +---------+---+    +----+----------+
                 | dispatch    |    | API adapter   |
                 +-------------+    +---------------+
                         ^                   ^
                         |                   |
                 +-------+------+            |
                 | simulation   |            |
                 +--------------+            |
                         ^                   |
                         +---------+---------+
                                   |
                           +-------+------+
                           | CLI/composer |
                           +--------------+
```

Arrows point toward a dependency. No library depends on an adapter, and no cycle
is allowed.

## 2. System context

```text
graph fixture ---> Roadrunner ---> route/assignment/simulation result
                       ^                         |
                       |                         v
             HTTP or CLI request        JSON or human output
```

Before Phase 7, graph fixtures and generated benchmark graphs are trusted local inputs.
Roadrunner has no dependency on a live mapping provider, traffic feed, database,
message broker, or external routing API.

## 3. Workspace boundaries

Crates are created when their implementation phase begins; Phase 1 creates only
`roadrunner-core` and `roadrunner-cli`.

### 3.1 `roadrunner-core`

Owns the stable routing foundation:

```text
geo       Coordinate, units, bounding boxes, Haversine distance
graph     GraphBuilder, FrozenGraph, nodes, segments, directed edges, geometry
cost      traversal evaluation, routing context, objectives, travel time
routing   Dijkstra, A*, route reconstruction, alternatives, RouteResult
```

It must not know about HTTP, CLI parsing, files, databases, orders, riders,
simulation events, or wall-clock time. Core routing algorithms are implemented in
this crate rather than delegated to a routing service or graph library.

### 3.2 `roadrunner-dispatch`

Owns order, rider, assignment, delivery, candidate scoring, and rejection reasons.
It depends on `roadrunner-core` through routing and domain types. It does not own a
second graph representation and does not implement shortest-path algorithms.

The dispatch engine accepts read-only routing capability through a narrow
interface, enabling deterministic fakes in unit tests without hiding production
routing behavior.

### 3.3 `roadrunner-simulation`

Owns simulation time, scenarios, events, event ordering, seeded randomness,
mutable simulation state, and aggregate metrics. It depends on `core` and
`dispatch`. Neither dependency imports simulation types.

Simulation time is distinct from wall-clock time. Equal-time events are ordered
by a monotonically increasing sequence number so a seed and scenario reproduce
the same result.

### 3.4 `roadrunner-api`

Owns Axum routes, HTTP request/response DTOs, JSON serialization, status-code
mapping, request limits, and server lifecycle. It translates external identifiers
and values into validated domain types, calls library APIs, and translates results
back to versioned response types.

The v0 API exposes routing only and depends on `core`. Later order and dispatch
endpoints may add a dependency on `dispatch`, but domain crates never depend on
API DTOs.

### 3.5 `roadrunner-cli`

Owns command-line parsing, configuration loading, graph-fixture selection, output
formatting, tracing initialization, and top-level composition. It may depend on
all application crates. Business rules must not live in command handlers.

## 4. Data ownership

| Concept | Canonical owner | Mutable in v0 | Notes |
| --- | --- | --- | --- |
| Coordinates and units | `core::geo` | No | Validated value types |
| Graph builder | `core::graph` | Yes | Cannot be used for routing |
| Frozen graph snapshot | `core::graph` | No | Validated and densely indexed |
| Traversal evaluators and context | `core::cost` | No | Feasibility, objective, and elapsed time |
| Routes and routing errors | `core::routing` | No | Route results are immutable |
| Orders and riders | `dispatch` | Yes | State transitions are validated |
| Assignment decisions | `dispatch` | No | Includes all candidate explanations |
| Delivery state | `dispatch` | Yes | Simulation drives transitions |
| Event queue and clock | `simulation` | Yes | Never exposed as wall-clock time |
| HTTP DTOs | `api` | Per request | Not imported by domain crates |
| Process configuration | `cli`/binary adapters | At startup | Converted to typed settings |

Identifiers and unit types come from their canonical owner. Other crates must not
create aliases with the same meaning.

## 5. Core interfaces

The examples below describe boundaries, not frozen Rust syntax.

### 5.1 Graph lifecycle and access

Routing consumes only `FrozenGraph`. `GraphBuilder` accepts mutable construction
input and produces a validated, deterministic snapshot. Nodes, road segments,
directed edges, adjacency, and geometry use contiguous indexable storage.

`NodeId`, `RoadSegmentId`, and `EdgeId` are meaningful only inside one graph
snapshot. Durable or detached references pair them with `GraphSnapshotId`.

```text
GraphBuilder -> validate/finalize -> FrozenGraph(snapshot_id)
```

### 5.2 Traversal evaluation

The routing boundary evaluates a transition rather than returning only a scalar:

```rust
enum TraversalEvaluation {
    Traversable {
        objective_cost: RouteCost,
        travel_time: Seconds,
    },
    Forbidden,
}
```

Evaluation errors are distinct from `Forbidden`. Search labels carry objective
cost and elapsed travel time. The supported capabilities are static non-negative
additive routing and FIFO earliest-arrival routing. Evaluators that require
expanded or multi-label state are rejected by node-state algorithms.

Heuristics are explicit policies independent from traversal evaluation. Unknown
or unproved combinations use `ZeroHeuristic`. Custom evaluators accept only that
fallback unless they explicitly declare a stronger named compatibility contract.
Travel-time Haversine parameters are validated once and tied to a graph snapshot.

### 5.3 Graph artifacts

`FrozenGraph` is not directly deserializable. Canonical artifact bytes decode to
an untrusted payload; schema, integrity, build identity, offsets, dense
references, ordering, geometry, capabilities, and numeric domains are validated
before a trusted graph is returned. Build identity records source identity and
integrity, compiler and normalization versions, profile, jurisdiction policy,
and canonical build configuration. Publication writes and synchronizes a
temporary file before atomically renaming it into place.

### 5.4 Routing

```rust
trait Router {
    fn route(
        &self,
        request: &RouteRequest,
    ) -> Result<RouteResult, RoutingError>;
}
```

`RouteRequest` contains snapshot-scoped node identifiers, selected traversal
policy, routing context, algorithm, and compatible heuristic. It does not contain
an HTTP DTO or file format. Alternative-route
generation composes one or more shortest-path searches behind a separate
interface and returns the same canonical `Route` type.

### 5.5 Dispatch routing dependency

Dispatch needs travel-time routes for candidate legs but must not choose an API or
global graph:

```rust
trait RouteProvider {
    fn travel_time_route(
        &self,
        from: NodeId,
        to: NodeId,
        context: &RoutingContext,
    ) -> Result<Route, RoutingError>;
}
```

The application supplies an adapter backed by `roadrunner-core`. Test doubles are
allowed for isolated scoring tests; integration tests exercise the real router.

## 6. Runtime flows

### 6.1 Route request

```text
HTTP/CLI input
  -> adapter syntax and size validation
  -> domain value construction
  -> graph node validation
  -> selected router and cost model
  -> route reconstruction and invariant checks
  -> canonical RouteResult
  -> HTTP/CLI response mapping
```

Input validation failures, absent nodes, no-route outcomes, and internal invariant
violations remain distinct through the flow.

### 6.2 Assignment request

```text
order + riders
  -> reject unavailable riders
  -> route rider to pickup
  -> route pickup to drop-off
  -> reject unroutable candidates
  -> calculate deterministic scores
  -> sort by (score, RiderId)
  -> AssignmentDecision with candidate explanations
```

The common pickup-to-drop-off leg may be reused within one assignment operation.
Cross-request caching is deferred until measurements justify it.

### 6.3 Simulation run

```text
scenario + seed
  -> validate initial graph and entities
  -> enqueue initial events
  -> pop lowest (simulation_time, sequence_number)
  -> apply state transition
  -> call dispatch/routing as required
  -> enqueue resulting events
  -> repeat until queue is empty or configured horizon is reached
  -> derive metrics from recorded outcomes
```

The simulator must not sleep to represent travel. Travel produces a future event.

## 7. Error model

Each library crate exposes typed errors using `thiserror`. `anyhow` is permitted
only at binary composition boundaries where additional process context is useful.

Error categories are:

- validation: malformed coordinate, unit, graph, entity, or scenario;
- not found: an identifier is absent;
- no route: valid nodes are disconnected under the requested model;
- infeasible assignment: no rider satisfies candidate requirements;
- invalid state transition: an event or command violates the entity lifecycle;
- cost failure: a cost is negative, non-finite, or unsupported; and
- invariant violation: internal data is inconsistent and indicates a defect.

The API maps these to stable machine-readable codes and appropriate HTTP status
classes. Internal error details are logged with request context but are not exposed
as implementation leaks.

## 8. Determinism and numerical policy

- Deterministically assigned snapshot-local identifiers break exact-cost ties.
- Simulation sequence numbers break equal-time event ties.
- Randomized scenarios require an explicit seed recorded in their result.
- Floating-point inputs must be finite and within their domain range.
- Costs use a total ordering wrapper only after rejecting NaN and infinities.
- Canonical stored coordinates are fixed-point WGS 84 values with a schema-defined
  scale; geographic calculations use validated `f64`.
- Shortest-path relaxation uses exact finite cost ordering. Tolerances never alter
  heap or relaxation ordering.
- Segment distance is derived from canonical geometry. Validation comparisons may
  use one documented tolerance policy.
- Iteration order from hash-based collections must not determine public results.

## 9. Configuration and state

v0 loads one validated frozen graph snapshot at process startup. The graph is shared read-only across route
requests. Configuration includes graph fixture, listening address, log filter,
and safety limits such as maximum alternatives. Defaults live in adapters and are
documented; domain libraries do not read environment variables.

There is no durable state. Restarting the process discards in-memory simulation,
order, rider, and delivery data. The route endpoint itself is stateless.

## 10. Testing boundaries

| Test class | Primary responsibility |
| --- | --- |
| Unit | Value validation, graph operations, costs, reconstruction, scoring |
| Algorithm | Known graphs with known Dijkstra, A*, and alternative results |
| Property | Oracle agreement, route connectivity/totals, A*/Dijkstra equivalence |
| Integration | Fixture to graph to route; order to assignment to route; simulation completion |
| Regression | A minimal permanent case for every significant fixed defect |
| Benchmark | Reproducible measurements after correctness checks pass |

Adapters receive contract tests for serialization and error mapping. Benchmark
code is not used to assert correctness.

## 11. Dependency and implementation rules

- `serde` may serialize domain values, but external API DTOs remain adapter-owned.
- `tracing` instrumentation belongs at meaningful operation boundaries; libraries
  do not install subscribers.
- `petgraph` may support validation or experiments, but its shortest-path routines
  do not implement Roadrunner's algorithms.
- `tokio` and Axum remain outside `core`, `dispatch`, and deterministic simulation
  logic unless a later measured requirement changes the boundary.
- No `unsafe` code is introduced without tests, documentation, and benchmark
  evidence.
- Crates are not split into services without a concrete operational reason.

## 12. Evolution seams

The architecture deliberately leaves these extension points:

- a staged OSM extractor produces a versioned `NormalizedOsmDataset`, which a
  profile compiler feeds into `GraphBuilder`;
- traffic-aware and FIFO time-dependent models implement the traversal boundary;
- alternative algorithms preserve the canonical route contract;
- spatial indexes feed candidate riders to dispatch without changing scoring;
- persistence adapters store domain records without owning business rules;
- asynchronous events drive the same state transitions used by simulation; and
- ML predictions may become a cost input only after deterministic baselines and
  evaluation datasets exist.

These are seams, not v0 implementation commitments.

## 13. Pre-Phase-7 boundary decision

Phase 7 does not begin until the frozen graph, traversal/search capability model,
explicit heuristic policy, oracle-backed correctness gate, corrected
`expanded_states` diagnostics, and revised synthetic benchmark baseline pass.
Turn restrictions are preserved during Phase 7 but not enforced; Phase 7.5 adds
maneuver-aware routing before serious reference-engine route validation.

The accepted decisions are normative in [adr/](adr/).

## 14. Phase 7 OSM source pipeline

The implemented Phase 7 adapter is documented in
[OpenStreetMap ingestion](osm-ingestion.md), with acceptance evidence in the
[Phase 7 completion report](phase-7-completion.md). `roadrunner-osm` owns staged PBF
extraction, the independently validated normalized dataset artifact, the
versioned delivery-motorcycle and Nigeria policy tables, contraction, and build
diagnostics. `roadrunner-core` remains unaware of OSM types and tags.

Turn-restriction source members and via split points cross this boundary, while
the resulting graph metadata and manifest explicitly report that restrictions
are not enforced. No Phase 7.5 maneuver state or Phase 8 reference-engine
comparison is part of this pipeline.
