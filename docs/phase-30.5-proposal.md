# Phase 30.5 specification (accepted)

Status: **accepted and authorized on 2026-10-17; all A/B/C milestones requested together before opening a PR**.
Phase 30 stops at evidence-qualified overhead 2D delivery execution. This specification
does not amend the engine's routing/access capabilities or authorize Phase 31.

## A: perspective 2.5D

Deliver an overhead/perspective switch, restrained pitch/bearing transitions,
route-aligned follow camera, perspective route highlights and cinematic reveal,
and a readable camera-compatible rider. Optional extruded buildings use a declared
provider; absent buildings or terrain leave a useful road/navigation scene.
The result must be independently usable without a city model or rigged scooter.

Prerequisites: completed Phase 30, checked directed paths, stable actor/action keys,
causal progress sampling, stage authority, and an explicit camera/asset policy.
The Phase 30 overhead SVG is a visual identity reference, not proof of a perspective asset.

Acceptance: preserve rider/action/path, selected operational context, time and progress
through every switch; verify heading, camera offsets, occlusion, scaling and placement
at supported pitches/zooms. Honor reduced motion, allow exiting follow mode, and
retain inspection and evidence labels. Test missing geographic data and camera interruptions.
No mandatory terrain, full-city download, rig or external renderer for this milestone.

## B: ambient vehicle visualization

Deliver a separate renderer-owned synthetic scene layer with car, truck, van and
motorcycle archetypes, compatible with the blue scooter's visual style. Overhead
vehicles use recognizable proportions/silhouettes; perspective representations must
account for camera heading, pitch, scale and placement. Declare supported modes
rather than leaving obviously flat sprites in a perspective scene.

Prerequisites: scene-owned stable IDs, declared trajectories/time domain, deterministic
sampling and seeded procedural generation if used, selectable density/quality budgets,
and distinct styling/inspection from authoritative delivery riders.

Acceptance: repeat identical declared scene inputs/time samples reproducibly; maintain
identity through view changes; disable the entire ambient layer without impairing
operations. Measure actor/frame/resource budgets on declared hardware. Actor count
is scene configuration, never inferred physical density from a traffic multiplier.

Ambient actors cannot change traffic snapshots, ETA, dispatch eligibility, capacity,
accessibility, frozen routes or accepted transitions. They are neither GPS observations
nor dispatch candidates. Cosmetic car/truck actors do not establish legal car/truck
routing under `delivery_motorcycle_v2`. Do not claim collision avoidance, traffic-law
compliance, realistic density or congestion effects without separately verified producers.

## C: immersive 3D

Deliver stylized scooter/rider assets, consistent vehicle presets, reusable model
animations (including wheel animation where supported), supported urban scenery,
road-surface/terrain placement, follow/orbit modes and smooth transitions sharing the
same geographic progress and route-reveal state.

Prerequisites: user authorization (received), renderer evaluation driven by actual model/rig
needs, asset rights/provenance, supported geographic alignment, loading/failure policy,
and explicit device/fidelity/resource budgets. MapLibre is not assumed to implement
all advanced model/rig functions. Evaluate a complementary renderer only when needed.

Acceptance: preserve authority and selected time/route/action across 2D/2.5D/3D;
verify orientation, scale, road placement, animation and missing-asset fallbacks;
retain usable navigation with reduced assets and absent terrain/buildings. Record
frame-time and memory evidence for declared actor counts/devices instead of promising
unmeasured fidelity or city scale. Renderer animation cannot establish execution effects.

## Shared scope boundary

Authoritative entities are riders, orders, accepted plans/actions/routes and pinned
contexts. Derived state is projected location, progress, route-derived bearing and
presentation timing. Ambient actors have their own identity and trajectory scope.
Do not combine these into one ambiguous vehicle type. A future interacting traffic
simulator that generates congestion and routing costs is separate engine work requiring
design, quantitative verification and explicit authorization.

## Accepted visual refinements

Use the supplied `references/2.5dview.webp` for projected-building navigation composition
and `rider_top_back_front_views.png` / `rider_rear_view.png` for scooter silhouettes.
Retain the monochrome workspace and distinct rider palettes. Perspective buildings
are enabled by default where provider geometry/heights exist. Selecting a rider
starts a cinematic rear-follow transition unless auto-follow is disabled. Manual
camera changes exit follow; a small XYZ gizmo provides orbit, axis and front/rear
controls. Distant perspective/3D riders become compact face/helmet location tags,
using the same identity/coordinate, rather than unreadably small vehicle models.

See [the completion evidence](phase-30.5-completion.md) for actual fidelity and limitations.
