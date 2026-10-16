# Roadrunner — Visual Experience Addendum to the Phase 26 Architecture Audit

I've reviewed the consolidated Q1–Q49 Observability/Visualization Contract Architecture Audit.

Before I give final architecture sign-off, I want to add an important product requirement.

**Roadrunner's visualization shouldn't merely display routes, maps, and motorcycle riders. It should make the routing and dispatch engine visually impressive and understandable through carefully designed cinematic animations.**

I want the experience to feel like an advanced routing/dispatch command center, combined with a polished navigation application.

This is a refinement to the future visualization product requirements, not a request to reopen the completed architecture grill or begin implementation.

## 1. Cinematic Route Reveal

When Roadrunner calculates a route, I want the future interface to visually reveal the result.

Imagine:

1. The map frames the selected motorcycle courier and relevant destinations.
2. A luminous line emerges from the route's starting position.
3. The line progressively traces the actual road geometry, following intersections and curves.
4. A bright traveling highlight moves along the line's leading edge.
5. The camera may smoothly pan or zoom to follow or frame the reveal.
6. Once complete, the line transitions into the normal selected-route appearance.
7. Distance, estimated travel time, and other supported route details become visible.

This should feel similar to cinematic route/path visualizations seen in futuristic interfaces or movies.

### Technical requirements

The line must follow Roadrunner's actual graph-qualified, directed route geometry.

Do not animate a straight line between endpoints as though it were the computed road route.

Preserve:

- Ordered traversal occurrences.
- Actual directed geometry.
- Repeated traversals.
- Route identity.
- Graph provenance.
- Applicable route evaluation.
- Canonical route-distance semantics.

The reveal can be sampled by cumulative polyline distance for smooth visual pacing, but animation timing is a presentation choice.

A route may pass through multiple stops. Do not assume the rider travels directly to the delivery dropoff when the accepted plan requires reaching a pickup location first.

The future animation should correctly represent the supported route legs and their established order.

**Crucially, the animated line represents the route Roadrunner calculated. It does not mean the motorcycle physically drove that route during the reveal.**

Distinguish:

```text
Route calculation/search
        ↓
Computed result available
        ↓
Cinematic route reveal
        ↓
Route remains displayed
        ↓
Separate execution/movement visualization
```

The route reveal should be available without requiring detailed Dijkstra/A* search instrumentation.

A genuine step-by-step algorithm search visualization remains a separate feature requiring retained diagnostic traces.

## 2. Spatial Rider Discovery — Radial Search Animation

I also want a cinematic animation when Roadrunner evaluates possible delivery riders around a geographic search origin.

My visual concept:

**A radar-style beam or expanding circular pulse originates from the search center and spreads outward across the map.**

As the pulse progresses:

- Nearby motorcycle markers become visually emphasized.
- Supported eligible/evaluated riders can receive temporary highlighting.
- The selected rider becomes visually distinct.
- Other candidates return to their normal appearance.
- The selected rider's route can then receive the cinematic route reveal.

A possible visual sequence:

```text
Dispatch Request
      ↓
Spatial Search Pulse
      ↓
Candidate Discovery / Evaluation Display
      ↓
Selected Courier Highlight
      ↓
Computed Route Reveal
      ↓
Fleet/Navigation View
```

### Important evidence distinction

Inspect the actual spatial shortlist and candidate-coverage implementations.

Do not assume Roadrunner always searches a literal geographic circle with a fixed radius.

Determine whether a given dispatch policy actually supplies:

- A geographic search center.
- A declared radius and distance semantics.
- A bounded or unbounded candidate set.
- Eligibility or coverage rules.
- Search ordering or candidate-ranking evidence.
- The selected result and publication status.

If an actual radius exists and is retained, the UI may draw a geographically meaningful search boundary based on it.

If the algorithm uses a different candidate-discovery mechanism, a radar pulse can still be a decorative visualization metaphor, but it must not claim that the engine conducted an actual radial sweep or considered every rider inside the depicted circle.

**Do not manufacture candidate evaluation order from geographic distance.**

Similarly, do not animate riders becoming eligible, evaluated, or rejected at particular moments unless the underlying diagnostic evidence supports those transitions.

If only the final candidate set and selection evidence exist, the future UI may play a clearly presentation-derived discovery sequence using those results.

An animation displayed while computation is pending is not automatically evidence of intermediate algorithm progress.

### Selection versus commitment

The highlight must distinguish:

```text
Candidate evaluated

Candidate feasible

Candidate recommended

Proposal submitted

Assignment committed

Active frozen execution
```

Do not show a rider as successfully assigned merely because the evaluator recommended them.

If the search is incomplete, no eligible rider is found, or publication fails, the animation must reflect the actual outcome.

Do not invent a successful rider selection for dramatic effect.

## 3. Visual Modes — Cinematic and Engineering

I want the future UI to showcase the engine at two levels.

### Cinematic presentation mode

Focused on making the system beautiful and understandable.

Features may include:

- Animated route drawing.
- Spatial discovery pulses.
- Courier-selection highlights.
- Smooth camera movement.
- Road-aligned scooter animations.
- Attractive selected/alternative route styling.
- Pickup and dropoff markers.
- Subtle glows, pulses, and depth effects.
- Smooth 2D/3D transitions.

### Engineering inspection mode

Focused on explaining what actually happened.

Features may include:

- Exact route and traversal identities.
- Candidate coverage and eligibility.
- Search results and termination.
- Selected versus rejected decisions.
- Cost and objective breakdowns.
- Graph/traffic context.
- Evidence provenance.
- Supported search summaries.
- Detailed algorithm traces when future producers exist.

These modes should share the same underlying domain evidence.

Cinematic animations are derived presentations; engineering diagnostics should make their source and limitations inspectable.

A user should be able to inspect a revealed route or selected rider and understand which Roadrunner result supports the visualization.

## 4. 2D and 3D Compatibility

The existing reference images remain binding:

- The Google-Maps-inspired image for map quality, visual environment, and perspective navigation.
- The stylized blue delivery scooter with helmeted courier for the default rider design.

The cinematic route reveal must work in both 2D and 3D.

In 2D, the highlighted line follows the mapped road geometry.

In 3D, the equivalent route is rendered over the supported road/scene presentation with perspective-correct styling and appropriate depth behavior.

Both views use the same canonical route and progress references.

Do not independently generate different routes for 2D and 3D.

The same principle applies to candidate-discovery pulses and rider-selection highlights.

The 3D scene may use enhanced lighting, glow effects, or ground-level presentation, but these cannot redefine actual geographic search coverage or route geometry.

## 5. Motion Design Requirements

I want deliberate, high-quality animation rather than excessive decorative effects.

The future motion system should consider:

- Smooth easing and transitions.
- Distance-based route reveal.
- Head/tail highlighting.
- Camera framing and follow behavior.
- Clear visual hierarchy between candidates and the selected rider.
- Stable visual appearance across mode switches.
- Configurable animation speed.
- Pause, replay, skip and reduced-motion support where applicable.
- Responsive behavior with several riders or routes.
- Performance under representative workloads.

Presentation animation time must remain separate from operational and simulation time.

A route reveal should be replayable without rerunning the routing algorithm.

A paused animation must not pause authoritative dispatch.

A slow animation must not imply the underlying route calculation is still running after its result has already been produced.

## 6. Roadrunner Data-Contract Implications

Check whether the accepted Q1–Q49 architecture already provides the necessary domain information.

For **route reveal**, relevant dependencies include:

- Original directed route geometry.
- Traversal occurrence boundaries.
- Route and evaluation identity.
- Canonical distance/progress mapping.
- Selected/proposed/committed status where applicable.

For **rider discovery**, relevant dependencies include:

- Dispatch evaluation identity.
- Actual candidate coverage policy.
- Supported geographic selection scope, if any.
- Candidate identifiers and evaluation status.
- Returned recommendation.
- Publication outcome.
- Evidence granularity and completeness.

Do not add animation keyframes, glow parameters, camera scripts, or material settings to the Rust routing core.

Those are renderer/presentation responsibilities.

If the audit identifies missing producer evidence needed for an accurate visualization, classify it as:

1. Already covered by the approved Phase 26 producer package.
2. A small proposed Phase 26 contract clarification.
3. A conditional producer requiring explicit authorization.
4. A later-phase diagnostic or UI capability.

Do not silently expand mandatory Phase 26 implementation.

## 7. Phase Ownership

Preserve the existing roadmap.

**Phase 26:** freeze and implement the approved route, provenance, selection, and diagnostic contracts.

**Phase 27 and later visualization phases:** implement the actual route-reveal animation, discovery pulses, selected-rider highlighting, motion system, map integration, and cinematic presentation.

Detailed historical search-frontier animation still requires separately authorized instrumentation.

If existing later roadmap phases already own route drawing, delivery movement, or dispatch inspection, map these features to those phases explicitly instead of inventing a competing roadmap.

The intended initial renderer direction remains React/MapLibre unless separately revised.

## 8. Addendum Acceptance Tests for the Future UI

Record future visual verification requirements:

1. A completed route is revealed along its actual directed road geometry, not a straight-line shortcut.
2. Route reveal is repeatable without rerunning route optimization.
3. A route with multiple stops reveals its supported leg order correctly.
4. Repeated traversals and reversed edges animate correctly.
5. An uncommitted proposed assignment is not presented as committed.
6. A spatial search circle is geographically authoritative only when the actual producer establishes its center, radius, units and coverage semantics.
7. A decorative radar pulse does not fabricate candidate evaluation order or excluded riders.
8. No feasible/selected courier produces an appropriate unsuccessful or unavailable state.
9. Switching 2D/3D preserves route, selection, progress and courier identity.
10. Skipping or replaying an animation does not change operational state.
11. Reduced-motion presentation communicates the same meaningful results without requiring elaborate animation.
12. Performance degradation may reduce visual effects without altering domain evidence.

These are future client acceptance obligations, not evidence that the current renderer or capabilities exist.

## 9. Requested Audit Update

Please prepare a **targeted addendum** titled:

**Cinematic Routing and Spatial Dispatch Visualization — Product Experience Contract**

In it, document:

- Cinematic route reveal.
- Radial candidate-discovery/search presentation.
- Selected-courier emphasis.
- The transition from discovery to selected route.
- The distinction between actual algorithm evidence and illustrative presentation.
- Required existing domain contracts.
- Missing producer evidence, if any.
- 2D/3D compatibility.
- Motion and accessibility requirements.
- Future tests and phase ownership.

Identify exactly which sections of the consolidated Q1–Q49 audit should reference this addendum.

Do not reopen settled architecture decisions unless this new requirement exposes a genuine contradiction.

Do not begin implementation.

Do not create animation assets or write renderer code.

**Return the updated architecture-sign-off summary, with this product-experience requirement included, for my confirmation.**
