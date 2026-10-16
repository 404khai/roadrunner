# new thread, new branch Roadrunner — Phase 26 Implementation Authorization

I've reviewed the complete Observability/Visualization Contract Architecture Audit (Q1–Q49) and the Cinematic Routing and Spatial Dispatch Visualization Product Experience Contract.

## Gate A — Architecture Confirmation

**APPROVED.**

I confirm the consolidated Q1–Q49 architecture and the subsequent cinematic visualization addendum as the architectural reference for Phase 26 and the relevant later visualization phases.

The approved product direction includes:

- Accurate, graph-qualified route visualization.
- Cinematic route-reveal animations.
- Spatial rider discovery and selection highlights.
- A polished 2D/3D navigation experience.
- Stylized motorcycle-delivery riders, using the provided reference.
- Future rider and motorcycle cosmetic customization.
- Interactive dispatch, routing, and algorithm inspection.
- Explicit distinction between authoritative engine evidence and illustrative animation.

These product requirements must remain compatible with the established roadmap.

The ten-round architecture grill is complete.

Do not start another architecture-question cycle unless implementation uncovers a genuine unresolved contradiction that prevents a correct decision.

## Gate B — Phase 26 Implementation

**AUTHORIZED.**

You may now begin implementing the finite Phase 26 Observability/Visualization Contract work package.

Use the consolidated architecture audit, the product-experience addendum, the existing AGENTS.md roadmap, relevant ADRs, tests, and actual repository code as your implementation references.

The primary goal is to make Roadrunner's existing routing, geometry, cost, decision, and diagnostic evidence safely consumable by later visualization clients.

Do not implement the React/MapLibre UI during Phase 26.

### Implementation order

**Workstream 1 — Baseline and specification reconciliation**

- Verify the current repository state and existing test baseline.
- Inspect affected interfaces and compatibility requirements.
- Reconcile relevant ADRs and architecture documentation.
- Preserve existing published behavior unless a versioned change is explicitly justified.
- Reproduce or characterize the suspected graph-builder/artifact-decoder zero-distance discrepancy.
- Record any additional blocking contradictions before changing established graph semantics.

**Workstream 2 — Checked route geometry and identity**

Implement the approved graph-qualified route exposure.

Required properties include:

- Full graph-digest validation.
- Ordered directed traversal occurrences.
- Exact canonical geometry association.
- Correct forward/reverse geometry.
- Explicit traversal boundaries.
- Repeated traversal support.
- No-travel route support.
- Typed unavailable and invalid outcomes.
- No invented connectors or wrong-graph resolution.

Preserve compatibility with existing clients or provide an explicit versioned replacement.

**Workstream 3 — Route provenance and typed metrics**

Implement consistent supported information across single-route and alternative-route exposure:

- Route and evaluation identities.
- Graph and routing-profile context.
- Traffic/evaluation context where established.
- Departure context where applicable.
- Canonical distance.
- Predicted travel duration.
- Objective type and units.
- Alternative membership/rank semantics.
- Exact search/result termination semantics.

Correct the identified single-route versus alternative-route provenance asymmetry.

Do not introduce new routing cost calculations merely for visualization.

**Workstream 4 — Existing dispatch and selection evidence**

Expose the existing retained decision evidence needed by future dispatch visualization.

Preserve:

- Candidate coverage.
- Feasible/rejected/nonselected distinctions.
- Recommendation versus publication versus commitment.
- Insertion/fleet/recovery evidence granularity.
- Named algorithm-specific objective semantics.
- Accepted commitment protections.
- Search completeness and quality claims.

Do not fabricate individual candidate records from aggregate statistics.

Do not change the existing incomplete-search publication rules.

**Workstream 5 — Search summary diagnostics**

Expose and verify the existing Dijkstra, A*, alternatives, insertion, fleet, and recovery diagnostic summaries.

Preserve the exact current meaning of every counter.

In particular:

- `expanded_states` is not unique nodes visited.
- Source-equals-destination retains its established convention.
- Turn-aware search states remain distinguishable.
- A* repeated processing must not be misrepresented.
- Strict objective improvement remains the relaxation rule.
- Algorithm-specific work units must not be flattened into generic "steps."

Detailed frontier instrumentation remains deferred.

**Workstream 6 — Versioned API/export contracts**

Finalize the approved renderer-neutral representations.

Future clients should be able to consume supported:

- Directed route geometry.
- Route/evaluation provenance.
- Selected alternatives.
- Typed metrics.
- Existing decision evidence.
- Existing search summaries.
- Explicit capability and availability status.

Preserve the cinematic addendum's compatibility requirements:

- Route reveal can consume the actual computed path.
- Route-reveal progress is distinct from journey progress.
- Spatial search presentation uses actual candidate-coverage evidence where available.
- Candidate result order is not historical discovery order.
- Selected, recommended, proposed, committed, and active states remain distinct.

Do not add camera animations, glow settings, rider models, or renderer-specific keyframes to Rust domain contracts.

### Conditional implementation boundaries

Execution/timeline wrappers, specialized visualization snapshots, general geometry resolvers and additional historical adapters are not automatically mandatory.

Only implement a conditional producer if it is necessary for the approved finite package and its inclusion is explicitly justified under the audit.

Do not expand into new subscription infrastructure or a broad persistence redesign.

### Verification requirements

Implement the applicable Phase 26 adversarial proof matrix.

Tests should include:

- Wrong-graph dense-ID collisions.
- Complete directed geometry and interior shape points.
- Repeated traversals.
- No-travel and invalid geometry cases.
- Single/alternative provenance parity.
- Metric and objective units.
- Exact summary counters.
- Search completeness.
- Decision publication semantics.
- Required/optional version compatibility.
- Existing authorization boundaries.
- Representative resource and payload measurements.
- Noninterference with routing and dispatch results.

Run the relevant tests and benchmarks.

Do not claim passing evidence from tests merely inspected during the architecture grill.

### Documentation and completion evidence

Update approved specifications and ADRs to reflect the actual implemented contracts.

Reconcile stale documentation where appropriate.

Provide:

1. Exact implementation summary.
2. Changed modules/files.
3. New or modified contracts.
4. Tests executed and their actual results.
5. Resource/benchmark evidence.
6. Reproduced defects and their resolutions.
7. Compatibility implications.
8. Remaining known limitations.
9. Deferred capabilities and owning future phases.
10. Phase 26 conformance verdict.

Do not describe Phase 26 as visually complete.

## Implementation discipline

Work through the approved workstreams in dependency order.

Use focused commits or clearly identifiable implementation checkpoints where appropriate.

Continue autonomously through routine engineering decisions that are compatible with the frozen architecture.

Escalate material contradictions, required changes to frozen routing/dispatch semantics, or unavoidable scope expansion rather than silently deciding them.

The finish line is:

**Phase 26 implemented, tested, documented, and ready for a producer-conformance review.**

Not a mock UI, not a newly invented renderer, and not another architecture grill.

After Phase 26 is complete, stop and provide the completion evidence.

We will then proceed to Phase 27, with particular attention to a polished map foundation and the cinematic route-reveal experience.
