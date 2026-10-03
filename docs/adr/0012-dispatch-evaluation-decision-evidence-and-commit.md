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
