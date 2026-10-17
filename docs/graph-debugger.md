# Developer graph debugger

Phase 29 adds a read-only graph inspector to the existing routing workspace. The
map still fills the viewport; **Routing / Developer graph** switch within the
collapsible SPATIAL WORKSPACE card. No additional service, package or migration is
required. Use the [existing setup commands](../web/README.md#run).

## Use

1. Connect the engine using the plug control, then select **Developer graph**.
2. Toggle nodes, directed edges, direction arrows and objective-weight labels.
3. Pick an edge from the selector or its offset line on the map. Pick a node to
   inspect exact coordinates and page-local incoming/outgoing counts.
4. Use the crosshair control to focus the selected entity; the adjacent fit control
   restores the loaded page. Selecting entities and changing layers do not recenter
   the camera. Opening/collapsing the workspace adjusts framing for its footprint.
5. Choose free-flow time, distance or traffic-aware time for standalone edge costs.
   **Refresh**, entering developer mode, cost changes and paging capture a new view;
   they do not rerun routing. Pages replace the capture, rather than accumulating
   edges from different traffic contexts. Previous/Next retains exact cursor history.
6. Expand capture evidence or download its original JSON. Switch back to Routing
   to recover the unchanged original route/alternative selection and source evidence.

Both directions are drawn separately with a small **screen-space** lateral offset.
Local arrow graphics follow the directed geometry, including reversed curves. This
is presentation placement, not evidence of physical lanes or motorcycle motion.
Canonical integer coordinates remain unchanged in the source/download. Crossing
lines do not establish connectivity; actual node IDs and directed endpoints do.
Coincident parallel edges can be ambiguous to pick; the selector identifies every
exported directed edge unambiguously. Weight labels are rounded and retain units;
inspect/download the original numeric fields for precision.

## Meaning of costs and unavailable fields

Each exported edge is evaluated independently using Roadrunner's actual static
DistanceCost, TravelTimeCost or TrafficAwareCost, at elapsed time zero and the
captured server departure/clock, with default private/permit access false. An
edge's `traversable` status does **not** establish a legal incoming maneuver, turn
or complete route. A forbidden traversal has null evaluated cost/time, not zero.
Its compiled access class, physical distance and base time remain inspectable.

An adopted multiplier is a graph-bound snapshot fact, even when inspecting a
nontraffic objective. The evaluator applies that factor only for traffic-aware
costs; its `traffic_digest` is null for distance/free-flow costs. With no adopted
snapshot, traffic multiplier and traffic-aware cost are explicitly unavailable;
geometry, base time, distance and free-flow evaluation still work. This UI does
not adopt/edit traffic, mutate operations, or claim live values. A capture stays
pinned until explicitly replaced. The independently retained route keeps its own
original context; graph refresh cannot rewrite it.

**Unsupported data requirement:** frozen graph edges and their compiled provenance
do not retain road class. The inspector reports it unavailable; it never substitutes
basemap classifications or guesses from default speed. Enabling a true class layer
requires graph-qualified source enrichment with verified mapping/provenance. No
routing artifact, graph digest or compiler policy is changed to fill this gap.
This limitation is established by the approved [audit §10](phase-26-source/audit.md#10-navigation-and-maneuver-boundaries).

Explored-node membership and ordered search events are also absent. The optional
explored-node overlay is disabled with an explanation. The last routed result's
actual algorithm, state mode, exact expanded-state count and counter semantics are
available separately. Counts are not unique nodes or a reconstructible frontier.
A*/Dijkstra include the goal in accepted non-stale pop counts and retain their
source-equals-destination convention. Yen retains its root/spur path-pop semantics.
A selected path never becomes an invented search trace. See [audit §16–17](phase-26-source/audit.md#16-dijkstraa-diagnostic-semantics).

## HTTP contract

`GET /v1/visualization/graph?graph_digest=<full-sha256>&node_offset=0&edge_offset=0&objective=travel_time`

Uses the existing graph/routing **read** capability and persistent-writer health
gate. Default objective is travel_time; distance and traffic_aware are also supported.
Unknown parameters, invalid objectives and out-of-range offsets reject. Full graph
identity is checked before dense IDs resolve. This is a new inspection representation;
existing route/alternative endpoints, graph artifacts and optimizer behavior are unchanged.

Version 1 carries:

- `semantics: bounded_directed_graph_inspection/v1`, full graph metadata and global
  node/edge counts, independent node/edge `{offset,end,next_offset}` ranges (end exclusive).
- Up to **500 node-page entries**, plus exact endpoints of exported edges; the
  deduplicated ordered `nodes` array has at most **900** entries. Each carries
  `node` and exact `coordinate_e7`. Isolated nodes remain accessible via node pages.
- Up to **200 directed edges** and **20,000 total directed geometry vertices**.
  Each edge carries ID, physical segment, from/to, orientation, exact ordered
  `points_e7`, canonical distance meters, base seconds, adopted traffic multiplier,
  compiled access, nullable road class, cost status, objective cost and elapsed road seconds.
- `evaluation`: objective/unit, evaluator identity, captured departure/clock,
  access context, applicable traffic digest, and the explicit scope
  `standalone_directed_traversal_default_access;not_maneuver_or_route_feasibility`.
- Adopted `traffic_snapshot_digest` separately from whether that overlay affects
  the chosen objective. Availability flags state that road class, explored-node
  membership and ordered search traces are not retained.
- `graph_view_id: graph-view-v1:<sha256>` hashes compact serde JSON of the complete
  returned value **before adding this identity**. It identifies a page/capture,
  not a route, committed plan, mutable operational snapshot or graph replacement.

Pages follow dense graph order, not spatial proximity. Geometry budgets stop only
between edges; the next cursor begins at the next complete edge. A single edge
exceeding the vertex budget returns **413 graph_geometry_limit**, never a clipped
road. Empty tails are valid. Serialized output is capped at **4 MiB**; byte-limit
failure returns 413. Existing nonblocking worker/export admission returns **503
visualization_busy** under saturation; slow/dropped bodies retain/release their
export permits. Geometry/evaluation/serialization runs on a blocking worker.
Traffic and departure are captured together under the coordinator lock; existing
traffic-snapshot cloning behavior is retained. No graph-independent latency or
memory claim is made for large adopted snapshots. There is no operational mutation,
route recording, durable migration, subscription or search instrumentation.

The client checks required versions/capabilities, graph/request/page identity,
complete node coverage, directed endpoint/geometry consistency, budgets, units,
static evaluation parity, unavailable/forbidden nullability and traffic applicability.
Malformed or unsupported evidence is an explicit error, with stale captures cleared.
Canonical points are not simplified or reversed a second time. Existing conservative
polar/antimeridian presentation limits apply. Basemap content remains external context.

## Verification and scope

See [Phase 29 completion](phase-29-completion.md) and its retained producer/client/
browser evidence. Output bounds are not a large-network rendering benchmark. WebGL,
external contextual tiles/fonts and the existing MapLibre bundle remain dependencies.

Phase 30 rider/delivery movement, fleet rendering, dispatch animation, historical
seeking, immersive environments and ordered graph-search animation are not included.
