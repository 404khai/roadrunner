# Dispatch visualization — Phase 31

Phase 31 implements inspectable dispatch evaluation and an evidence-qualified cinematic presentation. Open **Dispatch visualization** from the routing workspace. The map fills the viewport; setup, candidate inspection and presentation controls are collapsible floating cards. Existing delivery simulation also exposes a dispatch audit card.

## Original decisions and routes

`POST /v1/visualization/dispatch` evaluates a disposable World using the existing Basic or Preparation-aware dispatcher. It requires namespace-wide read and simulate authorization, the loaded graph digest, graph-node pickup/customer/rider anchors, capacities and availability. It accepts at most 16 riders, limit 16, radius 0–5000 metres and readiness 0–3600 seconds. Existing bounded worker/admission controls and an 8 MiB response cap apply. No live coordinator state is mutated.

The response retains the submitted inputs, original decision, policy, eligibility, coverage, feasible metrics, typed rejections and recommendation. A wrapping RouteProvider captures actual RoutedLegs during that evaluation. Geometry is exported from those exact legs, including original directed occurrences and curvature; clicking candidates never reroutes. Geometry limits are 2000 occurrences and 16000 points per leg. Unsupported geometry remains unavailable. Costs are free-flow estimates under the declared routing provenance; no physical traffic, GPS or unretained cost ledger is inferred.

Optional publication commits the decision only in the disposable demo World. Recommendation, isolated publication and active execution are distinct. This endpoint never starts rider execution; existing delivery simulation remains the execution demonstration.

## Cinematic semantics

A completed evaluation starts a presentation sequence: pickup highlight and search pulse, simultaneous supported candidate emphasis, recommendation emphasis and directed hypothetical route reveal, then normal fleet tags. Replay, pause and skip alter presentation time only. They neither repeat dispatch nor publish assignments. Computation itself has a loading state, with no invented engine search progress. Reduced motion presents the final evidence directly.

Only original Basic Spatial policy supports a geographic circle: pickup center, inclusive Haversine metres, original radius, eligible `basic-idle/v1` population and candidate limit. Spatial coverage remains `PotentiallyIncomplete`. A pulse depicts the declared filter, not physical scan coverage or discovery chronology. Candidate rows retain canonical rider order. Exhaustive policy uses a labeled decorative pulse without a geographic radius. Candidate inspection reveals original pickup then customer geometry, with no rider movement or arrival implied.

`POST /v1/visualization/deliveries` now additionally retains basic, insertion, fleet, recovery and unavailable-prediction audit records. The delivery audit card shows original aggregate search evidence, completion/termination flags and publication receipts. Fleet/insertion/recovery use an explicitly decorative nonradial metaphor, optionally anchored at an associated order pickup. That anchor is not asserted to be the engine's search center. Missing associations disable the geographic presentation. Aggregate evidence does not supply per-candidate hypothetical routes or rankings; none are invented. Basic simulation audit lacks the originating snapshot association required for a radius presentation and directs users to the isolated evaluation instead.

## Validation and limits

Real API outputs are retained in [evidence/phase-31](evidence/phase-31): spatial, exhaustive, empty shortlist, isolated commit, capacity rejection, fleet publication, incomplete insertion search, recovery and unavailable forecast. The graph has 58 nodes and 114 edges; every start and stop is a graph node. Spatial evaluation produces five candidates from six configured riders, recommendation 2, a genuine NoRoute rejection, a curved pickup leg and reverse-directed route occurrences.

The Rust workspace passes 317 tests and workspace Clippy. Frontend suite passes 73 tests; an additional unavailable-forecast test passes separately (74 total cases). Production build passes with the existing large-chunk advisory. New tests validate policy/context binding, selected feasibility, missing geometry, radius mathematics, candidate inspection/replay without repeated API calls, empty results, aggregate incomplete/recovery evidence and unavailable forecast receipts. API tests check bounded inputs, actual captured routes, isolated commit and unchanged live assignments.

Running-app inspection confirmed candidate scores, recommendation/publication labels and six distinct rider markers; presentation returns to normal fleet emphasis. [Desktop screenshot](evidence/phase-31/dispatch-desktop.png) records the full-map layout and helmet identities. Browser capture became unreliable during final verification (repeated host snapshot/resize timeouts; alternate browser also timed out), so final mobile visual verification is **not complete**. Mobile layout uses mutually exclusive floating setup/result cards but requires a reliable browser follow-up. No animation-performance benchmark or mobile visual pass is claimed.

Phase 31's functional and evidence requirements are implemented. Final mobile visual acceptance remains outstanding due to browser-host failures. Phase 32 now adds [saved-run historical replay](historical-replay.md).

## Changed interfaces

- API: new bounded isolated dispatch export; additive delivery `dispatch_audit`; OpenAPI endpoint registration.
- Web: `dispatchEvidence` decoder; `DispatchWorkspace`, `DispatchMap`, `DispatchAuditCard`; routing entry and delivery integration; floating-card styles. Shared route geometry, portrait assets and reveal hooks remain authoritative presentation helpers.
- Existing dispatch algorithms, execution transitions, traffic producers and live publication APIs are unchanged.
