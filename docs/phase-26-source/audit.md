# Roadrunner — Observability/Visualization Contract Architecture Audit

**Status:** Consolidated for architecture confirmation.
**Decisions:** Q1–Q49 accepted with the user’s refinements.
**Inspection baseline:** Git HEAD `0ef8c93156c03bb46ca7162c103eec4062b96c5f`, supplemented by the supplied reference images and accepted session decisions.

This audit is the read-only deliverable of the architecture grill. No repository files, ADRs, dependencies, assets, commits or implementation were created or changed. No tests or benchmarks were executed during the grill. References to tests mean inspected coverage; references to completion reports mean retained evidence reported by those documents.

No new material architectural contradiction was found during consolidation. Known documentation inconsistencies, capability limits and unverified findings remain explicit below.

## 1. Executive assessment and readiness verdict

**The architecture is ready for confirmation. Phase 26 producer conformance is not established, and implementation remains separately unauthorized.**

Roadrunner already has substantial foundations:

- Immutable graph identity and canonical directed road geometry.
- Manual Dijkstra/A*, maneuver-aware search and bounded route alternatives.
- Typed route costs and separate elapsed travel time.
- Structured dispatch, insertion, fleet and recovery evidence.
- Frozen execution with distinct action, plan and schedule identities.
- Retained synthetic start, delay, arrival and pending-work evidence.
- Coherent operational snapshots.
- Durable storage and local event recognition/replay mechanisms.

These foundations do **not** establish continuous physical tracking, turn-by-turn guidance, detailed historical search traces, browser subscriptions, general movement seeking, a map renderer or immersive 3D fidelity.

The accepted architecture is:

1. **Roadrunner domain evidence:** authoritative and supported routing, operational, simulation and historical facts.
2. **Shared presentation derivations:** versioned progress, geographic presentation position and route-derived orientation.
3. **Renderer-owned presentation:** map projection, camera, lighting, scenes, scooter assets, animation and cosmetic customization.

Phase 26 implements a bounded evidence-exposure package. Its design anticipates later clients without absorbing their implementation.

The four gates remain separate:

- **Gate A — Architecture confirmation:** this audit requires explicit confirmation.
- **Gate B — Producer conformance:** separately authorized Phase 26 implementation and verification.
- **Gate C — Client readiness:** actual later clients successfully consume the contracts.
- **Gate D — Visual fidelity:** rendering, assets, animation, alignment and performance are visually verified.

Acceptance of Q1–Q49 does not complete Gate A automatically and authorizes no coding.

## 2. Actual roadmap scope and dependencies

[AGENTS.md’s Phase 26 definition](/Users/admin/Developer/roadrunner/AGENTS.md:1971) prepares route data before UI implementation. It lists optional route identity, geometry, topology, distance, ETA, cost breakdown, algorithm diagnostics and alternative rank.

[Phase 27](/Users/admin/Developer/roadrunner/AGENTS.md:2031) introduces the TypeScript/React/MapLibre visualization application. Later phases own alternative comparison, graph debugging, delivery animation, dispatch inspection and historical replay.

The accepted reconciliation has three tiers.

**Tier A — Mandatory Phase 26 producers**

- Checked graph-qualified directed route geometry.
- Ordered traversal occurrences and explicit boundaries.
- Scoped spatial-path, evaluation, alternative and selection identities.
- Consistent provenance.
- Typed existing metrics and objectives.
- Existing decision/selection evidence at its retained granularity.
- Existing search-summary diagnostics.
- Versioned exposure, availability, compatibility and conformance tests.
- Authorization and resource verification for implemented exposure.

**Tier B — Conditional implementation**

- Frozen-execution visualization wrappers.
- Simulation timeline exports.
- Specialized coherent visualization snapshots.
- Artifact-resolution endpoints.
- Expanded historical adapters.

These are mandatory only if explicitly included in the approved implementation brief as necessary for the finite producer package. Otherwise, Phase 26 documents compatibility and assigns implementation elsewhere.

**Tier C — Deferred implementation**

Continuous interpolation producers, physical telemetry, guidance generation, detailed search tracing, seeking, subscriptions, scoped fleet projections, scene pipelines, scooter assets, animation, customization UI and the React/MapLibre application.

Phases 23–25 are not assumed complete from the roadmap alone. Actual dataset/baseline/learned-ETA code and completion evidence exist. [Phase 25’s conclusion](/Users/admin/Developer/roadrunner/docs/phase-25-completion.md:237) is specifically offline synthetic research with operational promotion rejected after improvement was not demonstrated.

No learned operational ETA, physical generalization or live-model integration follows from that completion.

## 3. Repository capability and evidence matrix

The following evidence identifiers are reused in the decision register.

**E1 — Roadmap and ownership.**
[AGENTS.md:1971](/Users/admin/Developer/roadrunner/AGENTS.md:1971) and [Phase 27:2031](/Users/admin/Developer/roadrunner/AGENTS.md:2031). Phase 26 is data preparation; rendering belongs later.

**E2 — Graph identity.**
[ADR 0001](/Users/admin/Developer/roadrunner/docs/adr/0001-graph-lifecycle-and-snapshot-identity.md:49). Full semantic SHA-256 digest is authoritative for detached references; compact IDs are convenience identities.

**E3 — Geometry ownership.**
[ADR 0002](/Users/admin/Developer/roadrunner/docs/adr/0002-road-segments-directed-edges-and-geometry.md:13) and [RoadSegment](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/graph/segment.rs:35). Contracted shape points remain in a shared canonical geometry pool; directed edges carry orientation.

**E4 — Coordinates and distance.**
[CanonicalCoordinate](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/geo/canonical_coordinate.rs:9), [exact OSM E7 conversion](/Users/admin/Developer/roadrunner/crates/roadrunner-osm/src/extract.rs:489), [Haversine](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/geo/haversine.rs:3) and [BoundingBox](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/geo/bounding_box.rs:6). WGS 84 E7 geometry, spherical distance and explicit antimeridian-box limitation exist.

**E5 — Current route export.**
[API geometry/response](/Users/admin/Developer/roadrunner/crates/roadrunner-api/src/routing.rs:46) and [reverse-geometry test](/Users/admin/Developer/roadrunner/crates/roadrunner-api/tests/http.rs:120). Directed shape reconstruction exists. Consecutive duplicates are removed; edge boundaries are not preserved by the flattened line alone. The helper has no independent graph-digest comparison.

**E6 — Graph validation and suspected degeneracy gap.**
[Builder endpoint checks](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/graph/network.rs:250), [distance accumulation](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/graph/network.rs:314), [decoder positivity check](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/graph/artifact.rs:510), [deep verification](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/graph/artifact.rs:827). Builder/decoder zero-distance consistency remains unreproduced.

Artifact decoding verifies canonical semantic identity; it does not itself call `verify_graph_deep`. These guarantees must not be conflated.

**E7 — Route result and summaries.**
[RouteResult](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/routing/result.rs:19). Graph identity, ordered nodes/edges, distance, objective, elapsed time and expansion count exist. Universal route/evaluation identity, full context and ordered trace do not.

**E8 — Search semantics and heuristics.**
[Dijkstra](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/routing/dijkstra.rs:75), [A*](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/routing/astar.rs:75), [evaluation](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/routing/search.rs:60), [ADR 0004](/Users/admin/Developer/roadrunner/docs/adr/0004-astar-heuristic-policy.md). Node/incoming-edge state modes, stale checks, strict relaxation and named heuristic policies are implemented.

**E9 — Alternatives.**
[AlternativeRoutes](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/routing/alternatives.rs:76), [selection](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/routing/alternatives.rs:285), [overlap](/Users/admin/Developer/roadrunner/crates/roadrunner-core/src/routing/alternatives.rs:421). Selected paths, options, work and termination exist; rejected candidate history is not exported.

**E10 — Basic dispatch evidence.**
[Decision contracts](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/decision.rs:24) and [ADR 0012](/Users/admin/Developer/roadrunner/docs/adr/0012-dispatch-evaluation-decision-evidence-and-commit.md). Candidate coverage, feasible/rejected evidence and selection reasons exist; full hypothetical geometry is optional.

**E11 — Insertion and accepted terms.**
[AcceptedTerms](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/pooling.rs:96), [impacts/search evidence](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/pooling.rs:300), [ADR 0016](/Users/admin/Developer/roadrunner/docs/adr/0016-multi-order-normative-audit.md). Immutable acceptance references, protections and scoped exhaustive insertion semantics exist.

**E12 — Fleet and recovery.**
[Fleet evidence](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/fleet.rs:135), [fleet comparator](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/fleet.rs:288), [recovery outcomes](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/recovery.rs:69). Algorithm-specific objectives, aggregate work and fail-closed incomplete-search behavior exist.

**E13 — Frozen execution.**
[Operational execution](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/operational.rs:124), [delay](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/operational.rs:749), [accepted effects](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/operational.rs:810). Action, plan, generation, route and effect identities are distinct.

**E14 — Simulation evidence.**
[SyntheticTrace](/Users/admin/Developer/roadrunner/crates/roadrunner-simulation/src/trace.rs:12), [endpoint movement](/Users/admin/Developer/roadrunner/crates/roadrunner-simulation/src/event.rs:73), [execution scheduling](/Users/admin/Developer/roadrunner/crates/roadrunner-simulation/src/engine.rs:1321), [ADR 0020](/Users/admin/Developer/roadrunner/docs/adr/0020-eta-dataset-evidence-and-synthetic-traces.md:20). Starts, delays, accepted arrivals and pending work exist; continuous positions and seeking do not.

**E15 — Coherent snapshot and current HTTP surface.**
[`/v1/state`](/Users/admin/Developer/roadrunner/crates/roadrunner-api/src/http.rs:83) and [router](/Users/admin/Developer/roadrunner/crates/roadrunner-api/src/http.rs:406). Coherent whole-state viewing exists; browser subscription and certified resume cursor do not.

**E16 — Persistence and artifact availability.**
[Persistence contract](/Users/admin/Developer/roadrunner/docs/persistence.md:124), [route recording](/Users/admin/Developer/roadrunner/crates/roadrunner-storage/src/lib.rs:458), [single-graph opening](/Users/admin/Developer/roadrunner/crates/roadrunner-storage/src/lib.rs:203), [storage geometry helper](/Users/admin/Developer/roadrunner/crates/roadrunner-storage/src/entities.rs:283). Retained evidence exists; general historical resolution and mixed-graph viewing are not established.

**E17 — Events and processing replay.**
[Events contract](/Users/admin/Developer/roadrunner/docs/events.md:85), [fact-effect identity](/Users/admin/Developer/roadrunner/crates/roadrunner-storage/src/events.rs:745), [reducer coverage](/Users/admin/Developer/roadrunner/crates/roadrunner-storage/src/events.rs:775). Source-local delivery and recognition exist. The current projection covers selected world/execution/context/effect facts, not the entire `/v1/state` representation.

**E18 — Motorcycle policy and guidance limitations.**
[Motorcycle policy](/Users/admin/Developer/roadrunner/crates/roadrunner-osm/src/policy.rs:11), [tag extraction](/Users/admin/Developer/roadrunner/crates/roadrunner-osm/src/extract.rs:358), [provenance](/Users/admin/Developer/roadrunner/crates/roadrunner-osm/src/provenance.rs:73). Motorcycle routing is real; comprehensive guidance metadata is absent.

**E19 — Authorization and resources.**
[Credentials/configuration](/Users/admin/Developer/roadrunner/crates/roadrunner-api/src/lib.rs:22), [snapshot construction](/Users/admin/Developer/roadrunner/crates/roadrunner-api/src/http.rs:94), [request-body limit](/Users/admin/Developer/roadrunner/crates/roadrunner-api/src/http.rs:440), [trusted-local boundary](/Users/admin/Developer/roadrunner/crates/roadrunner-api/README.md:27).

**E20 — Historical interpretation and versioning.**
[HistoricalRecord](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/history.rs:17), [ADR 0018](/Users/admin/Developer/roadrunner/docs/adr/0018-planning-time-and-historical-provenance.md:33), [strict route request](/Users/admin/Developer/roadrunner/crates/roadrunner-api/src/model.rs:28).

**E21 — Offline ETA research.**
[Phase 23 completion](/Users/admin/Developer/roadrunner/docs/phase-23-completion.md), [Phase 24 completion](/Users/admin/Developer/roadrunner/docs/phase-24-completion.md), [Phase 25 verdicts](/Users/admin/Developer/roadrunner/docs/phase-25-completion.md:220). These support narrow offline synthetic claims, not live learned routing or physical telemetry.

## 4. Visual references and product goals

### Navigation and environment reference

![Navigation-quality 2D and immersive 3D reference](/Users/admin/Developer/roadrunner/references/google-mps-3d-and-2d.webp)

The reference establishes the intended quality of route emphasis, readable roads, smooth travel presentation, follow cameras, perspective and urban surroundings.

It does not supply licensed reusable scene assets, Roadrunner telemetry, lane evidence, legal limits or elevation.

### Primary courier-design reference

![One courier and scooter shown from front, back and top](/Users/admin/Developer/roadrunner/references/rider_top_back_front_views.png)

The default future avatar is one stylized delivery scooter/courier:

- Rounded blue scooter.
- Gray helmet.
- Dark riding clothes.
- Substantial dark rear delivery box.
- Recognizable front, rear and overhead silhouette.
- Simplified, polished proportions suitable for navigation scale.

Future cosmetics include outfits, helmets, paint, delivery boxes and accessories. Compatible base models, attachments, rigs and animations should be reusable.

These images are inspected product references, not implemented sprites or usable 3D assets. Cosmetic choice cannot change routing profile, capacity, action identity or operational history.

## 5. Visualization purposes and audiences

Five purposes share evidence without sharing identical readiness or budgets:

1. **Route exploration:** selected paths, returned alternatives, geometry and cost.
2. **Fleet/dispatch inspection:** assignments, commitments, plans, frozen execution and supported explanations.
3. **Simulation visualization:** scenario execution with declared illustrative movement.
4. **Historical inspection/replay:** retained facts under original semantics and explicit temporal interpretation.
5. **Algorithm exploration:** summaries today; actual ordered traces only when later produced.

Consumer navigation presentation is a future client of these contracts, not a replacement for engineering inspection.

A standalone routing result is not automatically an assignment. A proposed assignment is not a commitment. A frozen route is not a measured trajectory.

## 6. Authority, derivation and evidence taxonomy

Every meaningful claim must retain or inherit:

- Subject identity and scope.
- Evidence/source identity.
- Authority and scientific source quality.
- Applicable time and clock domain.
- Derivation/model identity.
- Completeness and availability.
- Uncertainty or explicit unknown status.

The conceptual taxonomy is:

- Authoritative operational fact.
- Accepted source observation.
- Simulation fact.
- Deterministic prediction.
- Learned prediction.
- Derived interpolation.
- Hypothetical candidate.
- External presentation data.
- Unknown/unavailable.

These are semantic categories, not mandated enum names. An accepted observation is not automatically an independently verified physical measurement.

Shared provenance envelopes may avoid duplication. Individual claims must override inherited metadata when their source or meaning differs.

Missing position is not stationary position. A held marker is a display policy. A 60-FPS animation does not create 60 observations per second.

Temporal meanings remain separate: simulation time, operational effective time, source occurrence, processing, publication evidence, playback selection and frame time.

## 7. Spatial identity and canonical route contract

The canonical contract is semantic, not “one GeoJSON line.”

It must preserve:

- Full graph semantic digest.
- Ordered directed traversal **occurrences**.
- Explicit occurrence boundaries.
- Exact association with pinned normalized segment geometry.
- Direction/orientation.
- Route anchors and supported no-travel semantics.
- Originating evaluation and applicable context.
- Availability and validation outcomes.

An occurrence index is meaningful only within its immutable route identity. It is not response-array identity. Repeated use of the same EdgeId must remain distinguishable.

Four identities remain separate:

1. **Spatial path:** graph plus ordered traversals.
2. **Evaluation:** path under request, objective, context, departure and algorithm.
3. **Returned alternative:** member of a particular result set.
4. **Operational selection:** proposal, commitment or frozen execution relationship.

No universal identifier is required. Each produced stable identity needs a documented scope, canonicalization/generation rule and tests. Rank is never durable identity.

## 8. Coordinate, projection and precision semantics

Canonical geometry is WGS 84 E7 latitude/longitude. Supported exact encodings must round-trip E7-origin values.

GeoJSON positions use `[longitude, latitude]`.

Keep distinct:

- E7 representation precision.
- Source accuracy.
- Floating-point conversion/rounding.
- Geographic distance model.
- Screen projection.
- Local scene coordinates.
- Terrain/road placement.

OSM ingestion rejects finer-than-E7 source precision; the general floating-point conversion helper explicitly rounds. They are different boundaries.

Segment distance is the canonical polyline’s accumulated spherical Haversine distance. It is not ellipsoidal, terrain-following or physical wheel-path distance.

No altitude is present. Local scene `z = 0` is not observed altitude zero.

Antimeridian-crossing boxes are deferred. Global seam and polar rendering are not established merely because coordinate ranges accept global values. Unsupported transforms must preserve original evidence and report their limitation.

## 9. Directed reconstruction, continuity and failure outcomes

The approved exporter must validate:

- Matching full graph identity.
- Valid referenced nodes, edges and segments.
- Directed occurrence orientation.
- Topological continuity.
- Geometry endpoint consistency.
- Supported anchor semantics.
- Encoding requirements.

Coordinate equality does not create connectivity. Crossing lines do not establish an intersection.

The current API reverses segment geometry correctly and removes consecutive duplicate display positions. That normalization must not erase the ability to inspect exact canonical points and traversal boundaries.

Conceptual outcomes include:

- Valid ordinary route.
- Valid no-travel route.
- Geometry unavailable.
- Unsupported connector/snapping.
- Graph mismatch.
- Invalid/discontinuous geometry.
- Unsupported historical interpretation.

A source-equals-destination route has zero traversals. Its two-identical-position GeoJSON encoding is an encoding convention, not an ordinary zero-distance road segment.

No exporter may bridge gaps, infer connectors or substitute a new graph. Partial historical inspection may expose valid portions only with explicit incompleteness.

## 10. Navigation and maneuver boundaries

Three layers remain distinct:

1. Route/geometric evidence.
2. Semantic maneuver classification.
3. Localized human-readable instruction.

A road bend is not necessarily a turn instruction. A restriction name is not guidance. A roundabout directionality rule does not establish exit numbering.

Current frozen edges do not retain names, road class, junction type, ramps, lanes or signals. Some source metadata survives normalization; names, references, destination signs and `turn:lanes` are dropped by current extraction.

Future guidance enrichment should be immutable and graph-qualified, with correct source-way-to-segment/direction mapping. It must distinguish original source evidence, later enrichment and current basemap content.

Guidance anchors must identify a traversal occurrence or incoming/outgoing occurrence transition, optionally with canonical route distance. Repeated junctions and loops cannot be anchored by coordinate alone.

Guidance consumes the pinned motorcycle route and context. It cannot authorize access or silently reroute through a car SDK. Missing guidance does not invalidate a supported route.

Legal speed, routing speed, animation speed and measured speed are separate typed claims. A class fallback or profile maximum cannot populate a legal-limit sign.

Full guidance generation remains deferred.

## 11. Execution identity and movement timeline

Existing execution separates:

- Operational namespace/revision.
- RiderId.
- ActionId.
- Originating plan revision.
- Frozen route/provenance.
- Current attempt and schedule generation.
- Execution stage.
- Accepted effects.

Stages are `Travelling`, `Arrived`, `Waiting` and `Servicing`. Prediction timers do not establish arrival, readiness, custody or completion.

A delay preserves ActionId and original route while changing scheduling. Updated `leg.travel` is not elapsed-trimmed remaining travel time; consumers must derive the remaining presentation interval from actual temporal anchors.

Editable suffix changes do not restart a frozen action. Context adoption does not replace its graph or original traffic prediction.

Existing execution/timeline adapters are conditional Phase 26 implementation, not automatically new services.

## 12. Simulation interpolation and deterministic movement replay

Smooth motion is permitted as a **versioned illustrative derivation**.

A future initial model may parameterize canonical route distance between supported temporal anchors using an explicit uniform-progress assumption. This is not measured driving.

For a causal schedule intervention:

1. Reconstruct progress using evidence available before the intervention.
2. Preserve that progress.
3. Model subsequent remaining progress toward the revised arrival.
4. Do not rebase earlier frames using later duration.

Handle zero-travel actions, simultaneous transitions, invalid geometry and expired predictions explicitly. RoadDelay does not establish where the rider stopped or slowed.

If predicted arrival passes without accepted arrival, presentation may hold an estimated endpoint or show uncertainty. It cannot continue beyond the route or complete the action.

Deterministic semantic seeking requires sufficient retained starts, schedule changes, transitions, ordering and model configuration. A latest execution snapshot alone may omit intervening schedule history and cannot establish exact past presentation frames.

Current source-replay verification reruns compatible simulation inputs and compares results. It is distinct from playback of retained decisions. No arbitrary-time seeking service is established.

## 13. Live observation quality and position uncertainty

Roadrunner can know that a rider is travelling on a frozen action without knowing precise physical position.

Distinguish:

- Accepted reported position.
- Simulation-authoritative endpoint.
- Interpolated presentation position.
- Extrapolated estimate.
- Planned future position.
- Unknown position.

Physical heading, speed, altitude and uncertainty bounds require appropriate source evidence. Do not invent numeric uncertainty intervals or measurement precision.

Route-derived bearing may orient the scooter. It is not measured heading. Coincident points, sharp turns, short segments, stationary periods and missing geometry require explicit orientation policies.

Lean, steering, wheel rotation and pose remain renderer-derived. Shared geographic progress and directional basis feed both views; each renderer can animate differently without changing the journey.

Live GPS, map matching and measurement validation remain deferred.

## 14. Alternatives and comparison groups

Alternative objects must identify what varies:

- Path.
- Rider.
- Stop placement/order.
- Fleet plan.
- Recovery move.
- Context or algorithm.

Current route alternatives return selected diverse paths, not all evaluated candidates. Rank means position among returned selected routes.

Diversity uses shared physical-segment distance relative to the shorter route. Reciprocal use of one segment counts as shared. This is not a geometric-proximity or user-preference measure.

Termination distinguishes requested count, exhaustion, cost limit and budget. Requested count does not imply exhaustive enumeration.

Yen’s search scope is legal loopless paths. With maneuver-dependent legality, cheapest legal loopless path and cheapest legal walk are not universally interchangeable.

Historical candidates and fresh hypothetical evaluations remain separate. An unchosen rider’s predicted result is not an observed counterfactual.

## 15. Cost, objective, feasibility and accepted commitments

Every metric carries type, units, applicable context, evaluation reference, prediction source and accounting horizon.

Keep separate:

- Canonical road distance.
- Estimated elapsed road travel.
- Optimized route objective.
- Waiting.
- Service.
- Preference penalties.
- Deadline slack.
- Accepted protection impacts.
- Realized outcomes.

Comparison contracts remain algorithm-specific:

- **Basic/preparation dispatch:** named strategy; preparation penalties are preferences, not extra road travel.
- **Insertion:** signed incremental road seconds, then meters and canonical ties.
- **Fleet:** maximize newly admitted count; minimize remaining evaluable road seconds; minimize meters; canonical tie-breaking.
- **Recovery:** road travel plus explicit churn penalties, then distance/canonical ordering, under health/cooldown/threshold rules.

Fleet objectives exclude frozen/isolated constants according to their accounting contract. Admission count is not delivered count.

Original acceptance references remain immutable. Current and accepted protections cannot be weakened by rewriting plans. Cumulative consumption is nonnegative; improvements do not bank allowance.

Legacy soft deadlines do not become hard constraints. Hard planning protection is a model-qualified feasibility claim, not guaranteed physical delivery.

Unsupported base/traffic/turn decompositions remain unavailable.

## 16. Dijkstra/A* diagnostic semantics

Diagnostics distinguish graph nodes, logical search states and queue-entry occurrences.

State mode is node-based or incoming-edge-qualified with a source sentinel.

Precise operations include discovery, enqueue, pop, stale pop, accepted non-stale pop, actual traversal evaluation, strict relaxation, non-improvement, goal processing and termination.

`expanded_states` counts non-stale pops including the goal, with a source-equals-destination convention of `1` despite no ordinary queue processing. It is not unique nodes, all pops, outgoing evaluations or successful relaxations.

A* has no permanent closed set. Repeated improvements/processing must remain representable. “Settled” requires a supported proof, not visual convention.

A* `g`, actual computed `h` and `f` retain objective units; elapsed time is separate. Neighbor heuristics are computed only after strict improvement passes. Missing calls cannot acquire fictional historical values.

Maneuver rejection, generic evaluator `Forbidden`, non-improvement and evaluation failure are different outcomes. Generic prohibition cannot become a detailed access explanation.

Mandatory Phase 26 scope is summaries. Ordered detailed capture remains deferred.

## 17. Optimizer diagnostics and bounded capture

Shared envelopes may carry identity, parent scope, versions, context, capture and result metadata. Payloads and work units remain algorithm-specific.

- Alternatives: root/spur searches, path ranking, deduplication and overlap.
- Insertion: baselines, placement submissions, exclusions and deltas.
- Fleet: construction/neighborhood work, precedence exclusions and objective comparison.
- Recovery: eligibility, moves, feasibility, penalties and churn gates.

Counts are not interchangeable. Rejection-reason occurrences can exceed rejected submissions. Fleet feasible counts may include repeated submissions across starts. Baselines and exclusions may be outside search budgets.

Future capture states are disabled, complete, prefix-truncated, sampled and failed. Completeness is defined against the selected capture level, not array nonemptiness.

Capture budgets are independent of authoritative work budgets. Full buffers, allocation/serialization failure or slow consumers cannot alter search, tie-breaking or publication. Terminal outcome/capture summaries must remain available independently.

Prefix replay is valid only through its retained boundary. Sampling does not establish an exact frontier without additional state evidence.

Initial future traces may be bounded offline artifacts. No streaming service is required.

## 18. Snapshots, updates and resynchronization

Initial synchronization is coherent whole-snapshot replacement.

Snapshots identify supported namespace, view/source, schema, revision boundary, clock semantics, contexts, completeness and capture metadata. Older compatible revisions cannot silently overwrite newer state. Equal revision with a later time sample is not a new domain transition.

Incremental synchronization is deferred. Its preferred future design is a transactionally captured visualization projection and processed-source cursor, with explicit lag and reconciliation status.

Operational revision, storage revision, source sequence and projection position are different. A separately sampled stream high-water mark cannot certify `/v1/state`.

Message identity and business-effect identity are separate. Identical redelivery must not duplicate actions; conflicting identity must not become last-write-wins. Complete fact deltas apply atomically once under originating commit semantics.

The current Phase 22 projection is not a complete visualization replica. Cursor advancement after an approved omission is not omission-free coverage.

Resume requires compatible source/view/scope/continuity. Otherwise report resnapshot, missing predecessor, incompatible semantics, omission, conflict or unavailability. A fresh snapshot restores current viewing, not missing history.

## 19. Historical graph/context compatibility and temporal interpretation

Historical inspection, original geometry reconstruction, cost reproduction, decision replay, movement presentation and scene reproduction have different dependencies.

Two temporal interpretations are explicit:

- **Acceptance-causal:** evidence available through the selected processing/revision boundary.
- **Retrospective occurrence:** later retained reports about earlier occurrences, bounded by the chosen review source.

Late reports cannot create retroactive knowledge in acceptance-causal playback.

A semantic frame needs history/run scope, interpretation, source boundary, playback instant, equal-time ordering and presentation-model version. Sequential playback and direct seek must agree where supported.

Current graph/traffic cannot replace pinned historical inputs. Domain preservation of old-graph execution does not prove mixed-graph durable resolution. The current durable adapter remains single-graph.

Missing originals narrow reproduction claims. Modern recomputation is a new derived evaluation.

## 20. Renderer-neutral 2D/3D responsibilities

Both views consume the same:

- Rider/action identity.
- Pinned route and graph.
- Temporal boundary.
- Evidence-qualified canonical progress.
- Shared directional basis.
- Intended visual preset.

Camera transitions, loading and mode changes cannot reroute, restart, advance simulation time or adopt new context.

Canonical overlay is the default. Visual alignment is a separately versioned derivation binding canonical geometry, scene source and method. It requires a supported canonical-progress-to-rendered-placement correspondence.

Parallel/stacked roads and intersections can make proximity ambiguous. Unknown alignment is not verified alignment.

Alignment cannot reorder traversal occurrences, reverse direction, skip sections or redefine distance, cost, guidance anchors or execution.

## 21. Scene data, scooter assets, providers and licensing

Distinct capabilities include:

1. Flat perspective.
2. Terrain-supported placement.
3. Grade-separated road placement.
4. High-fidelity immersive scene.

Terrain is not bridge-deck elevation. Bridge/layer tags do not establish metric height. Decorative building heights and ground offsets remain presentation approximations.

Asset identity separates operational rider, visual preset, actual asset realization and animation state. Presets should support compatible modular scooters, riders, outfits, helmets and boxes. Procedural selection is stable under declared run/rider/configuration scope.

Fallbacks preserve journey identity. Semantic replay, preset replay, exact asset replay and pixel-identical rendering are different claims.

Provider evaluation must establish area/version-specific coverage, freshness, alignment, offline rights, caching, historical retention, redistribution, attribution, platform compatibility, performance and cost.

OSM data licensing and hosted tile-service permissions differ. OSM requires attribution under its licensing terms; the public standard raster service prohibits bulk/offline prefetch. Actual integrations must verify all relevant data/style/font/model licenses. [OSM licensing](https://www.openstreetmap.org/copyright), [tile policy](https://operations.osmfoundation.org/policies/tiles/).

No provider or additional rendering engine is selected by this audit.

## 22. Performance, payload and resource controls

Separate mutable views, immutable domain artifacts and presentation resources.

Portable exports retain declared dependencies. Routine polling need not repeat immutable geometry. References require real resolution and authorization.

Chunked snapshots must belong to one retained coherent capture, with completeness/integrity validation. Mixed-revision pages and partial installation are invalid.

Geometry and evaluation caches have different keys. A matching line does not establish matching cost/context. Content addressing does not grant access.

Operational events, polling, local progress sampling and rendering frame rate are independent. A 60-FPS client does not require 60 authoritative updates per second.

Implemented producers need measured CPU, memory, payload, capture-lock, serialization, transfer and authority-latency behavior. Outside-lock serialization requires a coherent immutable copy or equivalent stable capture.

Representative workloads must identify graph, point, rider/order/action, history, detail and concurrency characteristics. No fleet size, latency or memory promise is made here.

Overload must produce explicit outcomes or declared narrowing, not silently omitted riders/evidence.

## 23. Access, privacy and retention

Access purposes differ: public synthetic demo, current fleet inspection, rider-scoped inspection, engineering diagnostics, historical replay and export.

Authorization applies before transmission to linked routes, orders, other riders, explanations, aggregates and artifacts. Client-side hiding is insufficient. The current namespace snapshot is not a rider-scoped projection.

Synthetic origin alone does not authorize publication. Public demos require reviewed inputs and must not point at private operational namespaces.

Controlled caches and cursors follow authorization scope. Revocation stops future controlled exposure but cannot guarantee deletion of copies already exported.

Retention classes are:

- Authority/recognition/restoration evidence.
- Active execution dependencies.
- Historical domain artifacts.
- Detailed diagnostics.
- Client caches/exports.
- Presentation assets.

Current indefinite retention is not a universal policy. Changing restoration/idempotency dependencies requires an approved lifecycle design. Conflicting authorized deletion and restoration requirements must be explicitly reconciled.

Redaction creates identified derivatives. Optional evidence loss narrows corresponding reproduction claims; it does not authorize modern substitution.

## 24. Versioning and compatibility

Independent boundaries include envelope, route semantics, geometry encoding, graph identity, cost policy, diagnostic family, snapshot/view, presentation model, preset and asset/rig semantics.

Avoid redundant fields where an existing identity already supplies the guarantee, but do not collapse unrelated meanings into one version.

Compatibility is defined per direction and parser behavior. Additive fields are not universally compatible with strict decoders.

Unknown required coordinate, objective or authority meaning must fail safely. Unsupported explicitly optional features may degrade narrowly.

Preserve E7 round-trip, exact counter integers and established numeric comparison meaning. Display rounding is not search arithmetic.

Historical conversions retain original artifact identity/semantics, transformation identity and limitations. They produce derivatives rather than silently relabeling archives.

Availability, compatibility, authorization and current applicability are independent states.

## 25. Adversarial proof matrix

**Execution status for all cases below:** not freshly executed during this grill. Existing coverage is credited only for its inspected scope.

Classes:

- **P26:** mandatory when the relevant producer is included.
- **Conditional:** mandatory only for an explicitly included adapter.
- **Future:** later producer/client/infrastructure.
- **External:** provider or real-source validation.

### Geometry and identity

- **G1 — Overlapping dense IDs in wrong graphs.** Oracle: two deliberately distinct digests/geometries with matching local IDs. Existing request mismatch checks do not test detached-export misuse. **P26 blocker:** reject before resolution.
- **G2 — Forward/reverse shape.** Oracle: explicit canonical interior-point sequence and expected orientation. Existing API test checks count/endpoints, not every interior point. **P26 blocker:** full sequence and occurrence association.
- **G3 — Contracted shape points.** Oracle: segment geometry containing interior non-decision points. **P26 blocker:** no endpoint-only shortcut.
- **G4 — Repeated traversal occurrences.** Oracle: route with repeated directed edge occurrences. **P26 blocker:** preserve order and distinct occurrence associations.
- **G5 — No-travel route.** Oracle: one node, no edges, zero metrics; separate encoding convention. Existing core alternative coverage exists. **P26 blocker:** no invented road traversal.
- **G6 — Entirely zero-distance ordinary segment.** Oracle: actual builder→artifact round-trip reproducer, plus ingestion reachability analysis. **Unverified risk:** resolve/contain if reachable on the approved path.
- **G7 — Harmless repeated coordinates.** Oracle: positive-length canonical segment with duplicate adjacent points. **P26:** distinguish exact geometry from normalized encoding.
- **G8 — Discontinuous or reversed topology.** Oracle: independently checked edge endpoints and occurrence chain. **P26 blocker:** no bridge or repair.
- **G9 — Snapping/connectors.** Oracle: coordinate not matching explicit anchor. Existing API rejects unsupported snapping. **P26:** preserve typed boundary.
- **G10 — Detached geometry unavailable.** Oracle: unresolved declared dependency. **P26:** explicit unavailability, no current-graph substitution.
- **G11 — Coordinates and boundaries.** Oracle: E7 integers, axis-order fixtures and unsupported seam cases. **P26:** exact round-trip; no unsupported global claim.

### Evaluation and decision evidence

- **D1 — Single/alternative provenance parity.** Oracle: captured request/context and actual traffic snapshot. **P26 blocker:** consistent applicable objective/profile/traffic/departure.
- **D2 — Same path, different evaluation.** Oracle: identical topology with different contexts. **P26:** share geometry without merging evaluation identity.
- **D3 — Rank versus identity.** Oracle: reordered result sets. **P26:** stable artifact identity independent of returned rank.
- **D4 — Metric/unit mismatch.** Oracle: source evaluator’s tagged objective and elapsed-time accumulation. **P26 blocker:** no time/distance coercion.
- **D5 — Unsupported breakdown.** Oracle: producer lacking component evidence. **P26:** unavailable breakdown, not fabricated decomposition.
- **D6 — Alternatives scope/termination.** Oracle: tiny known graphs and configured count/cost/budgets. Existing tests cover several cases. **P26:** exact selected/ranked/work meanings.
- **D7 — Proposed versus committed.** Oracle: actual publication outcome and revisions. **P26:** stale/rejected proposals cannot appear committed.
- **D8 — Feasible versus rejected/unselected.** Oracle: retained candidate results and comparator. **P26:** no structural rejection inferred from losing rank.
- **D9 — Aggregate-only history.** Oracle: insertion/fleet evidence without candidate records. **P26:** no invented candidate rows/geometry.
- **D10 — Fleet ordering.** Oracle: independently constructed competing objective tuples. **P26:** count maximization precedes travel/distance minimization.
- **D11 — Acceptance integrity.** Oracle: immutable original terms and current candidate impacts. Existing protection tests support foundations. **P26:** no reset or banked delay credit.
- **D12 — Incomplete search.** Oracle: budgets stopping before completion. Existing insertion/fleet/recovery tests exist. **P26 blocker:** no publishable incumbent leakage.
- **D13 — Failure strength.** Oracle: generic `NoRoute`/`Forbidden` and missing inputs. **P26:** no invented singular cause or infeasibility proof.

### Summaries, compatibility and resources

- **S1 — Exact expansion counters.** Oracle: hand-traced deterministic operations, including stale and goal pops. **P26 blocker:** existing conventions preserved.
- **S2 — Trivial-route convention.** Oracle: shortcut path with no queue processing. **P26:** count 1 without fictional events.
- **S3 — Incoming-edge states.** Oracle: maneuver fixture with distinct arrival states. Existing reconstruction test exists. **P26:** no node-state collapse.
- **S4 — A* repeated processing/equal objectives.** Oracle: suitable valid heuristic/evaluator fixture and explicit label decisions. **P26:** preserve repeated processing and strict objective comparison.
- **S5 — Heuristic invocation.** Oracle: actual call sites/controlled evaluator evidence. **P26:** no values for unmade calls.
- **S6 — Algorithm work units.** Oracle: explicit submission, path-pop and exclusion accounting. **P26:** no generic “steps” substitution.
- **C1 — Required/optional version changes.** Oracle: supported readers, unknown required variants and missing optional fields. **P26 blocker:** tested directional compatibility.
- **C2 — Numeric preservation.** Oracle: exact E7/counter/objective source values. **P26 blocker:** no semantic rounding/truncation.
- **C3 — Authorized linked exposure.** Oracle: credentials with restricted capabilities and linked artifacts. **P26:** enforce existing scope; do not claim absent scoped services.
- **C4 — Resource/admission limits.** Oracle: declared producer workloads and measured memory/lock/payload behavior. **P26 blocker for implemented producer:** bounded or explicit overload.
- **C5 — Noninterference.** Oracle: identical inputs before/after exposure changes, comparing domain outputs/state. **P26:** no changed routing, objectives or publication semantics.
- **C6 — Snapshot capture/chunks.** Oracle: mutations concurrent with retained capture and incomplete transfers. **Conditional:** one coherent boundary; no partial install.

### Later implementation and external validation

- **F1 — Causal delays and overruns.** Oracle: recorded schedules/model anchors. **Future:** no backward jump, future leakage or timer-authorized completion.
- **F2 — Seeking/frame-rate equivalence.** Oracle: same scoped semantic selection under direct seek and sequential playback. **Future:** identical semantic progress.
- **F3 — Frozen work/context changes.** Oracle: accepted action/plan/context histories; existing domain tests support identity. **Future client:** no restart or old-ID/new-graph resolution.
- **F4 — 2D/3D and presets.** Oracle: same frame/preset realized in both modes. **Future:** identity/progress continuity and declared fallback.
- **F5 — Alignment and stacked roads.** Oracle: known canonical-to-scene correspondences and ambiguous parallel/overpass fixtures. **Future/External:** no unsupported nearest-road substitution.
- **F6 — Terrain/elevation.** Oracle: qualified road-surface/terrain data. **External:** terrain is not automatically road-deck height.
- **F7 — Detailed capture.** Oracle: off/on/full/truncated capture equivalence and retained-operation reconciliation. **Future:** capture cannot alter computation.
- **F8 — Snapshot/cursor continuity.** Oracle: commit/publication races, duplicates, gaps, omissions and disconnects. **Future:** certified cut and truthful recovery.
- **F9 — Scoped views/revocation/export.** Oracle: server-side scope enforcement and controlled-cache lifecycle. **Future:** no linked-data disclosure; no promise to erase external copies.
- **F10 — Physical telemetry.** Oracle: qualified measurement source and accuracy methodology. **External:** no physical position/speed/heading claim from simulation.
- **F11 — Visual fidelity/provider rights.** Oracle: selected licenses, coverage evidence and actual rendered scenes/workloads. **Future/External:** no fidelity PASS from schema completeness.

## 26. Risk, blocker and remediation register

All implementation remains subject to separate authorization.

**R1 — Zero-distance construction discrepancy.**
Evidence E6; reproduction **not performed**. Affects ordinary-segment validity and artifact round-trip. Supported ingestion reachability is unestablished. Required: minimal reproducer, invariant analysis and no-travel separation. Block only affected approved producer paths until resolved or contained. Foundational remediation needs explicit scope.

**R2 — Reusable-export graph guard.**
Evidence E5/E16; missing independent guard is established, wrong-current-API output is not demonstrated. Current callers use the matching graph. Required: explicit digest boundary and overlapping-ID adversarial test. Mandatory for reusable/detached Phase 26 export; storage integration is included only where approved.

**R3 — Flattening loses occurrence/vertex association.**
Evidence E5. Current GeoJSON is usable display geometry, not exact canonical reconstruction evidence. Required: preserve canonical association/boundaries and declared normalization. Mandatory producer work; no change to canonical distance.

**R4 — Route/alternative provenance asymmetry.**
Evidence E5/E9. Applicable objective/traffic context is missing from the alternative envelope. Required: parity test against actual captured inputs. Mandatory Phase 26 conformance requirement.

**R5 — Exact-summary verification gaps.**
Evidence E7/E8/E9/E11/E12. Current tests do not establish every exported counter convention. Required: independent exact fixtures, including trivial routes, stale pops and work scopes. Mandatory Phase 26 verification.

**R6 — Snapshot lock/payload growth.**
Evidence E15/E19. Full JSON construction under coordinator lock is established; quantified impact is unmeasured. Required if modified/included: coherent capture and resource/admission measurements. Otherwise documented future fleet-view risk, not blanket redesign authorization.

**R7 — Historical/mixed-graph resolution.**
Evidence E16. Durable adapter is single-graph; serializers are not general digest-qualified resolvers. Required for reference-only claims: real resolver/retention/authorization. Self-contained checked exports support a narrower contract. General service is conditional/deferred.

**R8 — Snapshot/stream gap and projection coverage.**
Evidence E15/E17. No certified correspondence exists; current projection is narrower than `/v1/state`. Blocks incremental claims only. Future producer requires atomic cut, reducer/version/scope binding and race tests.

**R9 — Historical movement evidence sufficiency.**
Evidence E13/E14/E20. Latest state alone may lack schedule/transition history. Blocks exact seeking claims when dependencies are absent. Future playback must return narrow unavailability rather than rerun modern decisions.

**R10 — Guidance/scene/telemetry gaps.**
Evidence E18 plus reference-only assets. These are legitimate deferred capabilities, not Phase 26 defects. No lane, elevation, physical speed or immersive readiness claim.

**R11 — Privacy and lifecycle limits.**
Evidence E19/E16. Trusted-local capabilities are not production identity; scoped projections/public export policies are absent; durable retention has dependencies. Required: existing authorization on approved exposure; new services/lifecycle changes separately approved.

**R12 — Stale documentation.**
[architecture.md:332](/Users/admin/Developer/roadrunner/docs/architecture.md:332) says no durable state while [its later storage extension](/Users/admin/Developer/roadrunner/docs/architecture.md:504) documents persistence. Treat as stale prose. Amend later; do not reinterpret implemented persistence as nonexistent.

## 27. Complete accepted decision register — Q1–Q49

All entries are **accepted with the user’s refinements**. Detailed refinements are binding through the corresponding audit sections; recommendations are not substitutes for those refinements.

Each entry records the problem/evidence, decision/invariant, rationale/deferred alternative and implication/owner.

### Round 1

**Q1 — Coverage.**
Problem/evidence: route-only scope would ignore existing execution evidence; E1/E13/E14. Decision/invariant: mandatory route/cost/identity/summary producers, compatible existing-evidence contracts, explicitly deferred advanced capabilities. Rationale: durable shared semantics without phase expansion; route-only and implement-everything alternatives rejected. Implication/owner: finite Phase 26 package; later clients/services retain ownership.

**Q2 — Evidence classification.**
Problem/evidence: one scene mixes facts, predictions and interpolation; E13/E14. Decision/invariant: claim-level semantics with shared envelopes and overrides, explicit clocks and unavailable states. Rationale: a “live” screen label is insufficient; duplication is avoidable. Implication/owner: Phase 26 evidence contracts; clients cannot upgrade source quality.

**Q3 — Shared journey.**
Problem/evidence: 2D coordinates do not supply a 3D scene; E3/E4. Decision/invariant: one journey/progress model feeds independent scenes; transforms and elevation remain derived. Rationale: independent pipelines risk divergent facts; engine-owned cities are unnecessary. Implication/owner: renderer-neutral Phase 26 foundation, later scene systems.

**Q4 — Completion gates.**
Problem/evidence: schemas and geometry cannot verify fidelity; E1. Decision/invariant: architecture, producer conformance, client readiness and visual fidelity are separate. Rationale: neither a document-only PASS nor a mandatory renderer is appropriate. Implication/owner: explicit user gates and phase-specific evidence.

### Round 2

**Q5 — Canonical representation.**
Problem/evidence: flattened GeoJSON loses boundaries/duplicates; E3/E5. Decision/invariant: canonical semantic route with exact occurrence/geometry association; GeoJSON is derivative. Rationale: convenient encoding must not become sole authority; reference-only is insufficient universally. Implication/owner: Phase 26 checked exposure.

**Q6 — Detached availability.**
Problem/evidence: digest identity does not guarantee retrieval; E2/E16. Decision/invariant: embedded and referenced forms, self-contained portable default, typed missing dependencies. Rationale: avoid imagined resolvers and current-graph substitution. Implication/owner: Phase 26 supported exports; general archival resolution conditional.

**Q7 — Spatial validity.**
Problem/evidence: no-travel differs from malformed segments; E5/E6. Decision/invariant: strict validation with precise degeneracy/unavailability classes; reproduce zero-distance suspicion first. Rationale: reject silent repairs without excluding legitimate repeated points. Implication/owner: producer guards/tests; graph-semantic remediation separately scoped.

**Q8 — Derived geometry.**
Problem/evidence: simplification changes measured line length; E3/E6. Decision/invariant: parent identity, method/version/parameters, coordinate/error semantics and occurrence mapping; no canonical metric/progress mutation. Rationale: no immediate simplification service required. Implication/owner: Phase 26 rules; initial simplification renderer-side.

**Q9 — Geographic semantics.**
Problem/evidence: E7, floating rounding, Haversine and seam limits differ; E4. Decision/invariant: preserve exact geographic meaning and explicit transform/elevation limitations. Rationale: global readiness cannot follow from coordinate range. Implication/owner: Phase 26 encodings; later geographic rendering proof.

### Round 3

**Q10 — Smooth movement.**
Problem/evidence: endpoints/schedules lack continuous trajectories; E13/E14. Decision/invariant: versioned causal illustrative progress, pinned route, no retroactive rebasing. Rationale: smooth simulation need not wait for telemetry; physical interpretation rejected. Implication/owner: compatibility now, interpolation producer deferred.

**Q11 — Stops and stages.**
Problem/evidence: predicted arrival is not accepted arrival; E13. Decision/invariant: stages/readiness/custody/completion remain evidence-driven; overruns and no-travel explicit. Rationale: timer-driven completion is invalid. Implication/owner: existing evidence exposure; later presentation policies.

**Q12 — Orientation.**
Problem/evidence: geometry supplies direction, not measured heading; E3/E13. Decision/invariant: shared route-derived bearing with degenerate policies; rig animation stays derived. Rationale: independent orientation risks cross-view inconsistency. Implication/owner: compatibility contract and later orientation/animation producers.

**Q13 — Historical interpretation.**
Problem/evidence: occurrence and acceptance differ; E14/E20. Decision/invariant: acceptance-causal default plus separately bounded retrospective inspection; deterministic semantic frames. Rationale: occurrence-only replay invents contemporaneous knowledge. Implication/owner: contracts now, seeking infrastructure deferred.

**Q14 — Fleet tracks.**
Problem/evidence: fixture IDs repeat across runs; E14. Decision/invariant: shared playback selection, individually scoped tracks, frame-rate independence and stable appearance. Rationale: independent marker clocks and unqualified IDs drift/collide. Implication/owner: identity compatibility; future fleet client.

### Round 4

**Q15 — Guidance layers.**
Problem/evidence: bends/restrictions are not instructions; E18. Decision/invariant: geometry, semantic maneuvers and localized instructions remain distinct; freeze evidence boundary. Rationale: immediate instruction generation expands scope. Implication/owner: Phase 26 compatibility; guidance producer deferred.

**Q16 — Enrichment.**
Problem/evidence: names/lane/sign details are absent or unsupported; E18. Decision/invariant: immutable graph-qualified future enrichment with original/retrospective/current-source distinctions. Rationale: basemap labels cannot silently become historical facts. Implication/owner: later ingestion/enrichment; no current graph expansion.

**Q17 — Motorcycle legality.**
Problem/evidence: supported motorcycle policy exists but is incomplete; E18. Decision/invariant: guidance consumes pinned route/profile/context; no car-SDK replacement or cosmetic privileges. Rationale: avoid a second legality engine. Implication/owner: Phase 26 provenance, future guidance client.

**Q18 — Maneuver anchors.**
Problem/evidence: repeated edges/junctions make coordinate anchors ambiguous; E3/E7. Decision/invariant: route-qualified occurrence/transition anchors and canonical distance; route replacement invalidates incompatible guidance. Rationale: schedule/suffix changes must not duplicate maneuvers. Implication/owner: compatibility now, guidance generation later.

**Q19 — Speed claims.**
Problem/evidence: effective speed and optional legal limit differ; E18. Decision/invariant: legal, routing, animation and measured speeds retain units/source/applicability. Rationale: fallback speed cannot become a legal sign. Implication/owner: existing evidence exposure; telemetry/current-limit feeds deferred.

### Round 5

**Q20 — Subject identity.**
Problem/evidence: route, candidate, proposal and execution differ; E7/E10–E13. Decision/invariant: typed subjects and scoped evaluation/authority stages; rank is not identity. Rationale: generic alternatives erase meaning. Implication/owner: Phase 26 identity/association contracts.

**Q21 — Explanation depth.**
Problem/evidence: basic dispatch detail exceeds newer aggregate histories; E10–E12. Decision/invariant: summaries by default, bounded optional capture, explicit retention granularity; hypotheticals stay new evaluations. Rationale: aggregate counts cannot reconstruct candidates or localize fleet causality. Implication/owner: existing evidence now, diagnostic capture later.

**Q22 — Metrics/objectives.**
Problem/evidence: algorithm objectives and protection horizons differ; E10–E12. Decision/invariant: typed values and named comparisons, immutable acceptance terms, no fabricated breakdowns. Rationale: a scalar score loses priorities and physical meaning. Implication/owner: Phase 26 exposure, no policy change.

**Q23 — Outcomes.**
Problem/evidence: rejection, unknown inputs, nonselection and stale publication differ; E10–E12. Decision/invariant: structured evidence-linked outcomes; text cannot exceed causal evidence. Rationale: generic failure and speculative explanations mislead. Implication/owner: Phase 26 mappings, richer causality separately produced.

**Q24 — Quality dimensions.**
Problem/evidence: coverage, local search and publication differ; E8–E12. Decision/invariant: independent input/coverage/termination/retention/quality/publishability; preserve incumbent suppression. Rationale: local optimum and requested count are not global/exhaustive proof. Implication/owner: Phase 26 summaries; domain changes separately approved.

### Round 6

**Q25 — Search vocabulary.**
Problem/evidence: nodes, incoming-edge states and queue entries differ; E8. Decision/invariant: precise operations, strict relaxation and legacy counter conventions; no universal settlement. Rationale: “visited nodes” is inaccurate. Implication/owner: Phase 26 summary conformance, later traces.

**Q26 — Trace boundary.**
Problem/evidence: result counters contain no operation history; E7/E8. Decision/invariant: summaries mandatory; future ordered queue/label traces with explicit completeness. Rationale: final routes cannot generate historical frontier fiction. Implication/owner: compatibility now; detailed instrumentation/UI deferred.

**Q27 — Actual values.**
Problem/evidence: heuristics are conditionally invoked and `Forbidden` is generic; E8. Decision/invariant: actual numeric values/calls and rejection stages only. Rationale: recomputation and fabricated zeros misstate history. Implication/owner: summary evidence now; richer evaluator reasons separately scoped.

**Q28 — Diagnostic families.**
Problem/evidence: algorithm work units differ; E8–E12. Decision/invariant: shared envelopes, separate payload families and nested scopes. Rationale: one universal step corrupts comparisons. Implication/owner: Phase 26 summaries/compatibility; future family-specific traces.

**Q29 — Bounded capture.**
Problem/evidence: capture limits must not become search budgets; E8–E12. Decision/invariant: mechanically declared capture states, independent terminal summaries, deterministic scoped order and noninterference. Rationale: unbounded or search-stopping capture is unacceptable. Implication/owner: requirements now; measured offline capture later.

### Round 7

**Q30 — Initial synchronization.**
Problem/evidence: independent mutable queries can mix revisions; E15. Decision/invariant: coherent whole-snapshot replacement, scoped metadata and authorization. Rationale: incremental infrastructure is unnecessary initially. Implication/owner: existing capability; new view adapters conditional.

**Q31 — Certified cut.**
Problem/evidence: outbox publication lags commits; E15/E17. Decision/invariant: future atomic projection snapshot/cursor with view/scope/lag/omission binding. Rationale: separately sampled positions are unsafe. Implication/owner: compatibility now, incremental producer deferred; authority-cut alternative retained.

**Q32 — Recognition/order.**
Problem/evidence: multiple messages can carry one effect; E17. Decision/invariant: source-local order, distinct message/effect recognition, commit atomicity and visible gaps/conflicts/omissions. Rationale: timestamp last-write-wins is invalid. Implication/owner: future reducer contract, no universal sequence.

**Q33 — Layered context.**
Problem/evidence: active work survives suffix/context changes; E13/E16. Decision/invariant: current context, editable plan, frozen route and generation stay distinct. Rationale: domain provenance is not mixed-graph adapter readiness. Implication/owner: Phase 26 references; resolver/hot replacement separately scoped.

**Q34 — Recovery.**
Problem/evidence: no browser continuity protocol exists; E15/E17/E20. Decision/invariant: validated resume or explicit resnapshot/unavailability; freshness and authorization preserved. Rationale: current recovery cannot recreate missed history. Implication/owner: compatibility now, services/seek later.

### Round 8

**Q35 — Scene boundary.**
Problem/evidence: domain geometry is not a scene; E1/E3/E18. Decision/invariant: domain evidence, shared derivations and renderer composition separate. Rationale: neither giant scene responses nor incompatible renderer interpretations are needed. Implication/owner: renderer-neutral Phase 26; React/MapLibre direction preserved.

**Q36 — Alignment.**
Problem/evidence: basemap roads may differ from canonical geometry; E3/E4. Decision/invariant: canonical overlay default; versioned correspondence with ambiguity and progress mapping. Rationale: nearest-road snapping can select another road. Implication/owner: compatibility now, alignment producer later.

**Q37 — Vertical placement.**
Problem/evidence: no graph altitude/road deck evidence; E3/E4/E18. Decision/invariant: flat, terrain and grade-separated capabilities distinct; no invented physical elevation. Rationale: perspective need not wait for full 3D, but terrain cannot stand in for bridges. Implication/owner: later scene producer/client.

**Q38 — Visual identity.**
Problem/evidence: reference images are not model assets; rider profile is domain-only. Decision/invariant: shared versioned presets, compatible realizations, stable selection/fallback and separate reproduction guarantees. Rationale: hard-coded or independent assets break continuity. Implication/owner: future renderer-owned assets, no capability mutation.

**Q39 — Provider readiness.**
Problem/evidence: rendering features do not supply datasets or rights; E1 plus cited provider sources. Decision/invariant: per-area/version/operation readiness and separate domain/scene reproduction. Rationale: one “3D” flag conceals gaps. Implication/owner: selection criteria now, integration/visual proof later.

### Round 9

**Q40 — Payloads.**
Problem/evidence: full responses duplicate immutable material; E15/E16. Decision/invariant: mutable/artifact/presentation categories, real resolvers, coherent chunks and scoped cache keys. Rationale: always-everything and live mixed-page designs fail efficiency/consistency. Implication/owner: approved bounded Phase 26 representations.

**Q41 — Resources.**
Problem/evidence: lock-held JSON and request-only limit; E19. Decision/invariant: measured workload/admission gates, coherent capture and independent cadences. Rationale: worker counts are not scale proof. Implication/owner: producer measurements in Phase 26; renderer performance later.

**Q42 — Access.**
Problem/evidence: trusted-local and allowlists are limited; E19. Decision/invariant: purpose-specific server-side disclosure, linked minimization, reviewed demos and scope-aware caches. Rationale: filtering and hashes are not authorization. Implication/owner: approved exposure now; new identity/projection/export systems separately scoped.

**Q43 — Retention.**
Problem/evidence: current durable retention has dependencies; E16/E17. Decision/invariant: purpose/dependency classes, no silent weakening, no universal indefinite policy, traceable redaction. Rationale: neither blanket expiry nor retain-everything is sufficient. Implication/owner: requirements now, lifecycle changes separately approved.

**Q44 — Evolution.**
Problem/evidence: independent historical versions and strict parsing; E20. Decision/invariant: explicit directional compatibility, exact numbers, safe unknown-required handling and traceable derivatives. Rationale: additive changes are not universally safe. Implication/owner: Phase 26 versions/tests; future migrations independently verified.

### Round 10

**Q45 — Finite package.**
Problem/evidence: broad architecture can overrun E1 ownership. Decision/invariant: mandatory geometry/identity/provenance/metrics/evidence/summaries/schemas; adapters conditional; advanced producers deferred. Rationale: route-only is insufficient, implement-everything excessive. Implication/owner: exact later-authorized work package.

**Q46 — Findings.**
Problem/evidence: E5/E6/E15–E19 show mixed risk classes. Decision/invariant: classify evidence, reproduction, reachability and conformance impact; guard/contain approved paths. Rationale: suspicion is not verified failure and deferral is not defect. Implication/owner: bounded remediation with separate foundational authorization.

**Q47 — Proof.**
Problem/evidence: adjacent tests do not prove the new contract. Decision/invariant: claim-linked independent oracles, fixtures, actual execution status and phase owners. Rationale: neither one generic checklist nor field-only tests suffice. Implication/owner: mandatory producer tests; future/external proofs retained.

**Q48 — Verdicts.**
Problem/evidence: foundations and future capabilities differ. Decision/invariant: separate architecture, foundation, implementation, conformance and capability states. Rationale: one PASS or maturity percentage misleads. Implication/owner: explicit audit and completion verdicts.

**Q49 — Sign-off.**
Problem/evidence: accepted design is not coding permission. Decision/invariant: self-contained audit, finite workstreams, named prerequisites and separate architecture/implementation approvals. Rationale: broad unspecified approval risks scope drift. Implication/owner: this read-only audit closes the grill; user controls both subsequent gates.

## 28. Required specification and ADR amendments

These are recommendations for later authorization, not edits performed.

1. **Phase 26 specification:** finite mandatory/conditional/deferred package; four gates; producer acceptance criteria.
2. **Geometry/identity ADR:** semantic route versus encoding, occurrence identity, exact association, checked export preconditions, portable/reference availability.
3. **Evidence/observability ADR:** authority taxonomy, clocks, typed metrics, comparison scopes, unavailable outcomes and explanation strength.
4. **Diagnostic contract amendment:** exact counters, state modes, algorithm families, deferred traces and capture noninterference.
5. **Execution/presentation compatibility:** causal illustrative progress, stage separation, run scoping, orientation and replay interpretations. No new execution authority.
6. **Snapshot compatibility:** whole-snapshot baseline; certified incremental cut requirements and current projection limitations.
7. **Renderer responsibility specification:** shared semantic frame, scooter references, presets, alignment, elevation and capability degradation.
8. **Compatibility/access/resource specification:** version directions, artifact authorization, dependency/retention classes and measured producer gates.
9. **Stale architecture prose:** reconcile early no-persistence statements with implemented Phase 21/22 extensions.
10. **Roadmap clarification:** preserve Phase 27+ ownership; recommend explicit placement of immersive 3D/assets/enrichment where the existing roadmap lacks a precise phase owner.

Existing ADRs should be amended or supplemented coherently, not duplicated with contradictory authority. Exact file organization is an implementation-time engineering choice.

## 29. Readiness verdicts and completion gates

Vocabulary follows Q48. For the new design, **architecture decisions are accepted; final audit confirmation is pending**.

1. **Route Visualization Data Contract:** foundation evidenced; new producer work outstanding; conformance unverified; existing route display limited.
2. **Directed Geometry Conformance:** orientation foundation evidenced; exact occurrences/guards outstanding; conformance unverified.
3. **Route/Evaluation Identity:** graph identity evidenced; distinct exported identity package outstanding; conformance unverified.
4. **Cost/Objective Observability:** underlying types/comparators evidenced; unified truthful exposure outstanding; limited support.
5. **Decision/Selection Evidence:** retained evidence implemented at varying granularity; exposure/conformance outstanding; no complete candidate archive.
6. **Search Summary Diagnostics:** counters implemented; exact-summary conformance unverified; detailed trace deferred.
7. **Movement/Timeline Compatibility:** execution/trace foundations evidenced; accepted compatibility design; interpolation producer deferred.
8. **Navigation Guidance:** motorcycle routing evidenced; guidance metadata/producer not established; generation deferred.
9. **Whole-Snapshot Viewing:** coherent endpoint implemented; visualization resource/conformance verification unverified; namespace-wide limited capability.
10. **Incremental Streaming/Resume:** local event foundations evidenced; visualization cut/service outstanding and out of mandatory scope; deferred.
11. **Historical Inspection:** retained records implemented; access/resolution limited; not a general historical query/replay service.
12. **Movement Playback/Seeking:** endpoint/timeline evidence partly available; playback producer not established; deferred.
13. **2D Renderer Data Readiness:** basic directed line data evidenced; Phase 26 package outstanding; actual client absent.
14. **3D Scene Data Readiness:** geographic route foundation only; scene/elevation pipelines not established; deferred.
15. **Motorcycle Visual Assets:** primary design reference available; models/rigs/markers not implemented; deferred.
16. **Phase 26 Implementation Readiness:** design ready for confirmation; bounded work package proposed; implementation authorization and conformance evidence outstanding.

**Producer-conformance PASS requires:**

- Every included mandatory producer implemented.
- Correct checked geometry/identity and provenance.
- Exact existing metric/counter/quality semantics.
- Typed unavailable/unsupported outcomes.
- Compatibility and existing authorization enforced.
- Applicable adversarial tests freshly executed.
- Resource behavior measured and declared for implemented producers.
- No changes to routing/dispatch/publication authority.
- Suspected reachable defects resolved or explicitly contained.
- Completion evidence distinguishing verified, unavailable and deferred capabilities.

A renderer is not required for this PASS. A schema alone is insufficient.

## 30. Phase 26 — Proposed Implementation Work Package

All workstreams below are proposals pending architecture confirmation and separate implementation authorization.

### A. Existing-contract reconciliation — mandatory

**Modules:** AGENTS Phase 26, architecture/runtime/routing/API documentation, ADRs 0001–0007 and 0012/0016–0022 as applicable.
**Behavior:** reconcile terminology, identity and phase ownership; record existing counter and availability contracts.
**Dependencies:** confirmed audit.
**Acceptance/tests:** trace each mandatory deliverable to producer, schema, invariant and proof case; resolve stale documentation without policy changes.
**Interface impact:** documentation/specification only.
**Risk:** accidentally promoting illustrative types or future capabilities into present support.
**Roadmap fit:** direct Phase 26 specification work.

### B. Graph-qualified geometry and identity — mandatory

**Modules:** core graph/routing result, API geometry adapter; storage geometry only if included.
**Behavior:** reusable checked export boundary, exact occurrence/geometry association, orientation, no-travel and typed failures; scoped identity rules.
**Dependencies:** A; R1 reproduction/containment where reachable.
**Acceptance/tests:** G1–G11, independent point sequences, matching graph digest and repeated occurrences.
**Interface impact:** new/versioned representation or compatible adapter; existing wire behavior must be explicitly preserved or versioned.
**Risk:** duplicate normalization, wrong-graph local IDs, overbroad graph-semantic changes.
**Roadmap fit:** central Phase 26 work.

### C. Provenance and typed metrics — mandatory

**Modules:** API `routing.rs`/`model.rs`, core route/cost contracts, applicable CLI/export adapters.
**Behavior:** consistent single/alternative context, typed objectives/units/horizons and availability; no new cost model.
**Dependencies:** B.
**Acceptance/tests:** D1–D6, exact captured context and source metrics; no fabricated breakdown.
**Interface impact:** explicit schema compatibility; original historical outputs remain historical.
**Risk:** deriving unavailable context from current state.
**Roadmap fit:** direct Phase 26 work.

### D. Existing decision/selection evidence — mandatory exposure, bounded adapters

**Modules:** dispatch decision/pooling/fleet/recovery/history; API runtime exposure.
**Behavior:** preserve typed subject, evaluation/publication stage, coverage, aggregate/detail status, comparison contract and accepted terms.
**Dependencies:** A/C.
**Acceptance/tests:** D7–D13; current incomplete searches produce no publishable proposal; no invented candidate geometry.
**Interface impact:** versioned inspection/exposure, not new candidate retention or planning endpoints unless explicitly included.
**Risk:** assuming legacy basic-dispatch detail exists in current operational optimizer evidence.
**Roadmap fit:** approved observability extension; full dashboards remain later.

### E. Search-summary conformance — mandatory

**Modules:** core Dijkstra/A*/alternatives result exposure; dispatch work summaries.
**Behavior:** named state mode, exact counter/termination scope, actual heuristic/evaluator metadata where supported.
**Dependencies:** A/C.
**Acceptance/tests:** S1–S6, including trivial routes, stale entries, strict relaxation and incoming-edge states.
**Interface impact:** semantic metadata; no detailed event producer.
**Risk:** altering established counters to match UI vocabulary.
**Roadmap fit:** explicit diagnostics requirement.

### F. Versioned API/export representations — mandatory core; adapters conditional

**Modules:** approved API/CLI/export boundary.
**Behavior:** portable route representation, explicit optional capabilities, exact encodings and compatibility outcomes.
**Dependencies:** B–E.
**Acceptance/tests:** C1/C2, reference availability and supported reader behavior.
**Interface impact:** chosen versioning strategy documented and tested; no universal giant scene response.
**Conditional additions:** execution wrappers, timeline exports, specialized snapshots, resolver endpoints and historical adapters only if the implementation brief includes them.
**Roadmap fit:** direct data-contract work; no new operational authority or persistence rebuild.

### G. Adversarial correctness/compatibility — mandatory

**Modules:** relevant core/API/dispatch/storage tests and fixtures.
**Behavior:** implement the applicable P26 proof matrix with independent oracles.
**Dependencies:** parallel with B–F, completed after final changes.
**Acceptance:** all required checks freshly pass; opt-in database checks run when touched paths require them, otherwise identified as unexecuted rather than silently credited.
**Interface impact:** none intended.
**Risk:** testing the same faulty helper against itself or changing historical goldens without explanation.
**Roadmap fit:** mandatory engineering verification.

### H. Authorization, resources and payloads — mandatory for included producers

**Modules:** API credentials/admission, exporters, snapshot capture only if included.
**Behavior:** enforce existing permissions; bounded output/admission; immutable reuse; explicit overload and incompleteness.
**Dependencies:** F.
**Acceptance/tests:** C3–C6; representative hardware/dataset/configuration/run evidence, lock/allocation/payload and authority-latency measurements. No invented thresholds.
**Interface impact:** explicit limit/error behavior; no scoped fleet service by implication.
**Risk:** treating read-only work or request limits as response isolation.
**Roadmap fit:** producer conformance; rendering performance excluded.

### I. Documentation, ADRs and completion evidence — mandatory

**Modules:** approved ADR/spec/API/benchmark/completion documentation.
**Behavior:** record final implemented contracts, exact scope, compatibility, tests, measurements, risks and deferred owners.
**Dependencies:** all included workstreams.
**Acceptance:** Gate B evidence reconciles actual implementation with the confirmed audit; no unsupported visual/streaming claims.
**Interface impact:** authoritative implementation documentation.
**Risk:** calling future capability requirements implemented.
**Roadmap fit:** required completion evidence.

**Recommended sequence:** A → R1 reproduction as applicable → B → C → D/E → F → G/H completion → I. Tests develop alongside producers; final conformance follows the completed package.

## 31. Architecture confirmation and implementation boundary

### Architecture readiness verdict

**Ready for architecture confirmation.** The 49 accepted decisions form a consistent bounded design. Existing foundations are substantial; producer conformance remains unverified.

### Phase 26 entry prerequisites

- Explicit confirmation of this audit.
- Separate implementation authorization.
- An implementation brief naming mandatory and conditional producers.
- Baseline validation and affected-interface inventory.
- Focused reproduction/reachability assessment for R1.
- Defined identity generation/canonicalization and versioning rules before their producers are finalized.
- Declared resource workloads and acceptance evidence for included exporters/snapshots.

### Finite mandatory implementation checklist

- Checked directed geometry and ordered occurrences.
- Scoped route/evaluation/selection identities.
- Single/alternative provenance parity.
- Typed existing metrics/objectives.
- Truthful existing decision evidence.
- Exact summary diagnostics.
- Versioned representations and explicit unavailable outcomes.
- Applicable correctness, compatibility, authorization and resource tests.
- Documentation/ADR/completion evidence.

### Conditional or separately authorized items

Execution/timeline wrappers, specialized snapshots, historical adapters, resolver endpoints, snapshot resource redesign and any foundational graph-semantic remediation must be explicitly included or separately authorized.

### Deferred capabilities

Rendering UI, scooter models/rigs/customization, scene providers, terrain/buildings, continuous interpolation producers, physical telemetry, guidance generation, detailed search instrumentation, live subscriptions, restricted fleet services and general historical seeking.

### Producer-conformance acceptance

Gate B requires freshly verified implementations of the approved package and the applicable proof matrix. It cannot be satisfied by this audit, adjacent completion reports or unexecuted tests.

### Recommended implementation sequence

Reconcile contracts; establish checked geometry/identity; expose provenance/metrics; adapt existing evidence and summaries; finalize versioned representations; verify correctness/access/resources; publish accurate completion evidence.

### Explicit sign-off request

**Do you confirm this audit as the Phase 26 architecture and proposed implementation-planning reference?**

Confirmation approves the architecture only. Repository edits, ADR changes, commits and Phase 26 implementation still require your separate authorization.
