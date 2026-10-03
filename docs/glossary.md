# Roadrunner Glossary

Status: Normative for v0
Last updated: 2026-10-03

This glossary defines how Roadrunner uses domain terms. Code, API documentation,
tests, and other design documents should use these meanings consistently.

## Algorithm

The named procedure used to calculate a result, such as Dijkstra or A*. An
algorithm is distinct from the cost model it optimizes.

## Assignment

Current committed OrderId -> RiderId responsibility. It is distinct from an immutable
AssignmentDecision, which proposes an exact responsibility and RiderPlan change and
retains structured evaluation evidence. Commit applies both together or rejects.

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

The rule that assigns the non-negative objective contribution minimized by a
routing algorithm. It is one part of traversal evaluation and is distinct from
elapsed travel time unless the objective explicitly minimizes arrival time.

## Delivery

The execution of an order, ending at completed dropoff. Execution records are not
the fundamental planning result; RiderPlan describes intended remaining logical work.

## Dispatch

The process of evaluating orders and riders to make assignment decisions. Routing
is an input to dispatch; dispatch is not a shortest-path algorithm.

## Edge

A snapshot-local directed, potentially permitted traversal of a road segment. A
two-way road normally has two directed edges sharing one segment. Static
directionality is represented by edge existence; contextual or dynamic access may
still make an existing edge forbidden for a request.

## Expanded state

A search-state expansion event in Dijkstra or A*. A state may be expanded more
than once under algorithms that permit reopening. This replaces the ambiguous
term `visited_nodes`.

## Frozen graph

An immutable, validated, densely indexed graph snapshot produced by deterministic
builder finalization or validated artifact loading.

## Graph snapshot ID

A compact in-process convenience derived from the full graph digest. Durable or
detached graph-element references carry the authoritative GraphSnapshotDigest.

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

A vertex in one road-network snapshot with a snapshot-local `NodeId` and
coordinate. Nodes
can be origins, destinations, pickup points, drop-off points, or intermediate
intersections. A node is not an arbitrary address or necessarily a physical road
intersection.

## Normalized OSM dataset

A deterministic, versioned, profile-independent routing-source artifact produced
from staged PBF extraction. It is neither a complete OSM mirror nor a compiled
vehicle routing graph.

## Graph snapshot digest

The authoritative collision-resistant identity of exact compiled routing
semantics. It binds canonical graph and provenance content plus semantic source,
schema, profile, policy, compiler, configuration, and capability metadata.
Snapshot-local IDs are durable only when paired with this full digest; the
derived 64-bit snapshot ID is an in-process convenience.

## Order

Fulfillment request facts: identity, validated pickup/dropoff coordinates, creation,
optional deadline, and scalar capacity demand. Mutable expected/actual readiness,
fulfillment progress, custody, and planning state are separate records keyed by OrderId.

## Path

An ordered sequence of connected graph edges, with its corresponding nodes. A
path is structural. It becomes a route when accompanied by calculation inputs,
costs, and metadata.

## Rider

A delivery resource with a stable RiderProfile (identity, supported routing profile,
maximum scalar capacity) and mutable RiderState (coordinate, operational availability).
Assignments, custody, and plans are separate authoritative records. Basic Dispatch
considers only coherently idle available riders as an eligibility policy.

## Road class

A domain classification for an edge, derived from fixture or source road data.
It may influence later speed and access policies. It does not by itself determine
v0 route cost.

## Road segment

A snapshot-local physical corridor between routing nodes. It references canonical
geometry once and owns compiler-derived physical distance. One or more directed
edges may traverse it.

## Route

An immutable computed path between an origin node and destination node under a
specific algorithm, traversal policy, and routing context. It includes ordered
nodes and edges, total distance, selected cost, elapsed time, and calculation metadata. Recalculation
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

Immutable per-route context including explicit logical departure time and access
permissions. Phase 11 supports deterministic FIFO traffic profiles. Dispatch pins graph,
traffic, and profile per decision and propagates each leg departure through stop timing.

## Traversal evaluation

The result of evaluating an outgoing directed edge for a search label and request:
either `Forbidden` or a traversable objective-cost and elapsed-time contribution.
Evaluation failures are errors, not forbidden edges.

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

A dimensionless factor applied to free-flow travel time. `1.0` means unchanged travel
time; a value greater than `1.0` means slower traversal. Phase 10 uses validated
static factors in a graph-bound traffic overlay, with an implicit default of `1.0`.
Phase 11 also supports deterministic time profiles on directed edges. Neither
model uses a production traffic feed.

## Travel time

The duration required to traverse an edge, route, or trip. Free-flow travel time
is a deterministic uncongested profile estimate; selected route cost equals
travel time only when using the travel-time objective.

## Trip

A rider's planned or actual movement for operational work. A trip can reference
one or more routes over time. Unlike a route, a trip has execution state and may
continue after a route is recalculated.

## Vehicle type

The supported routing/capability profile on a RiderProfile. A rider cannot silently
use a graph compiled for another profile; compatibility is enforced before routing.

## Zero heuristic

An A* heuristic that always returns zero. It is the mandatory fallback when no
stronger compatible lower bound has been proved and gives Dijkstra-equivalent
guidance.

## RiderPlan

Ordered remaining logical Pickup/Dropoff work, validated against responsibility and
fulfillment/custody. Road routes are derived legs. No permanent two-stop limit exists.

## CapacityUnits

One unit is an abstract normalized reference-parcel slot. Both order demand and
rider maximum capacity are non-negative integer counts of this same slot. Caller
inputs must use consistent normalization; this is not weight, volume, or package count.
Onboard load derives from custody; validate load after every logical stop. It is not
an active-order count. Multidimensional cargo is deferred.

## DispatchSnapshot

Coherent immutable world view with version, explicit evaluation instant, requests,
readiness/fulfillment, profiles/states, assignments/plans, and pinned routing inputs.

## DispatchInstant

Explicit logical instant supplied by the caller, distinct from Seconds durations.
RoutingEpoch defines a checked conversion to routing departure seconds.

## RoutingAnchor

Caller-supplied node projection of a stable coordinate on one graph digest. Coordinates
remain authoritative location facts. Missing or mismatched anchors are input errors.

## Candidate coverage

Complete means every Basic Dispatch eligible rider is evaluated. PotentiallyIncomplete
means bounded spatial screening; Unassigned then only claims no feasible evaluated rider.

## SoftObserved

Deadline treatment that records lateness at completed dropoff without rejecting a
candidate or contributing to the Phase 13 baseline score.
