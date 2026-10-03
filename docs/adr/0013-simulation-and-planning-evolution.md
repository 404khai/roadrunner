# ADR 0013: Simulation and Planning Evolution

Status: Accepted

## Context

Accepted Q1–Q31 dispatch architecture decisions require separation before Phase 13.
Earlier illustrative single-order types do not establish permanent domain limits.

## Decision

Simulation owns clock, queue, seeded randomness, and orchestration and drives shared
dispatch/domain assignment, readiness, pickup, custody, and delivery transitions. Dispatch
does not depend on simulation. Strategy comparisons use identical fixed exogenous inputs;
observed execution metrics are distinguished from predicted candidate metrics.
Comparisons start from fresh equivalent worlds. Metric schemas declare populations,
time windows, unfinished work, units, and versions rather than comparing only finished
orders or substituting predicted outcomes for observed events.

Future multi-order insertion evaluates whole-plan deltas, including existing work.
Custody is a hard re-dispatch boundary, not a score penalty. Fleet planner APIs,
FleetPlan, PlanId/PlanVersion, acceptance, handoffs, churn policies, and distributed
transactions are deferred. This remediation implements no simulation queue, insertion,
VRP, preparation-aware ranking, or dynamic re-dispatch.

## Consequences

Basic Dispatch remains narrow while shared domain boundaries support later policy changes.
The pre-Phase-13 remediation report records implementation and entry-gate evidence.

## Rejected / deferred alternatives

Live mutable evaluation inputs, score penalties for infeasibility, and permanent
single-order domain invariants are rejected. Later-phase algorithms remain deferred.
