Yes. I accept the final audit verdict:

**No — Roadrunner must return to focused Phase 7 remediation and may not begin Phase 7.5 yet.**

The eleven accepted findings are now the exact Phase 7 remediation gate.

The grilling session is complete.

Stop asking audit questions and move into implementation.

# Objective

Remediate Phase 7 so that the OSM pipeline satisfies the accepted ADRs and all eleven findings from the Post-Phase-7 OSM Graph Audit.

Do **not** implement Phase 7.5.

Do **not** begin Phase 8.

Do **not** redesign later Roadrunner phases.

The scope is:

```text
Phase 7 implementation
        ↓
fix accepted audit findings
        ↓
rebuild fixtures/artifacts
        ↓
correctness + artifact + routing validation
        ↓
benchmark/statistics evidence
        ↓
Phase 7 readiness gate
```

Only after every mandatory item passes may Phase 7 be restored to complete.

---

# Source of truth

Before modifying code, reread:

```text
AGENTS.md

all relevant ADRs

the Post-Phase-7 OSM Graph Audit

the existing Phase 7 implementation

Phase 7 tests and fixtures

benchmark/statistics artifacts
```

The accepted audit decisions override implementation assumptions that contradict them.

If an ADR must be refined to reflect an accepted audit decision, update it explicitly rather than allowing code and architecture documentation to diverge.

---

# Implementation strategy

Do not implement the eleven findings in arbitrary numerical order.

First determine their dependency graph.

A reasonable implementation order is likely:

```text
1. Normalized source schema
        ↓
2. Access / directionality / speed semantics
        ↓
3. Source-to-graph provenance
        ↓
4. Graph compilation invariants
        ↓
5. Snapshot identity
        ↓
6. Artifact bundle + validation
        ↓
7. Topology diagnostics
        ↓
8. Real routing corpus
        ↓
9. Statistics + larger fixture
        ↓
10. lifecycle benchmark
        ↓
11. final Phase 7 gate
```

Refine this after inspecting the repository.

Make incremental changes and keep the repository buildable whenever practical.

---

# Remediation 1 — Restore the profile-independent normalization boundary

Fix `NormalizedOsmDataset` so it no longer irreversibly drops routing-relevant semantics simply because `delivery_motorcycle_v1` does not currently use them.

Preserve routing-relevant node semantics including the supported source schema for things such as:

```text
barrier
access
vehicle
motor_vehicle
motorcycle
ford
routing-relevant node highway semantics
```

Do not copy every OSM tag indiscriminately.

Maintain an explicit, versioned routing-source schema.

Expand candidate-way extraction beyond:

```text
highway=*
```

where Roadrunner's supported routing-source schema requires connectors such as ferries.

Preserve generic and vehicle-specific restriction variants independently.

For example:

```text
restriction=...
restriction:motorcycle=...
```

must not collapse into one value during extraction.

Source normalization may normalize representation.

It must not perform vehicle-profile interpretation.

Update the normalized-source schema/version as required.

Old incompatible normalized artifacts may require rebuilding rather than migration.

---

# Remediation 2 — Correct contextual access

Replace the single semantically collapsed contextual-access state with reason-specific effective access information.

Preserve distinctions such as:

```text
General

Destination
Delivery
Customers
Private
PermitRequired
UnknownExplicit
```

or an equivalent model.

The exact enum is not prescribed.

The invariant is that different restriction reasons survive compilation.

Built-in traversal evaluators must no longer ignore access.

The conservative rule is:

```text
General
→ Traversable

supported request-specific authorization
→ Traversable

otherwise
→ Forbidden
```

Do not introduce a generic:

```text
allow_contextual_access = true
```

escape hatch.

`delivery_motorcycle_v1` itself does not authorize every `access=delivery` road.

If correct endpoint-aware semantics for:

```text
destination
delivery
customers
```

are not yet implemented, keep those traversals forbidden by default rather than allowing them as shortcuts.

Unknown explicit access values must be diagnostic and conservative.

Add route-level regressions proving restricted roads cannot become unauthorized through-route shortcuts.

---

# Remediation 3 — Add source-to-graph provenance

Compilation must emit deterministic provenance connecting source OSM identities to compiled graph identities.

Phase 7.5 must eventually be able to resolve:

```text
from OSM way
+
via OSM node
+
to OSM way
```

into:

```text
incoming DirectedEdge
+
outgoing DirectedEdge
```

without reconstructing temporary `GraphBuilder` internals.

Provide mappings conceptually sufficient for:

```text
OSM node
→ routing NodeId

OSM way
→ ordered RoadSegmentIds

RoadSegment
→ Forward DirectedEdge if present
→ Reverse DirectedEdge if present
```

Preserve source-way-relative ordering and orientation semantics.

The exact storage layout is implementation-defined.

Provenance may live:

```text
inside the graph artifact
```

or as an:

```text
integrity-bound companion artifact
```

Do not pollute routing-hot structures unnecessarily.

Provenance must be:

```text
versioned
canonical
deterministic
validated
bound to the exact graph snapshot
```

Add fixtures containing complete:

```text
from
via
to
```

restriction relations.

The existing via-only splitting test is insufficient.

---

# Remediation 4 — Harden the graph snapshot artifact boundary

`FrozenGraph` must remain a trusted domain type.

Artifact bytes must first decode into an untrusted representation.

Normal loading must validate all inexpensive invariants required for routing correctness and structural safety.

This includes:

```text
canonical artifact representation
schema compatibility
coordinate ranges
dense-ID correspondence
adjacency offsets
adjacency/source correspondence
RoadSegment references
DirectedEdge orientation
duplicate traversal prevention
canonical geometry ranges
geometry bounds
numeric domains
capabilities
manifest agreement
provenance agreement
snapshot identity
```

Do not allow direct deserialization to bypass trusted constructors/invariants.

In particular, `CanonicalCoordinate` must not accept invalid WGS84 E7 values simply because Serde decoded them.

## Snapshot bundle

Treat:

```text
graph artifact
+
source provenance
+
manifest
```

as one logical:

```text
GraphSnapshot
```

They must be integrity-bound.

A graph from snapshot A must not successfully load alongside:

```text
manifest B
provenance B
```

even if all individual files are structurally valid.

## Deep verifier

Implement an explicit deep verification path.

Conceptually:

```bash
roadrunner graph verify --deep ...
```

or equivalent API.

Deep verification should independently recompute/check expensive invariants including:

```text
geometry-derived segment distance

polyline >= endpoint geodesic lower bound

free-flow travel-time derivation

adjacency reconstruction

important graph statistics

capability consistency

source-to-graph provenance
```

Deep verification is required during:

```text
artifact publication
audit fixtures
artifact regression tests
```

Ordinary startup may use the cheaper mandatory validator after integrity is established.

Publish graph snapshots atomically as a coherent bundle.

---

# Remediation 5 — Make topology explainable

Do not prune small components.

Instead make the compiler capable of explaining the final real graph.

Add deterministic diagnostics for:

```text
weak components
strong components
isolated nodes
tiny components
component bounds
extract-boundary interaction where reliable
```

Add split-reason diagnostics for routing nodes.

Conceptually:

```text
Intersection
WayEndpoint
RestrictionVia
Barrier
AccessBoundary
AttributeBoundary
ExtractBoundary
OtherSemanticBoundary
```

A node may have multiple reasons.

Also provide contraction diagnostics explaining why a source node was safe to contract.

Distinguish graph fragmentation caused by:

```text
source geography
extract boundary
profile filtering
access policy
directionality
missing connector
malformed source reference
compiler defect
contraction defect
```

After all upstream remediations are complete, rebuild the real fixture and explain every material component outside the largest WCC.

Do not spend effort preserving the current exact component distribution if the upstream corrections legitimately change it.

---

# Remediation 6 — Make directionality conservative

Keep the current architecture:

```text
RoadSegment = physical corridor
DirectedEdge = permitted traversal
```

Routing must not reinterpret `one_way` at runtime.

Create an explicit directionality compilation model distinguishing supported states from unsupported ones.

Conceptually:

```text
Bidirectional
ForwardOnly
ReverseOnly

UnsupportedDynamic
Contradictory
UnknownExplicit
```

Do not silently compile:

```text
unknown
contradictory
reversible
alternating
unsupported
```

directionality as bidirectional.

When semantics cannot be safely determined, preserve diagnostics and do not manufacture traversal permissions.

Document and test precedence involving:

```text
oneway:motorcycle
oneway
junction
directional access
profile-specific rules
jurisdiction policy
```

Correctly handle source orientation for:

```text
oneway=-1
```

Do not interpret it against later canonical segment orientation accidentally.

Audit/document the supported behavior of:

```text
junction=roundabout
junction=circular
```

Preserve provenance explaining why every one-direction-only segment exists.

Add compiler and route-level regressions.

---

# Remediation 7 — Make free-flow speed policy explicit

Preserve:

```text
free_flow_travel_time
=
deterministic uncongested traversal time
```

Do not turn it into traffic or live ETA.

Separate:

```text
legal speed limit

profile road-class default

profile maximum

surface constraint

tracktype constraint

smoothness constraint

effective free-flow speed

free-flow travel time
```

Missing source data is different from:

```text
invalid explicit data
unsupported explicit data
```

Preserve those distinctions diagnostically.

Define a versioned:

```text
delivery_motorcycle_v1 + ng_v1
```

policy for supported physical-suitability semantics.

Do not implement:

```text
surface=unpaved
→ Forbidden
```

merely because it is unpaved.

Instead establish and document the intended motorcycle policy:

```text
normal
speed capped
unsupported/forbidden
unknown explicit
```

or equivalent.

The exact speed values must come from the documented profile/policy, not arbitrary numbers inserted to make the audit pass.

The rebuilt fixture must explain the treatment of its unpaved ways.

Preserve speed-decision provenance sufficient to explain the effective speed.

---

# Remediation 8 — Enforce geometry-derived invariants

Preserve source identity even when coordinates coincide.

Do not merge nodes because:

```text
coordinate(A) == coordinate(B)
```

However, an ordinary physical road segment with:

```text
distance == 0
```

must not currently become an ordinary routable zero-cost traversal.

Explicitly diagnose/classify:

```text
repeated consecutive geometry points

coincident distinct endpoints

self-loops

fully zero-distance segments

lower-bound violations

geometry endpoint mismatch
```

For the current road model:

```text
ordinary zero-distance RoadSegment
→ non-routable / quarantined
```

while retaining provenance.

Enforce:

```text
polyline_distance
>=
Haversine(segment start, segment end)
```

within one centrally documented numerical tolerance.

Validate geometry orientation:

```text
geometry.first ↔ canonical start endpoint
geometry.last  ↔ canonical end endpoint
```

according to the final representation.

Verify reciprocal edges share:

```text
RoadSegment
geometry
physical distance
```

while using opposite traversal orientations.

Deep validation must recompute these properties.

Keep generic routing-algorithm zero-cost tests; those are separate from whether OSM compilation should generate zero-distance road traversals.

---

# Remediation 9 — Fix graph snapshot identity

The authoritative snapshot identity must identify the exact compiled graph semantics.

Introduce a full collision-resistant semantic identity conceptually equivalent to:

```text
GraphSnapshotDigest
```

using the existing SHA-256 infrastructure or equivalent.

It must change whenever canonical graph semantics change.

Its semantic inputs must account for, as appropriate:

```text
NormalizedOsmDataset identity

normalization/schema version

routing profile version

jurisdiction-policy version

compiler semantic version

graph schema version

provenance schema version

semantic build configuration

canonical graph payload

canonical provenance

capabilities
```

Prefer binding canonical produced outputs rather than relying solely on manually maintained input versions.

Avoid self-referential hashing.

Document the digest preimage.

Keep distinct concepts for:

```text
source PBF hash
normalized artifact hash
graph payload hash
provenance hash
GraphSnapshotDigest
```

The existing compact 64-bit `GraphSnapshotId` may remain as a runtime convenience if useful.

It must not remain the sole authoritative durable identity.

If retained:

```text
CompactSnapshotId
=
derived from GraphSnapshotDigest
```

and collisions must be detected when resolving it.

Durable:

```text
routes
replays
caches
benchmarks
provenance
overlays
```

must bind to the full snapshot identity.

Expand determinism tests for:

```text
repeated builds
normalized encounter-order changes
builder insertion-order changes
worker-count variation when parallel compilation exists
semantic configuration changes
```

Same semantic inputs must produce identical artifacts and digest.

Changed graph semantics must change the authoritative digest.

---

# Remediation 10 — Add the real compile–serialize–load–route corpus

After Remediations 1–9 are complete, rebuild the real fixture.

Then create a deterministic, versioned, preferably provenance-addressed real-data query corpus.

Do not pin the currently defective dense IDs.

Cover:

```text
short reachable route
medium reachable route
long reachable route

same origin/destination

cross-component unreachable pair

one-way route in both directions

parallel roads

component boundaries

restricted-access alternative

paved route
unpaved route
mixed-surface route
```

where the final fixture supports those cases.

For every query, compare:

```text
freshly finalized graph
```

against:

```text
serialize
→ load
→ mandatory validation
→ FrozenGraph
```

and require semantic equivalence.

Run both:

```text
Dijkstra
A* with compatible admissible heuristic
```

Use an independent route validator.

For every successful route verify:

```text
origin
destination

edge continuity

RoadSegment references

DirectedEdge orientation

route geometry orientation

physical distance reconstruction

objective-cost reconstruction

elapsed-time reconstruction

capabilities

GraphSnapshotDigest
```

Fresh and loaded graphs must produce equivalent deterministic results under the defined tie contract.

This is an internal Roadrunner correctness gate.

Do not introduce OSRM/GraphHopper comparison yet; that belongs to Phase 8.

---

# Remediation 11 — Expand statistics and lifecycle evidence

Keep the current tiny real fixture as the fast correctness fixture.

Add at least one materially larger pinned real-OSM engineering fixture.

It does not need to be city/state/country scale.

It only needs to be large and varied enough to provide useful lifecycle and memory evidence beyond the current tiny fixture.

Record a machine-generated statistics schema covering at least:

## Source

```text
PBF size

nodes seen / retained

ways seen / candidate / retained / rejected

relations seen

restriction relations seen/preserved

referenced nodes requested/resolved/missing
```

## Normalized source

```text
artifact size

normalized nodes
normalized ways
normalized relations

routing-relevant tag/value distributions

unsupported explicit semantics
```

## Compilation

```text
routing nodes

shape nodes contracted

split/preservation reasons

RoadSegments
DirectedEdges
geometry points

directionality distributions

access distributions

speed-decision distributions

physical-suitability distributions

zero-distance/geometry diagnostics
```

## Connectivity

```text
WCC count/distribution
SCC count/distribution
isolated nodes
tiny components
```

## Restrictions

```text
seen
preserved

node-via
way-via

no_*
only_*

generic
motorcycle-qualified
conditional

resolvable
ambiguous
unresolved
unsupported
```

where classification belongs to Phase 7.

## Artifact lifecycle

Measure separately:

```text
PBF extraction

normalized serialization

normalized load/validation

graph compilation/finalization

graph serialization

provenance serialization

snapshot publication

snapshot load + mandatory validation

deep verification

real query corpus
```

Record sizes for:

```text
graph payload
provenance payload
manifest
complete snapshot bundle
```

Measure peak memory, with documented methodology, for the important lifecycle stages where practical:

```text
extraction
compilation
load/validation
deep verification
```

Do not claim bounded-memory ingestion unless measurements support it.

Be explicit about current memory behavior.

Per-query allocation measurement is useful if practical but is not the sole Phase 7 blocker.

---

# Performance rule

Do not introduce speculative performance work while implementing these remediations.

Do not change:

```text
CSR layout
integer widths
geometry encoding
adjacency representation
allocator
mmap strategy
unsafe Rust
SIMD
graph compression
```

unless the new measurements expose a concrete problem and the change is independently justified.

The rule remains:

```text
Correct
↓
Tested
↓
Measured
↓
Profiled
↓
Optimized
```

---

# Testing requirements

Add focused unit, integration, property/regression, artifact, and end-to-end tests as appropriate.

Every defect discovered during remediation should receive a permanent regression test.

After each logical implementation unit, run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

Do not disable or weaken existing tests merely to complete the remediation.

Use the repository's existing benchmark commands where appropriate.

---

# Documentation requirements

Update:

```text
ADRs
AGENTS.md where Phase 7 semantics changed
architecture documentation
routing documentation
OSM/import documentation
benchmark documentation
glossary
```

where relevant.

The already-added:

```text
Phase 7.5 — Maneuver-Aware Routing & Turn Restrictions
```

section should remain.

Do not implement Phase 7.5 during this work.

Ensure documentation does not continue claiming:

```text
profile-independent normalization
trusted artifact loading
complete access semantics
stable snapshot identity
```

in ways that contradict the actual remediated implementation.

---

# Commit discipline

Keep commits logically scoped.

Do not put the entire remediation into one giant commit if sensible subsystem boundaries exist.

Possible commit families include:

```text
fix(osm): preserve profile-independent routing semantics

fix(profile): enforce contextual access policy

feat(graph): persist source-to-graph provenance

fix(artifact): harden graph snapshot validation

fix(graph): add topology diagnostics

fix(profile): make directionality conservative

fix(profile): derive explainable free-flow speed policy

fix(graph): enforce geometry invariants

feat(graph): add semantic snapshot digest

test(osm): add real artifact routing corpus

bench(osm): add real lifecycle benchmark
```

These are examples, not mandatory exact commit messages.

---

# Final Phase 7 gate

When all implementation work is complete, evaluate these exact eleven items:

```text
[ ] 1. Profile-independent normalization corrected

[ ] 2. Reason-specific contextual access preserved and safely enforced

[ ] 3. Validated source-to-graph provenance exists

[ ] 4. Graph/provenance/manifest snapshot boundary is hardened and integrity-bound

[ ] 5. Final real-fixture contraction and component fragmentation are causally explained

[ ] 6. Directionality is conservative, complete for the supported subset, and provenance-traceable

[ ] 7. Legal speed, defaults, physical suitability, effective speed, and free-flow time are separated and explainable

[ ] 8. Geometry, positive-distance, endpoint-lower-bound, reciprocal, and orientation invariants are enforced

[ ] 9. Authoritative semantic GraphSnapshotDigest exists and determinism/identity tests pass

[ ] 10. Real compile–serialize–load–route equivalence corpus passes

[ ] 11. Expanded statistics plus larger real lifecycle/memory benchmark exist
```

Also require:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

to pass.

Then run all relevant artifact deep verification and Phase 7 integration/benchmark commands.

---

# Final deliverable

When implementation is finished, produce:

```text
docs/phase-7-remediation-report.md
```

or an equivalent report.

For each of the eleven findings include:

```text
PASS / FAIL

what changed

relevant files/modules

tests proving it

artifact/fixture evidence

remaining limitations
```

Also include:

```text
final fixture statistics

larger-fixture lifecycle measurements

known unsupported OSM semantics

GraphSnapshotDigest

artifact/deep-verification result

real query corpus result

fmt/clippy/test results
```
Open a detailed PR when done

Then give one final verdict:

```text
Phase 7 COMPLETE
```

or:

```text
Phase 7 NOT COMPLETE
```

based strictly on the eleven-item gate.

If any mandatory item fails, do not weaken the gate to declare Phase 7 complete.

If every mandatory item passes, Phase 7 may be restored to complete.

## Stop condition

Even if the final verdict is:

```text
Phase 7 COMPLETE
```

stop there.

Do **not** implement Phase 7.5 automatically.

Do **not** implement Phase 8.

I want to review the remediation report first.

Begin by inspecting the dependencies among the eleven findings, then implement the Phase 7 remediation.
