# Pre-Phase-7 Implementation Plan

Status: Accepted implementation sequence
Last updated: 2026-09-19

This plan implements the accepted ADRs without beginning PBF parsing, OSM source
extraction, `delivery_motorcycle_v1`, or `ng_v1` policy work.

## 1. Domain foundations

1. Add canonical E7 WGS 84 coordinates and checked conversions.
2. Add snapshot, segment, orientation, access, and capability domain types.
3. Preserve validated `f64` units for distance, duration, and accumulated cost.

These types unblock graph finalization and artifact schemas without introducing
OSM-specific types.

## 2. Builder and frozen graph

1. Replace the mutable routing graph with `GraphBuilder` construction records.
2. Derive segment distance from canonical geometry.
3. Deterministically order nodes, segments, directed edges, and adjacency.
4. Assign dense snapshot-local IDs during finalization.
5. Expose immutable indexed lookup and outgoing-edge slices from `FrozenGraph`.

Synthetic fixtures migrate to the builder as part of this step. No parser is
introduced.

## 3. Traversal and search contracts

1. Replace scalar `CostModel` evaluation with `TraversalEvaluator`.
2. Add `TraversalEvaluation::{Traversable, Forbidden}` and separate errors.
3. Add static and FIFO earliest-arrival search capabilities.
4. Track objective cost and elapsed travel time in search labels.
5. Reject incompatible evaluator/search combinations.

## 4. Explicit heuristic policy

1. Add `Heuristic`, `ZeroHeuristic`, distance Haversine, and maximum-speed
   travel-time Haversine implementations.
2. Validate objective/capability compatibility before search.
3. Remove graph-wide per-query heuristic derivation.

## 5. Route migration and diagnostics

1. Migrate Dijkstra, A*, predecessor reconstruction, and route totals to
   `FrozenGraph` and directed segment edges.
2. Rename `visited_nodes` to `expanded_states`.
3. Add elapsed travel time and snapshot identity to durable route output.
4. Retain exact finite `f64` relaxation and deterministic tie-breaking.

## 6. Artifact boundary

1. Define a canonical versioned artifact DTO distinct from `FrozenGraph`.
2. Serialize canonical payloads deterministically.
3. Validate schema, integrity, offsets, references, order, numeric values,
   capabilities, and manifest agreement before producing `FrozenGraph`.
4. Add malformed-artifact tests. Atomic filesystem publication is adapter work;
   core supplies bytes plus validation.

## 7. Correctness gate

1. Add an independent Bellman–Ford oracle and route validator.
2. Add seeded generated multigraph comparison tests.
3. Add targeted stale-entry, parallel-edge, self-loop, zero-cycle, forbidden,
   evaluator-error, numeric, capability, deterministic-finalization, and FIFO
   regressions.
4. Preserve any discovered failure as a named fixture.

## 8. Revised benchmark baseline

1. Separate builder finalization, artifact encode/decode-validation, and routing
   benchmarks.
2. Use fixed seeded query corpora and identical snapshots for algorithm comparison.
3. Record `expanded_states`, artifact size, and latency distributions.
4. Preserve Phase 2–6 results as historical artifacts.

## 9. Phase 7 readiness report

Evaluate every mandatory gate in `docs/phase-7-readiness.md` as `PASS`, `FAIL`, or
`DEFERRED`, with evidence links. Stop after the report; do not begin OSM ingestion.

## Validation cadence

After each logical implementation step:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

Temporary breakage is allowed only for an indivisible API migration and must be
resolved before the next step.
