# Phase 26 producer contract

Status: implemented producer package; see [fresh completion evidence](phase-26-completion.md).
The approved [audit](phase-26-source/audit.md), [final refinements](phase-26-source/final-refinements.md),
[cinematic addendum](phase-26-source/cinematic-addendum.md), and [authorization](phase-26-source/authorization.md)
are retained in full with trailing whitespace normalized. Their historical pending/read-only labels describe the source conversation;
the later authorization approves Gates A and B. [ADR 0023](adr/0023-checked-visualization-evidence.md)
records implementation choices. No new architecture approval is required.

## Finite producers and ownership

Core `routing::checked_route_geometry` resolves a detached path against its original graph.
Core `routing::route_evidence` exports an existing `RouteResult` with the **original caller-supplied**
`EvaluationProvenance`. The latter is a producer attestation, not something a context-free RouteResult
can recover or verify. The API supplies it from the actual evaluator/context used in that call.
Both functions are renderer-neutral Rust library exports, available to embedded and CLI consumers.

`POST /v1/visualization/routes` and `POST /v1/visualization/routes/alternatives` accept the same strict
request as `/v1/routes`. They expose portable, embedded evidence version 1. They neither mutate
operational state nor create durable route records. Existing persistent `/v1/routes` retains its
recording behavior. This is a separate inspection representation, not a migration of stored records.

Basic dispatch `AssignmentDecision::inspection()` qualifies its existing per-candidate evidence.
Insertion, fleet, and recovery `inspection()` qualify their existing aggregate evidence. Their HTTP
command outcomes carry an additive `decision.inspection` or `noncommit.inspection` object alongside
the original evidence. No new candidate archive, command, snapshot or authority was introduced.

| Mandatory deliverable | Producer / representation | Verification / dependency |
| --- | --- | --- |
| Checked geometry | core geometry export; E7 embedded path and occurrences | geometry tests; pinned full graph digest |
| Scoped identities | spatial path, route evaluation, returned alternative member | independent SHA oracle; original evaluator/context |
| Provenance / metrics | route evidence + matching single/alternative provenance | HTTP traffic/time/objective parity tests |
| Existing decisions | inspection metadata + unchanged original evidence/publication | entry gate, pooling, fleet, recovery, HTTP tests |
| Search summaries | route search family and existing optimizer work | hand-traced counters and existing independent optimizer oracles |
| Version / access / resources | semantics preflight, schema, authenticated endpoints, admission | schema mutations, auth/admission tests, retained measurements |

Additional frozen-execution wrappers, timeline exports, specialized snapshots, resolvers and expanded
historical adapters are **not necessary** for this finite portable route + existing-decision package
and were not included. Current state/history/execution contracts remain compatibility references.

## Geometry and identity

Full semantic graph digest is checked **before any dense-ID resolution**, even when graph IDs overlap.
The geometry checker verifies the node/edge cardinality, referenced graph entities, directed endpoints,
continuity, exact segment endpoints, segment length and output admission limits. It does not perform
new routing, infer intersections, supply snapping, infer cost feasibility, or authorize a maneuver.
A geometry-only detached path is not itself proof of a successful route evaluation.

`points_e7` preserves every canonical integer vertex. Each occurrence's `[point_start, point_end)`
includes both directed endpoints; shared joints and repeated adjacent points are deliberately duplicated.
Occurrences preserve edge, segment, direction and ordinal. Repeated traversals have different ordinals.
The node list is retained, including the single no-travel anchor. No-travel has one point, no occurrences,
and no road traversal. Ordinary zero-length segments are invalid for checked exposure.

Coordinates are WGS84 latitude/longitude E7, without altitude. Canonical distance is accumulated
spherical Haversine polyline meters using radius 6,371,008.8 m. E7 precision is not source accuracy.
GeoJSON uses longitude then latitude; legacy GeoJSON removes consecutive duplicate positions and
uses two identical points for no-travel. It is a display derivative, not canonical multiplicity evidence.
No seam-crossing box, polar-rendering, elevation or scene-alignment guarantee is established.

`GeometryError` distinguishes unavailable original graph, graph mismatch, invalid geometry,
unsupported semantics, and export limits. No fallback to the current graph, connectors, gap repair,
or implied resolver exists. Existing HTTP node-projection/snapping rejection remains `invalid_input`;
checked-export failures use `geometry_*` codes. Unsupported semantics is also the reader's version failure.

Identities have separate scopes:

* `path-v1:<sha256>` hashes UTF-8 `roadrunner/spatial-path/v1\0`, full graph digest ASCII,
  LE64 node count, ordered LE32 node IDs, LE64 edge count, ordered LE32 directed edge IDs.
  The NUL above is one zero byte. E7 shape and profile are pinned transitively by full graph identity.
* `evaluation-v1:<sha256>` hashes compact serde JSON of the tuple
  `(1, path_id, provenance, algorithm, tagged_objective, elapsed_seconds)`.
  Provenance field order is the declared Rust struct order: evaluator, traffic_digest,
  departure_seconds, routing_context, time_domain, heuristic. `routing_context` includes
  departure_time, private_access and permit_access. Integer IDs, option nulls and float-roundtrip
  encoding are frozen for v1; encoding changes require a new identity version.
  This is a content identity, not a unique invocation/request ID. Different algorithms or applicable
  contexts may identify different evaluations of the same spatial path. Caller policy names/configuration
  must actually identify their evaluator; the exporter cannot guess custom evaluator configuration.
* `alternatives-v1:<sha256>` hashes compact JSON of
  `(1, provenance, options, ordered_evaluation_ids, termination, search_states)`.
  `member_id` joins that set identity and member evaluation ID. Rank is only returned selection position.
* Operational decision, namespace/revision, proposal, publication, plan/action and schedule-generation
  identities retain their existing separate scopes. There is no universal route/assignment identity.

## Provenance, metrics and search

Single/alternative evidence includes actual pinned graph, profile/jurisdiction, evaluator policy,
full routing access context, applicable traffic digest, departure and clock domain, algorithm and
selected heuristic policy/configuration. Explicit RFC3339 input is Unix seconds; omitted departure
uses the API clock and its advertised domain. Static evaluators have null traffic context even if
an overlay is currently adopted. Legacy alternatives now also expose objective and traffic digest.

Distance meters, predicted elapsed **road** seconds and tagged objective are distinct. Objectives are
`distance`/meters or `travel_time`/seconds. No waiting, service, preference, penalty or realized travel
is relabelled as road duration. Unsupported base/traffic/turn breakdowns are explicitly unavailable.
No cost model, recomputation, display simplification or learned operational ETA was added.

Dijkstra/A* `expanded_states` means accepted non-stale pops **including the goal**. For source==destination,
1 is a special convention without queue operations or heuristic calls. State mode is node or
node/incoming-edge plus source sentinel. Multiple incoming states at one graph node count separately;
A* has no permanent closed set and the counter cannot be called unique nodes or settled nodes.
Strict objective improvement alone replaces a label. Neighbor heuristics are computed only after it.
No actual per-event g/h/f history is retained or reconstructed; detailed traces are deferred. Generic
Forbidden/NoRoute remains generic and cannot explain an unretained singular cause.

Yen has a separate `loopless_path` work family: per-route count belongs to its originating root/spur
search; envelope `search_states` sums budget-counted complete-path pops across searches, including
found goals. `ranked_paths` counts distinct complete paths ranked, not candidate history. Returned
routes are selected diverse legal loopless paths under a static nonnegative evaluator; this is not
exhaustive candidate retention or an unrestricted legal-walk optimum claim. Shared physical segment
distance divided by shorter-route distance is the diversity measure. RequestedCount, Exhausted,
CostLimit and Budget keep their exact meanings. A primary can be returned with Budget/truncated;
insertion/fleet/recovery incomplete searches still suppress every publishable incumbent.

## Decision inspection and authority

Inspection metadata accompanies, and never replaces, original evidence. Input sufficiency, population
coverage, termination/search completeness, retention, comparison/quality and proposal availability
remain independent. Insertion's signed incremental seconds/meters, fleet's admission-count-first
lexicographic objective and recovery's road-plus-churn comparator remain algorithm-specific.
Original contexts, immutable accepted terms, current/accepted protections, customer impacts and exact
termination/work fields remain in the original evidence. Improvements do not bank delay allowance.
Counts, exclusions, reason occurrences, starts and rounds cannot become interchangeable "steps."

Basic evidence distinguishes feasible and rejected candidates. A feasible nonselected rider stays
feasible. Basic result order is canonical RiderId order, not discovery time. Its Spatial policy has an
inclusive Haversine-meter radius and limit over Basic Dispatch eligible riders; its center requires
the originating order's pickup from the correct snapshot. A detached decision without that association
has **no standalone geographic center**. Exhaustive has no finite radius. Spatial policy coverage is
always potentially incomplete. Operational insertion/fleet/recovery is nonradial and does not inherit
Basic Dispatch's radius or per-candidate records.

Pure evaluation metadata says recommended proposal, never committed. The API command envelope's
publication result is the authority for commitment; `published` alone on arbitrary commands is not an
assignment claim. Rejected/stale publication cannot be upgraded by inspection metadata. Original
publication records and active frozen execution supply their own stages/identities. Recovery cooldown
skips neighborhood search (`search_complete=false`); its original termination remains visible.
Missing candidate leg geometry remains unavailable; stop order or aggregate timing does not supply
ordered road legs. Multi-stop cinematic reveal must consume supported ordered leg geometry later.

## Compatibility, authorization and budgets

The [route schema](schemas/route-evidence-v1.schema.json) describes the nested route evidence object,
not the HTTP wrapper. Required semantics version/encoding/authority and objective kind/unit must be
understood. Rust readers can deserialize `RouteEvidenceSemantics` then call `check()` before using
geometry, and decode tagged `RouteCost`; this is a header preflight, not a full detached-import validator.
Unknown required meanings reject; unknown optional fields and absent optional_capabilities may be
ignored. JSON integer counters/E7 are exact: clients must use integer-preserving parsing when their
native number range is insufficient. Display rounding never changes source comparisons.

Legacy route requests and route fields remain. New canonical representations have separate endpoints.
Legacy alternative objective/traffic fields and decision inspection are additive; strict response
readers must explicitly accept these extensions or keep their own versioned adapter. No old archived
record is rewritten, reinterpreted or backfilled with current context. Historical missing evidence
stays missing. Durable restoration, frozen work and event recognition were not redesigned.

New endpoints require existing bearer read capability before transmission. They expose graph routing
inputs/results, not other riders' or orders' data. Original namespace-wide/scoped restrictions and
principal-scoped command outcome lookup remain. Content digests do not grant retrieval rights.
Trusted-local authentication is not production tenant isolation; no public demo data approval or
rider-scoped fleet service is claimed.

Default core limits are 20,000 occurrences / 200,000 embedded vertices **per route**, checked before
geometry allocation. API alternatives remain at most five selected routes. HTTP exports have a
32 MiB serialized response cap, fail explicitly with 413, and never silently omit geometry. These are
admission choices, not scalability targets. A separate export semaphore (sized to configured workers)
bounds active export computation/response bodies. Saturation returns 503; no export queue is created.
Operational worker admission is separate; a slow transfer retains only its export slot. Serialization
runs on the blocking worker and the body transfers in bounded chunks. Dropping the body releases
admission. Traffic capture briefly locks the coordinator to clone the pinned context; geometry,
JSON encoding, transfer and cinematic replay do not hold that lock. Allocation failure is not claimed
recoverable. Measurements and workload limits are in the completion report.

Existing `/v1/state` remains full-world/history/publication JSON built under its lock, without output
budget/pagination or a certified stream cursor. This producer was not modified; its scaling risk
belongs to later fleet-view work. Phase22 source sequence, storage/operational revision and projection
position remain different, and that projection is narrower than the state snapshot.

## Cinematic and later-client obligations

A reveal uses the actual checked result; reveal progress is presentation-only and cannot move a rider,
advance simulation, imply current algorithm progress, publish a proposal, or rerun optimization.
A spatial pulse is geographic only with the supported original center/radius/units/eligibility/limit
association; otherwise it is a declared metaphor. Candidate display order is not recorded discovery.
Both 2D/3D clients share path/evaluation/selection and progress semantics. Renderer alignment,
orientation/lean, camera/easing/glow, speed controls, skip/replay and reduced motion remain renderer-owned.
The navigation and blue scooter references remain reference-only; cosmetics never change motorcycle
routing profile or capacity. `delivery_motorcycle_v2/ng_v2` is a supported subset, not comprehensive legality.

Phase27 owns map foundation/initial reveal; 28 alternatives; 29 graph/summary inspection (traces require
new producers); 30 courier movement/assets; 31 discovery/selection presentation; 32 supported history
playback. Guidance, GPS/map matching, scene elevation/terrain/buildings/provider rights, general seeking,
continuous interpolation and snapshot-plus-stream/scoped projections need later explicit producer scope.
Retained frozen routes must use original dependencies; the durable adapter is still single-graph.
No conditional historical resolver, timeline wrapper or new snapshot service was necessary here.
The cinematic addendum's future acceptance matrix remains binding and **unexecuted**.

## Phase 28 selected-path inspection extension

The alternative envelope optionally adds `segment_inspection` to each member.
Portable route evidence, its version/identities, result-set/member identity, search
and selection semantics are unchanged. The extension is versioned independently:

```text
schema_version: 1
semantics: pinned_static_path_evaluation/v1
evaluation_id, graph_digest, traffic_digest
base_seconds
traffic_delay_seconds: number | null
segments[]:
  ordinal, edge
  objective_cost                  # unit inherited from member.route.objective_unit
  predicted_elapsed_seconds
  base_seconds
  traffic_factor: number | null
```

The API evaluates only returned directed occurrences using the **same captured
static evaluator, immutable graph, routing/access context and traffic snapshot**
used by that query. It does not invoke another search, fetch current traffic after
selection, or retain a search-event history. Geometry export must succeed first;
its 20,000-occurrence bound also bounds this array. Existing worker admission,
200,000-point/route, five-route and 32 MiB response limits still apply. The route
and prediction totals are checked against the sums before export. Export failure
returns an error; it cannot silently substitute a new path or drop geometry.

`base_seconds` is the sum of the returned path's directed free-flow times. For a
traffic-aware query, `traffic_factor` comes from the pinned snapshot, including its
actual default of 1, and `traffic_delay_seconds` is the evaluated elapsed road time
minus that same path's base time. This is **not** the time difference against the
free-flow optimal route. With distance/free-flow objectives, traffic factor and
traffic delay are null (not applied), even if the server has adopted an overlay.
No pickup, wait, service, maneuver penalty, legal speed or observed travel claim is
introduced. Portable `cost_breakdown: unavailable_not_retained` remains truthful;
this additive export describes bounded static path evaluation, not original
search diagnostics or a retained component ledger.

Clients may render older responses or unknown extension versions with inspection
weights/delay marked unavailable. Supported extension values must match the member's
full graph/evaluation/traffic identity, occurrence ordinals and directed edges, and
reconcile to its totals. User selection, line visibility, inspected occurrence and
reveal progress are renderer state. They cannot rewrite engine ranks or source bytes.

## Phase 29 graph view representation

The independently versioned [bounded graph view](graph-debugger.md#http-contract)
adds a read-only inspection endpoint without altering portable route evidence.
It preserves the full graph identity, exact directed geometry and original static
cost inputs. Its graph-view identity scopes dense node/edge selection to a specific
capture/page; it is not a route evaluation or operational commitment identity.
Road class and search-node membership remain unavailable in current sources.
