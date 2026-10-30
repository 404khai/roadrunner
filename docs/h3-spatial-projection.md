# H3 geographic dispatch projection

Roadrunner can use H3 to analyze rider supply and pickup demand at consistent
geographic resolutions. This implementation adds a read-only projection in
`roadrunner-dispatch::spatial`, consuming an existing coherent `DispatchSnapshot`.
It does not change dispatch selection, routing costs, execution, or persistence.
The routing workspace also provides a browser H3 inspection overlay and matched
Dijkstra/A* requests for checking the different roles interactively.

## Why H3 fits

Uber's [H3 introduction](https://www.uber.com/ng/en/blog/h3/) describes bucketing
marketplace events into geographic cells for analysis and optimization. The
[H3 project](https://github.com/uber/h3) provides global cell identities and a
hierarchy suitable for joining geographic datasets. In Roadrunner, this is useful
for inspecting where unassigned pickups and eligible idle riders are located,
and for grouping the same snapshot at several levels of detail.

The existing unit-sphere tree already provides exact radius screening and a
linear correctness oracle. Replacing it with H3 has no demonstrated benefit yet.
H3 grid distance measures cell hops, not road distance or travel time, and
[grid traversal has pentagon-related limitations](https://h3geo.org/docs/api/traversal/).
Roadrunner's manually implemented routing and routed dispatch scoring remain the
authority for reachability, maneuvers, traffic, deadlines, and plan feasibility.

## Implemented contract

`H3Resolution` validates integer resolutions 0 through 15. The caller selects the
resolution explicitly; the example uses 9, without claiming it is optimal for Lagos
or any fleet. `H3Cell::at` indexes the original WGS 84 coordinate. `H3Cell` parses
validated cell identifiers and serializes them as hexadecimal strings, avoiding
JavaScript integer precision loss. H3 includes pentagons; callers must not assume
every cell is hexagonal.

`H3DispatchProjection::from_snapshot` retains the source world's caller-scoped
identity, world version, and evaluation instant. These identify the local source;
they are not a runtime namespace, routing provenance, or an authorization/commit
token. The output owns its data and cannot change when the world changes later.
Rebuild from a new coherent snapshot to reflect changes. It reads authoritative
coordinates, never visual interpolation, synthetic ambient actors, or cell centers.

Each occupied cell retains two sorted lists:

- `idle_riders`: riders satisfying the existing Basic Dispatch eligibility rule
  (available, no active assignment, no custody, empty remaining plan).
- `unassigned_orders`: awaiting-pickup orders without committed responsibility,
  located at their pickup coordinate. Assigned, picked-up, cancelled, and delivered
  orders are excluded. Unready orders remain demand.

List lengths give rider and order counts. A rider count is not available parcel
capacity; an order count is not parcel demand units. Eligibility does not imply
that a particular rider can serve an order: routing profile, capacity, deadlines,
readiness, and route feasibility still need evaluation. Riders with ongoing work
that might accept pooled orders are outside this idle-supply metric. No shortage
ratio, ready-food count, price, or rebalancing instruction is inferred. Empty cells
are omitted, including for an empty snapshot. Every included entity appears exactly
once; cells and their member IDs are deterministically sorted.

`coarsen` groups existing cells by logical H3 parent and preserves source metadata
and entity identities. Equal-resolution aggregation is an identity operation;
finer resolutions are rejected even for empty projections. As documented by
[H3's indexing guide](https://h3geo.org/docs/highlights/indexing/), logical parent
containment is exact while geographic containment is approximate. Consequently,
coarsening an existing projection can differ from re-indexing original coordinates
directly at the coarser resolution. Use `from_snapshot` again for the latter.

The dependency is the pure Rust H3 implementation `h3o`, with no native C build or
service dependency. Version 0.8.0 is pinned for the workspace's Rust 1.85 baseline;
the reviewed 0.11 source uses newer standard-library integer operations. Only the
dispatch crate depends on it; the routing core remains independent. Public
Roadrunner wrappers keep H3O types out of the API. H3O is BSD-3-Clause licensed;
Uber's reference H3 implementation is Apache-2.0 licensed.

## Run and inspect

```bash
cargo run --locked -p roadrunner-dispatch --example basic_dispatch
cargo test --locked -p roadrunner-dispatch --test h3_projection
```

The synthetic example emits `h3_before_dispatch` and `h3_logical_rollup` alongside
the existing exhaustive/spatial decisions and accepted execution result. The
projection is taken before assignment and never participates in the decision.
It is an engine-produced analysis artifact, not a live-location feed or HTTP API.

Given an existing `snapshot`, a caller can use:

```rust,ignore
use roadrunner_dispatch::spatial::{H3DispatchProjection, H3Resolution};

let grid = H3DispatchProjection::from_snapshot(&snapshot, H3Resolution::try_from(9)?)?;
for cell in grid.cells() {
    println!("{}: {} idle riders, {} unassigned pickups",
        cell.cell, cell.idle_riders.len(), cell.unassigned_orders.len());
}
let district_view = grid.coarsen(H3Resolution::try_from(7)?)?;
```

Tests cover the official H3 quick-start coordinate/cell pair, validated wire
representations, all supported resolutions, poles, antimeridian equivalence,
pentagon identity, deterministic ordering, source preservation, logical rollup
conservation/composition, empty snapshots, assignment, availability, cancellation,
custody, completion, and projection isolation from later world transitions.

## Browser inspection

Start the API and web client using [the web run guide](../web/README.md#run), then
open `/workspace?h3=1`. Select origin and destination and find a single route.
The **H3 inspection** button toggles the overlay; **Cell resolution** changes
the direct coordinate indexing from resolution 6 through 12. Pale cells contain
loaded graph anchors; gold cells contain vertices of the selected route geometry.
The member list retains node IDs and vertex sample counts. This browser overlay
uses pinned `h3-js` 4.4.0; both adapters test the official quick-start cell ID.

These are graph-node and geometry-vertex buckets, distinct from the Rust dispatch
projection's idle-rider/pickup metrics. They do not represent demand, traffic,
road coverage, or all cells crossed between consecutive geometry vertices. The
overlay uses the loaded catalog page (at most 500 anchors), renders at most 1,000
occupied cells, and explicitly reports withholding polar/seam polygons or truncation.
Changing resolution never makes a new route request or changes route evidence.

**Compare Dijkstra and A*** runs both real engine searches with the same anchors,
objective, and a pinned departure time. Original per-search heuristic provenance
is retained; all other routing provenance, graph/profile and jurisdiction must
match before the pair is displayed. Controls switch the selected result on the
map without recomputing it. Metrics are engine-reported expanded search states,
road distance/time and optimal objective cost. Tied optimal paths may differ.
These individual requests are not a runtime benchmark. H3 has no shortest-path
cost or expanded-search-state metric and is not compared as a third route algorithm.
Changes to endpoints, graph, objective, source evaluation instant, or credentials
clear the comparison and cancel in-flight requests. Empty/error states remain explicit.

Browser tests cover adapter/reference parity, conservation across resolutions,
geometry immutability, unsupported polygons, actual route-envelope decoding,
heuristic differences, map result switching and rejection of changed context.

## Follow-on integration gates

An H3 candidate policy would need a conservative geographic cover followed by
the existing exact Haversine inclusion and distance/ID ordering. A fixed number
of cell rings inferred from average edge size is not proof of radius coverage.
Validate globally against the linear oracle, especially near cell boundaries,
pentagons, poles, and the antimeridian. Retain `PotentiallyIncomplete` evidence
for bounded shortlists and routed plan evaluation after screening.

Benchmark construction, memory, updates, and queries against both current
locators at the existing 100/1K/10K/100K rider populations, across sparse,
clustered, and global cases with multiple radii and resolutions. Record hardware,
dataset, graph size where applicable, configuration, runs, median, p95, and p99.
This PR makes no speed, memory, or dispatch-quality claim. Projection construction
reuses the existing eligibility helper, including its per-rider assignment scans;
it is not an incremental or latency-qualified fleet index.

A future dispatch map layer can consume the Rust cell IDs and counts, derive
boundaries with an H3-compatible renderer, and retain source and metric labels. Snapshot/replay
and HTTP export need their existing namespace and evidence contracts before being
exposed. Any future traffic aggregation must use retained graph-qualified facts;
geographic grouping cannot invent vehicle density or observed traffic. Rebalancing,
service zones, persisted cell columns, and ML features require separate semantics
and evaluation. These follow-on semantics are not implemented by this projection.
