# ADR 0015: Deterministic Simulation Runtime

Status: Accepted

## Context

Phase 14's small caller-driven comparisons do not provide a general event queue,
scenario loader, or horizon-aware metric artifact. Phase 15 requires reproducible
order arrivals, actual readiness, rider execution, traffic changes, and observable
outcomes without consulting wall-clock time or duplicating domain invariants.

## Decision

`roadrunner-simulation` depends on core and dispatch and owns scenario generation,
a BinaryHeap ordered by `(DispatchInstant, insertion_sequence)`, event orchestration,
run state, and observed metrics. Core and dispatch never import simulation types.
The CLI loads a self-contained synthetic graph or validated core graph artifact,
then invokes the same simulation library.

All external inputs are validated before execution, even events beyond the horizon.
Seed-derived readiness is materialized in ascending OrderId order using versioned
SplitMix64/upper-53-bit uniforms before dispatch runs. The explicit seed and concrete
ready instants appear in results. Different dispatch choices do not consume different
random draws. Each invocation constructs a fresh world; inputs remain immutable.

Initial equal-time events are inserted in this order: traffic replacements (input
order), order creations (OrderId), and ready observations (OrderId). Generated events
use increasing checked sequence numbers. Event actions are atomic with respect to the
queue. Assignment actions evaluate then immediately commit their exact decision before
another assignment action runs. Pending orders are retried on order creation/readiness,
traffic updates, and completed delivery; unassigned evaluations alone trigger no retry
loop. Pickup/delivery actions are generated from actual readiness and executed road legs.

World::register_order atomically adds request, readiness, and AwaitingPickup state.
Other transitions reuse World::commit, update_rider_state, observe_ready, pickup, and
deliver. OrderReady receives already-ready stock at creation but preserves the original
ready fact instant. Future observations are never inserted into a dispatch snapshot.

Static traffic events replace the complete overlay for subsequently departing legs.
A road leg pins its route, cost, and provenance at departure. Later traffic events do
not rewrite that in-flight leg. Pickup-to-customer routing uses the actual pickup time
and currently active overlay, so observations can differ from assignment predictions.
RiderMoved records completed whole-leg endpoints with path/edges; no partial distance
or continuous interpolated position is invented. Dynamic rerouting remains Phase 19.

Execution processes every event at or before the inclusive horizon without sleeping.
Results use the full configured time window, even if the queue finishes early, and
preserve queued future events and every scheduled order's optional milestones.

Metric populations are explicit: predicted ETA includes committed assignments;
delivery duration includes completed dropoffs measured since creation; pickup waiting
includes completed pickups, even if delivery is unfinished. Partial waiting is recorded
per order separately. Uncreated, never-assigned, assigned-unfinished, delivered-late,
and outstanding-past-deadline counts prevent hiding unfinished work. Empty distributions
are unavailable. Utilization is responsibility occupancy, including travel/waiting,
divided by initially available rider-seconds; unavailable riders do not inflate the
population. Distance includes completed road legs only.

Replay output contains no machine or wall-clock values. External process timing records
hardware, graph/dataset/configuration, runs, median/p95/p99, raw samples, and output hash.
Small-fixture timing establishes the scheduling behavior without claiming scalability.

## Consequences

The engine supports the eight Phase 15 events plus DispatchUnassigned for valid failed
attempts. It preserves typed evaluation errors as failures rather than infeasibility.
Small fixed/seeded scenarios can execute complete or partial horizons and replay exactly.
Preparation forecasts remain distinct from actual ready events during execution.

## Deferred

Fleet strategy benchmarking, initial busy plans, rider shift/availability events,
continuous movement interpolation, per-edge changes during travel, cancellation,
re-dispatch, multi-order insertion, advanced restaurant/store state, UI/replay controls,
persistence, live traffic, and production load claims remain later work. Restaurants
and customers are pickup/dropoff locations; this phase models neither kitchen queues
nor separate customer accounts.
