# Roadrunner routing workspace

Phase 27's TypeScript / React / MapLibre GL client consumes the real version-1
portable route endpoint. This is a local engineering interface. No hosted deployment
or fleet visualization is included.

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
ship in the respective npm packages. Icons use Lucide (ISC). No supplied reference
image, proprietary navigation asset or scooter asset is redistributed.

Full alternative comparison, graph/search animation, dispatch/fleet views, scooter
journeys, guidance, physical telemetry, historical playback, streaming and immersive
3D remain deferred. See [Phase 27 completion](../docs/phase-27-completion.md) for
actual verification scope and screenshots.
