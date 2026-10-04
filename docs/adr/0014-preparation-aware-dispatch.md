# ADR 0014: Preparation-Aware Dispatch

Status: Accepted

## Context

Phase 13's idle-rider road-travel baseline intentionally uses zero waiting. Phase 14
must account for restaurant/store readiness, report pickup waiting independently,
and prefer completion time plus an explicit rider idle penalty. OrderReadiness,
StopTimeline, immutable snapshots, and shared world transitions already exist.

## Decision

`preparation_aware_dispatch` shares candidate generation, eligibility, feasibility,
routing validation, decisions, and atomic commit with `basic_dispatch`. Each rider
starts immediately at the snapshot instant and proposes Pickup(A) then Dropoff(A).
No new request, plan, custody, or simulation domain is introduced.

Observed readiness takes precedence over the current expected readiness. Without
an observation, use the expectation. If neither exists, return the typed evaluation
error MissingReadiness rather than assuming readiness or rejecting individual riders.
Future observations remain invalid snapshot facts. Forecasts and observations may
precede request creation, as supported by the existing domain.

Pickup waiting is `max(effective_ready_at - pickup_arrival, 0)`. Pickup departure is
arrival plus waiting, with zero service. Route the delivery leg at that propagated
departure through the pinned core provider, including FIFO time-dependent traffic.
Completion duration is completed dropoff minus snapshot evaluation instant. Deadline
lateness remains SoftObserved and uses completed dropoff after waiting.

The pure PreparationAwareStrategy ranks feasible evaluations by:

```text
completion_time_seconds + idle_penalty_weight * waiting_seconds
```

Weight is finite and non-negative, default 1.0. Zero minimizes completion duration
while still honoring readiness in timing. Multiplication and addition are checked;
non-finite contributions are evaluation errors. Exact score ordering and lower RiderId
ties reuse the baseline ranking mechanism. A positive weight may prefer a slightly
later completion with less idle time; no earliest-delivery or hard-SLA guarantee is
claimed. Waiting occurs once in physical completion and once more as an explicit
preference penalty, not as extra elapsed travel.

Evidence records both readiness inputs, effective instant/source, weight, versioned
strategy identity, per-candidate completion/waiting/penalty, and the exact proposal.
Preparation-specific fields are absent from baseline evidence. PlanEvaluation adds
an explicit elapsed completion duration alongside its absolute completion instant.
Phase 13 timing, objective, and selection remain unchanged.

Readiness updates advance the world version and invalidate outstanding decisions.
Commit does not route or change policy. Expected readiness never authorizes actual
pickup; execution still requires World::observe_ready and the shared pickup transition.

## Consequences

Small deterministic scenario executions compare strategies from fresh equivalent
worlds with identical fixed external ready events and road inputs. Predicted decision
metrics are reported separately from metrics reconstructed from executed road legs
and recorded fulfillment transitions, including forecast errors. These are synthetic
correctness comparisons, with no latency or production performance claim.

## Rejected / deferred alternatives

Treating unknown readiness as ready, conflating forecasts with observations, scoring
absolute timestamps, mutating the snapshot, and hiding waiting inside road cost are
rejected. Delayed rider departure, busy-rider insertion, a generic optimizer interface,
a simulation event queue, fleet benchmarks, hard deadlines, and re-dispatch remain
later work. Phase 15 owns the general simulation engine.
