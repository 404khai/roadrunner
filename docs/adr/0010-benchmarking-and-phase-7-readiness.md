# ADR 0010: Benchmarking and Phase 7 Readiness

Status: Accepted

## Context

Current deterministic synthetic benchmarks are useful historical evidence, but
their ring/backbone topologies and A* graph-wide preparation do not establish OSM
scalability.

## Decision

Before Phase 7, the revised core records a reproducible synthetic baseline without
an arbitrary latency or memory target. Construction, finalization, serialization,
load/validation, and routing are measured separately where practical.

Routing benchmarks use seeded query corpora and identical snapshots for Dijkstra
and A*. They report latency distributions and `expanded_states`; artifact size,
loaded memory, and allocations are recorded where practical. Structured results
are generated automatically where feasible. Existing results remain historical.

During Phase 7, a small versioned real OSM fixture supplies the first evidence for
layout and performance decisions. No OSM-scale claim is made from synthetic data.

Phase 7 begins only after the revised architecture, correctness gate, formatting,
Clippy, tests, and revised synthetic baseline pass. Phase 7 does not begin
automatically after the gate; its readiness report is reviewed first.

## Consequences

- Measurements detect regressions without inventing service objectives.
- Optimization waits for representative topology.
- Lack of a practical allocator profiler does not alone block Phase 7 when the
  limitation is documented.

## Rejected / deferred alternatives

- Pre-OSM latency targets and scalability claims are rejected.
- Deleting or overwriting historical benchmark results is rejected.
- Unsafe Rust, mmap, custom allocation, SIMD, and advanced compression are
  deferred until measurements justify them.

## Relationship to other ADRs

ADR 0009 supplies the correctness prerequisite. ADR 0007 supplies artifact load
work. ADR 0008 defines the later real-road fixture boundary.
