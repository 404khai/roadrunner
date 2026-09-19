# ADR 0004: A* Heuristic Policy

Status: Accepted

## Context

The Phase 6 A* implementation scans all edges per query and derives a minimum
cost-per-geodesic-meter ratio from the active cost model. This is expensive,
evaluates irrelevant edges, and is not a valid general contract for dynamic or
custom objectives.

## Decision

Heuristic policy is explicit and independent from traversal evaluation. A* only
runs a heuristic whose compatibility and lower-bound assumptions are known.

- Distance routing may use Haversine when compiled segment length is never below
  endpoint geodesic distance within the documented validation tolerance.
- Travel-time routing may use Haversine divided by a documented maximum possible
  speed for the immutable graph/profile configuration.
- A base lower bound remains valid when an objective adds only non-negative
  penalties.
- Unknown or unproved combinations use `ZeroHeuristic`.
- Incompatible evaluator, search, and heuristic capabilities are rejected.
- Custom evaluators are compatible only with `ZeroHeuristic` unless they
  explicitly declare a stronger named lower-bound contract. Travel-time
  heuristic parameters are validated once against, and bound to, a graph
  snapshot.
- Built-in floating-point geographic estimates are reduced by a fixed,
  documented relative numerical guard before entering A*. This protects the lower-bound
  contract from numerical overshoot without introducing tolerance into queue
  ordering or shortest-path relaxation.

Heuristic parameters belong to the graph snapshot or immutable routing profile;
they are not obtained by scanning and evaluating the graph per request. Dynamic
overlays may reuse a heuristic only when they cannot violate its lower bound.

## Consequences

- A* correctness is conservative and auditable.
- Custom objectives receive Dijkstra-equivalent zero guidance by default.
- Query latency excludes graph-wide heuristic preparation.
- Stronger heuristics require an explicit proof and tests.

## Rejected / deferred alternatives

- Automatically inferring a heuristic from arbitrary edge costs is rejected.
- Selecting a heuristic merely because it is faster is rejected.
- A complex type-level compatibility framework is deferred; a clear runtime
  capability check is sufficient initially.

## Relationship to other ADRs

ADR 0003 defines search capabilities. ADR 0006 establishes the geometric lower
bound. ADR 0009 requires A*/Dijkstra equivalence testing.
