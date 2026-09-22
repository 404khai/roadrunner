# AGENTS.md

# Roadrunner

> A real-time last-mile routing and delivery optimization engine.

Roadrunner is an engineering-focused routing, dispatch, and logistics optimization system designed to model real-world last-mile delivery networks.

The project should eventually support:

* road-network graph construction
* shortest-path routing
* traffic-aware routing
* time-dependent edge costs
* alternative-route generation
* ETA estimation
* nearest-rider discovery
* rider-to-order assignment
* restaurant/store preparation-time awareness
* multi-order routing
* vehicle routing optimization
* dynamic re-dispatch
* delivery simulation
* historical route replay
* route comparison
* real-time visualization through a web UI

Roadrunner is **not** a Google Maps clone.

The objective is to build the optimization infrastructure that a delivery platform could place on top of a mapping system.

Conceptually:

```text
Orders
   │
   ▼
Dispatch Engine
   │
   ├──────────────┐
   ▼              ▼
Assignment     ETA Engine
   │              │
   └──────┬───────┘
          ▼
    Routing Engine
          │
          ▼
   Road Network Graph
          │
          ▼
OpenStreetMap / routing data
```

The system should eventually answer questions such as:

```text
What is the fastest route from A to B?

What are the best alternative routes?

Which rider should receive this order?

When should a rider leave for the restaurant?

Can this rider carry another order without violating an SLA?

How should 100 orders be distributed across 25 riders?

How does traffic change the optimal route?

Should an active delivery be rerouted?

How much better is Roadrunner than the baseline?
```

---

# 1. Core Engineering Principles

All agents working on Roadrunner must follow these rules.

## 1.1 Correctness before optimization

Never optimize an algorithm whose correctness has not been established.

The progression should generally be:

```text
Correct
↓
Tested
↓
Measured
↓
Profiled
↓
Optimized
```

Do not reverse this order.

---

## 1.2 Algorithms must be visible

Roadrunner is an algorithms and systems project.

Do not hide core functionality behind third-party routing APIs.

External systems such as OSRM or GraphHopper may be used for:

* validation
* benchmarking
* map matching
* generating comparison datasets
* reference implementations

They must not replace Roadrunner's core routing implementation.

Roadrunner must implement its own important algorithms.

---

## 1.3 No premature machine learning

Do not introduce ML simply because the project involves prediction.

Start with deterministic models.

Example:

```text
estimated_travel_time =
distance / expected_speed
```

Then:

```text
estimated_travel_time =
base_time × traffic_multiplier
```

Only introduce ML when:

1. a deterministic baseline exists
2. a dataset exists
3. evaluation metrics exist
4. the ML system can be compared against the baseline

---

## 1.4 No premature distributed architecture

Do not create unnecessary microservices.

Begin as a modular monolith/workspace.

Split services only when there is a clear architectural reason.

---

## 1.5 Benchmark everything important

Roadrunner should make quantitative claims only when backed by reproducible benchmarks.

Never fabricate performance numbers.

Every benchmark must record:

```text
hardware
dataset
graph size
algorithm
configuration
number of runs
median
p95
p99 where applicable
```

---

## 1.6 Maintain explainability

The system should be able to explain decisions.

Instead of:

```json
{
  "rider": "rider_12"
}
```

prefer:

```json
{
  "rider": "rider_12",
  "score": 0.82,
  "estimated_pickup_seconds": 241,
  "estimated_delivery_seconds": 934,
  "restaurant_ready_in_seconds": 300,
  "additional_distance_meters": 412,
  "reason": "Lowest predicted completion time"
}
```

This becomes especially important once optimization becomes sophisticated.

---

# 2. Proposed Technology Stack

## Core routing and optimization

Use:

```text
Rust
```

Reasons:

* performance
* memory safety
* excellent systems programming experience
* strong concurrency primitives
* appropriate for graph-heavy infrastructure
* allows routing benchmarks to become a major part of the project

Primary libraries may include:

```text
serde
serde_json
tokio
axum
tracing
thiserror
anyhow
criterion
rayon
petgraph
geo
geo-types
```

Core routing algorithms should still be implemented manually where educational and architecturally important.

Do not simply call a library's shortest-path function.

---

# 3. Supporting Stack

## API

```text
Rust
Axum
Tokio
Serde
```

## Database

```text
PostgreSQL
PostGIS
```

PostGIS should eventually handle:

* spatial queries
* delivery zones
* rider proximity
* geographic filtering
* route geometry storage

## Cache / ephemeral state

```text
Redis
```

Potential uses:

* current rider locations
* active delivery state
* routing cache
* ETA cache
* dispatch locks

Redis is optional during early phases.

---

## Event streaming

Later phases may use:

```text
Redpanda / Kafka
```

Example events:

```text
order.created
order.ready
rider.location_updated
rider.assigned
delivery.picked_up
delivery.completed
traffic.updated
dispatch.recompute_requested
```

Do not introduce Kafka until the synchronous simulation works.

---

# 4. Machine Learning

Use Python only for the ML/research layer.

Potential stack:

```text
Python
PyTorch
Polars
NumPy
scikit-learn
Jupyter
Matplotlib
```

Possible ML tasks:

```text
travel-time prediction
restaurant preparation-time estimation
ETA calibration
traffic forecasting
delivery-delay prediction
```

The Rust engine should be capable of functioning without the ML layer.

---

# 5. Frontend

The visualization application comes later.

Recommended:

```text
TypeScript
React
Next.js or Vite
MapLibre GL
deck.gl where useful
WebSockets
```

The frontend should eventually visualize:

```text
road graph
active riders
restaurants
customers
orders
selected path
alternative paths
traffic weights
dispatch decisions
delivery timelines
ETA changes
historical route replay
```

---

# 6. Suggested Repository Structure

Start with:

```text
roadrunner/
│
├── AGENTS.md
├── README.md
├── Cargo.toml
├── Cargo.lock
│
├── crates/
│   │
│   ├── roadrunner-core/
│   │   └── src/
│   │       ├── graph/
│   │       ├── routing/
│   │       ├── geo/
│   │       ├── cost/
│   │       └── lib.rs
│   │
│   ├── roadrunner-dispatch/
│   │   └── src/
│   │       ├── assignment/
│   │       ├── scoring/
│   │       └── lib.rs
│   │
│   ├── roadrunner-simulation/
│   │   └── src/
│   │
│   ├── roadrunner-api/
│   │   └── src/
│   │
│   └── roadrunner-cli/
│       └── src/
│
├── ml/
│   ├── notebooks/
│   ├── src/
│   ├── models/
│   └── experiments/
│
├── web/
│   └── README.md
│
├── data/
│   ├── raw/
│   ├── processed/
│   ├── fixtures/
│   └── README.md
│
├── benchmarks/
│
├── scripts/
│
├── docs/
│   ├── architecture.md
│   ├── algorithms.md
│   ├── data-model.md
│   ├── routing.md
│   ├── dispatch.md
│   └── benchmarks.md
│
└── tests/
```

Do not create empty directories purely to imitate this structure.

Create components when their phase begins.

---

# 7. Core Data Model

Design these carefully.

## Node

```rust
pub struct Node {
    pub id: NodeId,
    pub latitude: f64,
    pub longitude: f64,
}
```

## Edge

Conceptually:

```rust
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,

    pub distance_meters: f64,
    pub base_travel_time_seconds: f64,

    pub road_class: RoadClass,

    pub one_way: bool,

    pub speed_limit_kph: Option<f32>,

    pub traffic_multiplier: f32,

    pub reliability_score: f32,
}
```

This is illustrative.

The final representation should consider memory efficiency.

---

## Order

```text
id

pickup_location
dropoff_location

created_at
ready_at

priority

size / capacity requirement

delivery_deadline

status
```

---

## Rider

```text
id

location

availability

capacity

current_orders

vehicle_type

status
```

---

## Delivery

```text
order

assigned_rider

route

estimated_pickup_time

estimated_delivery_time

actual_pickup_time

actual_delivery_time
```

---

# PHASE 0 — Project Specification

## Goal

Freeze the initial scope before implementation.

Create:

```text
docs/spec-v1.md
docs/architecture.md
docs/glossary.md
```

The specification must define:

### MVP

Roadrunner v0 should support:

```text
graph construction
Dijkstra
A*
route cost calculation
alternative-route comparison
basic rider assignment
simulation
benchmarking
HTTP route API
```

Explicitly exclude initially:

```text
machine learning
production traffic feeds
multi-region deployment
Kafka
complex VRP
mobile applications
authentication
billing
production maps UI
```

Define terminology:

```text
node
edge
route
trip
order
rider
dispatch
assignment
ETA
cost function
traffic multiplier
SLA
```

## Testing

No implementation tests yet.

Architecture documents should receive a consistency review.

## Completion condition

Do not proceed until the data model and module boundaries are clear.

---

# PHASE 1 — Rust Workspace Foundation

## Goal

Establish the project.

Create:

```text
roadrunner-core
roadrunner-cli
```

Configure:

```text
cargo fmt
cargo clippy
cargo test
```

Add:

```text
tracing
serde
thiserror
```

Set strict linting.

Create CI for:

```text
formatting
clippy
tests
```

## Deliverable

Running:

```bash
cargo test --workspace
```

must succeed.

No routing algorithm yet.

---

# PHASE 2 — Graph Representation

## Goal

Implement Roadrunner's internal road-network representation.

Implement:

```text
NodeId
Node
Edge
Graph
Adjacency List
```

Start with an adjacency list.

Conceptually:

```text
Node
 ├── Edge → Node
 ├── Edge → Node
 └── Edge → Node
```

Required operations:

```text
add_node
add_edge
remove_edge
neighbors
node_count
edge_count
contains_node
```

Handle:

```text
directed edges
one-way streets
invalid nodes
duplicate edges
disconnected graphs
```

## Tests

Test:

```text
empty graphs
single-node graphs
directed edges
bidirectional edges
cycles
disconnected components
invalid edge insertion
```

## Benchmark

Measure graph traversal overhead on generated graphs.

Do not optimize prematurely.

---

# PHASE 3 — Geographic Foundations

## Goal

Introduce real geographic calculations.

Implement:

```text
Coordinate
BoundingBox
Distance
```

Implement Haversine distance.

Test known coordinate pairs.

Provide:

```rust
fn haversine_distance(a: Coordinate, b: Coordinate) -> Meters
```

Do not scatter raw `f64` values across the application.

Introduce domain types where useful:

```text
Meters
Seconds
KilometersPerHour
```

## Deliverable

Roadrunner should correctly estimate straight-line geographic distance.

---

# PHASE 4 — Cost Model

## Goal

Separate route topology from route cost.

Introduce:

```text
CostModel trait
```

Example:

```rust
pub trait CostModel {
    fn edge_cost(
        &self,
        edge: &Edge,
        context: &RoutingContext
    ) -> f64;
}
```

Implement:

### DistanceCost

```text
cost = distance
```

### TravelTimeCost

```text
cost = base_travel_time
```

This abstraction is critical.

Later:

```text
TrafficAwareCost
TimeDependentCost
MLPredictedCost
```

can exist without rewriting A*.

---

# PHASE 5 — Dijkstra Baseline

## Goal

Implement Dijkstra's shortest-path algorithm manually.

Use:

```text
BinaryHeap / priority queue
HashMap or indexed vectors
predecessor tracking
```

Return:

```text
RouteResult {
    path,
    total_distance,
    total_cost,
    visited_nodes
}
```

Handle:

```text
source = destination
unreachable destination
cycles
disconnected graph
invalid nodes
```

## Tests

Create small deterministic graphs where expected paths are known.

Example:

```text
A --2--> B --2--> D
 \               ↑
  ----10--> C ---1
```

Expected:

```text
A → B → D
```

## Benchmark

Generate graphs of increasing size:

```text
1K nodes
10K nodes
100K nodes
```

Record:

```text
search time
visited nodes
memory usage where practical
```

This becomes Roadrunner's baseline.

---

# PHASE 6 — A* Routing

## Goal

Implement A*.

Use Haversine distance as the geographic heuristic.

Ensure the heuristic is admissible for the selected cost model.

Compare:

```text
Dijkstra
vs
A*
```

Metrics:

```text
same optimal cost
nodes explored
runtime
```

A* must not return a different optimal result from Dijkstra when using an admissible heuristic.

## Benchmark

Produce reproducible results demonstrating when A* reduces search space.

Document findings in:

```text
docs/benchmarks.md
```

---

# PHASE 7 — OpenStreetMap Data Pipeline

## Goal

Move from synthetic graphs to real road networks.

Support importing OpenStreetMap-derived road data.

Build a preprocessing pipeline:

```text
OSM data
   ↓
Parser
   ↓
Road filtering
   ↓
Node normalization
   ↓
Edge generation
   ↓
Roadrunner graph
```

Extract useful properties such as:

```text
road type
one-way restrictions
speed limits where available
road access
coordinates
```

Do not attempt to support every OSM tag initially.

Create a documented supported subset.

## Important

Keep OSM ingestion separate from the routing engine.

Routing should operate on Roadrunner's internal graph representation.

---

# PHASE 7.5 — Maneuver-Aware Routing & Turn Restrictions

## Goal

Make routing aware of legal maneuvers whose validity depends on the incoming
traversal. Enforce the supported turn restrictions preserved during Phase 7
before beginning serious reference-engine route-validity comparison.

## Initial supported subset

Support applicable motorcycle-relevant node-via restrictions that can be
deterministically resolved from imported OSM restriction relations, including:

```text
no_left_turn
no_right_turn
no_straight_on
no_u_turn

only_left_turn
only_right_turn
only_straight_on
```

## Search-state change

Routing may need to move from:

```text
NodeId
```

to conceptually:

```text
(NodeId, IncomingEdgeId)
```

because permitted outgoing traversals can depend on how the current node was
entered.

## Requirements

* consume restriction provenance preserved by Phase 7
* resolve source OSM `from / via / to` members against normalized graph traversals
* preserve deterministic compilation
* reject forbidden maneuvers during routing
* keep unsupported restriction forms diagnosable
* test `no_*` and `only_*`
* test motorcycle-qualified restrictions
* test restrictions interacting with one-way topology
* test route reconstruction with expanded search state
* expose accurate graph capability metadata

Until this phase is complete, graph metadata must continue to report:

```text
turn_restrictions_enforced: false
```

## Deferred

Phase 7.5 does not need to solve every OSM restriction semantic. Complex cases
such as:

```text
multi-way via paths
conditional/time-dependent restrictions
complex vehicle qualification
unsupported relation forms
```

may remain preserved but unsupported, with explicit diagnostics.

## Completion condition

Phase 7.5 is complete only when the supported turn-restriction subset affects
routing correctly and:

```text
turn_restrictions_enforced: true
```

accurately describes that supported capability.

---

# PHASE 8 — Baseline Validation

## Goal

Validate Roadrunner routes against an established routing system.

Possible reference:

```text
OSRM
GraphHopper
```

For a test dataset:

```text
generate coordinate pairs
route using Roadrunner
route using reference engine
compare
```

Measure:

```text
distance difference
travel-time difference
route geometry difference
routing failures
```

Do not expect exact equality because cost models may differ.

Document why differences occur.

Create:

```text
benchmarks/reference-comparison/
```

---

# PHASE 9 — Alternative Routes

## Goal

Return more than one reasonable route.

API concept:

```text
GET /route?
from=A
&to=B
&alternatives=3
```

Response:

```json
{
  "routes": [
    {
      "rank": 1,
      "distance_meters": 6200,
      "eta_seconds": 920
    },
    {
      "rank": 2,
      "distance_meters": 6800,
      "eta_seconds": 970
    }
  ]
}
```

Investigate appropriate algorithms.

Candidates include:

```text
k-shortest paths
Yen's algorithm
penalty-based alternative routing
```

Alternative paths must have meaningful diversity.

Avoid returning three routes that differ by one tiny edge.

Create a route similarity metric.

---

# PHASE 10 — Traffic-Aware Routing

## Goal

Make shortest distance different from fastest route.

Introduce:

```text
traffic_multiplier
```

For example:

```text
edge_time =
base_time × traffic_multiplier
```

Support:

```text
normal
moderate
heavy
severe
```

internally represented numerically.

Example:

```text
Route A
6.2 km
traffic-adjusted ETA: 25 min

Route B
7.4 km
traffic-adjusted ETA: 17 min
```

Roadrunner should select Route B.

## Testing

Construct tests where:

```text
shortest path != fastest path
```

---

# PHASE 11 — Time-Dependent Routing

## Goal

Traffic costs should depend on time.

Replace:

```text
cost(edge)
```

with the ability to calculate:

```text
cost(edge, departure_time)
```

Example:

```text
07:00 → 1.2
08:00 → 1.8
09:00 → 2.3
11:00 → 1.1
17:00 → 2.6
```

Implement deterministic traffic profiles first.

Example:

```rust
TrafficProfile {
    edge_id,
    time_windows
}
```

Route computation must propagate time through the path.

If:

```text
A → B takes 10 minutes
```

then evaluation of:

```text
B → C
```

must occur using the expected arrival time at B, not the original departure time.

Document assumptions around FIFO time-dependent networks.

---

# PHASE 12 — Rider Spatial Index

## Goal

Efficiently locate nearby riders.

Naive:

```text
scan every rider
```

should serve as the baseline.

Then implement spatial indexing.

Candidates:

```text
R-tree
PostGIS spatial index
geohash buckets
```

API:

```text
nearest_riders(location, radius, limit)
```

Return:

```text
rider ID
distance
estimated arrival time
```

Benchmark:

```text
100 riders
1K riders
10K riders
100K riders
```

Compare indexed lookup against linear scanning.

---

# PHASE 13 — Basic Dispatch Engine

## Goal

Assign orders to riders.

Start simple.

For each order:

```text
find nearby available riders
↓
calculate rider → pickup ETA
↓
calculate pickup → customer ETA
↓
score candidates
↓
select best rider
```

Initial scoring:

```text
score =
pickup_eta
+
delivery_eta
```

Return an explanation.

Example:

```json
{
  "assigned_rider": "rider_42",
  "pickup_eta_seconds": 280,
  "delivery_eta_seconds": 920,
  "estimated_completion_seconds": 1200,
  "reason": "Lowest estimated completion time"
}
```

---

# PHASE 14 — Preparation-Time-Aware Dispatch

## Goal

Model restaurant/store readiness.

An order should have:

```text
expected_ready_time
```

Bad assignment:

```text
rider arrival: 5 min
food ready: 18 min

idle time: 13 min
```

Better assignment:

```text
rider arrival: 15 min
food ready: 18 min

idle time: 3 min
```

Change scoring:

```text
completion_time
+
rider_idle_penalty
```

Track:

```text
pickup ETA
wait time
delivery ETA
total completion time
```

Create simulations proving why nearest-rider assignment is not always optimal.

---

# PHASE 15 — Dispatch Simulation Engine

## Goal

Build a deterministic environment for testing optimization strategies.

The simulation should model:

```text
riders
restaurants
customers
orders
road network
traffic
time
```

Support events:

```text
ORDER_CREATED
ORDER_READY
RIDER_MOVED
RIDER_ASSIGNED
RIDER_ARRIVED_PICKUP
ORDER_PICKED_UP
ORDER_DELIVERED
TRAFFIC_CHANGED
```

Implement a priority queue ordered by simulation time.

The simulator should run faster than wall-clock time.

Example:

```bash
roadrunner simulate scenario.json
```

Output:

```text
Orders: 10,000
Delivered: 9,982

Median ETA: ...
p95 ETA: ...

Rider utilization: ...
Mean idle time: ...

Total distance: ...
Late deliveries: ...
```

Use seeded randomness.

Simulations must be reproducible.

---

# PHASE 16 — Dispatch Strategy Benchmarking

## Goal

Compare algorithms.

Implement baseline strategies:

### Strategy A

Nearest rider.

### Strategy B

Lowest pickup ETA.

### Strategy C

Lowest total completion time.

### Strategy D

Preparation-aware assignment.

Compare:

```text
average delivery time
median delivery time
p95 delivery time
rider idle time
distance traveled
late orders
rider utilization
```

Results should be generated automatically.

Never hardcode benchmark conclusions.

---

# PHASE 17 — Multiple Orders Per Rider

## Goal

Allow riders to carry multiple active orders.

Introduce:

```text
capacity
```

Example:

```text
rider capacity = 3
```

Before adding an order, determine:

```text
current route
+
candidate pickup
+
candidate delivery
```

Estimate incremental cost.

Reject combinations violating:

```text
capacity
delivery deadlines
maximum detour
service constraints
```

This introduces route insertion.

Implement simple insertion heuristics before advanced optimization.

---

# PHASE 18 — Vehicle Routing Problem

## Goal

Move from individual rider assignment toward fleet optimization.

Model:

```text
N orders
M riders
```

Objective:

```text
minimize total cost
```

Possible components:

```text
travel time
distance
late deliveries
rider idle time
unassigned orders
SLA violations
```

Subject to:

```text
vehicle capacity
pickup before delivery
time windows
rider availability
maximum route duration
```

Begin with heuristics.

Possible progression:

```text
greedy insertion
↓
local search
↓
2-opt
↓
simulated annealing or tabu search
```

Do not immediately attempt an industrial-grade exact VRP solver.

Benchmark against simpler strategies.

---

# PHASE 19 — Dynamic Re-dispatch

## Goal

Allow optimization decisions to change as the system changes.

Triggers:

```text
new order
traffic change
rider delay
restaurant delay
rider offline
order cancellation
unexpected route delay
```

Architecture:

```text
Event
 ↓
State Update
 ↓
Recompute Candidate Assignments
 ↓
Cost Comparison
 ↓
Keep Existing Plan
       OR
Apply New Plan
```

Prevent excessive route churn.

Introduce:

```text
reroute_penalty
assignment_stability_penalty
```

Roadrunner should not change riders every few seconds just because another solution is marginally better.

---

# PHASE 20 — HTTP API

## Goal

Expose the engine.

Use:

```text
Axum
Tokio
Serde
```

Initial endpoints:

```text
POST /v1/routes
POST /v1/routes/alternatives

POST /v1/orders
GET  /v1/orders/:id

POST /v1/riders
PATCH /v1/riders/:id/location

POST /v1/dispatch
GET  /v1/deliveries/:id

POST /v1/simulations
```

Route request:

```json
{
  "origin": {
    "latitude": 0,
    "longitude": 0
  },
  "destination": {
    "latitude": 0,
    "longitude": 0
  },
  "departure_time": "..."
}
```

Return route geometry in a frontend-friendly format.

Prefer:

```text
GeoJSON
```

where appropriate.

---

# PHASE 21 — Persistence

## Goal

Introduce PostgreSQL/PostGIS.

Persist:

```text
orders
riders
deliveries
routes
dispatch decisions
simulation runs
```

Use PostGIS for spatial data.

Design migrations carefully.

Store important dispatch decisions so they can be replayed later.

---

# PHASE 22 — Event-Driven Runtime

## Goal

Introduce asynchronous system behavior.

Only begin this phase after the synchronous implementation is stable.

Potential architecture:

```text
API
 │
 ▼
Event Stream
 │
 ├── dispatch engine
 ├── ETA engine
 ├── simulation/replay
 └── notification consumers
```

Possible events:

```text
order.created
order.ready

rider.location_updated
rider.available
rider.offline

dispatch.assigned
dispatch.reassigned

delivery.picked_up
delivery.completed

traffic.updated
```

Use:

```text
Redpanda / Kafka
```

only if it provides measurable architectural value.

---

# PHASE 23 — ETA Dataset

## Goal

Prepare Roadrunner for ML.

Create a structured dataset containing:

```text
origin
destination
road segment
distance
road class

departure time
day of week

expected travel time
observed travel time

traffic multiplier

route
```

Simulation-generated data may be used initially but must be clearly marked as synthetic.

Create:

```text
ml/data/
```

and document provenance.

---

# PHASE 24 — ETA Prediction Baseline

## Goal

Predict travel time.

Before neural networks, create baselines.

Examples:

```text
historical mean
linear regression
gradient boosted trees
```

Features may include:

```text
distance
hour
weekday
road type
historical speed
traffic multiplier
```

Metrics:

```text
MAE
RMSE
MAPE where appropriate
```

Document train/test methodology.

Avoid leakage.

---

# PHASE 25 — ML ETA Engine

## Goal

Evaluate whether a learned ETA model improves routing.

Possible model:

```text
PyTorch MLP
```

Do not introduce unnecessarily complex architectures.

Compare against deterministic baseline.

The model should only be integrated if it improves relevant metrics.

Export predictions through a clear interface:

```text
predict(edge, context)
```

The Rust engine should not depend directly on PyTorch internals.

---

# PHASE 26 — Route Visualization Data Contract

## Goal

Prepare the engine for the UI before building the UI.

Every route should optionally expose:

```text
route_id

geometry

nodes

edges

distance

ETA

base ETA

traffic-adjusted ETA

cost breakdown

algorithm used

nodes explored

alternative rank
```

Example:

```json
{
  "route_id": "route_a",
  "algorithm": "astar",
  "distance_meters": 7340,
  "eta_seconds": 1020,

  "cost_breakdown": {
    "base_travel_seconds": 810,
    "traffic_penalty_seconds": 180,
    "turn_penalty_seconds": 30
  },

  "geometry": {
    "type": "LineString",
    "coordinates": []
  }
}
```

This phase is critical for the future UI.

---

# PHASE 27 — Visualization UI Foundation

## Goal

Build the Roadrunner visual debugging environment.

Create:

```text
web/
```

Use:

```text
TypeScript
React
MapLibre GL
```

The initial interface should display:

```text
map
origin
destination
selected route
route statistics
```

No elaborate dashboard yet.

The visualization exists primarily as an engineering tool.

---

# PHASE 28 — Alternative Route Comparison UI

## Goal

Make Roadrunner's algorithms visually understandable.

Display:

```text
Route A
Route B
Route C
```

Each route should have:

```text
distance
ETA
traffic delay
cost
algorithm
```

Users should be able to toggle routes.

Example:

```text
Route A
6.2 km
24 min

Route B
7.4 km
17 min

Route C
8.0 km
18 min
```

Clearly show why Roadrunner selected Route B.

Allow selecting a route to inspect:

```text
individual road segments
edge weights
traffic factors
```

---

# PHASE 29 — Graph Debugger

## Goal

Expose the underlying graph.

Create developer mode.

Allow visualization of:

```text
nodes
edges
edge direction
edge weight
road class
traffic multiplier
```

Selecting an edge should reveal:

```text
edge ID

distance

base travel time

traffic multiplier

current travel cost
```

Allow optional display of:

```text
nodes explored by Dijkstra
nodes explored by A*
```

This would make an excellent visual demonstration of algorithm behavior.

---

# PHASE 30 — Live Delivery Visualization

## Goal

Visualize the dispatch system.

Map entities:

```text
restaurant / pickup
customer / destination
rider
active route
```

Animate rider movement during simulation.

UI should show:

```text
current location
next stop
ETA
assigned orders
capacity
route
```

No fake real-time data.

Use simulation data until an actual data source exists.

---

# PHASE 31 — Dispatch Visualization

## Goal

Make dispatch decisions inspectable.

When an order arrives:

```text
Order
   ↓
Candidate riders
   ↓
Candidate scores
   ↓
Selected rider
```

UI should show candidates.

Example:

```text
RIDER    PICKUP    DELIVERY    WAIT    SCORE

#17      5m        18m         9m      31
#32      7m        14m         1m      22  ← selected
#51      3m        22m         12m     37
```

Clicking a rider should show the hypothetical route if that rider had been selected.

---

# PHASE 32 — Historical Replay

## Goal

Replay previous simulations or dispatch sessions.

Provide:

```text
play
pause
scrub timeline
speed 1x / 2x / 5x / 10x
```

Visualize:

```text
new orders
rider assignment
movement
traffic changes
reroutes
deliveries
```

The replay should be deterministic.

This becomes one of Roadrunner's strongest demo features.

---

# PHASE 33 — Performance Engineering

## Goal

Optimize only after profiling.

Profile:

```text
graph loading
A*
Dijkstra
route reconstruction
nearest rider search
dispatch scoring
VRP heuristics
serialization
```

Investigate:

```text
compact node indexing
structure-of-arrays layouts
allocation reduction
route caching
parallel evaluation
SIMD where justified
```

Do not introduce unsafe Rust without:

```text
documented justification
benchmarks
tests
```

---

# PHASE 34 — Large-Scale Simulation

## Goal

Stress Roadrunner.

Target scenarios such as:

```text
100 riders
1,000 riders
10,000 riders

1,000 orders
10,000 orders
100,000 orders
```

Do not claim production scalability solely from synthetic benchmarks.

Measure:

```text
dispatch latency
routing latency
memory
CPU
throughput
queue backlog
```

Useful service goals may eventually include:

```text
route p50
route p95

dispatch p50
dispatch p95

location updates/sec

orders/sec
```

Set actual thresholds only after baseline measurements exist.

---

# PHASE 35 — Observability

## Goal

Make runtime behavior inspectable.

Use:

```text
structured tracing
metrics
request IDs
simulation IDs
dispatch decision IDs
```

Potential metrics:

```text
routing_requests_total

routing_latency_seconds

nodes_explored

dispatch_latency_seconds

orders_unassigned

active_riders

delivery_eta_error

reroutes_total
```

Never log sensitive user information.

---

# PHASE 36 — CLI

## Goal

Make Roadrunner usable without the UI.

Commands may include:

```bash
roadrunner graph import
roadrunner graph stats

roadrunner route
roadrunner route compare

roadrunner simulate

roadrunner benchmark

roadrunner serve
```

Example:

```bash
roadrunner route \
  --from "6.5244,3.3792" \
  --to "6.6018,3.3515" \
  --alternatives 3
```

Output should be readable and machine-serializable.

Support:

```text
--json
```

---

# PHASE 37 — Benchmark Suite

## Goal

Create a reproducible benchmark artifact for the repository.

Benchmarks should compare:

```text
Dijkstra vs A*

static vs traffic-aware routing

nearest-rider vs ETA assignment

basic dispatch vs preparation-aware dispatch

single-order vs multi-order optimization
```

Generate structured results.

Example:

```text
benchmarks/results/*.json
```

The README may summarize results but must link them to reproducible benchmark definitions.

---

# PHASE 38 — Demo Scenario

## Goal

Create one polished demonstration.

Example:

```text
50 restaurants
500 customers
100 riders
2,000 orders
changing traffic
restaurant preparation delays
```

Run:

```bash
roadrunner demo
```

Then open the visualization.

The demo should show:

```text
orders appearing

riders being selected

alternative routes

traffic changes

reroutes

multi-order deliveries

ETA updates
```

The objective is for someone evaluating the repository to understand Roadrunner within several minutes.

---

# PHASE 39 — Documentation

Create high-quality documentation.

Required:

```text
README.md

docs/
├── architecture.md
├── algorithms.md
├── routing.md
├── traffic.md
├── dispatch.md
├── simulation.md
├── optimization.md
├── machine-learning.md
├── benchmarks.md
└── api.md
```

Include diagrams.

Document mathematical assumptions.

Explain algorithms rather than simply naming them.

---

# PHASE 40 — v1.0

Roadrunner should not be considered v1.0 until it has:

```text
real road-network ingestion

Dijkstra

A*

traffic-aware routing

time-dependent routing

alternative routes

nearest-rider lookup

dispatch engine

preparation-aware assignment

multi-order routing

simulation framework

benchmark suite

HTTP API

visual route comparison

dispatch visualization

historical replay

documentation

CI

tests
```

Optional for v1.0:

```text
ML ETA prediction
Kafka
large distributed deployment
mobile client
```

---

# Testing Strategy

Roadrunner should have multiple test classes.

## Unit tests

For:

```text
graph operations
cost calculations
distance calculations
route reconstruction
dispatch scoring
```

## Algorithm tests

Known graphs with known answers.

## Property tests

Useful properties:

```text
route distance >= 0

source == destination → zero-cost route

A* cost == Dijkstra cost
when heuristic is admissible

every returned edge connects consecutive nodes

pickup occurs before delivery

rider capacity is never exceeded
```

## Integration tests

Examples:

```text
OSM → graph → route

order → dispatch → route

simulation → delivery completion
```

## Regression tests

Every significant routing bug should receive a regression test.

---

# Benchmark Discipline

Every optimization change should answer:

```text
What became faster?

By how much?

On what dataset?

At what memory cost?

Did correctness change?
```

Reject meaningless claims such as:

```text
"significantly faster"
"high performance"
"optimized"
```

without measurements.

---

# Commit Guidelines

Prefer Conventional Commits with detailed PR description.

Examples:

```text
feat(graph): add adjacency-list road network

feat(routing): implement dijkstra shortest path

feat(routing): add A* geographic heuristic

feat(traffic): support time-dependent edge costs

feat(dispatch): add ETA-based rider assignment

feat(simulation): implement deterministic event queue

feat(web): visualize alternative routes

perf(routing): reduce allocations during A* search

test(dispatch): add preparation-time assignment cases

docs(architecture): document routing pipeline
```

Keep commits scoped.

Do not combine unrelated phases into one commit.

---

# Agent Behavior

When an agent receives a request such as:

```text
Implement Phase 6
```

the agent must:

1. Read this AGENTS.md.
2. Read relevant existing documentation.
3. Inspect the current implementation.
4. Confirm earlier phase assumptions from the code.
5. Implement only the requested phase.
6. Add tests.
7. Run formatting.
8. Run linting.
9. Run tests.
10. Add benchmarks where the phase requires them.
11. Update relevant documentation.
12. Summarize architectural decisions and trade-offs.

Do not begin the next phase automatically.

---

# Agent Prohibitions

Agents must NOT:

```text
rewrite working modules unnecessarily

replace Roadrunner algorithms with external API calls

invent benchmark results

add ML without evaluation

add distributed infrastructure prematurely

introduce UI before backend contracts exist

silently change public APIs

silently change cost semantics

skip tests

disable failing tests to make CI green

introduce unsafe Rust without justification

claim production readiness without evidence
```

---

# Architecture Rule

Maintain this dependency direction:

```text
                    roadrunner-api
                         │
                         ▼
                  roadrunner-dispatch
                         │
                         ▼
                    roadrunner-core


                roadrunner-simulation
                     │          │
                     └────┬─────┘
                          ▼
                 dispatch + core
```

The routing core should know nothing about:

```text
HTTP
React
PostgreSQL
Kafka
machine learning frameworks
```

The routing engine should remain independently testable.

---

# Long-Term Architecture

Roadrunner may eventually resemble:

```text
                        ┌─────────────────────┐
                        │     Roadrunner UI   │
                        │                     │
                        │ Map + Simulation    │
                        │ Route Comparison    │
                        └──────────┬──────────┘
                                   │
                              WebSocket/HTTP
                                   │
                        ┌──────────▼──────────┐
                        │    Roadrunner API   │
                        └──────────┬──────────┘
                                   │
                ┌──────────────────┼───────────────────┐
                │                  │                   │
                ▼                  ▼                   ▼
        ┌──────────────┐   ┌───────────────┐   ┌──────────────┐
        │   Dispatch   │   │    Routing    │   │  Simulation  │
        │    Engine    │   │    Engine     │   │    Engine    │
        └──────┬───────┘   └───────┬───────┘   └──────────────┘
               │                   │
               ▼                   ▼
        ┌──────────────┐    ┌───────────────┐
        │ ETA / Cost   │    │  Road Graph   │
        │    Engine    │    │               │
        └──────┬───────┘    └───────┬───────┘
               │                    │
               │                    ▼
               │             OpenStreetMap
               │
               ▼
        Optional ML Models


          PostgreSQL/PostGIS
                 │
                 ▼
               Redis
                 │
                 ▼
         Optional Event Bus
```

---

# What Makes Roadrunner Interesting

Roadrunner should demonstrate that:

> The closest rider is not necessarily the best rider.

> The shortest route is not necessarily the fastest route.

> The fastest route now may not be the fastest route five minutes from now.

> Dispatch and routing are interconnected optimization problems.

> Restaurant readiness matters just as much as rider proximity.

> Optimizing one delivery independently may produce worse fleet-level performance.

That is the engineering thesis of the project.

---

# Final Project Description

Roadrunner should ultimately be describable as:

> **Roadrunner is a real-time last-mile logistics optimization engine written primarily in Rust. It models road networks as time-dependent weighted graphs and combines traffic-aware pathfinding, ETA estimation, rider assignment, preparation-time-aware dispatch, and multi-order vehicle routing to optimize delivery operations. A companion visualization environment allows developers to inspect routes, compare alternatives, simulate fleets, and replay dispatch decisions.**

---

# MVP Success Condition

The first genuinely impressive Roadrunner demo should be able to show:

```text
Order created
       │
       ▼
Candidate riders discovered
       │
       ▼
Estimated completion time calculated
       │
       ▼
Best rider selected
       │
       ▼
Multiple candidate routes generated
       │
       ▼
Traffic-aware route selected
       │
       ▼
Delivery simulated
       │
       ▼
Route visualized
```

And then demonstrate:

```text
Naive strategy

vs

Roadrunner strategy
```

with actual measured differences in:

```text
delivery time
rider idle time
distance
SLA violations
fleet utilization
```

That comparison is more important than any flashy UI.

Build the engine first.

Make it correct.

Make it measurable.

Then make it fast.

Then make it visible.
