# ADR 0018: Adopted context, temporal applicability and historical provenance

Status: Accepted (audit Q8–Q9, Q12–Q13, Q18, Q28).
Implementation: semantic graph identity, in-memory envelopes and static-road proof.

## Context

Equal context values do not detect ABA adoption or forecast expiry. A decision
computed at an earlier instant cannot claim current feasibility/selection solely
because no state revision changed. Historical accepted facts cannot be recreated
from current predictions. Asserted graph hashes previously omitted compiled content.

## Decision

Publish only against runtime-adopted immutable semantic identities and monotonic
adoption revisions, under an explicit named time domain. Preserve evaluation,
publication and commit instants independently. A sealed decision binds original
source context; certification proves an explicit interval. There is no universal
TTL, equality-only time rule, hidden reevaluation or modern-input substitution.

Initial `static-road-monotone/v1` certifies original Phase 17 exhaustive selection
through static objective invariance and monotone feasibility, Phase 18 incumbent
feasibility and Phase 19 recovery applicability. Endpoint feasibility, baseline
health, forecast validity, frozen progress and original accepted protections remain
mandatory. It demonstrably permits nonzero elapsed time. Time-dependent providers
are unsupported for this certificate; broader proofs are future extensions.

Graph schema 4/compiler v4 bind canonical compiled routing semantics and required
provenance. Modifier changes reseal identity. Loaders recompute identity, not just
checksum. Schema 3 historical claims remain preserved, inspection-only and explicitly
unverified. Frozen routes retain their originating digest/local IDs.

Versioned historical envelopes distinguish schema, algorithm, policy semantics and
configuration, prediction, graph and operational revision. Record actual accepted
terms and original references. New acceptance time is actual commit; old terms
never reset. Inspection restores facts. Recomputation requires the recorded inputs
and compatible semantics; missing support returns a typed unavailable result.

## Alternatives and consequences

Arbitrary universal TTLs, timestamp equality, remote-provider distributed transactions
and recalculating historical acceptance baselines are rejected. Adopted inputs may
lag remote sources; report that boundary honestly. Endpoint certification adds routing
work and can reject useful cases outside its proof. Durable artifact/history retention
and live-clock conversion remain Phase 21/20 responsibilities respectively.

See [proof and compatibility details](../runtime-boundary.md).
