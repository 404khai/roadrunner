# Dispatch

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
to pickup; delivery travel is the subsequent road leg. `delivery_completed_at` is
an absolute `DispatchInstant`, while `completion_time` is elapsed `Seconds` since evaluation.

Evaluation leaves the world unchanged. Commit verifies world identity/version and
exact prior responsibility/plan, then validates and publishes the replacement state
atomically. A stale or invalid commit changes nothing. Callers must explicitly
reevaluate after a stale decision; commit never selects another rider.

## Preparation-aware dispatch (Phase 14)

`OrderReadiness.expected_at` is the existing mutable forecast; no second expected
ready field is added to Order. `observed_at`, when present, overrides that forecast.
A snapshot rejects an observation after its evaluation instant. If neither readiness
input exists, `preparation_aware_dispatch` returns `DispatchEvaluationError::MissingReadiness`.
An estimate before arrival produces zero waiting, even when the estimate is in the past.

Call the new entry point with the same snapshot, order, decision identity, and candidate
policy as Basic Dispatch, plus `PreparationAwareStrategy::default()`:

```rust,ignore
let decision = preparation_aware_dispatch(
    &snapshot,
    order_id,
    decision_id,
    CandidatePolicy::Exhaustive,
    PreparationAwareStrategy::default(),
)?;
world.commit(&decision)?;
```

All riders depart immediately at the evaluation instant. For each feasible rider:

```text
pickup_arrival = evaluated_at + pickup_road_travel
pickup_waiting = max(effective_ready_at - pickup_arrival, 0)
pickup_departure = pickup_arrival + pickup_waiting
completed_dropoff = pickup_departure + delivery_road_travel(pickup_departure)
completion_time = completed_dropoff - evaluated_at
score = completion_time + idle_penalty_weight × pickup_waiting
```

Service remains zero. Delivery routing uses the departure after waiting, so traffic
can change both its travel duration and chosen road route. Distance/travel exclude
waiting. `PlanEvaluation` records pickup arrival/departure, delivery travel, absolute
completed delivery, elapsed `completion_time`, waiting, and soft-observed lateness.

The default weight is 1.0 penalty second per waiting second.
`PreparationAwareStrategy::new(weight)` accepts finite non-negative values; zero
still accounts for physical waiting but minimizes completion duration alone. A positive
weight can trade a later delivery for lower rider waiting. This is an explicit scoring
preference; deadlines remain soft and no fleet optimality guarantee is made. Pure ranking
retains exact finite comparisons, deterministic ties, and checked arithmetic.

The versioned objective is `preparation-completion-wait/v1`. Decision evidence includes
`preparation` with both original readiness facts, effective ready time/source, and weight.
Each feasible score's `preparation` breakdown contains completion duration, waiting,
and weighted idle penalty. `score.total` is the actual objective, while pickup/delivery
road contributions remain separately available. `explanation()` names the selected
objective. The baseline omits preparation-specific evidence and keeps its road-travel score.

Candidate coverage, capacity/profile checks, error boundaries, immutable proposals,
and atomic commit reuse Phase 13. `World::estimate_readiness` or `observe_ready` advances
world version, so outstanding decisions become stale. Forecasts affect predicted
pickup timing only: actual pickup requires an observed ready event at or before pickup.
See [ADR 0014](adr/0014-preparation-aware-dispatch.md).

## Run the preparation comparisons

```bash
cargo run -p roadrunner-dispatch --example preparation_aware_dispatch --locked
```

The [example](../crates/roadrunner-dispatch/examples/preparation_aware_dispatch.rs)
prints JSON for three deterministic synthetic scenarios: long preparation,
already-ready stock, and underestimated preparation. Each strategy starts from a fresh
equivalent world with identical graph, order, rider positions, evaluation time, and
external actual ready event. Both road legs are executed with the fixed provider;
observed pickup/delivery times come from shared fulfillment transitions. The output
separates decision predictions from executed metrics and final fulfillment state.

In the long-preparation fixture, the nearer rider arrives after 5 minutes and waits
13; the farther rider arrives after 15 minutes and waits 3. Both finish delivery
28 minutes after evaluation. The baseline selects the nearer rider; preparation-aware
scoring selects the farther rider with less idle time. These values follow from the
fixture's deterministic roads and recorded execution, and are checked by tests. When
stock is ready, both select the nearer rider. With a late actual ready event, observed
completion differs from the forecast in the decision.

These small caller-driven scenario executions satisfy Phase 14's comparison requirement.
The general event queue and scalable simulation runtime belong to Phase 15. There is
no randomness, performance comparison, or production dataset in this demonstration.

## Verification and scope

```bash
cargo test -p roadrunner-dispatch
cargo run -p roadrunner-dispatch --example basic_dispatch
cargo run -p roadrunner-dispatch --example preparation_aware_dispatch --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

[Entry-gate fixtures](../crates/roadrunner-dispatch/tests/entry_gate.rs) cover routed
selection versus proximity, coverage, eligibility, feasibility, error boundaries,
propagated time-dependent departures, deterministic ties, soft deadlines, stale
commit, custody, and plan progression. The [completion report](phase-13-completion.md)
maps these to Phase 13 requirements.

[Phase 14 fixtures](../crates/roadrunner-dispatch/tests/preparation_aware.rs) cover
readiness precedence/absence, waiting boundaries, traffic after waiting, exact ties,
configuration/overflow, soft deadlines, stale readiness updates, shared execution,
and forecast-versus-observed comparisons. See the [Phase 14 completion report](phase-14-completion.md).

The general simulation engine, fleet strategy benchmarking, multi-order insertion,
HTTP dispatch endpoints, persistence, and UI remain later phases. The rider lookup
benchmark is preserved; these phases make no new dispatch latency or scalability claim.
