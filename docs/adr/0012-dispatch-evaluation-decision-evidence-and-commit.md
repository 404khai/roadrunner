# ADR 0012: Dispatch Evaluation, Decision Evidence, and Commit

Status: Accepted

## Context

Accepted Q1–Q31 dispatch architecture decisions require separation before Phase 13.
Earlier illustrative single-order types do not establish permanent domain limits.

## Decision

One immutable DispatchSnapshot carries coherent world version and explicit time.
Routing pins graph, traffic, and profile, with checked epoch conversion and propagated
per-leg departure. Route travel remains separate from stop waiting/service.

Candidate generation declares Complete exhaustive or PotentiallyIncomplete spatial
coverage. Feasibility is structural, metrics typed, and strategy ranking pure. The
baseline is exactly pickup plus delivery road travel, finite exact ordering then lower
RiderId. SoftObserved deadlines measure lateness at completed dropoff and affect neither
feasibility nor score.

Assigned and Unassigned are completed outcomes; EvaluationError means evaluation failed.
Every completed decision preserves compact structured identity/provenance, policies,
coverage, canonical candidate metrics/score contributions or rejections, exact proposal,
and tie reason. Full geometry/traces are optional.

Commit compares source world version and exact prior responsibility/plan, rechecks
transition invariants, and applies both together or rejects without mutation. No implicit
reevaluation or alternative rider selection occurs. Whole-world versioning suffices.

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
