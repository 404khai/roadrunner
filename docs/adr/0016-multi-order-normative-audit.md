# ADR 0016: Normative multi-order admission and deferred fleet optimization

Status: Accepted. Source: user-confirmed seven-round architecture audit, 2026-10-07.
This contract supersedes earlier ADR deferrals for Phases 17–18. Phase 18 was explicitly
authorized after the Phase 17 completion report on 2026-10-08. Phase 19 remains deferred.

## Phase 17 flow and boundary

One created unassigned order → all eligible riders → explicit frozen execution and
projected post-prefix state → every order-preserving pickup/dropoff insertion pair
→ whole-plan routing/readiness/service/load evaluation → hard feasibility and accepted
protection checks → signed incremental road-travel seconds, road distance, canonical
rider/semantic-plan tie break → complete-search proposal → atomic assignment, effective
plan and acceptance terms → current-plan-driven pooled execution.

Existing obligations are mandatory. No handoffs, ownership replacement, custody
transfer, mid-leg diversion, active wait/service abandonment, or batch allocation.
Plan validity is broader than this algorithm's order-preserving search restriction.

## Timing and protections

Route every leg at its propagated departure, using pinned graph/traffic/profile and
logical time. Pickup is arrival, readiness wait, deterministic service, departure.
Dropoff completion includes deterministic service. Colocated stops remain independent.
Observed readiness wins; predictions require explicit freshness/validity, or a named,
versioned fallback. Past forecast timestamps never establish observed readiness.
Required missing/stale predictions are evaluation failures, not infeasibility.

Deadlines are data, with mixed per-order SoftObserved or hard admission policy.
Hard predicted violations are infeasible. Optional typed cumulative completion delay
is relative to authentic acceptance-time predicted completed dropoff, with no numeric
default. Consumption is max(candidate minus accepted, zero); improvements do not bank
allowance. Raw signed candidate-current and candidate-accepted deltas remain evidence.
Deadline and cumulative protection are independent. New authoritative acceptance terms
exist only after successful commit and include policy id/version, acceptance instant,
completion reference, deadline/detour terms and pinned prediction provenance. Rewrites
never reset these terms. Current policy may strengthen but never weaken protections.
Unknown versions, missing hard-policy data and protection without an authentic historic
reference are errors. Hard admission is model feasibility, not a realized guarantee.
Realized violations after valid admission remain observable; uncertainty margins deferred.

Legacy Phase 13–16 behavior is explicitly named/versioned: soft deadlines, zero service,
historical readiness, no invented historic completion reference and no cumulative bound.

## Domain, projection and execution

Validate owner, fulfillment, custody, exact remaining stop multiplicity/precedence,
initial onboard load and load after every stop. Pending responsibility is not onboard
load. A picked-up order has no pickup and exactly one owner-bound remaining dropoff.
Structural corruption fails world validation rather than candidate rejection.

A departed leg, destination stop and required wait/service complete unchanged. Dispatch
receives explicit execution identity, projected time/location/anchor, custody/load,
completed effects and editable suffix. A stale last-completed endpoint grants no freedom.
Freeze only the active prefix, not the entire plan. The simulator owns clock/queue and
completes the active action through shared domain transitions, then reads the current
plan's next stop. Editable queued actions must be generation-bound. The Phase 17 runtime implementation
queues no editable future work, so it does not create such actions. Active execution identity survives plan replacement.
Each dropoff releases only its order. A rider stays occupied until all responsibility
and active execution are gone; overlapping responsibility uses one busy interval.

## Baseline health, coverage and publication

A predicted accepted hard breach blocks new admission on that rider while preserving
execution/terms. Soft misses do not. Unknown readiness is neither healthy nor breached.
Phase 17 cannot skip an unevaluable eligible rider and publish a fleet-optimal result:
required unavailable prediction is a typed failure of the complete decision.

For suffix n enumerate (n+1)(n+2)/2 pairs. Canonical rider/position order and deterministic
candidate work budget; one complete submission consumes one unit, including early
rejection. Any pruning needs correctness proof and counts. Input, rider, and placement
coverage are separate. Complete declared search with sufficient input yields best
feasible insertion or scoped NoFeasibleInsertion. Budget/interruption yields
SearchIncomplete and no commit, even when a feasible candidate exists. No arbitrary
resequencing optimality claim; spatial shortlist mode deferred.

Bind the exact coherent world identity/version, evaluation instant, graph, traffic,
readiness, routing profile, service/optimizer policy and execution/projection contexts,
and prior assignments/plans. Any relevant mismatch rejects the entire proposal with
zero semantic mutation. Atomic publication includes new assignment, replacement plan,
and accepted terms. No partial salvage or distributed transactions.

## Proof and evidence

Independent tiny oracle enumerates rider × placements and compares objectives without
production generation/pruning/ranking (authoritative evaluator may be shared).
Hand-calculated timing/service/readiness/load/custody/slack/delay fixtures, seeded
invariants, canonical unordered construction and deterministic replay are required.
Test ownership, immutable terms, exact stops/load, all prefixes, frozen survival,
superseded-event nonmutation, atomic success/stale failure and no leaked unplaced work.

Versioned evidence identifies scenario/snapshot/prediction/policies, baseline health,
input order, expected/proposed plans/assignments/terms, horizon accounting, accepted /
current / candidate completions, signed deltas, consumed delay/slack/readiness, per-rider
travel/distance/stops/load/frozen/suffix, separate coverage, typed reasons, work counts,
termination/optimality and publication. Chosen result reproducible; full candidate route
traces optional. Hardware/time/memory are external measurements, never semantic input.
Benchmarks record hardware, dataset/hash, graph/scenario size, algorithm/config, compiler/
build, run count, raw/median/p95/p99, deterministic work and memory or unmeasured.

## Phase 18 contracts (implementation authorized 2026-10-08)

Jointly allocate a current unassigned batch and resequence editable suffixes, retaining
committed owners. Maximize newly admitted order count, then remaining fleet road seconds,
distance and canonical fleet identity. Demand is feasibility only; admitted ≠ delivered.
Isolate baseline-breached riders unchanged and optimize healthy remainder; unchanged
breaches may remain but no candidate-induced hard violation. Isolate unevaluable riders
and new orders with explicit incomplete input coverage. Fleet publication is all-or-nothing
assignments/plans/terms. Greedy traps, local optimum/oracle quality belong to Phase 18.
Phase 19 owns committed reassignment/recovery. No speculative optimizer framework.

Core remains ignorant of orders/plans/custody/protection. Dispatch owns authoritative
evaluation/domain/proposals/publication. Simulation owns time, queue, prediction and
observations/outcomes. CLI owns loading/workflow/adapters. Tests own independent oracle.

The Phase 18 implementation and qualified heuristic coverage are documented in
[fleet optimization](../fleet-optimization.md).
