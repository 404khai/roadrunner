# Landing page and field guide

The root page follows the Roadrunner Paper landing design inspected through Paper
MCP on 2026-10-17: Space Grotesk display typography, #FAFAF6 paper, #30332F ink,
#66765B sage, three engine layers, a dark order journey, and a documentation CTA.
Source: https://app.paper.design/file/01M4KT9SCTWHZDSJ4GZM24V87Y/p-1-0.
The supplied local logo is copied into the public assets; its original stays untouched.

## Surfaces

- `/`: landing page with delivery, dispatch and route replay tabs, evidence summaries,
  scrubber, automatic playback with play/pause/replay, workspace links and illustrated bento sections.
- `/docs`: getting started, routing, dispatch, simulation, evidence limits and searchable
  API operations with complete generated request/response references and schema dictionary.
- `/workspace`: existing routing/graph workspace. `?mode=delivery` and `?mode=dispatch`
  open the corresponding operational workspaces and connect to the real API.

The site and docs permit document scrolling. The workspace retains its full-viewport
map and independent floating cards. Mini maps use cooperative gestures so ordinary
wheel/touch scrolling can reach the surrounding page.

## Engine evidence

The public demo payloads are exact copies, not marketing animations or invented GPS:

| Public file | Source | Meaning |
| --- | --- | --- |
| `demos/catalog.json` | `web/tests/catalog.json` | Graph-bound Lagos anchors |
| `demos/delivery.json` | `web/tests/delivery-distributed.json` | Six-rider deterministic simulation |
| `demos/dispatch.json` | `docs/evidence/phase-31/spatial.json` | Isolated spatial dispatch evaluation |
| `demos/route.json` | `web/tests/forward.json` | A* free-flow original directed route |
| `demos/route-request.json` | `web/tests/forward-request.json` | Exact route request |

The existing decoders validate schema, identity and geometry before display. Delivery
uses the existing `frameAt` and `sampleRider` renderer, retaining causal delay and accepted
execution authority. Dispatch reveal is presentation order; riders do not move to candidate
positions. Routing reveal is a path prefix, not a vehicle. No new routing, traffic,
dispatch, execution or ambient vehicle semantics are introduced.

Each tab mounts one map. Payload loading is abortable, failures expose retry, and
playback begins automatically on load and tab selection except under reduced motion. The display clock stops at the end and does not accumulate hidden-tab
elapsed time. Reduced motion leaves manual scrubbing available. Evidence summaries
remain textual. Basemap tiles require external network access and WebGL. The delivery
mini-view can display neutral imported graph geometry independently of basemap tiles.

Below zoom 18, overhead 2D now uses the same helmet location tag as perspective/3D.
Only marker presentation changes; rider marker, ActionId, coordinates and route remain
unchanged. The regression test checks identity and position through zoom out and in.

## API and asset maintenance

Run from the repository root:

```sh
python3 scripts/export_web_reference.py
python3 scripts/export_web_reference.py --check
```

The script copies exact retained engine evidence and exports OpenAPI through
`cargo run -q -p roadrunner-api -- --openapi`. It does not need a token or listener.
The reference currently contains 30 paths and 40 schemas. Read each operation's
security, required parameters, request body and responses, then resolve `$ref` in the
schema dictionary. `/swagger-ui/` on the running API provides interactive execution.
The guides distinguish visualization outputs from operational envelopes, explain bearer
capabilities, errors, explicit graph anchors, traffic context and execution authority.

For deployment, host the Vite output with history fallback for `/docs` and `/workspace`,
serve `/demos/*` and `/docs/*` as static files, and proxy `/v1` server-side to the API.
`vite preview` is suitable for local built-output inspection. Never expose the API token
as a `VITE_*` variable. Deployment files already present in the checkout are outside this PR.

## Verification

T3 preview was tried first. Two image snapshot calls returned exactly:
`Preview automation snapshot timed out after 15000ms.` Verification therefore used
local headless Chrome through temporary Playwright tooling outside the repository.
This fallback does not establish acceptance of earlier Phase 31 mobile work.

Frontend tests cover real payload decoding, scrubbing, tab switching without API
mutation, reduced motion, missing-evidence retry, generated API filtering, and overhead
portrait identity. Existing renderer/evidence tests remain applicable. Browser captures
and the final verification results are retained under `docs/evidence/landing-page`.
Guide screenshots in `web/public/docs/{delivery,dispatch,routing}.png` are actual running workspaces. The bento uses the user-supplied immersive, perspective dispatch and alternatives screenshots from `references`, copied as `engine-*.png`; these are reference captures, not new browser-verification evidence.

Final checks: 82/82 frontend tests in 21 files passed using one worker and a 15-second
per-test timeout on this busy host; initial parallel runs hit existing interaction-test
timeouts. Lint, format, production build and public-reference drift check passed.
The build retains Vite's large-chunk advisory for the MapLibre/3D workspace bundle.
Fresh anchors, delivery, dispatch and route producer requests returned 200. Browser
checks found no JavaScript exceptions, no horizontal overflow at 390px in landing/docs,
and confirmed 600px of normal wheel scrolling. Reduced-motion playback was disabled.
See [desktop](evidence/landing-page/desktop.png), [mobile](evidence/landing-page/mobile.png),
[docs desktop](evidence/landing-page/docs-desktop.png), and
[docs mobile](evidence/landing-page/docs-mobile.png). This is UI/contract verification,
not a load test or a claim of physical device telemetry.

The delivery mini-view exposes overhead, perspective 2.5D and immersive 3D via the existing renderer. Scene controls expose follow, buildings, route reveal and explicitly labeled synthetic ambient actors. Camera mode changes do not reset the evidence clock or rider identity.

User-requested follow-up verified automatic playback and paused-time preservation when switching the mini-view to perspective and immersive modes. See [mini perspective](evidence/landing-page/mini-perspective.png), [mini 3D](evidence/landing-page/mini-3d.png), and [mobile 3D](evidence/landing-page/mini-3d-mobile.png). Delivery playback spans the captured event interval through its final retained frame, rather than padding the demonstration with the scenario’s idle configured horizon.

Every shell snippet, API operation and schema code block includes a copy control and syntax coloring. Copying preserves exact source bytes; success and clipboard-denial states are announced. Browser verification confirmed exact clipboard content and no mobile horizontal overflow. See [mobile copy controls](evidence/landing-page/docs-copy-mobile.png). React text rendering keeps example markup inert.

Navigation follow-up: a sticky site header provides an accessible mobile hamburger dropdown (Escape, outside-pointer and link-selection dismissal). Landing hover links have no underline; GitHub actions use the filled GitHub mark. Routing-workspace launchers now use dark text and icons on visible light button surfaces. Browser checks confirm sticky positioning, menu dismissal, no mobile overflow and delivery-workspace entry. See [mobile navigation](evidence/landing-page/mobile-menu.png), [workspace buttons](evidence/landing-page/workspace-buttons.png), and [sticky header and GitHub CTA](evidence/landing-page/sticky-header-github.png). Bento captures are matched to their actual scene: immersive fleet, tilted dispatch and alternative routes.

## Hosted route entry points

The frontend is a Vite single-page application. Vercel must serve `index.html` for
`/docs` and `/workspace` (including trailing-slash entry points); otherwise direct
navigation and refresh return a host-level 404 before React runs. `web/vercel.json`
now retains the existing hosted `/v1/*` API proxy and adds explicit rewrites for
those four entry paths. The rules deliberately do not rewrite `/docs/*` assets:
OpenAPI JSON and guide images remain static files with their original content types.
The Vercel project uses repository root directory `web`.
