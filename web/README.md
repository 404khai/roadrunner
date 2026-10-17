# Roadrunner routing workspace

The TypeScript / React / MapLibre GL client consumes the real version-1
portable route and alternative endpoints. This is a local engineering interface. No hosted deployment
is included. Phase 30 adds bounded synthetic delivery execution visualization.

## Run

Requires Rust from the repository's toolchain file, Node **22.12+** (Node 24 is used
in CI), npm, a current browser with WebGL, and network access for the contextual
basemap. The Rust engine uses the checked-in graph independently of the basemap.

From the repository root, start the API:

```sh
export ROADRUNNER_API_TOKEN="$(openssl rand -hex 32)"
export ROADRUNNER_API_BIND=127.0.0.1:3100
cargo run --locked -p roadrunner-api -- --trusted-local \
  data/fixtures/phase-10/snapshot.semantic-v2/graph.rr-graph
```

In another terminal:

```sh
cd web
npm ci
ROADRUNNER_API_URL=http://127.0.0.1:3100 npm run dev
```

Open **http://127.0.0.1:5173** and use the **plug icon** in the map controls to enter the token from the
API terminal. Credentials are held only in memory and disappear on reload. You can
also supply `ROADRUNNER_API_TOKEN` to the Vite process, or put it and
`ROADRUNNER_API_URL` in ignored `web/.env.local`. The proxy injects a configured token
on the server side; it is never a `VITE_*` value or compiled into the browser bundle.
The default proxy target is `http://127.0.0.1:3000`. Both development listeners bind
loopback. Restart Vite after installing/changing dependencies.

The map fills the viewport. **SPATIAL WORKSPACE** is a floating card: use its
header to collapse or reopen it without changing the route or reveal. On small
screens, a successful request collapses it automatically so the route is visible;
a compact floating metrics card keeps distance and road time in view.

Select origin/destination using the node selectors or click a visible graph point.
The active field determines which anchor a click sets. These are exact graph nodes;
there is no arbitrary-coordinate routing, geocoder, hidden snapping or connector.
For the bundled fixture, try **0 → 22**, **22 → 0**, **0 → 49** (disconnected), and
**0 → 0** (no travel). Node **35** gives a short curved route from node 0.

Choose A* or Dijkstra and travel time or distance. Traffic-aware time requires an
already adopted server traffic context; unavailable inputs remain an explicit error.
The UI does not create traffic contexts or change the world.

## Route presentation

The muted path shows the entire returned result immediately. Its white prefix and
leading highlight progressively reveal the directed canonical geometry, then become
the normal selected route. A/B mark origin/destination. A=B marks no travel.

Replay, pause, resume, skip and 0.5×/1×/2× speed affect only this presentation.
They do not fetch routes, advance a journey, or operate on the Rust engine. The
motion toggle and OS `prefers-reduced-motion` produce a static equivalent. Routes
over 16,000 canonical points use a static presentation to cap animation work; this
is a conservative presentation budget, not a measured performance guarantee.

Distance and predicted elapsed **road** seconds come directly from the API. They
exclude waiting and service. The expandable evidence panel exposes the original
profile/jurisdiction, graph/path/evaluation identities, objective and temporal/access/
traffic context. Source download preserves the original response bytes. Large JSON
integers are parsed before native Number conversion can round them. Unknown required
versions, authority/geometry/objective meanings, inconsistent topology, discontinuities
and graph/request mismatches reject. Polar and seam-crossing presentation is unsupported.

## API surface

- `GET /v1/visualization/anchors?offset=0` — authenticated, immutable graph metadata
  and at most **500** exact anchors in dense node order. `next_offset` is null at
  the end. Previous/Next changes the visible page; selected anchors from another
  page remain available only under the same full graph identity. Graph replacement
  clears selection. Invalid offsets reject. No operational snapshot is fetched.
- `POST /v1/visualization/routes` — existing Phase 26 endpoint, unchanged request
  and evidence semantics. Read capability is required. No durable route record or
  operational mutation is created.

- `POST /v1/visualization/routes/alternatives` — ranked Yen members under shared
  provenance, with exact directed paths and bounded termination/work evidence. The
  optional selected-path inspection extension supplies pinned static segment weights
  and traffic factors. See [comparison controls](#compare-alternatives-phase-28).

The catalog is selection infrastructure, not a graph debugger: no edges, weights,
restriction inspection or search traces are exposed by it. It shares the existing
routing endpoint's graph-read access boundary. Serialization is bounded to one
500-node slice and does not lock operational state. Graph pages belong to the
immutable process graph; there is no mutable multi-page snapshot assembly.

## Checks and production bundle

```sh
npm test
npm run lint
npm run format:check
npm run build
```

`npm run build` emits static assets in `web/dist`. Serve those behind an operator-
configured **same-origin `/v1` reverse proxy** to Roadrunner for a non-development
host. Vite's development credential proxy is not compiled into `dist`, and
`npm run preview` serves the built assets locally and inherits the configured
Vite proxy for verification. No deployment, public access policy,
production identity system or TLS setup is supplied by this phase.

The bundle currently emits Vite's large-chunk warning for MapLibre and the client
(~1.38 MB minified main JS, ~0.39 MB gzip, plus the bundled worker). This is documented;
no production payload or frame-rate claim is made.

## Basemap, fonts and limitations

The locally authored monochrome style uses [OpenFreeMap](https://openfreemap.org/quick_start/)
vector tiles and glyphs. Keep its OpenFreeMap/OpenMapTiles/OpenStreetMap attribution
visible. The provider [documents no API-key requirement](https://openfreemap.org/).
It supplies external visual context, not routing evidence or verified road alignment.
Data freshness may differ from the pinned Roadrunner graph. No bulk/offline tile
cache or scene reproduction guarantee is included. Basemap failure keeps the
canonical overlay and reports unavailable context. WebGL failure keeps route metrics
and evidence available with a map error.

DM Sans and Space Grotesk are bundled through Fontsource under OFL-1.1; their licenses
ship in the respective npm packages. Icons use Lucide (ISC). Original Phase 30 SVG scooters and helmet portraits are authored locally; supplied reference
bitmaps and proprietary navigation assets are not redistributed.

Full alternative comparison, graph/search animation, dispatch/fleet views, scooter
journeys, guidance, physical telemetry, historical playback, streaming and immersive
3D remain deferred. See [Phase 27 completion](../docs/phase-27-completion.md) for
actual verification scope and screenshots.

## Compare alternatives (Phase 28)

Choose **Compare up to 2 / 3 / 5 routes** in Route results, then Find route. Comparison
uses the engine's Yen enumeration with Dijkstra internally; its algorithm control
is fixed accordingly. Single-route mode preserves A*/Dijkstra selection. On the
bundled graph, **0 → 22** and **22 → 0** return three routes for travel-time routing.
No additional server, environment variable or dependency is required.

Route A/B/C are positions in this returned result set. **ENGINE PRIMARY** always
marks rank 1, while the selected card/white map line describe your current view.
Cards show source distance, predicted elapsed road time, objective cost/unit,
algorithm and traffic delay when supported. The difference against the primary
uses objective units. A distance optimum is not labeled fastest. Expand Selection
rules & search limits for overlap/cost policies, actual termination and bounded
work units. Fewer routes and partial budget results never imply complete coverage.

Click a card or an unselected map line to select it. Eye controls hide/show paths;
at least one remains visible. Hiding the selected path selects the first remaining
visible member, and selecting a hidden member shows it. The selected route is solid
white; other routes use gray dash patterns and map labels. Shared sections can
coincide, so cards remain the unambiguous way to select overlapping alternatives.
Collapsed metrics provide A/B/C route switches, including on mobile. The floating
workspace remains scrollable and collapsible; successful comparison scrolls to its
cards. Scroll back up to edit endpoints/objective.

Select a traversal by clicking the white route or expanding **Inspect selected
route segments**. Previous/Next and the traversal-number input step through exact
occurrences, including reversed curves. The map highlights that occurrence's
original point interval. Inspection shows directed edge/segment IDs, direction,
canonical distance, objective weight/unit, base/evaluated road seconds and applicable
traffic factor. These values come from a bounded API export evaluated under the
original static context; they are not search events. Unsupported/older optional
exports show unavailable values. No global graph debugger is included.

Traffic delay is evaluated time minus the **same returned path's** free-flow time,
not a difference between independently optimal paths. Nontraffic objectives show
**Not applied** rather than invented delay. Traffic routing requires an already
adopted backend snapshot. This UI does not adopt or edit traffic.

Changing selection replays that member's reveal; manual replay/pause/skip/speed,
visibility, segment inspection and collapse never rerun routing. Reduced motion
and large-route static fallbacks still apply. Source download contains the **entire
original alternative envelope**, retaining every member and result-set identity.
See [completion/visual QA](../docs/phase-28-completion.md) for evidence and limits.

## Developer graph debugger (Phase 29)

Use **Developer graph** inside SPATIAL WORKSPACE. Toggle nodes, directed edges,
direction arrows and objective weights; select an edge from the map or selector.
Node picks inspect exact E7 coordinates and page-local degree counts. The crosshair
focuses the selected entity, while Fit loaded graph page restores the page view.

Cost selection, page navigation and Refresh capture a new immutable view without
rerunning routing. Traffic-aware cost requires an already adopted engine snapshot;
missing input stays unavailable. Road class and explored-node overlays are unsupported
by the current producer and are labeled accordingly. Existing search summaries can
be inspected under their original routed-result context. Returning to Routing keeps
that original route/alternative result intact.

The debugger uses the same API/proxy/setup; no new service, dependency or migration.
See [controls, semantics and limits](../docs/graph-debugger.md) and
[completion/visual evidence](../docs/phase-29-completion.md). Phase 30 is not included.

## Delivery simulation (Phase 30)

Choose **Delivery simulation** for actual isolated Roadrunner execution results.
The map stays full screen; setup, helmet-portrait rider roster, selected execution,
orders and traffic are separate collapsible floating cards in the routing workspace's
monochrome style. Riders have stable distinct cosmetic palettes shared by their
portraits, overhead scooters and route accents. Collapsed cards retain rider portraits,
with overlapping portraits for the fleet. No sidebar reserves map space.

Run the supplied one-rider, two-rider or delay/suffix preset using explicit graph
nodes (default 0/35/22 on the Phase 10 graph). Play/pause, time selection, event step
and speed sample that finite run; they cannot mutate live authority or complete work.
The selected rider card shows actual assigned orders, onboard scalar load/capacity,
next stops, arrival/service predictions and separate action/generation/plan identities.
Positions are labeled simulated estimates; accepted engine effects establish fulfillment.

Scenario JSON accepts existing synthetic traffic overrides/changes. The Traffic card
styles/inspects the bounded first graph page under the current simulation context,
separately from original frozen action traffic. No physical observations or road-user
density are inferred. Missing/unsupported traffic disables its overlay. Original scene
JSON is downloadable. Read **and simulation** capability are required; the UI does not
send operational lifecycle or traffic-adoption commands.

See the [bounded producer/motion contract](../docs/live-delivery-visualization.md),
[completion and screenshots](../docs/phase-30-completion.md), and
[Phase 30.5 navigation and renderer evidence](../docs/phase-30.5-completion.md).

Reproduce the checked-in scene fixtures with a running local API:

```sh
ROADRUNNER_API_URL=http://127.0.0.1:3110 \
ROADRUNNER_API_TOKEN="$ROADRUNNER_API_TOKEN" python3 scripts/verify_delivery_scene.py
```

Run this command from the repository root. It compares all five actual source artifacts
and checks that namespace, revision, world, execution, plans and publications are unchanged.

Dispatch candidates, original hypothetical routes and cinematic policy presentation are described in [Phase 31](../docs/dispatch-visualization.md).
