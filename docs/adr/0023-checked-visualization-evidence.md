# ADR 0023: Checked portable visualization evidence

Status: accepted for the explicitly authorized Phase 26 implementation. Gates A/B were approved in
[source item 122](../phase-26-source/authorization.md); this ADR records routine implementation choices.
It supplements ADRs 0001/0002 (graph/geometry), 0004 (heuristics), 0012/0016 (decisions), and 0017–0022
(runtime/history/research). It does not supersede their algorithms, accepted protections or authority.

## Context

The old API line is a display encoding with adjacent-point deduplication and no explicit traversal
boundaries. Its helper relied on same-graph callers. This was an independent-export precondition gap,
not a demonstrated wrong-graph current-API bug. Alternatives omitted objective and traffic context
that single routes returned. Existing decisions retain different detail levels; counters do not mean
unique visited nodes. A context-free RouteResult cannot establish its originating evaluation context.

## Decision

Add checked E7 geometry and portable route evidence to core routing, without HTTP or renderer concepts.
Check the full semantic graph digest before dense IDs; retain directed occurrence ranges and every
canonical vertex. Use embedded geometry as the portable default. Missing original dependencies remain
typed unavailability. Preserve valid no-travel, reject invalid ordinary zero-length traversals and
never supply connectors, snapping or wrong-graph substitutions.

Keep spatial-path content identity, contextual evaluation identity, returned-set member identity and
existing operational selection identities separate. Generation/canonicalization is frozen in the
[contract](../visualization-contract.md). Rank is not identity. The producing adapter must supply original
full routing context/evaluator configuration; costs are copied from the route, never recomputed for display.

Expose new version-1 inspection endpoints using existing strict route requests/read authorization.
Preserve legacy route responses; add the missing legacy alternative provenance. Add versioned decision
inspection dimensions alongside original command evidence, without converting aggregate work into
candidate histories. Pure proposal metadata does not claim successful publication or frozen execution.
No original historical record is migrated or contextually backfilled.

Bound core geometry before allocation. Bound serialized HTTP output and active export bodies separately
from operational workers; reject saturation explicitly. Serialize on workers and retain export admission
through body lifetime. Capture only original traffic context under the existing coordinator lock.
No specialized state snapshot, resolver, timeline or execution adapter is needed for this finite package.

## Confirmed discrepancy and containment

A real GraphBuilder -> freeze -> encode -> decode test confirms that builder-created entirely
zero-distance ordinary segments can encode but fail decoder positivity validation. The production
OSM compiler already quarantines these candidates; a fresh normalized-artifact -> compiler -> graph
roundtrip test verifies that behavior. Valid no-travel has no segment and is unrelated.

This ADR **does not change foundational graph semantics**. Checked exposure rejects any such lower-level
ordinary traversal. Builder/artifact harmonization remains separate graph-maintenance work; it is not
silently declared fixed. The affected new producer is contained and can conform. Existing legacy direct
builder consumers retain the discrepancy; artifact-loaded API graphs cannot contain that segment.

## Consequences and boundaries

Rust geometry/evidence exports and API inspection now serve later result-based cinematic reveal and
honest decision inspection. A schema does not establish client readiness or visual fidelity. Detailed
g/h/f history, discovery events and search frontier traces remain unavailable. Current built-in
Haversine policies preserve their original proof contracts; no inconsistent heuristic is introduced
just to manufacture a repeated-processing test. Counters/state distinctions remain exact and A* has
no closed-set assumption.

No persistence schema migration, event-source authority, learned inference, map renderer, animation,
movement interpolation, telemetry, guidance or scene assets are introduced. Existing full-state JSON
lock/payload risk and absent snapshot/cursor cut remain documented limitations for later fleet clients.
Future clients inherit the cinematic addendum's motion, reduced-motion, progress and multi-leg duties.

Acceptance evidence is [Phase 26 completion](../phase-26-completion.md), including independent E7/SHA/Haversine
oracles, algorithm/publication tests, authorization/admission checks, DB checks and actual resource data.
