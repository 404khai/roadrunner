# ADR 0009: Routing Correctness Gate

Status: Accepted

## Context

The existing example tests cover important cases but Dijkstra and A* share enough
implementation that agreement alone cannot rule out a common defect.

## Decision

The revised routing core must pass an independent, deterministic correctness gate
before OSM ingestion begins.

The gate includes a Bellman–Ford or exhaustive oracle, seeded generated directed
multigraphs, Dijkstra/oracle comparison, A* zero-heuristic and built-in-heuristic
comparison, an independent route validator, stale-heap regressions, parallel
edges, self-loops, zero-cost cycles, disconnection, forbidden traversal,
evaluator errors, numeric failures, capability mismatch, deterministic
finalization permutations, and FIFO arrival propagation.

Failing generated graphs become permanent regression fixtures. CI uses a bounded
fixed corpus; heavier stress runs may remain separate.

`visited_nodes` is replaced by `expanded_states`, meaning the number of
search-state expansion events. Other diagnostics are added only with precise
semantics.

## Consequences

- Routing correctness does not depend solely on two algorithms agreeing.
- The suite runs after the accepted interface refactor, avoiding tests for APIs
  that are immediately removed.
- CI cost increases but remains deterministic and bounded.

## Rejected / deferred alternatives

- Treating the current unit suite as sufficient is rejected.
- Requiring identical paths for all equal-cost optima is rejected unless the
  deterministic tie-break contract specifically requires it.
- A property-testing dependency and large continuous fuzz corpus are optional.

## Relationship to other ADRs

ADRs 0003 and 0004 define capability and heuristic properties under test.
ADR 0010 makes this gate a prerequisite for Phase 7.
