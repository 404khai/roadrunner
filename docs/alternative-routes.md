# Alternative routes (Phase 9)

Roadrunner enumerates loopless routes in increasing objective cost, using [Yen's path-deviation method](https://pubsonline.informs.org/doi/abs/10.1287/mnsc.17.11.712). For each ranked path, it fixes every possible root prefix in turn, blocks the next edge of previously ranked paths with that prefix, and searches for the cheapest permitted spur. A candidate heap ranks the resulting complete paths. Edge-ID sequences distinguish parallel traversals.

The spur search carries the root's incoming edge and checks the compiled maneuver table on every transition. It also excludes nodes already in the root or spur, so results have no repeated routing nodes. The current shortest-simple-path search expands complete path states by non-negative cost. This is exact within the declared static cost model and search budget, but can use substantial memory on dense graphs. A later measured optimization may replace the search strategy without changing the public result contract.

## Selection rules

The shortest loopless route is always rank 1. Further routes are selected only when they satisfy both rules:

1. **Cost:** candidate cost is at most `max_cost_factor × primary cost` (default `1.5`). The objective comes from the chosen evaluator; the CLI uses free-flow travel time.
2. **Diversity:** for every already selected route, shared physical segment length divided by the shorter route's length is at most `max_shared_distance_ratio` (default `0.8`). Segment IDs, rather than edge IDs, prevent a reversed traversal of the same physical road from appearing distinct. The output records the maximum overlap for each selected route.

A rejected similar path is still ranked and expanded. This matters because a useful later deviation may branch from it. The enumerator stops at the requested route count, when paths are exhausted, when the next cheapest candidate exceeds the cost limit, or when a work budget is reached. `termination` reports that reason; `truncated` is true only for a budget stop. Returning fewer routes is valid. The request options and ranked-path/search-state counts are included in the response.

The defaults limit expansion to 50,000 path states, also cap a spur-search queue at that size, and retain at most 1,000 complete candidate paths. If the initial route cannot be established within the state budget, the call returns `AlternativeSearchLimit` rather than treating the destination as unreachable. The capability currently requires `StaticNonNegative`; time-dependent objectives belong to a later phase.

## Use on an OSM snapshot

Compile a source snapshot as documented in [OSM ingestion](osm-ingestion.md), then request source-node routes:

```sh
cargo run -q -p roadrunner-cli -- route alternatives \
  <snapshot-directory> 5602610872 5594385916 --count 3
```

The CLI validates the snapshot and resolves the source OSM node IDs through its provenance. It emits JSON with rank, node and directed-edge IDs, distance, cost, travel time, overlap, graph snapshot identity, search work, options, and termination reason. The pinned Lagos Marina fixture returns two diverse routes for this pair under the defaults. The HTTP endpoint shown in the project plan belongs to Phase 20; the routing result is independent of transport.

## Validation

Deterministic tests cover primary cost agreement with Dijkstra, a nearly identical detour rejected before a later distinct corridor is found, one-way topology, turn restrictions, same-node routing, unreachable destinations, invalid options, and search budget failure. The OSM pipeline test exercises the committed PBF and checks that the returned routes are legal and diverse.
