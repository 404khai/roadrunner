# ADR 0017: Operational authority, identity and atomic publication

Status: Accepted (approved Runtime Boundary Audit Q1–Q7, Q10–Q13).
Implementation: volatile prerequisite foundation; HTTP and durable storage deferred.

## Context

`World` and private simulator active stops/cooldown previously formed separate
sources of correctness-relevant state. Public order-only mutations could not
establish active-action validity or same-effect recognition for external inputs.
WorldVersion alone did not span execution progression.

## Decision

Establish one logical namespace-scoped `OperationalState` with domain World,
execution stages/frozen routes, PlanRevision/ActionId/ScheduleGeneration/effect
recognition, adopted contexts, cooldown and whole OperationalRevision. Dispatch
validates transitions; a coordinator authenticates observations and coordinates
exclusive publication. Derived indexes/load/projections cannot override facts.

All authoritative effects of a transition publish through one serialized lane,
including context adoption and execution. Evaluate expensive work on detached
read-only snapshots; recheck exact authority before one complete replacement.
Whole-authority conflicts are deliberately conservative. No stale rebase, partial
fleet salvage or executing superseded future work. A preserved started ActionId
survives suffix replacement and schedule changes. Duplicate logical effects,
including distinct incoming IDs, cannot apply twice; conflicting reuse rejects.

Typed local IDs are durable only with their immutable namespace. Fresh volatile
lifetimes use new namespaces; future durable restarts restore counters/revisions.
Allocation checks exhaustion. Observation, command, event, action and effect scopes
remain distinct. Rejected/duplicate no-effect operations do not advance the domain
revision; recognition-only metadata may have a separate lifecycle.

Simulation and future operational drivers share domain transitions and frozen
projection; clocks, queues, synthetic observations and infrastructure remain driver
owned. Prediction time does not prove actual pickup/delivery.

## Alternatives and consequences

Independent mutable World/execution authorities and HTTP-only validity checks are
rejected. Fine-grained multi-writer publication is deferred until measured contention
justifies its proof cost. Full private staging is simple and atomic but copies state
and history; benchmark before optimizing. Compatibility World APIs remain trusted
library/simulation paths, inaccessible as mutable operational bypasses.

Phase 20 will assemble/authenticate external intent. Phase 21 must replace the
volatile linearization point with storage-enforced transaction commit and fenced
revision checking; a mutex alone cannot establish durable multi-process safety.

See [implemented API and invariants](../runtime-boundary.md).
