# Phase 30.5 — perspective, ambient vehicles and 3D

The user accepted the proposal on 2026-10-17 and requested A/B/C together before
opening a PR. Work is on `feat/perspective-delivery-navigation` in the original
checkout. Phase 31 and physically interacting traffic remain outside this change.

## Implemented presentation

- A: overhead/perspective switching, default projected buildings, smooth camera
  transitions, route-aligned rear follow, existing cinematic path reveal, eight
  camera-relative scooter illustrations, reduced-motion support and flat-context fallback.
- B: separate synthetic ambient car/truck/van/motorcycle actors, off by default,
  selectable 12/24/48 count budgets, stable renderer IDs and deterministic sampling.
- C: lazy Three.js custom MapLibre layer with original procedural scooter/rider
  and vehicle models, rounded bodywork, helmet/face, cargo case, headlights,
  mirrors, wheels and route-progress-based wheel rotation. No downloaded model or rig.

Selecting a rider in perspective/3D starts a 1.1-second rear-follow transition by
default. The preference can be disabled. Manual map gestures, fit, orbit and axis
presets exit follow; Rear resumes it. A small draggable XYZ gizmo also provides
Front/Rear buttons. Camera pitch is bounded to 70 degrees; perspective follows at
55 degrees/zoom 18.5 and 3D at 65 degrees/zoom 20.5. Reduced motion removes transition
animation. The ongoing follow target is sampled at most 10 Hz.

At zoom below 18, perspective/3D riders use compact face/helmet location tags.
Close 3D rider models are omitted at distant zooms; the same DOM marker, rider ID,
ActionId and sampled coordinate remain. The shared portrait source also supplies
the floating cards. Full-screen monochrome map and independent collapsible cards
remain; no sidebar was added.

## Authority, motion and placement

The original Phase 30 `sampleRider` model is unchanged: causal uniform-distance
interpolation on retained directed route geometry, respecting delay history and
accepted execution stages. Switching presentation never changes selected simulation
time, dispatch assignment, action, route or accepted completion. No GPS is invented.
Unknown route geometry remains unavailable. Idle orientation uses cosmetic north
when no route-derived bearing exists; it is not measured heading.

Models use local +Y forward/+Z up, with Mercator Y inversion and clockwise compass
heading. A regression test covers all four cardinal headings. A nearby Mercator
origin reduces coordinate precision loss. Wheels use sampled route distance, not
wall-clock authority; reduced motion freezes wheel animation.

The supported geographic surface is flat altitude zero. No terrain provider,
bridge-deck altitude, slope banking or collision placement is claimed. Delivery
models use a declared 1.6x cosmetic emphasis for readability. Urban context comes
from the existing OpenFreeMap basemap, independently of engine graph evidence.

Building extrusions use the provider's `building` layer, `render_height` and
`render_min_height`, excluding `hide_3d`. Missing heights resolve to zero; missing
buildings leave useful route navigation. Provider height approximations are not
surveyed city geometry. Sources: [OpenMapTiles schema](https://openmaptiles.org/docs/schema/)
and [building producer schema](https://github.com/openmaptiles/openmaptiles/blob/master/layers/building/building.yaml).
MapLibre's [custom render input](https://maplibre.org/maplibre-gl-js/docs/API/type-aliases/CustomRenderMethodInput/)
and [Mercator coordinates](https://maplibre.org/maplibre-gl-js/docs/API/classes/MercatorCoordinate/)
provide the projection interface; Three.js owns vehicle materials and geometry.

## Synthetic ambient scene contract

`ambient:<run_identity>:<index>` identifies each actor. Configuration v1 uses exported
graph edge `(index * 17 + 7) % edge_count` as a declared cosmetic trajectory, archetype
`index % 4`, speed `3 + index % 5` metres/second, and phase `((index * 37) % 101) / 101`.
Sampling uses selected logical time minus scene start. At an edge endpoint the actor
reappears at the trajectory start. This periodic reset is explicit, not a legal
U-turn or a connected journey. Zero-length trajectories remain stationary.

These trajectories establish neither car/truck route legality under the motorcycle
profile nor traffic-law compliance, collision avoidance, congestion or physical city
density. Actors never enter the domain rider list or affect routing cost, traffic
multipliers, ETA, capacity, dispatch candidates or accepted execution. Disabling
ambient actors leaves the operational scene intact. No random per-frame generation.

## Resources and fallback

Two-dimensional/2.5D navigation works without loading Three.js. The 3D module is a
separate approximately 135 kB gzip chunk (see build log for the exact current size).
Assets are repository-authored procedural meshes/SVGs with no external model rights
or loading dependency. Renderer/import failure leaves illustrated rider/ambient
fallbacks and an explicit unavailable message. Models are reused by scene identity
and disposed when removed or 3D is disabled. Sprite caches are bounded by supported
presets/views (ambient cache capped at 256 entries).

Ambient count is capped at 48; the API already caps authoritative riders at 32.
Default ambient count is zero. Maximum quality is an explicit desktop demonstration
budget, not a guarantee for every mobile GPU. No shadows, terrain streaming, rigged
character animation or full-city asset download is required. GPU memory and sustained
frame-rate limits on low-end devices remain unmeasured.

## Verification and evidence

All 65 frontend tests pass. Frontend tests cover existing real engine fixtures, identity across action/suffix
updates, unknown geometry, reduced motion, camera interruptions, auto-follow preference,
distant tags, axis controls, deterministic ambient actors, cardinal heading, model
reuse/disposal, active-3D workspace teardown and renderer failure. See [test log](evidence/phase-30.5a/client-tests.log),
[build](evidence/phase-30.5a/client-build.log), [lint](evidence/phase-30.5a/client-lint.log)
and [format](evidence/phase-30.5a/client-format.log).

Running-app checks use the real API Distributed Fleet Demo on graph digest
`a3f19ae1814288d33c2db7bd3d60f402ee047d2cb0b8e60ad24e1d82a4273e95`
(58 nodes, 114 directed edges). Six identities remain distinct. Switching views and
selected rider preserves action and clock. Rear/front views, default buildings,
flat fallback, disabled auto-follow, multiple moving actors and mobile controls
were inspected in the actual browser. T3 preview intermittently timed out; Chrome
provided the remaining visual checks.

[Renderer samples](evidence/phase-30.5a/render-samples.json) record 30 sequential DOM
instrumentation samples in one running Chrome session on Apple M3/16 GiB, 48 ambient
actors plus six distant rider tags, fit-scene/1x. They measure Three.js CPU render submission,
not total frame time, GPU completion or a sustained FPS benchmark. The captured run
reported 1,176 draw calls/geometries and 225,696 triangles with 48 models and six
rider tags in view; CPU sample median 3.8 ms and p95 4.9 ms. This is bounded resource evidence,
not an unmeasured scalability claim. Distant rider tags omit rider models below zoom 18. No GPU-memory number is fabricated.

Visual evidence:

- [2.5D rear view](evidence/phase-30.5a/rear-2.5d.jpg)
- [3D front view](evidence/phase-30.5a/front-3d.jpg)
- [3D rear view](evidence/phase-30.5a/rear-3d.jpg)
- [Building-free fallback](evidence/phase-30.5a/flat-fallback.jpg)
- [Ambient scene](evidence/phase-30.5a/ambient-3d.jpg)
- [Distant 3D rider tags](evidence/phase-30.5a/distant-tags-3d.jpg)
- [Mobile rider tags](evidence/phase-30.5a/mobile-tags.jpg)

## Changed interfaces

`DeliveryMap` adds reduced-motion and selection-revision inputs. `DeliveryWorkspace`
increments selection revision even when the same rider is selected again. Existing
API/domain contracts are unchanged. Renderer modules: `CameraGizmo`,
`perspectiveScooter`, `riderPortraitAsset`, `ambientScene`, `vehicleModels` and
`immersiveLayer`; the shared `RiderPortrait` uses the same portrait asset as map tags.
`AGENTS.md`, the accepted proposal and READMEs record the authorization and behavior.

The bounded A/B/C presentation is implemented. It does not reproduce the reference's
photorealism, lane instructions, detailed road surfaces, trees, bridges or complete
3D geography. Those require additional geographic producers/assets; they are not
invented by this renderer. Phase 31 radar/candidate-selection acceptance remains in
the roadmap and has not been implemented here.
