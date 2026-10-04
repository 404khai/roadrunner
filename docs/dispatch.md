# Basic dispatch

Phase 13 is implemented in `roadrunner-dispatch`. It evaluates one new unassigned
order against available, coherently idle riders and proposes `Pickup(A) → Dropoff(A)`.
The existing general domain model and shared execution transitions remain the source
of truth. See [the specification](spec-v1.md#56-basic-rider-assignment) and
[ADR 0012](adr/0012-dispatch-evaluation-decision-evidence-and-commit.md).

## Run the example

```bash
cargo run -p roadrunner-dispatch --example basic_dispatch
```

The [example](../crates/roadrunner-dispatch/examples/basic_dispatch.rs) constructs a
small directed synthetic graph and two idle riders. Rider 9 is closer to pickup in
straight-line distance but has a slow access road. Rider 2 has a faster road route.
Exhaustive dispatch selects rider 2; a one-rider spatial shortlist selects rider 9.
The example asserts both results and prints the full decisions as JSON, including
per-candidate pickup/delivery travel, total score, distance, timing, provenance,
coverage, and selection reason.

It explicitly commits the exhaustive proposal, moves the selected rider to pickup,
records pickup, moves to dropoff, and records delivery using `World` transitions.
The order is observed ready at scenario time zero so the zero-wait baseline is
consistent with this execution. Final assertions check empty remaining work,
released responsibility, zero onboard load, and world validity. The final state is
also printed. Execution is caller-driven; no simulation scheduler is introduced.

## Evaluation and commit

```text
validated World + explicit instant/epoch + pinned RouteProvider + RoutingAnchors
  → immutable DispatchSnapshot
  → basic_dispatch(order, decision_id, candidate_policy)
  → AssignmentDecision (Assigned or Unassigned), or DispatchEvaluationError
  → World::commit(decision)
```

`RoutingAnchors` bind each evaluated location to a coordinate, graph digest, and
existing node. Callers supply these projections; the dispatcher does not snap
coordinates. `CoreRouteProvider` uses Roadrunner routing with pinned free-flow,
static traffic, or FIFO time-dependent traffic. Pickup-to-dropoff routing departs
at the propagated pickup departure instant.

`CandidatePolicy::Exhaustive` records `Complete` coverage over the eligible fleet.
`CandidatePolicy::Spatial { radius, limit }` uses authoritative eligible positions
and records `PotentiallyIncomplete` coverage, even if its result happens to include
all riders. Spatial screening ETA never enters ranking.

A rider is eligible only when available with no active responsibility, no custody,
and no remaining plan. Capacity, plan/custody consistency, and profile compatibility
are feasibility constraints. A valid disconnected road route rejects the candidate
with `NoRoute`; invalid input, missing anchors, provenance mismatch, arithmetic
failure, or routing-engine failure returns an evaluation error.

For each feasible candidate, the baseline minimizes:

```text
score_seconds = pickup_road_travel_seconds + delivery_road_travel_seconds
```

Scores use exact finite ordering with lower `RiderId` resolving equal scores.
Evidence is canonicalized by rider identity. Waiting and service are explicitly
zero. Deadlines are `SoftObserved`: completed-dropoff lateness is measured but does
not reject a candidate or change its score. Readiness forecasts do not affect this
baseline; actual pickup still requires an observed readiness fact.

`AssignmentDecision::evidence()` exposes candidate metrics or typed rejections and
policy/provenance information. `outcome()` exposes the exact proposed assignment
and remaining plan, or an unassigned scope. Exhaustive failure refers to the eligible
fleet; bounded failure refers only to evaluated candidates. `explanation()` provides
a short rendering of that evidence. Pickup travel is the duration from evaluation
to pickup; delivery travel is the subsequent road leg, while completion is an
absolute `DispatchInstant`.

Evaluation leaves the world unchanged. Commit verifies world identity/version and
exact prior responsibility/plan, then validates and publishes the replacement state
atomically. A stale or invalid commit changes nothing. Callers must explicitly
reevaluate after a stale decision; commit never selects another rider.

## Verification and scope

```bash
cargo test -p roadrunner-dispatch
cargo run -p roadrunner-dispatch --example basic_dispatch
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

[Entry-gate fixtures](../crates/roadrunner-dispatch/tests/entry_gate.rs) cover routed
selection versus proximity, coverage, eligibility, feasibility, error boundaries,
propagated time-dependent departures, deterministic ties, soft deadlines, stale
commit, custody, and plan progression. The [completion report](phase-13-completion.md)
maps these to Phase 13 requirements.

Preparation-aware waiting/scoring belongs to Phase 14. Simulation, fleet strategy
benchmarking, multi-order insertion, HTTP dispatch endpoints, persistence, and UI
remain later phases. The rider lookup benchmark is preserved; this phase makes no
new dispatch latency or scalability claim.
