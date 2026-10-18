# Live delivery visualization: bounded synthetic execution

Phase 30 implements the approved audit's movement policy over **real Roadrunner
simulation results**. It is a local execution lab, not a live GPS fleet feed.
The [approved contract](visualization-contract.md), Q1–Q49 audit and cinematic
addendum remain authoritative. See [completion evidence](phase-30-completion.md).

## Use and presentation

Start the API and Vite as described in [web/README.md](../web/README.md). Connect to
the graph, then choose **Delivery simulation**. Run one rider, two riders, or the
road-delay/active-suffix scenario. The defaults target the checked-in Phase 10 graph
(nodes 0, 35 and 22); select explicit graph nodes for another graph. Scenario JSON
allows supported engine traffic changes and other bounded inputs. The JSON editor
explicitly overrides presets when enabled. Invalid/unreachable inputs are engine
results or explicit errors, never invented paths.

Pickups use gold circles with restaurant utensils; customers use lavender circles
with person glyphs. Labels and the legend retain semantic type without relying on color alone.

The map covers the entire viewport. Setup, rider roster, selected execution, orders
and traffic use **separate collapsible floating cards**, following the routing
workspace's monochrome palette. Setup collapses after a successful run; mobile
execution details start collapsed. The roster shows helmet/head portraits, matching
each scooter and route accent. Collapsed roster cards show overlapping portraits of
all configured riders; collapsed selected-rider cards retain that rider's portrait.

Renderer-owned appearance slots follow canonical initial RiderId order for this
finite run. Eight body palettes and four helmet/jacket variants provide 32 distinct
combinations; they stay fixed through actions, generations and plan changes. Ochre,
sage, coral, lilac and other cosmetic presets do not imply routing profiles, priority,
physical attributes or authority. The overhead scooter and portraits are original
SVG artwork informed by the supplied reference; reference bitmap files are not bundled.

Play/pause, restart, next distinct event time, range scrub, exact logical seconds and
1/2/5/10× affect only presentation; playback starts at 1×. Reduced motion starts paused. This is deterministic
sampling of one finite captured run. [Phase 32](historical-replay.md) adds local saved-run
import and event navigation; it does not introduce a server-side session store.
At equal logical times the final captured queue boundary is shown; intermediate
Arrived states remain in source captures and accepted effects remain inspectable.
Leaving the delivery workspace discards its local presentation; routing results in
the parent workspace remain. No live operational command is issued.

## Producer and HTTP contract

`roadrunner_simulation::simulate_scene` is opt-in. Ordinary `simulate` artifacts and
execution semantics are unchanged. The optional `SimulationResult.scene_frames`
field is omitted in normal runs. Captures contain the real coherent world, frozen
execution, plan revisions and actual static traffic after each processed queue entry,
plus the initial boundary. Source vector order resolves equal times, including stale
attempts with unchanged state. Capture failure rejects the isolated run, never truncates.

`POST /v1/visualization/deliveries` accepts the existing strict `SimulationScenario`
JSON, including the **exact server graph digest**. It requires namespace-wide read
**and simulation** capabilities. Scoped/read-only credentials reject. It neither
reserves an operational command nor writes live state, outcomes or persistence.
No Idempotency-Key is required for this pure finite computation.

The version-1 response contains:

- `semantics: synthetic-delivery-scene/v1`, `motion_model: causal-uniform-distance/v1`.
- Scenario content identity `scene-v1:<SHA256>` over typed compact scenario JSON,
  including the graph digest; this is content identity, not a unique invocation ID.
- `time_domain: simulation_logical_seconds_no_civil_epoch`, start/end seconds.
- `frames[]`: logical instant, exact revision string, riders, orders, current static
  traffic overrides/digest. Riders retain actual ID, coordinate, availability,
  capacity/onboard load, assigned orders, ordered stops, plan revision and frozen action.
- `routes[]`: each committed start, rider/action join, original execution and traffic,
  checked original directed geometry or explicit `geometry_error` with null geometry.
- Original ordered `schedules[]`, accepted `effects[]`, engine summary and scenario.
- `roads`: unmodified Phase 29 standalone graph representation, first bounded page,
  captured with free-flow objective. Its original evaluation is not relabeled as traffic.
- `traffic_semantics: synthetic-static-edge-factors/v1` for the distinct current
  simulation traffic fields. Unsupported/missing traffic semantics disables the overlay.

Scene/action identity is `(scene content identity, RiderId, ActionId)`. Schedule
attempt/generation, originating plan, current plan revision and operational revision
remain distinct. Marker DOM identity is `(scene, RiderId)`; changing the current action
updates the same marker. No schedule generation creates another actor.

A worker and export permit are acquired without queuing. Saturation returns 503;
body transfer holds the export slot and releases the worker after computation.
The scenario must fit 32 riders, 64 orders, 128 external interventions, 86,400 seconds,
10,000 graph edges, 10,000 planning submissions per decision and 10,000 overrides per
context. At most 2,000 queue entries plus initial capture are admitted. Each checked
route is limited to 2,000 traversals/16,000 vertices; geometry failure leaves that
active position unavailable. The final encoded response is capped at 8 MiB (413),
without partial success. Existing request body limits still apply. These are conservative
admission limits, not supported frame-rate or production scalability claims. Full-world
capture allocations precede output encoding; this is not a streaming or fleet-scale API.

## Motion and authority

Canonical E7 vertices are preserved in original directed occurrence order, including
reverse curves and duplicated joints. A display path accumulates Haversine polyline
distance. `causal-uniform-distance/v1` assumes uniform distance progress between
supported temporal anchors; positions linearly interpolate within the selected E7
segment. This is illustrative travel, not measured speed or a physical traffic model.

At departure progress is zero. At a successful delay at `t`, compute progress with
the **previous** anchors, retain it, then spread remaining distance uniformly from
`t` to revised arrival. Future schedule changes never rebase earlier samples. Updated
`leg.travel` is not interpreted as remaining duration. Frozen original route and
traffic stay pinned when plan suffixes or current contexts change.

Travelling progress clamps to [0,1]. Expired prediction holds an **estimated endpoint,
awaiting accepted arrival**; it does not change stage, custody or fulfillment.
Arrived/Waiting/Servicing use the accepted synthetic endpoint. Pickup-before-delivery,
capacity and service completion come from engine state. Idle markers use the latest
simulation-authoritative endpoint; no HTTP snapshot coordinate becomes observed GPS.

Heading is the tangent of the directed noncoincident route segment, not measured
heading. At a joint use the next nonzero segment; at the end use the last. Waiting
and service retain the route's endpoint direction. No-travel/idle has no directional
evidence and uses a neutral north pose. Invalid/missing original geometry hides the
active marker and states position unavailable rather than drawing a connector.

Geometry uploads occur only on captured transitions/selection/context changes.
Animation frames move existing DOM markers; no per-frame route GeoJSON retransmission,
new routing search or random actor creation occurs. Source validation uses lossless
integer parsing, exact graph/action/path binding, occurrence direction/continuity,
frame ordering and version checks. Polar and seam-crossing rendering is unsupported.

## Supported traffic and unavailable evidence

Current synthetic context overrides are exported from the actual engine snapshot.
Omitted edges default to 1 **only under the supported complete static context**.
Road styling uses this context, independent of each frozen action's original context.
Directed sides are offset for readability; coverage is the first 200 edges / 20,000
vertices, not a spatial or city-wide query. Inspection names edge/from/to/direction,
factor, retained free-flow seconds and access status. Current standalone road time is
explicitly derived as `base_seconds × current_factor`; forbidden access is shown as
forbidden, and this is not incoming-maneuver or whole-route feasibility.

No live traffic feed, observed counts/density/speeds, numeric GPS uncertainty, turn
penalty ledger or unretained route cost breakdown exists. No ambient cars/trucks are
rendered. Supported traffic scenarios can be entered as explicit JSON; the default
presets use the engine's normal static context. Simulation changes do not adopt
traffic into live authority. Phase 29 continues to inspect an already adopted live
server overlay separately; these contexts are not combined.

Live scoped fleet snapshots/subscriptions, cross-graph historical resolution, measured
telemetry, prediction-based completion and physically interacting ambient traffic are
unsupported. Perspective/ambient/3D work remains the separately authorized
[Phase 30.5 proposal](phase-30.5-proposal.md).

## Distributed Fleet Demo and clustering investigation

The original two-rider preset intentionally reuses nodes 0, 35 and 22: order 1 is
35 → 22 and order 2 is 22 → 0, with starts at 0 and 22. Shared street geometry and
pickup/customer anchors caused the clustering. Original frozen paths and marker
samples agree; no evidence identified a routing or frontend binding bug.

**Distributed Fleet Demo** is a selectable default on the exact Phase 10 graph
(`a3f19ae1814288d33c2db7bd3d60f402ee047d2cb0b8e60ad24e1d82a4273e95`). It is
explicitly unavailable on other graphs, rather than interpreting dense IDs under a
new graph. Other presets and explicit scenario JSON remain available. It uses:

- Six distinct starts: nodes 1, 22, 23, 49, 24 and 5.
- Five pickup/dropoff pairs: 0 → 35, 16 → 18, 26 → 23, 55 → 50, 25 → 24.
- Existing directed components, readiness forecasts/observations, scalar capacities,
  service policies and ordinary Greedy fleet/recovery inputs. No assignment override.

At 20 logical seconds the actual engine yields three travelling riders (1/3/5),
one servicing rider (4) and two idle riders (2/6). Rider 1 owns both orders 1 and 2
and executes them sequentially within capacity; this legitimate choice is preserved.
Riders 3/4/5 own orders 3/4/5. All five complete by the horizon. Components naturally
constrain reachability; their disconnected roads are not joined for visual variety.

The entire fixture spans approximately 520 m north/south and 670 m east/west. This is
a broader **street-scale** demonstration, not multiple distant neighborhoods or a city.
A larger demonstration requires an imported, verified larger graph and explicit new
scenario configuration. Camera fitting considers every configured rider and every
exported original path, including idle endpoints; resizing/refit preserves full-map
coverage without synthetic coordinates. The roster scrolls for larger fleets.

Phase 31 owns the retained circular/radar discovery sequence. AGENTS.md now explicitly
requires a supported pickup center/radius/units/eligibility/limit association for a
geographic pulse, independently qualified candidate/selection/commitment evidence,
and a clearly labeled metaphor for nonradial insertion/fleet/recovery. Phase 30 adds
no radar, candidate discovery-order reconstruction or dispatch-selection choreography.
