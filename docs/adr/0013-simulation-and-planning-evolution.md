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

## Pre-Phase-20 operational amendment (2026-10-10)

OperationalState now provides the coherent volatile domain/execution authority.
OperationalRevision spans World and active execution; PlanRevision, ActionId,
ScheduleGeneration and applied-effect identity remain distinct. Shared dispatch
transitions/projected frozen stops serve simulation and future operational drivers.
Simulation retains clocks/queues/synthetic observations/metrics. Compatibility World
publication remains trusted internal use, inaccessible as a mutable operational
bypass. Operational planning binds authoritative adopted contexts and sealed original
decisions; static-road temporal certification checks continued applicability before
complete atomic publication. Historical acceptance references are immutable; actual
new acceptance time is stamped only at successful operational commit.
Simulation output declares `shared-execution/v2` independently of scenario schema.
See ADRs [0017](0017-operational-authority-and-publication.md) and
[0018](0018-planning-time-and-historical-provenance.md).
