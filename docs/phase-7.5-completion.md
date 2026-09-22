# Phase 7.5 completion

Phase 7.5 is complete for the declared motorcycle-relevant node-via subset.

Implemented behavior:

- resolves one `from` way, one `via` node, and one `to` way against compiled
  directed traversals;
- enforces `no_left_turn`, `no_right_turn`, `no_straight_on`, `no_u_turn`,
  `only_left_turn`, `only_right_turn`, and `only_straight_on`;
- applies `restriction:motorcycle` before generic `restriction` and honors
  motorcycle-relevant `except` values;
- compiles canonical forbidden `(incoming edge, outgoing edge)` pairs after
  one-way and access policy;
- searches `(node, incoming edge)` state in Dijkstra and A* and reconstructs the
  route through expanded predecessors;
- persists and validates maneuver tables in graph schema v3 and source mapping
  in provenance schema v2;
- preserves unsupported, conditional, way-via, malformed, ambiguous, and
  unresolved relations with deterministic statuses; and
- reports `turn_restrictions_enforced: true` in graph metadata and manifests.

Correctness coverage includes `no_*`, `only_*`, motorcycle qualification,
motorcycle exceptions, one-way topology, expanded-state route selection,
Dijkstra/A* agreement, and graph/provenance snapshot round trips.

Deferred forms remain the ones declared by the phase specification: multi-way
via paths, conditional/time-dependent restrictions, and broader vehicle
qualification semantics.
