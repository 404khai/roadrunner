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
A changed local incumbent below this gain has explicit `BelowThreshold`
termination and signed saving evidence, without publication. Cooldown suppresses optional churn. A hard-protection breach or movable work on
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

## API and reproduction

Construct `RecoveryContext::new(snapshot, pooling_inputs, policy, trigger,
last_applied)` with optimizer identity `dynamic-recovery/v1`. It pins all original
plans/owners/terms through a coherent world version and the entire clock, routing,
readiness, service, execution and churn bundle. Projections must cover every rider;
missing projections are invalid context, unavailable forecasts are
`PredictionUnavailable`, and an unroutable baseline is `BaselineUnavailable`.
Structural corruption and unsupported policies remain errors, not keep decisions.

`recover_fleet` returns `RecoveryDecision`; `evidence()` provides immutable
`RecoveryEvidence`, so callers cannot rebind an evaluated proposal. Only `proposal()` exposes a publishable
replacement. Rebuild the current authoritative context before
`World::commit_recovery(decision, current)`. Never echo an old proposal context.
The proposal replaces all owners/plans once and retains all accepted terms. The
public `evaluate_recovery_plan` accepts only the existing assignment key set and
shares authoritative physical evaluation with Phase 17/18. It does not publish.

`World::cancel_order(order, at, active_orders)` takes the caller's explicit current
execution locks. Domain state does not infer active execution from last-completed
coordinates or inspect simulator queues. It returns a typed `CancellationRefusal`
for active/non-awaiting/invalid-time requests, otherwise validates a complete
replacement before removing only this order's responsibility/stops. Historic
acceptance remains recorded after terminal `FulfillmentState::Cancelled`.

In schema 4, `dynamic_events` contain `{at_seconds, change}`. `change.kind` is
`availability`, `forecast`, `cancel`, or `road_delay`. Forecast receipt refreshes
the declared forecast generation instant without changing an observation. A road
delay retains the departed route/path/destination and adds observed duration;
its old arrival generation becomes inert. Waiting/service delay is represented
by actual readiness/service semantics, not a generic abandoned-action timer.

Equal-time external inputs run in traffic/order creation/readiness/dynamic-input
order, then generated recovery. Coalesced trigger identities are canonically
sorted. Recovery selects next effective-plan work; new-order admission follows
as an explicitly separate Phase 18 publication. Offline does not abandon already
committed obligations when recovery cannot find a safe placement.

The baseline's frozen road travel is excluded identically from all editable
objective totals; its exact destination arrival/wait/service and effects remain
in every projection. Completion impacts include frozen completions. Penalties
count changed original plans and existing owners; temporary search moves do not
reset the comparison baseline or accepted protections.

```bash
cargo +1.99.0 test -p roadrunner-dispatch --test recovery --locked -- --nocapture > target/phase19-oracle.log
cargo +1.99.0 build -p roadrunner-cli --release --locked
PYTHONDONTWRITEBYTECODE=1 python3 scripts/collect_recovery_benchmark.py --oracle-log target/phase19-oracle.log --output target/recovery-benchmark.json
```

See [completion evidence](phase-19-completion.md) and
[versioned process measurements](../benchmarks/results/2026-10-08-phase-19-recovery.json).
