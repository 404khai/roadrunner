# Roadrunner Glossary

Status: Normative for v0
Last updated: 2026-09-01

This glossary defines how Roadrunner uses domain terms. Code, API documentation,
tests, and other design documents should use these meanings consistently.

## Algorithm

The named procedure used to calculate a result, such as Dijkstra or A*. An
algorithm is distinct from the cost model it optimizes.

## Assignment

The decision that associates an order with a rider. An assignment includes the
selected rider, evaluated candidates, score components, feasibility outcomes, and
reason for selection. It is not the same as a delivery, which records execution.

## Bounding box

An axis-aligned geographic rectangle described by minimum and maximum latitude
and longitude. It can filter or validate geographic data; it is not a route.

## Candidate rider

A rider considered for an assignment. A candidate can be feasible or rejected.
Roadrunner retains the reason for rejection so the final decision is explainable.

## Coordinate

A validated WGS 84 latitude/longitude pair in decimal degrees. A coordinate is a
location value; a node is a graph vertex that has a coordinate.

## Cost function

The rule that assigns a non-negative scalar cost to traversing an edge in a
routing context. v0 cost functions minimize either distance in meters or travel
time in seconds. Topology answers where movement is possible; a cost function
answers how expensive that movement is.

## Delivery

The execution record for fulfilling an assigned order. It connects an order,
rider, planned routes, estimated times, actual simulation times, and delivery
status.

## Dispatch

The process of evaluating orders and riders to make assignment decisions. Routing
is an input to dispatch; dispatch is not a shortest-path algorithm.

## Edge

A directed, traversable connection from one graph node to another. An edge has a
stable identity and attributes such as distance and base travel time. A two-way
road is represented by two directed edges. A one-way road is represented only in
the permitted direction.

## ETA

Estimated time of arrival. An ETA must name both its target and reference time.
Roadrunner distinguishes pickup ETA, delivery ETA, and estimated completion time.
Durations are expressed in seconds in v0; adapters may also render timestamps.

## Graph

Roadrunner's internal directed road-network representation: a set of nodes and
edges plus adjacency relationships. The graph is independent of its source format
and independent of a selected routing cost.

## Heuristic

In A*, a lower-bound estimate of remaining cost from a node to the destination.
The heuristic is admissible when it never overestimates the true remaining cost.
When admissibility cannot be established, Roadrunner uses a zero heuristic rather
than risking an incorrect optimal route.

## Node

A vertex in the road-network graph with a stable `NodeId` and coordinate. Nodes
can be origins, destinations, pickup points, drop-off points, or intermediate
intersections. A node is not an arbitrary address or necessarily a physical road
intersection.

## Order

A request to move goods from one pickup node to one drop-off node. It includes
creation and readiness times, optional deadline, priority, capacity requirement,
and lifecycle status. In v0, one order is assigned to at most one rider.

## Path

An ordered sequence of connected graph edges, with its corresponding nodes. A
path is structural. It becomes a route when accompanied by calculation inputs,
costs, and metadata.

## Rider

The delivery resource that can be assigned an order. A rider has a graph location,
availability, capacity, vehicle type, and active orders. v0 assignment considers
only available riders with no active order.

## Road class

A domain classification for an edge, derived from fixture or source road data.
It may influence later speed and access policies. It does not by itself determine
v0 route cost.

## Route

An immutable computed path between an origin node and destination node under a
specific algorithm, cost model, and routing context. It includes ordered nodes and
edges, total distance, selected cost, and calculation metadata. Recalculation
produces a new route.

## Route alternative

A valid route between the same endpoints as the primary route that has a unique
edge sequence and a reported similarity to the primary route. Alternatives are
ordered by the same selected cost and deterministic tie-breaking rules.

## Route cost

The accumulated value minimized by a routing algorithm. Its meaning comes from
the selected cost function; distance cost and travel-time cost must not be compared
as if they shared a unit.

## Routing context

Immutable information available while evaluating a route, separate from graph
topology and edge attributes. v0 context is deterministic; later contexts may
include departure time or traffic state.

## Simulation

A deterministic discrete-event execution of riders, orders, routing, assignment,
and delivery state against simulation time. A simulation advances by processing
queued events and does not need to match wall-clock speed.

## Simulation time

The logical timestamp used to order simulation events. It is not the host's
wall-clock time. Equal timestamps are resolved by stable event sequence number.

## SLA

Service-level agreement: a measurable delivery commitment, commonly represented
by a deadline or maximum duration. An SLA violation occurs when the defined
commitment is missed. v0 records deadlines and late deliveries but does not
optimize assignments using complex SLA penalties.

## Traffic multiplier

A dimensionless factor applied to base travel time. `1.0` means unchanged travel
time; a value greater than `1.0` means slower traversal. v0 stores a validated
default multiplier of `1.0` but does not implement traffic-aware routing.

## Travel time

The duration required to traverse an edge, route, or trip. Base travel time is an
edge attribute; selected route cost equals travel time only when using the
travel-time cost model.

## Trip

A rider's planned or actual movement for operational work. A trip can reference
one or more routes over time. Unlike a route, a trip has execution state and may
continue after a route is recalculated.

## Vehicle type

A rider's mode of transport, such as bicycle, motorcycle, car, or foot. It is
modeled in v0 for future access and speed rules but does not alter v0 routing.

## Visited node

A node removed from the routing priority queue with its best-known cost finalized
under the algorithm's rules. `visited_nodes` is a diagnostic search-effort count,
not the number of nodes in the returned route.
