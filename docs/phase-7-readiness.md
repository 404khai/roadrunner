# Phase 7 Readiness Report

Date: 2026-09-19

## Verdict

**Roadrunner is ready for Phase 7.**

Every mandatory pre-Phase-7 gate passes. The items marked `DEFERRED` below are
explicitly assigned to Phase 7, Phase 7.5, or later measurement work and are not
hidden failures. No PBF/OSM ingestion, motorcycle-profile compilation, or
jurisdiction policy has been implemented as part of this refactor.

## Architecture and documentation

| Status | Gate | Evidence |
| --- | --- | --- |
| PASS | Accepted decisions are recorded as a coherent ADR set. | [ADRs 0001-0010](adr/) |
| PASS | Architecture, specification, glossary, and benchmark documentation use the revised terminology and boundaries. | [Architecture](architecture.md), [specification](spec-v1.md), [glossary](glossary.md), [benchmarks](benchmarks.md) |
| PASS | Implementation order was dependency-driven and recorded before production changes. | [Implementation plan](pre-phase-7-implementation-plan.md) |
| PASS | Phase 7.5 is a named maneuver-aware turn-restriction milestone. | [ADR 0008](adr/0008-osm-source-ingestion-boundary.md) |

## Graph and numerical model

| Status | Gate | Evidence |
| --- | --- | --- |
| PASS | Mutable construction is separated from immutable routing through `GraphBuilder::finalize -> FrozenGraph`. | `graph/network.rs`, [ADR 0001](adr/0001-graph-lifecycle-and-snapshot-identity.md) |
| PASS | `NodeId`, `RoadSegmentId`, and `EdgeId` are dense snapshot-local identities; durable route results carry `GraphSnapshotId`. | `graph/ids.rs`, `routing/result.rs` |
| PASS | Physical `RoadSegment` data and canonical geometry are stored separately from directional traversal edges. | `graph/segment.rs`, `graph/edge.rs`, [ADR 0002](adr/0002-road-segments-directed-edges-and-geometry.md) |
| PASS | Outgoing adjacency is contiguous and canonicalized once during deterministic finalization. | `graph/network.rs`, deterministic permutation test in `tests/routing_correctness.rs` |
| PASS | Canonical graph geometry uses validated fixed-point WGS 84 E7 coordinates. | `geo/canonical_coordinate.rs`, [ADR 0006](adr/0006-canonical-coordinate-and-distance-representation.md) |
| PASS | Segment distance is derived from canonical polyline geometry; caller-supplied physical distance is impossible through the public builder. | `graph/network.rs`, `graph/segment.rs` |
| PASS | Free-flow travel time is directional and derived from shared segment distance plus effective directional speed. | `graph/network.rs`, `graph/edge.rs` |
| PASS | Costs and units reject negative and non-finite values, normalize negative zero, use exact relaxation, and detect accumulation overflow. | `cost/route_cost.rs`, `geo/units.rs`, correctness tests |

## Traversal and routing correctness

| Status | Gate | Evidence |
| --- | --- | --- |
| PASS | Traversal evaluation separates feasibility, objective cost, elapsed travel time, and evaluator errors. | `cost/model.rs`, [ADR 0003](adr/0003-traversal-evaluation-and-search-capabilities.md) |
| PASS | Node-state routing accepts `StaticNonNegative` and `FifoEarliestArrival`; expanded-state and unsupported contracts are rejected. | `routing/search.rs`, capability tests |
| PASS | FIFO search propagates arrival time to each outgoing-edge evaluation. | `fifo_evaluation_receives_arrival_time_at_each_node` |
| PASS | Dijkstra uses indexed vectors, stale-entry checks, deterministic heap ordering, and edge-based predecessor reconstruction. | `routing/dijkstra.rs`, `routing/search.rs` |
| PASS | A* takes an explicit heuristic, performs no per-query graph scan, and rejects objective, capability, policy, and snapshot mismatches. | `routing/astar.rs`, `routing/heuristic.rs`, [ADR 0004](adr/0004-astar-heuristic-policy.md) |
| PASS | Unknown/custom evaluators are zero-heuristic-only unless they explicitly declare a stronger named contract. | `TraversalEvaluator::supports_heuristic`, compatibility tests |
| PASS | Distance and travel-time Haversine lower bounds are tested against Dijkstra; the travel-time speed bound is prevalidated against one snapshot. | `built_in_distance_heuristic_matches_dijkstra`, `built_in_travel_time_heuristic_matches_dijkstra` |
| PASS | `visited_nodes` was replaced by the precise `expanded_states` expansion-event metric. | `routing/result.rs`, [glossary](glossary.md) |

## Correctness suite

| Status | Gate | Evidence |
| --- | --- | --- |
| PASS | Seeded directed multigraphs compare Dijkstra with an independent Bellman-Ford oracle. | `seeded_dijkstra_matches_independent_bellman_ford_and_zero_astar` |
| PASS | A* with `ZeroHeuristic` agrees with Dijkstra. | Same seeded oracle test |
| PASS | An independent route validator checks endpoints, edge continuity, objective, elapsed time, and physical distance. | `validate_route` in `tests/routing_correctness.rs` |
| PASS | Regressions cover stale heap entries, parallel edges, self-loops, zero-cost cycles, disconnected graphs, isolated nodes, and source equals destination. | `tests/routing_correctness.rs` |
| PASS | `Forbidden` skips one traversal while evaluator failure aborts with a typed error. | `forbidden_edges_are_skipped_but_evaluator_errors_propagate` |
| PASS | Negative, NaN, infinity, negative zero, and arithmetic overflow behavior is tested. | unit tests plus `invalid_evaluator_values_are_routing_errors` and `objective_overflow_is_an_error` |
| PASS | Unsupported capabilities and unsafe evaluator/heuristic combinations fail before search. | `incompatible_capabilities_and_heuristics_are_rejected` |
| PASS | Insertion permutations produce identical canonical graph artifacts. | `finalization_is_independent_of_insertion_order_and_artifact_round_trips` |
| PASS | A discovered floating-point A* lower-bound regression is preserved permanently and requires exact Dijkstra equivalence. | `astar_geographic_rounding_regression_matches_dijkstra_exactly` |

## Artifact boundary

| Status | Gate | Evidence |
| --- | --- | --- |
| PASS | `FrozenGraph` has no public direct constructor or deserializer; bytes become routable only through validation. | `graph/artifact.rs`, [ADR 0007](adr/0007-graph-artifact-format-and-validation.md) |
| PASS | Loader validates magic, schema, payload hash, build metadata, ranges, offsets, dense references, orientation, ordering, access values, and numeric domains. | `validate_payload` in `graph/artifact.rs` |
| PASS | Artifact build identity includes source identity/integrity, compiler and normalization versions, profile, jurisdiction, and build configuration. | `GraphBuildIdentity`, artifact payload |
| PASS | Artifact publication uses a synchronized temporary file and atomic rename; the published file is validated by test. | `write_graph_artifact_atomic`, `atomic_artifact_publication_produces_a_valid_complete_file` |
| PASS | Corrupt, truncated, and wrongly framed artifacts are rejected. | `corrupt_artifacts_are_rejected` |
| DEFERRED | Deep recomputation of all derived geometry and connectivity facts, decoder fuzzing, binary encoding, mmap, and zero-copy loading. | Explicitly deferred by [ADR 0007](adr/0007-graph-artifact-format-and-validation.md); not required for safe normal loading |

## Benchmark gate

| Status | Gate | Evidence |
| --- | --- | --- |
| PASS | Historical Phase 6 results remain preserved and clearly labeled. | [Benchmark documentation](benchmarks.md), existing `benchmarks/results` artifacts |
| PASS | Revised harness measures builder construction, finalization, serialization, validated loading, adjacency traversal, Dijkstra, and A* separately. | `crates/roadrunner-core/benches/` |
| PASS | Dijkstra and A* use identical snapshots and a seeded five-category query corpus. | `benches/support/mod.rs`, `benches/astar.rs` |
| PASS | Baseline records hardware, compiler, configuration, sample count, median, p95, p99, expanded states, and artifact sizes. | [Structured revised-core baseline](../benchmarks/results/2026-09-19-apple-m3-revised-core.json) |
| PASS | Documentation explicitly rejects real-road scalability conclusions from the synthetic graph. | [Benchmark documentation](benchmarks.md) |
| DEFERRED | Loaded heap memory, routing allocation count, and peak build memory. | Measurement method is not yet controlled; ADR 0010 explicitly makes this non-blocking |

## Tooling gate

| Status | Gate | Evidence |
| --- | --- | --- |
| PASS | `cargo fmt --check` | Passed on 2026-09-19 |
| PASS | `cargo clippy --workspace --all-targets --all-features` | Passed on 2026-09-19 with no warnings |
| PASS | `cargo test --workspace` | Passed on 2026-09-19: 24 unit tests, 15 integration correctness tests, and doc tests |

## Intentionally deferred to Phase 7 or later

| Status | Item | Required milestone |
| --- | --- | --- |
| DEFERRED | PBF parsing and staged extraction into `NormalizedOsmDataset` | Phase 7 |
| DEFERRED | `delivery_motorcycle_v1`, `ng_v1`, OSM access/speed/directionality tag tables, contraction, provenance, component diagnostics, and the first real OSM fixture | Phase 7 |
| DEFERRED | Turn-restriction enforcement and incoming-edge search state | Phase 7.5 |
| DEFERRED | Non-FIFO, mixed-objective multi-label, resource-constrained, and strategic-waiting search | Later routing milestones |
| DEFERRED | Advanced graph compression, unsafe optimization, mmap, databases, services, traffic feeds, dispatch, VRP, ML, and UI | Later roadmap phases, only with evidence |

## Defects found by the gate

The new tests and benchmarks found and fixed three correctness defects before OSM
work began:

1. route distance was reconstructed in reverse accumulation order, producing a
   different floating result from traversal-order validation;
2. artifact integrity was checked after a decode/re-encode transformation rather
   than over the exact canonical payload bytes; and
3. a floating Haversine estimate could microscopically exceed the true stored
   route cost and let A* return an exactly more expensive path. Built-in
   geographic estimates now apply a conservative numerical lower-bound guard,
   and the failing topology is a permanent regression.

The benchmark also shows that the initial canonical JSON artifact is large and
that A* does not reduce expansions on the synthetic forward-banded topology.
Those are measured optimization inputs, not reasons to introduce compression or
specialized layouts before the first real OSM fixture.
