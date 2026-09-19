# ADR 0003: Traversal Evaluation and Search Capabilities

Status: Accepted

## Context

`CostModel::edge_cost(edge, context) -> RouteCost` cannot distinguish an
unavailable edge from an evaluation failure, propagate arrival time, or separate
optimization cost from elapsed travel time. One best label per node is also not
correct for arbitrary time-dependent or multi-objective problems.

## Decision

Routing evaluates traversal rather than requesting a scalar weight:

```text
Traversable { objective_cost, travel_time }
Forbidden
```

Evaluation errors remain separate. The evaluator receives immutable request
context, the current elapsed/arrival time, and the candidate edge. Its boundary
must allow later maneuver context without requiring it today.

Search labels track objective cost and elapsed travel time. Algorithms and
evaluators declare compatible capabilities:

- `StaticNonNegative`: non-negative additive scalar objective; one best label per
  node.
- `FifoEarliestArrival`: arrival time is the objective and every edge arrival
  function is non-decreasing; one earliest-arrival label per state.
- evaluators requiring expanded state or multi-label dominance are rejected by
  the current algorithms.

Static categorical access prohibitions may remove edges. Contextual access and
dynamic closures return `Forbidden`. Exact finite costs use strict ordering;
epsilon comparisons are prohibited in relaxation and heap ordering.

## Consequences

- Traffic and time-dependent costs can use actual arrival time.
- Distance, elapsed time, and composite objective values are not conflated.
- Unsupported search problems fail explicitly rather than returning plausible
  but unproved routes.
- Static distance and travel-time evaluators remain simple.

## Rejected / deferred alternatives

- Treating `Forbidden` as an error is rejected.
- Arbitrary time-dependent optimization with one label per node is rejected.
- Non-FIFO routing, strategic waiting, multi-label search, resource constraints,
  and maneuver-aware state are deferred.

## Relationship to other ADRs

ADR 0004 constrains heuristics by capability. ADR 0009 defines correctness tests.
Phase 7.5 will add incoming-edge state for turn restrictions.
