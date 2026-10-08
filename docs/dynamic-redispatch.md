# Phase 19: dynamic recovery v1

Phase 19 retains Phase 17 admission and Phase 18 batch construction contracts.
Recovery is a separate, atomic decision over already committed work. Its inputs
are a coherent snapshot, explicit execution projections, original acceptance
terms, trigger identity, last successful recovery time and a versioned policy.

Only awaiting-pickup orders outside the active frozen stop may change owner.
Custody is never transferred. Departed travel, its destination and required
waiting/service remain pinned. Offline means unavailable for new work, as in the
existing availability domain; it does not simulate physical immobilization.

The declared best-improvement neighborhood relocates an unstarted complete
pickup/dropoff pair to every available rider and every precedence-preserving
position, or relocates one editable stop within its current owner. Existing
orders remain mandatory. Candidates use authoritative whole-plan routing,
readiness, service, prefix capacity and accepted hard protections. An unavailable
required forecast blocks recovery with a typed evaluation failure; execution
continues. No global VRP optimum or global impossibility is claimed.

For a healthy baseline the score is remaining road-travel seconds plus explicit
reroute penalty per changed rider plan and assignment stability penalty per
changed owner. Distance then canonical fleet identity break ties. Penalties are
relative to the decision's original baseline, not each search iteration. A
strict positive saving meeting the configured minimum improvement is required.
Cooldown suppresses optional churn. A hard-protection breach or movable work on
an unavailable rider requires a fully feasible repair; repair bypasses cooldown
and optional saving thresholds but never weakens accepted protections. If no
single declared move repairs the baseline, report scoped no recovery and keep
execution unchanged. Multi-move infeasible intermediate repair is deferred.

Every complete candidate fleet submission consumes one work unit. Budget
exhaustion publishes nothing, including an incumbent. Complete rounds are
canonical and deterministic. Evidence separates baseline health, candidate
rejections, work, termination, changed owners/plans, penalty accounting and
per-order completion impacts. Publication compares the entire context and world
version, then replaces all affected assignments/plans in one validated write;
accepted terms are unchanged.

Scenario schema 4 supplies dynamic availability, forecast and cancellation
inputs in addition to traffic and order events. New orders use Phase 18 batch
admission after recovery. Cancellation is supported only before pickup and
outside the active frozen stop. Active/onboard cancellation is explicitly
refused; returns, handoffs and disposal need their own future domain workflow.
