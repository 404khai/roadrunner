# Phase 7 remediation report

Date: 2026-09-21

## Verdict

**Phase 7 COMPLETE.** All eleven mandatory remediation gates pass. Roadrunner's
OSM pipeline now preserves enough routing-source semantics, compiled provenance,
topology, geometry, deterministic identity, and artifact integrity to begin a
separate Phase 7.5 review. Phase 7.5 and Phase 8 were not implemented. Every
published snapshot still declares `turn_restrictions_enforced: false`.

## Eleven-item gate

### 1. Profile-independent normalization — PASS

`osm_normalization_v2` retains routing node semantics, ferry connectors,
ordered identities, relevant unsupported values, and independent generic,
vehicle-qualified, and conditional restriction values. Profile precedence is
applied only by `policy.rs`. Evidence: the source-semantics PBF and pipeline
tests. This is a versioned routing-source schema, not an OSM mirror.

### 2. Contextual access — PASS

The graph preserves destination, delivery, customer, private, permit, and
unknown-explicit reasons. Built-in evaluators allow general access and deny
restricted classes by default; authorization is reason-specific. Route tests
prove a cheaper restricted shortcut is skipped without aborting search.
Endpoint-region authorization remains conservatively unsupported.

### 3. Source-to-graph provenance — PASS

Canonical `GraphProvenance` maps OSM nodes to routing nodes and OSM ways to
ordered segments, forward/reverse edges, and actual compiled access/speed.
Restriction records classify resolvable, ambiguous, unresolved, and unsupported
member shapes. Loading validates IDs, ordering, traversal attributes, complete
segment coverage, and exact snapshot binding.

### 4. Hardened snapshot boundary — PASS

`graph.rr-graph`, `graph.rr-provenance`, and `manifest.json` form one atomically
published directory. Loading validates canonical bytes, coordinate domains,
IDs, unique traversals, segment/edge correspondence, contiguous geometry, CSR
adjacency, metadata, hashes, capabilities, and provenance. Publication and
audit tests run the deep verifier. Malformed regressions cover invalid
coordinates, duplicate traversals, geometry gaps, noncanonical framing,
integrity failures, and corrupted derived distance.

### 5. Explainable contraction and components — PASS

The small graph contracts 160 shape nodes while preserving 58 routing nodes for
explicit semantic reasons. Its five WCCs (`46,6,2,2,2`) are disconnected in the
retained source topology. The engineering WCCs (`601,17,10,9,5,4,2,2,2`) are
also attributed: components 0–5 are separated by `barrier=gate` policy on ways
`370566988`, `441900898`, `441900912`, `586376687`, `587183011`, and
`1064266457`; components 6–8 are source-disconnected. No component is pruned.

### 6. Conservative directionality — PASS

The compiler distinguishes bidirectional, forward-only, reverse-only,
unsupported-dynamic, contradictory, and unknown-explicit states. Unsupported
semantics never invent connectivity. Original way order controls `oneway=-1`,
motorcycle overrides precede general tags, and circular-junction policy is
explicit. Provenance explains every emitted direction; unit and route tests
cover reverse travel, overrides, and one-way behavior.

### 7. Explainable free-flow policy — PASS

Legal limit, class default, profile maximum, physical cap, effective speed, and
free-flow time are distinct. Missing and unsupported values remain diagnostic.
The small graph records 64 unpaved-capped directional decisions, two
paved-normal, and four without physical tags. The engineering graph records 426
unpaved-capped, two ground-capped, 46 paved/asphalt-normal, and 238 without
physical tags. Calibration remains Phase 8/later work.

### 8. Geometry invariants — PASS

OSM compilation quarantines ordinary zero-distance candidates. Finalization and
deep verification enforce E7 ranges, endpoint orientation, Haversine polyline
distance, one central endpoint lower-bound tolerance, positive OSM distance,
reciprocal segment sharing, and derived free-flow time. Coordinate equality
never merges distinct source identities.

### 9. Semantic snapshot identity and determinism — PASS

The authoritative SHA-256 digest binds normalized source, semantic versions and
configuration, graph/provenance semantics, and capabilities. The 64-bit ID is
derived convenience. Round-trip, insertion-order, repeated-build, and
semantic-change tests establish stable output and changed identity when
semantics change. The compiler is currently single-threaded; worker-count
equivalence is required before parallel compilation ships.

### 10. Real compile–serialize–load–route corpus — PASS

`route-corpus.json` runs short, medium, long, same-origin, cross-component,
one-way, parallel-road, and surface cases. Dijkstra and admissible A* run on
fresh and loaded graphs. Independent validation replays continuity, orientation,
geometry, distance, objective, elapsed time, capabilities, and snapshot identity.

### 11. Statistics and lifecycle evidence — PASS

The manifest records extraction retention, split/contraction reasons, policy
decisions, restrictions, geometry anomalies, WCC/SCC distributions, and causal
component evidence. Both pinned real fixtures are measured. Structured evidence
is in `benchmarks/results/2026-09-21-apple-m3-phase-7-remediation.json`.

## Real graph statistics

| Metric | Small correctness | Engineering |
| --- | ---: | ---: |
| PBF bytes | 18,932 | 157,489 |
| source nodes/ways seen | 2,505 / 469 | 22,215 / 4,552 |
| candidate ways | 37 | 384 |
| retained nodes/ways | 218 / 37 | 2,280 / 384 |
| routing nodes | 58 | 652 |
| contracted shape nodes | 160 | 1,628 |
| RoadSegments / DirectedEdges | 60 / 114 | 780 / 1,470 |
| geometry points | 272 | 3,095 |
| WCC / SCC | 5 / 10 | 9 / 22 |
| normalized bytes | 40,288 | 417,613 |
| graph / provenance / manifest bytes | 51,724 / 49,601 / 4,526 | 638,995 / 560,408 / 9,473 |

Neither real fixture contains a restriction relation. Restriction preservation
and resolution readiness are proved by `source-semantics.osm.pbf`, not inferred
from a zero count.

## Lifecycle and benchmark assessment

Apple M3 / 16 GB, macOS 27.0, Rust 1.96 release build:

- engineering extraction: 0.02 s wall, 7,077,888-byte max RSS;
- engineering compile/publication: 0.05 s wall, 9,125,888-byte max RSS;
- engineering load/deep verify: 0.02 s wall, 7,143,424-byte max RSS;
- Criterion extraction median 10.76 ms and compilation median 18.67 ms;
- graph validation/load median 5.63 ms and deep verify median 98.49 µs;
- deterministic long-route Dijkstra median 30.63 µs and A* 72.59 µs.

This is an engineering baseline, not a target. Extraction memory scales with
retained intermediate state, so no bounded-memory claim is made. Nothing here
justifies speculative graph-layout, integer-width, geometry, or mmap changes.

## Artifact and restriction assessment

Small digest:
`e133ec6094521c92c5554612e69396442f63d18c1854f4f6f997b0ff777a684c`.

Engineering digest:
`5f78a554f33be49897e0fb68d085c2a6d83c599da01fe168f2e40d7c854f9b5e`.

Both bundles load through mandatory validation and pass deep verification.
Phase 7.5 can consume the validated graph, manifest, and provenance without
replaying parsing or undocumented compiler ordering.

## Known unsupported/deferred semantics

- multi-way via paths and via-way enforcement;
- conditional/time-dependent restrictions and complex vehicle qualification;
- endpoint-aware destination/delivery/customer access regions;
- dynamic/reversible direction schedules;
- unsupported explicit physical values beyond conservative handling;
- production-scale or formally bounded-memory ingestion evidence;
- reference-engine comparison (Phase 8).

## Quality gates

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo +1.85.0 check --workspace --all-targets
cargo bench -p roadrunner-osm --bench osm_pipeline
roadrunner graph verify <small-snapshot> --deep
roadrunner graph verify <engineering-snapshot> --deep
```

All commands pass.

## Exact Phase 7.5 entry gate

Consume validated restriction provenance; resolve supported node-via
`no_*`/`only_*` pairs to incoming/outgoing edges; expand search state by incoming
traversal; enforce and reconstruct deterministically; retain unsupported forms
diagnostically; and set `turn_restrictions_enforced: true` only when the declared
supported subset genuinely affects routing.
