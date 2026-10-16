# Cinematic Routing and Spatial Dispatch Visualization — Product Experience Contract

**Status:** Proposed addendum for architecture confirmation.
**Effect on Q1–Q49:** Compatible refinement; no settled decision is reopened.
**Implementation:** Not authorized or begun. No files, assets or renderer code were changed. Tests were inspected, not executed.

Roadrunner’s future visualization should combine polished navigation with an understandable routing/dispatch command center. Cinematic and engineering modes consume the same evidence. Their presentation differs; their underlying claims do not.

## 1. Cinematic route reveal

Once a computed route is available, the client may frame the relevant courier and destinations, progressively illuminate the route, animate a leading highlight, and transition to the normal selected-route style.

The reveal must use the pinned, graph-qualified directed geometry and preserve:

- Route and evaluation identity.
- Ordered traversal occurrences and boundaries.
- Reversed and repeated traversals.
- Canonical distance semantics.
- Applicable candidate, proposal, commitment or execution status.

Reveal pacing may use cumulative canonical polyline distance. Its easing and duration are presentation choices.

**Route-reveal progress and journey progress are different quantities.** The luminous head describes how much of a computed result has been displayed; it does not locate the motorcycle or establish completed travel.

A no-travel route should receive an appropriate destination/point presentation, not an invented moving line. Missing geometry or discontinuities must remain explicit.

### Multiple stops and legs

A multi-stop reveal must follow the supported plan’s established order, including pickup before delivery where required. Each leg retains its own route/evaluation provenance.

A stop list or aggregate plan evaluation does not itself supply every leg’s road geometry. Where geometry was not retained, the client must report that limitation or perform a separately identified new evaluation. It cannot fill gaps with straight lines presented as computed roads.

### Replay and pending computation

The reveal must be repeatable from retained route evidence without rerunning optimization.

While computation is pending, a client may display a clearly nondiagnostic loading effect. It must not imply actual search progress. After the result exists, a slow reveal must not imply that routing is still running.

A genuine search animation remains separate and requires an actual diagnostic trace.

## 2. Spatial discovery: repository findings and permitted presentation

The basic-dispatch implementation does support a specific radius-based policy:

- It first constructs the **Basic Dispatch eligible** rider population.
- `CandidatePolicy::Spatial` searches around the order’s **pickup coordinate**, using radius and result limit.
- Lookup uses straight-line Haversine distance, includes the radius boundary, and returns nearest results ordered by distance then RiderId.
- Candidate generation subsequently sorts the resulting IDs canonically. That order is not a retained discovery timeline.
- Spatial policy coverage remains `PotentiallyIncomplete`; it does not become complete because a particular result count appears sufficient.

See [candidate generation](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/evaluation.rs:276), [lookup semantics](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/spatial/rider.rs:46) and [coverage policy](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/src/decision.rs:24).

The existing [lookup tests](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/tests/rider_lookup.rs:40) cover boundary, zero-limit and tie behavior. The [dispatch coverage test](/Users/admin/Developer/roadrunner/crates/roadrunner-dispatch/tests/entry_gate.rs:183) distinguishes spatial nearest from exhaustive routed selection.

Two limitations matter:

1. Radius and limit are retained in basic decision evidence, but its geographic center must be resolved from the correct originating order/snapshot evidence; it is not a standalone center field in that evidence envelope.
2. Current operational API planning exposes [insertion, fleet or recovery decisions](/Users/admin/Developer/roadrunner/crates/roadrunner-api/src/runtime.rs:648). These do not automatically inherit the basic-dispatch radial shortlist.

### Geographic boundary versus decorative pulse

A search boundary may be presented as geographically meaningful only when evidence establishes its center, radius, units, distance model, eligible population and limiting policy.

Even then, the circle describes the declared distance filter—not a physical scan, an evaluation sequence, or proof that every visible rider inside it was considered. Rendering that boundary requires a supported geographic projection.

For exhaustive or nonradial policies, a radar beam or pulse may be used as a **presentation metaphor**, with no implied finite search radius.

Markers may be emphasized from retained results. Any staged appearance order is presentation-derived unless actual ordered diagnostic evidence exists. Distance ordering must not be fabricated as historical evaluation order.

## 3. Selected-courier emphasis and route transition

The future sequence may be:

**Dispatch presentation → candidate emphasis → supported outcome → route reveal → normal fleet/navigation presentation.**

This is presentation choreography, not a required sequence of domain transitions.

The client must distinguish:

- Evaluated candidate.
- Feasible candidate.
- Recommended rider.
- Submitted proposal.
- Committed assignment.
- Frozen active execution.

A recommendation may receive an explicitly provisional highlight. Commitment styling requires publication evidence. An active-execution treatment requires the corresponding frozen action.

No feasible result, unavailable inputs, incomplete search or rejected publication must produce an appropriate unsuccessful, incomplete or unavailable presentation. There must be no invented successful courier selection.

Fleet decisions may justify only a fleet-level explanation. A visually emphasized rider must not acquire an unsupported claim such as “fastest courier.”

If newer evidence supersedes the displayed result, the animation remains bound to its original evaluation or is explicitly replaced. It must not silently change the meaning of an in-progress reveal.

## 4. Cinematic and engineering modes

**Cinematic mode** may provide animated lines, pulses, selected-courier emphasis, camera framing, glows, route styling and smooth scene transitions.

**Engineering mode** exposes the supporting identities, policies, coverage, objectives, result/termination, publication status and evidence limitations.

Every cinematic subject should be inspectable through its source association:

- Revealed line → route and evaluation.
- Candidate highlight → retained candidate evidence or declared aggregate limitation.
- Search boundary → actual geographic policy evidence.
- Assignment emphasis → recommendation/publication/execution stage.

Neither mode may manufacture missing diagnostics. Summary-only evidence supports result presentation, not historical frontier animation.

## 5. Shared 2D/3D semantics

Both renderers use the same route, traversal occurrences, selection evidence, courier identity and presentation selection.

Switching modes preserves:

- Pinned graph and route.
- Evaluation/selection identity and authority stage.
- Journey progress and playback boundary.
- Route-reveal progress, separately from journey progress.
- Candidate-presentation state.
- Intended scooter/courier preset.

The 3D renderer may use perspective, depth treatment, lighting and supported scene alignment. Those effects cannot redefine canonical geometry or geographic search coverage.

Canonical-to-rendered placement follows the accepted alignment contract. Missing road elevation cannot be concealed by treating a ground-level effect as verified road-surface placement.

The existing navigation/environment and blue delivery-scooter references remain binding product inputs.

## 6. Motion, accessibility and resource behavior

The renderer owns easing, leading/trailing highlights, camera motion, materials and effect parameters. None belongs in the Rust routing core.

The motion system should support:

- Configurable presentation speed.
- Pause, replay and skip where applicable.
- Reduced-motion alternatives.
- Clear hierarchy between candidates, recommendations and commitments.
- Meaning conveyed beyond color or animated effects alone.
- Stable appearance during mode switches.
- Representative multi-route and multi-rider performance verification.

Presentation animation time is separate from operational time, simulation time and historical playback selection.

Pausing a reveal cannot pause dispatch. Skipping it cannot advance a scooter’s journey. Replaying it cannot create an assignment or rerun optimization.

Reduced-motion mode must communicate the same result and authority status through static or restrained presentation. Under resource pressure, the client may reduce glows, particles or camera effects while preserving evidence and meaningful status.

## 7. Data-contract implications and scope classification

### Already covered by the proposed Phase 26 package

The existing audit already requires:

- Checked directed geometry and occurrence boundaries.
- Spatial-path and evaluation identity.
- Canonical distance semantics.
- Applicable provenance and typed metrics.
- Distinct selection/commitment stages.
- Candidate coverage and retained evidence granularity.
- Structured unsuccessful outcomes.
- Compatibility and explicit unavailable capabilities.

These support result-based route reveal and honest selection emphasis. They do not imply that the cinematic client exists.

### Small proposed contract clarifications

Add the following clarifications to the audit:

1. **Reveal progress is presentation-only**, distinct from execution progress and algorithm progress.
2. **Spatial coverage presentation must identify its evidence basis:** authoritative distance filter, approximate presentation, decorative metaphor or unavailable.
3. **Candidate result order is not historical discovery order.**
4. **Multi-leg presentation requires supported leg geometry and ordering**, not merely a plan’s stop list.
5. **Every cinematic effect remains associated with its source result and authority stage.**

These clarify accepted semantics without requiring keyframes, cameras, glow settings or new domain algorithms.

### Conditional producers requiring explicit authorization

Potentially needed only if selected for implementation:

- A detached geographic-scope descriptor when the original center/policy cannot otherwise be resolved.
- Additional leg-geometry export for plans that currently retain only timings and aggregates.
- Richer candidate-status adapters where existing exposure is insufficient.

They must not become mandatory through this addendum. Missing information can instead produce an honest limited presentation.

### Later-phase capabilities

- Route-reveal and radar animation.
- Candidate/selection motion design.
- Camera choreography and 2D/3D effects.
- Motion controls and reduced-motion UI.
- Actual ordered discovery/search animation.
- Additional diagnostic producers where that historical detail is desired.

No new mandatory Phase 26 producer is established solely by this addendum.

## 8. Roadmap ownership

- **Phase 26:** approved geometry, identity, provenance, selection and summary contracts, plus the clarifications above.
- **Phase 27:** basic map/route presentation and initial route-reveal implementation within the planned React/MapLibre direction.
- **Phase 28:** selected/alternative styling and comparative reveals.
- **Phase 29:** engineering graph/search inspection; detailed search animation only when supporting traces exist.
- **Phase 30:** scooter journey movement, fleet presentation and continuity between route reveal and execution display.
- **Phase 31:** dispatch discovery presentation, candidate emphasis, selection/publication distinctions and evidence inspection.
- **Phase 32:** historical playback integration using retained evidence and supported temporal interpretation.

A shared renderer-owned motion system can serve these phases. This does not create a competing roadmap or move all cinematic work into Phase 27.

## 9. Future client acceptance obligations

The addendum adds these future tests; **none is claimed passed today**:

1. Reveal follows the complete directed road geometry, including interior curves.
2. Reveal replays without rerunning optimization.
3. Multi-stop presentation preserves supported leg order.
4. Reversed edges and repeated traversal occurrences remain correct.
5. Proposed/recommended assignments remain distinguishable from commitments.
6. A geographic search boundary requires supported center, radius, units, distance and coverage semantics.
7. Decorative pulses do not imply recorded discovery order or complete candidate coverage.
8. Unsuccessful, incomplete and unavailable outcomes do not produce fictional selection.
9. Mode switching preserves route, courier, selection and relevant progress.
10. Skip/replay/pause of presentation effects do not alter operational state.
11. Reduced-motion presentation conveys equivalent meaningful results.
12. Performance degradation reduces effects without changing evidence.
13. Pending-computation effects remain distinguishable from post-result reveal.
14. Reveal progress cannot move the scooter or establish completed travel.
15. Missing leg geometry or ambiguous scene alignment produces explicit limitation.

## 10. Exact references to add to the consolidated audit

The addendum should be referenced from these existing sections; their accepted decisions remain unchanged:

- **§1–2:** product objective and unchanged phase boundary.
- **§3:** spatial-shortlist evidence and its limits.
- **§4–6:** cinematic goals, two visual modes and derived-presentation truth.
- **§7–9:** directed reveal, traversal occurrences and missing geometry.
- **§11–12:** separation of reveal time/progress from execution and movement.
- **§14–17:** candidate coverage, authority stages and distinction from genuine search traces.
- **§20–21:** shared 2D/3D effects, alignment and scooter presentation.
- **§22–24:** effect degradation, local sampling, authorization and presentation-version compatibility.
- **§25:** the future acceptance obligations above.
- **§27:** cross-reference under Q1–Q6, Q10–Q14, Q20–Q29 and Q35–Q41; no renumbering or replacement of decisions.
- **§28:** future product/motion specification reference.
- **§29:** cinematic readiness remains separate from producer conformance.
- **§30:** existing producer workstreams supply evidence; conditional gaps remain explicitly gated.
- **§31:** architecture sign-off includes this addendum while implementation remains separately authorized.

These are proposed audit references presented here; no document files have been edited.

## Updated architecture-sign-off summary

**The Q1–Q49 architecture remains consistent and ready for confirmation with this addendum.** No genuine contradiction was exposed.

The future product now explicitly includes cinematic route reveal, evidence-qualified spatial discovery presentation and selected-courier emphasis, alongside engineering inspection. All remain bound to actual Roadrunner results.

The finite mandatory Phase 26 package is unchanged. The four readiness gates remain separate. Cinematic animation, accessibility controls, assets and visual-performance verification belong to later clients; detailed algorithm animation still requires separately authorized traces.

**Do you confirm the consolidated architecture audit together with this Product Experience Contract?**

Confirmation approves the architecture and product requirements only. Repository changes and Phase 26 implementation still require separate authorization.
