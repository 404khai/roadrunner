# Phase 7 completion report

Date: 2026-09-19

## Verdict

Phase 7 is implemented within the boundary established by ADR 0008. The OSM
adapter produces a deterministic profile-independent source artifact and compiles
it through the first versioned motorcycle/jurisdiction policy into the existing
validated frozen graph.

## Deliverables

| Status | Deliverable | Evidence |
| --- | --- | --- |
| PASS | Staged `.osm.pbf` extraction | Two-pass `extract_pbf` in `roadrunner-osm` |
| PASS | Profile-independent `NormalizedOsmDataset` | Versioned model and independently framed artifact |
| PASS | Required nodes, ordered way references, relevant tags, unsupported values, restrictions, split points, and provenance | `model.rs`, extraction tests, artifact validation |
| PASS | Exact standard-OSM E7 source coordinates | Finer-than-E7 input is rejected rather than rounded |
| PASS | Deterministic canonical payload and integrity validation | SHA-256 envelope, canonical re-encoding check, corruption tests |
| PASS | `delivery_motorcycle_v1` plus `ng_v1` | Documented access, speed, road-class, and directionality tables |
| PASS | Topology contraction and directed edge generation | Endpoints, shared source nodes, and restriction-via nodes remain split points |
| PASS | Reproducible build identity and manifest | Source/normalized/graph hashes, versions, configuration, counts, components |
| PASS | All weak components retained with diagnostics | Deterministic component sizes in the build manifest |
| PASS | First versioned real OSM fixture | Provenance-pinned Lagos Marina PBF under `data/fixtures/phase-7/` |
| PASS | Reproducible real-fixture benchmark | `roadrunner-osm/benches/osm_pipeline.rs` and structured result snapshot |
| PASS | CLI artifact boundaries | `roadrunner osm extract` and `roadrunner osm compile` |
| PASS | Rust 1.85 compatibility | Workspace check passes on the declared minimum toolchain |

The real fixture normalizes 218 referenced nodes and 37 candidate ways. The
initial profile compiles 57 routing nodes, 58 physical segments, and 112 directed
edges across six retained weak components. These counts are pinned by the
end-to-end test.

## Quality gates

The following commands pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked
cargo test --workspace --locked
cargo +1.85.0 check --workspace --all-targets
cargo bench -p roadrunner-osm --bench osm_pipeline --locked
```

## Explicit boundary

Turn-restriction relations and their members are retained, and node-via members
affect graph splitting. They are not enforced. The graph metadata and build
manifest both state `turn_restrictions_enforced: false`.

This phase does not add incoming-edge search state, maneuver enforcement,
conditional policy evaluation, reference-engine validation, route comparison,
topology repair, full OSM tag coverage, or Phase 8 benchmark comparisons. Those
items remain assigned to Phase 7.5 or later phases.
