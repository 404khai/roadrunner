# Roadrunner — Observability/Visualization Contract Grill
## Final Round Decisions: Q45–Q49

I accept Q45–Q49, with the refinements below.

This concludes the interactive decision-making portion of the Phase 26 architecture grill, provided no new material contradiction is discovered during consolidation.

**The next deliverable is the complete, read-only Observability/Visualization Contract Architecture Audit covering Q1–Q49. It is not Phase 26 implementation.**

The governing principle is:

**Phase 26 must establish and verify a bounded, reusable data-contract foundation for future route visualization, motorcycle-fleet inspection, search diagnostics, and 2D/3D clients. It must not absorb the implementation responsibilities of later phases merely because their requirements informed its design.**

---

## Q45 — What is the finite Phase 26 implementation boundary?

**ACCEPT WITH REFINEMENT — define an explicit, finite producer package with separate mandatory, conditional, and deferred work.**

I agree with the recommended finite producer package.

### A. Mandatory Phase 26 producer work

Subject to confirmation in the final audit and subsequent implementation authorization:

**1. Checked directed route geometry**

- Full graph-qualified identity.
- Ordered directed traversal occurrences.
- Exact association with canonical normalized segment geometry.
- Explicit traversal boundaries.
- Correct handling of reversed geometry.
- Valid no-travel route semantics.
- Strict validation of graph identity and geometry consistency.
- Typed unsupported, unavailable, and invalid outcomes.

**2. Route and evaluation identity**

Preserve the distinction between:

```text
Spatial path identity
Route evaluation identity
Returned alternative identity
Operational selection/commitment identity
```

Do not require a universal identity scheme if the corresponding producers have different scopes.

Stable identities must be grounded in the established graph, request, evaluation, and operational conventions. Array indices and alternative ranks are not durable identities.

**3. Consistent route provenance**

Single-route and alternative-route responses must expose consistent applicable provenance.

The agent identified a concrete discrepancy: alternative responses currently omit objective and traffic-context fields present in single-route responses.

That should become a Phase 26 conformance requirement.

Where values are genuinely unavailable for a supported historical artifact, represent their availability honestly rather than fabricating them.

**4. Typed route metrics and objectives**

Expose existing supported:

- Canonical route distance.
- Estimated elapsed road travel.
- Optimized objective and its units.
- Relevant departure/evaluation reference.
- Routing profile and graph/traffic context.
- Existing cost breakdowns, only where actually established.

Do not introduce a new cost model.

**5. Existing decision and selection evidence**

Expose retained routing, assignment, insertion, fleet, and recovery evidence according to its actual granularity.

Preserve:

- Evaluated versus selected.
- Proposed versus committed.
- Candidate coverage.
- Rejection/unavailability outcomes.
- Algorithm-specific comparison rules.
- Search and publication completeness.

Do not require storing every candidate or inventing evaluations that were not retained.

**6. Existing search summary diagnostics**

Preserve exact current counter and termination semantics for Dijkstra/A*, alternatives, insertion, fleet, and recovery where approved.

In particular, `expanded_states` must retain its current meaning.

Do not introduce fictional frontier events or detailed tracing simply because the future algorithm explorer will need them.

**7. Versioned exposure and compatibility**

Define supported output schemas, evidence semantics, versioning, exact units, optional capabilities, and compatibility/error outcomes.

Include scoped provenance and authorization where the underlying producer actually supports them.

### B. Conditional Phase 26 work

I want to be particularly strict here.

The accepted architecture covers frozen execution, simulation timelines, and coherent snapshots, but this does not automatically mean Phase 26 must create new services for all of them.

Classify the following as **conditional implementation work**:

- Additional visualization-oriented wrappers around frozen execution.
- Simulation timeline exports.
- Specialized coherent visualization snapshots.
- New artifact resolution endpoints.
- Expanded historical-evidence adapters.

They become mandatory only when the final audit establishes that they are necessary for the explicitly approved Phase 26 producer package and can be implemented without absorbing later-phase infrastructure.

Otherwise, document their compatibility contracts and assign them to the correct future phases.

Do not use the phrase "versioned compatibility representation" to justify rebuilding Phase 21/22 persistence or introducing another operational authority.

### C. Explicitly deferred implementation

Preserve deferral of:

```text
Continuous movement interpolation producers

Live GPS and physical telemetry

Turn-by-turn guidance generation

Detailed search/frontier instrumentation

General historical playback seeking

Snapshot-plus-stream infrastructure

WebSocket/SSE visualization subscriptions

Rider-scoped fleet projection services

3D building and terrain pipelines

Scooter models, rigs and animations

Visual customization interfaces

React/MapLibre application implementation

Map and scene provider integration
```

These capabilities remain requirements or future compatibility targets where relevant.

They are not automatically required for Phase 26 producer conformance.

### Accepted Q45 invariant

**Phase 26 implementation is a finite contract-and-producer package covering checked directed geometry, route/evaluation identity, consistent provenance, typed metrics, existing decision evidence, summary diagnostics, schemas, and conformance verification. Existing execution/simulation/snapshot adaptations require explicit inclusion in the approved implementation brief. Future navigation, telemetry, detailed diagnostics, streaming, rendering, asset, and customization capabilities remain separately owned. Every mandatory deliverable must identify its producer, representation, acceptance tests, dependencies, and owner.**

---

## Q46 — Which repository findings block conformance?

**ACCEPT WITH REFINEMENT — classify findings by reproducibility, reachability, affected contract, and ownership.**

I agree with the proposed classifications.

### 1. Graph-builder/artifact-decoder zero-distance discrepancy

Status: **Suspected conformance defect — not yet reproduced.**

Before remediation:

1. Construct a minimal reproducing case using the actual graph-builder and artifact round-trip path.
2. Determine whether the suspected inconsistency is reachable through supported production ingestion or only through a lower-level unsupported construction path.
3. Establish which frozen invariants each component is intended to enforce.
4. Determine whether valid no-travel routes are implicated.
5. Propose the smallest compatible change only if the discrepancy is confirmed.

Do not silently change graph semantics or allow invalid ordinary segments merely to make tests pass.

If this issue affects an approved Phase 26 producer, it must be resolved or explicitly contained before that producer receives conformance PASS.

### 2. Missing independent graph-digest guard

Status: **Established reusable-export precondition gap.**

The current route serializer apparently relies on callers using the matching graph.

That is not proof that the existing API currently returns wrong-graph geometry.

However, an independently reusable or detached Phase 26 exporter must not rely solely on that assumption.

Require an explicit graph-identity validation boundary and adversarial tests.

A route containing dense IDs from graph G1 must not resolve successfully against graph G2 merely because those IDs exist in both graphs.

### 3. Exact search-counter tests

Status: **Required Phase 26 verification gap.**

Add tests for the established meanings of exposed counters.

In particular:

- Non-stale queue pops.
- Goal pop inclusion.
- Source-equals-destination convention.
- Incoming-edge state semantics.
- A* repeated state processing where applicable.
- Stale queue entries.
- Algorithm-specific work units.

Do not alter counter meaning to make generic UI terminology easier.

### 4. Snapshot construction under coordinator lock

Status: **Established resource risk, with remediation ownership dependent on Phase 26 scope.**

If Phase 26 creates or modifies a visualization snapshot producer, require representative measurements and bounded resource behavior.

If Phase 26 only consumes the existing snapshot contract as a compatibility reference, do not automatically authorize a wholesale storage/API redesign.

Still record the existing lock contention risk clearly for later fleet-view implementation.

Any attempt to serialize outside the lock must preserve coherent captured state rather than access mutable authority after releasing synchronization.

### 5. Historical geometry resolution

Status: **Explicit availability limitation.**

Self-contained historical geometry can satisfy a narrower portable exposure claim.

Reference-only historical exports cannot claim general resolvability without an actual supported artifact resolver and retention contract.

No substitution with the currently adopted graph is permitted.

### 6. Snapshot/stream correspondence

Status: **Deferred incremental synchronization blocker.**

The current coherent `/v1/state` endpoint may support whole-snapshot replacement.

It does not establish a safe resume cursor for event streaming.

This blocks claims of gap-free incremental synchronization, not the narrower snapshot viewing contract.

### 7. Remediation discipline

For every identified issue, record:

```text
Finding identity
Evidence and reproduction status
Affected invariant
Supported reachability
Conformance impact
Required test
Responsible phase
Remediation scope
Approval requirement
```

Do not treat an unreproduced suspicion as a confirmed defect.

Do not classify an established unsupported capability as an implementation failure if it is legitimately outside Phase 26.

### Accepted Q46 invariant

**Correctness risks on approved producer paths require verified guards, reproduction, remediation, or explicit containment before conformance PASS. Suspected defects are not established failures until reproduced. Deferred historical resolution, streaming, physical telemetry, and renderer capabilities may remain unavailable without failing a narrower truthful Phase 26 contract. Foundational semantic changes and broad infrastructure redesigns require separate authorization.**

---

## Q47 — How should the adversarial proof matrix be structured?

**ACCEPT WITH REFINEMENT — claim-linked tests with independent oracles, evidence levels, and explicit phase ownership.**

I accept the proposed four proof classes:

1. Existing inspected evidence.
2. Mandatory Phase 26 producer tests.
3. Future producer/client tests.
4. External-data and real-telemetry validation.

But I want the proof matrix to be especially concrete.

### Mandatory Phase 26 tests

Include, as applicable to the approved producer package:

**Geometry and identity**

- Wrong graph digest rejection, even when dense IDs overlap.
- Forward and reversed directed traversal.
- Exact original interior shape points.
- Repeated edge/traversal occurrences.
- Graph-qualified route identity.
- Source-equals-destination no-travel route.
- Invalid/discontinuous geometry rejection.
- Unsupported snapping/connector cases.
- Unavailable detached geometry.
- Correct geometry encoding and coordinate order.

**Route evaluation and alternatives**

- Single-route and alternative-route provenance parity.
- Objective type and units.
- Graph, profile, traffic, and departure applicability.
- Route versus evaluation identity.
- Returned rank versus durable identity.
- Alternatives termination and search-scope semantics.
- No accidental use of display-simplified geometry for canonical metrics.

**Decision and objective evidence**

- Proposed versus committed.
- Feasible versus not selected.
- Incomplete versus infeasible.
- Fleet lexicographic objective ordering.
- Accepted commitment reference integrity.
- Waiting/churn penalty semantics.
- Aggregate evidence not promoted to candidate-level detail.
- Incomplete insertion/fleet/recovery results remain nonpublishable.

**Search summary diagnostics**

- Exact `expanded_states` semantics.
- Maneuver-aware incoming-edge states.
- A* repeated processing.
- Counter scope and termination.
- Source-equals-destination special behavior.
- No invented rejection causes or heuristic evaluations.

**Contracts and resources**

- Version compatibility.
- Unknown mandatory semantic rejection.
- Optional capability degradation.
- Exact numeric preservation.
- Authorized exposure of approved data.
- Resource/admission limits of implemented exporters.
- No modification of authoritative routing/dispatch results.

### Future-only tests

Retain explicit obligations for:

- Causal schedule interpolation.
- Exact semantic seeking and replay.
- 2D/3D progress equivalence.
- Motorcycle asset/preset compatibility.
- Visual alignment and ambiguous parallel roads.
- Terrain and overpass placement.
- Detailed search-capture noninterference.
- Snapshot/cursor atomicity and recovery.
- Authorization-scoped fleet projection.
- Physical telemetry accuracy.
- Provider coverage, asset licensing, and immersive visual fidelity.

These should not be reported as Phase 26 producer failures merely because their implementation does not yet exist.

### Independent test oracles

Where possible, tests must verify outputs against underlying graph, evaluation, or operational evidence—not merely compare a newly serialized response against a second representation generated by the same potentially faulty helper.

For example, reverse-geometry conformance should check the actual expected canonical point sequence and traversal association, not only matching endpoints.

Every case must identify:

```text
Claim under test
Evidence source/oracle
Input fixture
Expected invariant or outcome
Existing coverage
Required additional proof
Owning phase
Blocking status
Actual execution status
```

Inspected tests are not the same as freshly executed passing tests.

### Accepted Q47 invariant

**Every readiness claim is tied to a claim-specific proof obligation, independently grounded oracle where feasible, execution status, and phase owner. Existing tests are credited only for what they establish. Mandatory Phase 26 tests validate implemented producer semantics, while future rendering, telemetry, streaming, detailed trace, and external map-data tests remain visible but deferred.**

---

## Q48 — What verdict vocabulary should the final audit use?

**ACCEPT — separate architecture acceptance, foundation evidence, producer implementation, conformance, and capability readiness.**

The final audit should use explicit verdict dimensions.

Suggested states:

```text
Architecture:
    ACCEPTED / PENDING

Existing Foundation:
    EVIDENCED / PARTIALLY EVIDENCED / NOT ESTABLISHED

Producer Implementation:
    IMPLEMENTED / OUTSTANDING / OUT OF SCOPE

Conformance:
    VERIFIED / UNVERIFIED / FAILED / NOT APPLICABLE

Capability:
    SUPPORTED / LIMITED / DEFERRED / UNAVAILABLE
```

Do not aggregate these into a misleading maturity percentage.

Assess separately:

1. Route Visualization Data Contract.
2. Directed Geometry Conformance.
3. Route and Evaluation Identity.
4. Cost and Objective Observability.
5. Decision and Selection Evidence.
6. Search Summary Diagnostics.
7. Movement/Timeline Compatibility.
8. Navigation Guidance.
9. Coherent Whole-Snapshot Viewing.
10. Incremental Streaming/Resume.
11. Historical Inspection.
12. Deterministic Movement Playback/Seeking.
13. 2D Renderer Data Readiness.
14. 3D Scene Data Readiness.
15. Motorcycle Visual Asset Readiness.
16. Phase 26 Implementation Readiness.

A written architecture is not producer conformance.

Existing route geometry and operational evidence can be marked evidenced without claiming that the new Phase 26 visualization producer has been implemented.

Likewise, missing 3D models and terrain do not invalidate an otherwise properly scoped Phase 26 route exporter.

**Accepted Q48 invariant:** Each verdict names the precise claim and layer assessed. Architecture acceptance, existing foundation evidence, implemented producer readiness, conformance verification, and future visual capability are independently evaluated. No audit may claim completed visualization rendering, streaming, guidance, or detailed search replay from adjacent repository evidence alone.

---

## Q49 — What must be settled at architecture sign-off?

**ACCEPT WITH REFINEMENT — frozen architecture, finite implementation scope, explicit blockers, and independently authorized execution.**

I agree that the final architecture audit should become the authoritative planning reference for Phase 26 implementation.

However, distinguish two approvals.

### Approval A — Architecture confirmation

The final audit consolidates Q1–Q49, establishes the accepted contracts, reconciles phase ownership, and identifies implementation obligations.

I will review and explicitly confirm that audit.

### Approval B — Implementation authorization

Only after architecture confirmation will I separately authorize the agent to implement Phase 26.

The implementation authorization will determine the exact work package, tests, documentation changes, and final verification requirements.

Acceptance of Q1–Q49 is not itself authorization to edit the repository.

### 1. Final audit must contain an actionable implementation package

In addition to the full architectural analysis, include a dedicated:

**Phase 26 — Proposed Implementation Work Package**

Organize it into ordered workstreams such as:

```text
A. Existing-contract reconciliation

B. Graph-qualified route geometry and identity

C. Route/alternative provenance and typed metrics

D. Existing decision and selection evidence

E. Search summary diagnostic conformance

F. Approved versioned API/export representations

G. Adversarial correctness and compatibility tests

H. Authorization, resource and payload verification

I. Documentation, ADR and completion evidence
```

For each workstream, state:

- Exact existing producer/module involved.
- Proposed contract or behavior.
- Dependencies.
- Mandatory versus conditional scope.
- Acceptance criteria.
- Tests.
- Risks.
- Expected changes to existing interfaces.
- Whether the work is permitted under the original Phase 26 roadmap.

Do not treat these proposed workstreams as permission to modify files yet.

### 2. Preserve the original roadmap

The original roadmap assigns Phase 26 to observability/data contracts and Phase 27 to UI implementation.

The architecture audit may add requirements for future phases, but must not silently rewrite their ownership.

If a roadmap amendment is necessary, recommend it explicitly for later authorization.

### 3. Resolve documentation contradictions in the audit

The agent identified an outdated statement in `architecture.md` that conflicts with later implemented persistence documentation.

Classify it as stale documentation.

Recommend the appropriate amendment, but do not change it during the grill.

Do not reinterpret currently implemented persistence as nonexistent because an earlier paragraph is outdated.

### 4. Do not reopen accepted decisions without evidence

Q1–Q44 already establish extensive architecture.

Q45–Q49 should finalize scope and proof obligations, not restart earlier debates.

If consolidation reveals a genuine contradiction between accepted invariants or implementation evidence, report it with exact references and explain what decision is required.

Otherwise, preserve the accepted design.

Routine code organization, naming, and equivalent implementation techniques may remain engineering decisions after implementation authorization.

### 5. Preserve the visual product specifications

The final audit must include both previously supplied reference images:

**Navigation/environment reference**

- Polished 2D street-level map.
- Smooth movement.
- Perspective-based 3D presentation.
- Buildings and urban detail when supported.
- Seamless 2D/3D switching.

**Delivery rider reference**

- Stylized blue delivery scooter.
- Helmeted courier.
- Dark riding clothes.
- Rear delivery box.
- One consistent model represented by front/back/top reference views.
- Future cosmetic customization of clothing, helmet, scooter colors, delivery boxes, and accessories.

These are future client/product requirements, not Phase 26 rendering deliverables.

Retain the distinction between the existing `delivery_motorcycle_v2` routing profile and the future renderer-owned scooter model.

### 6. Final audit must be self-contained

The completed architecture audit should allow an implementation agent to understand Phase 26 without needing to reconstruct all 49 conversational questions.

Include:

- Executive summary and readiness verdict.
- Repository evidence and roadmap reconciliation.
- Complete accepted decision register Q1–Q49.
- All accepted refinements.
- Frozen invariants.
- Required producer contracts.
- Typed availability and failure semantics.
- Implementation versus future ownership.
- Risk and blocker register.
- Required ADR/specification amendments.
- Adversarial proof matrix.
- Exact conformance gates.
- Ordered proposed work package.
- Explicit sign-off boundary.

Keep the detail necessary for implementation rather than reducing the accepted architecture to a short summary that loses essential conditions.

### Accepted Q49 invariant

**The architecture grill closes with a comprehensive, evidence-grounded, read-only audit and finite Phase 26 implementation proposal. Final architecture confirmation and implementation authorization are separate user-controlled gates. The audit must preserve accepted decisions, identify remediation prerequisites, document deferred capabilities, and supply an actionable work package. No repository changes are authorized by accepting individual questions or by completing the audit.**

---

# Final Round Decision Register

| Question | Accepted decision |
|---|---|
| Q45 | Finite mandatory producer package; conditional existing-evidence adapters; later visualization, streaming, and instrumentation explicitly deferred |
| Q46 | Reachability- and evidence-based conformance blockers; reproduce suspected defects before remediation; protect approved export boundaries |
| Q47 | Claim-linked adversarial proof matrix with independent oracles, actual test status, and phase ownership |
| Q48 | Separate architecture, existing foundation, producer implementation, conformance, and visual-capability verdicts |
| Q49 | Frozen architecture with exact implementation package, explicit prerequisites, and separate audit sign-off and coding authorization |

---

# Next Action — Consolidate the Complete Architecture Audit

Record Q45–Q49 as accepted with the refinements above.

We have now completed ten rounds and **49 architectural decisions**.

Do not continue generating questions simply to extend the grill.

Review the complete decision register and repository evidence for consistency.

If no genuine unresolved architectural contradiction remains, produce:

# Roadrunner — Observability/Visualization Contract Architecture Audit

The audit should consolidate all accepted decisions from Q1–Q49.

Preserve the full Phase 26 scope reconciliation, the verified-versus-unverified distinction, the two visual references, motorcycle rider requirements, and the four readiness gates.

At the end, provide:

1. **Architecture readiness verdict.**
2. **Phase 26 entry prerequisites.**
3. **Finite mandatory implementation checklist.**
4. **Separately authorized or conditional remediation items.**
5. **Deferred future-phase capabilities.**
6. **Phase 26 producer-conformance acceptance criteria.**
7. **Recommended implementation sequence.**
8. **Explicit request for my architecture sign-off.**

No files should be changed, no ADRs written, no commits made, and no Phase 26 implementation begun as part of this consolidation.

**Conclude the grill and present the consolidated audit for my review.**
