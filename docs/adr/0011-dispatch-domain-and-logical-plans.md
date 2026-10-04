# ADR 0011: Dispatch Domain and Logical Plans

Status: Accepted

## Context

Accepted Q1–Q31 dispatch architecture decisions require separation before Phase 13.
Earlier illustrative single-order types do not establish permanent domain limits.

## Decision

Order represents request facts; readiness estimates and actual observations are separate
from fulfillment. RiderProfile and RiderState separate capabilities from mutable location
and availability. Dispatch owns RiderId and the rider spatial projection; core owns
geographic math and routing only.

CommittedAssignment is active responsibility, distinct from an immutable decision.
RiderPlan is ordered remaining Pickup/Dropoff work, with no fixed length. AwaitingPickup
requires pickup before dropoff; PickedUp requires only dropoff with the custody rider;
Delivered requires no remaining work. Custody forbids reassignment without a future
explicit handoff model. One CapacityUnit means one abstract normalized reference-parcel slot, with identical
normalization for rider capacity and order demand. Scalar demand/load is enforced after each stop;
load derives from custody, never order count.

Stable locations are validated coordinates. Graph-digest/node anchors are explicit
caller projections, validated before routing. Missing/wrong-graph anchors are errors.
Basic Dispatch's idle eligibility and two-stop proposals are policy restrictions.

## Consequences

Basic Dispatch remains narrow while shared domain boundaries support later policy changes.
The pre-Phase-13 remediation report records implementation and entry-gate evidence.

## Rejected / deferred alternatives

Live mutable evaluation inputs, score penalties for infeasibility, and permanent
single-order domain invariants are rejected. Later-phase algorithms remain deferred.
