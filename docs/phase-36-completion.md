# Phase 36 — CLI workflows and terminal branding

Implemented on `feat/cli-workflows-and-banner`, directly in the main checkout from
`57dbcb61`. No worktree or new dependency was introduced.

## Delivered behavior

- Block-letter ROADRUNNER help and terminal-only branding for new human summaries;
  no banner or ANSI escapes in machine output.
- `--version`, `-V` and `-h`, preserving the existing command families.
- `graph import`: existing PBF extraction, normalization artifact integrity validation,
  motorcycle compilation and atomic snapshot publication; refuses existing targets.
- `graph stats`: deeply verified bundle or standalone artifact identity, capabilities,
  node/segment/directed-edge counts and provenance availability.
- `route --graph … --from … --to …`: Dijkstra or A*, with WGS84 coordinates,
  graph-local node IDs or bundle-qualified source OSM IDs. Coordinates resolve to the
  nearest node by Haversine distance, ties by node ID, within an explicit radius
  (default 1,000m). No fabricated connectors or implicit reachable-node substitution.
- `route --alternatives N`: existing bounded Dijkstra/Yen enumeration with unchanged
  diversity, work limits and termination evidence. A* is rejected for explicit alternative
  enumeration rather than mislabeling the producer.
- `route compare`: both free-flow algorithms, their actual paths/directed edges,
  travel costs, canonical expansion counts and observed cost delta.
- New graph/routing adapters provide readable summaries and schema-version-1 JSON
  envelopes. Their JSON errors go to stderr with a failure status and no success
  stdout. Existing command success payloads and text-error conventions remain intact.
- `serve`: forwards arguments/environment/streams to a separately built sibling
  `roadrunner-api`. Unix uses process replacement, preserving API exit/signals.
  The launcher does not build the API or alter its authorization defaults.
- Existing simulation and dispatch benchmark commands remain available in human and
  JSON forms; traffic and time-dependent routing retain their explicit commands.

See the [CLI reference](../crates/roadrunner-cli/README.md) for invocations and contracts.

## Correctness boundaries and limits

Adapters call existing core producers. They do not change graph identities, directed
maneuvers, evaluator policies, routing tie-breaking, optimizer completeness, simulation
ordering, operational authority, durability or historical replay.

The new generic route objective is motorcycle **free-flow travel time**, not traffic
or departure-dependent evaluation. Nearest-node resolution is a linear scan, not edge
snapping/map matching. Import intentionally encodes/decodes normalized data in memory
at the existing integrity boundary; it is not a reduced-memory import pipeline.
Each new command loads/verifies its graph independently. No caches, concurrency
changes, performance claims, default graph discovery or interactive map were added.
Comparison is algorithm evidence, not a latency benchmark. Embedded core route records
are existing runtime records, not a new frozen portable route artifact.

`serve` requires the API executable beside the CLI. Its non-Unix fallback waits for
the API and reports a nonzero status as CLI failure; exact exit/signal forwarding was
verified only for Unix process replacement. Launcher tests use a disposable shell
stub; no new server/database build or live durable workload was run for this change.

## Verification

On macOS arm64, Rust 1.96.0, using the existing target directory:

```bash
cargo fmt --package roadrunner-cli -- --check
CARGO_INCREMENTAL=0 cargo clippy -p roadrunner-cli --all-targets --locked -- -D warnings
CARGO_INCREMENTAL=0 cargo test -p roadrunner-cli --locked
```

All 22 CLI integration tests passed. Six new workflow tests cover actual PBF import,
deep verification, overwrite refusal, standalone provenance boundaries, Dijkstra/A*
comparison on the pinned 58-node graph, alternative termination evidence, exact-node
coordinate resolution, source-equals-destination expansions, distant-coordinate
rejection, strict flags, machine errors, human output and API delegation.
The focused workflow suite was rerun after final human-summary formatting changes.
Help and successful human route branding were also inspected through a real PTY.

No full workspace/release/web build was needed for these CLI adapters. Incremental
compilation was disabled, existing artifacts were reused and free disk remained
approximately 1.8 GiB after verification. This is storage accounting for this checkout,
not a performance result. User deployment/API/web changes were excluded from the commit.
