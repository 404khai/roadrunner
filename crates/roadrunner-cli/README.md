# roadrunner-cli

The `roadrunner` executable: file-loading and output adapters around OSM ingestion,
core routing, dispatch comparison and simulation. It composes the workspace libraries;
it does not contain an HTTP server or replace their algorithms.

See [project setup](../../README.md#setup). All examples run from the **repository root**.

## Build and invoke

```bash
cargo +1.99.0 build -p roadrunner-cli --release --locked
./target/release/roadrunner --help
# Or compile/run directly through Cargo:
cargo +1.99.0 run -p roadrunner-cli --locked -- --help
```

For a storage-constrained checkout, reuse the existing debug target rather than
building another release profile: `CARGO_INCREMENTAL=0 cargo build -p roadrunner-cli --locked`.
This does not create a worktree or add dependencies.

Cargo's package name is `roadrunner-cli`; the binary name is `roadrunner`. No arguments,
`help` and `--help` display the command list. Invalid arguments print an error to stderr
and exit nonzero. Use the explicit `--` between Cargo flags and program arguments.

## Terminal workflows

Help displays block-letter ROADRUNNER branding. New human graph/routing commands
also display it when stdout is a terminal; piped summaries stay compact. Machine
output never includes the banner or ANSI escapes. `--version` / `-V` prints the version.

```bash
roadrunner graph import data/fixtures/phase-7/lagos-marina.osm.pbf \
  /tmp/marina-snapshot --source-id local-marina --json
roadrunner graph stats /tmp/marina-snapshot
roadrunner route --graph /tmp/marina-snapshot \
  --from osm:5602610872 --to osm:5594385916 --alternatives 3 --json
roadrunner route compare --graph /tmp/marina-snapshot \
  --from osm:5602610872 --to osm:5594385916
# Coordinates are latitude,longitude (not GeoJSON longitude,latitude):
roadrunner route --graph /tmp/marina-snapshot \
  --from '6.450,3.390' --to '6.452,3.395' --algorithm astar --json
```

Graph import composes the existing PBF normalization, versioned artifact validation,
motorcycle compilation and atomic bundle publication. It refuses an existing target;
there is no overwrite flag. Normalized data is encoded/decoded in memory to preserve
the existing compiler integrity boundary. It is not a streaming import optimization.
Graph stats and routing load and deeply verify the complete bundle or standalone
`.rr-graph` artifact. Standalone artifacts have no OSM provenance.

New route endpoints accept `node:<snapshot-local-id>`, `osm:<source-id>` (bundle
required), or WGS84 `latitude,longitude`. Coordinates use a linear nearest-node
scan, with node ID as the distance-tie breaker and a default 1,000m maximum.
`--snap-radius-meters <finite-nonnegative-number>` changes that explicit boundary.
The output includes resolved coordinates, IDs, method and snap distance. This is
**node snapping**, not edge snapping or map matching; no connector distance/time is
added, and unreachable nodes are not replaced by farther reachable candidates.

These new routes minimize **motorcycle free-flow travel time**, defaulting to
Dijkstra. `--algorithm astar` uses the existing graph-qualified admissible heuristic
with the graph's maximum effective free-flow speed. `route compare` runs both and
reports their routes, canonical expansions and observed cost delta; it is not a
runtime benchmark or a guarantee of identical paths. It rejects algorithm/alternative
flags. `--alternatives N` uses the existing bounded Dijkstra/Yen producer (default
50,000 search states, 1,000 candidate paths); it preserves termination and truncated
result evidence. Explicit alternative enumeration cannot be combined with A*, including a requested count of one.
Traffic and departure-dependent routing remain explicit legacy commands below.

New graph/routing options can appear in any order after the command/positional
paths; unknown, duplicate and missing options fail. `--json` selects schema-version-1
adapter envelopes embedding existing domain records, not a new portable route format.
New graph/routing errors with `--json` go to stderr as `{"error":"..."}`, with empty stdout and nonzero
exit status. Existing legacy successful JSON formats remain unchanged.

`roadrunner serve [API arguments]` starts a separately built `roadrunner-api`
executable **beside** the CLI, inheriting environment, streams and API configuration.
On Unix the CLI replaces itself so API signals and exit status remain intact.
It neither builds the server nor supplies authorization/trusted-local flags for you.
See [API setup](../roadrunner-api/README.md) for its security and database requirements.

## Complete command reference

| Command | Input / output |
| --- | --- |
| `graph import <input.osm.pbf> <new-snapshot-directory> --source-id <identity> [--json]` | Normalize, validate, compile and atomically publish a new bundle |
| `graph stats <snapshot-directory-or-artifact> [--json]` | Verified identity, capabilities and topology counts |
| `route [compare] --graph <path> --from <endpoint> --to <endpoint> [options] [--json]` | Free-flow route, alternatives or Dijkstra/A* comparison |
| `serve [API arguments]` | Delegate to the sibling API executable |
| `osm extract <input.osm.pbf> <output.rr-osm> --source-id <identity>` | Normalize PBF to a versioned artifact; readable count summary |
| `osm compile <input.rr-osm> <snapshot-directory>` | Compile motorcycle graph/provenance/manifest; atomic directory publication and count summary |
| `graph verify <snapshot-directory> --deep` | Validate bundle and provenance against graph; readable identity/count summary |
| `route corpus <snapshot-directory> <route-corpus.json>` | Resolve source OSM endpoints and route each pair; JSON including reachable/unreachable cases |
| `route alternatives <snapshot-directory> <from-osm-node> <to-osm-node> --count <n>` | Diverse static free-flow routes; JSON with ranks, metrics, overlap, work and termination |
| `route traffic <snapshot-directory> <from-osm-node> <to-osm-node> --scenario <traffic.json>` | Compare distance, free-flow and static-traffic routes; JSON |
| `route schedule <snapshot-directory> <from-osm-node> <to-osm-node> --scenario <profile.json> --depart <seconds>` | Compare free-flow and FIFO time-dependent routes; JSON |
| `simulate <scenario.json> [--json]` | Readable or structured simulation/planning/publication/actual outcome evidence |
| `benchmark dispatch <scenario.json> [--idle-penalty-weight <weight>] [--json]` | Four-policy schema 1 comparison; readable or JSON summaries |

Legacy positional `route corpus/alternatives/traffic/schedule` commands already
emit JSON and do not accept an extra `--json`. Their endpoints are source OSM IDs.
Departure seconds are logical scenario time, not ISO timestamps. Legacy commands
retain positional parsing. Fleet optimization uses schema 3 simulation.

## Offline OSM routing walkthrough

```bash
mkdir -p target/cli-readme
./target/release/roadrunner osm extract \
  data/fixtures/phase-7/lagos-marina.osm.pbf \
  target/cli-readme/marina.rr-osm --source-id readme-cli-marina
./target/release/roadrunner osm compile \
  target/cli-readme/marina.rr-osm target/cli-readme/marina-snapshot
./target/release/roadrunner graph verify target/cli-readme/marina-snapshot --deep
./target/release/roadrunner route corpus \
  target/cli-readme/marina-snapshot data/fixtures/phase-7/route-corpus.json
./target/release/roadrunner route alternatives \
  target/cli-readme/marina-snapshot 5602610872 5594385916 --count 3
```

Choose a new snapshot directory when rerunning compile. A snapshot contains graph,
provenance and manifest, and compiled graph identity is sensitive to source provenance.

For traffic/profile routing, use the published `data/fixtures/phase-10/snapshot.semantic-v2`
and `phase-11/snapshot.semantic-v2` bundles for the pinned traffic fixtures; see the
root README commands. Recompilation on another platform can change compiled float
bits and requires a separately authored overlay.

## Delivery workflows

```bash
./target/release/roadrunner simulate data/fixtures/phase-15/seeded-deliveries.json
./target/release/roadrunner simulate data/fixtures/phase-17/successful-pooling.json --json
./target/release/roadrunner simulate data/fixtures/phase-17/complete-infeasibility.json --json
./target/release/roadrunner benchmark dispatch data/fixtures/phase-16/paired-strategies.json --json
```

Simulation documents contain a graph source plus a versioned scenario. Inline graphs
are self-contained; artifact paths resolve relative to the document and require a graph
digest binding. Schema 1 is legacy single-order behavior. Schema 2 is explicit pooled
admission with named scenario/policies and forecast validity. Schema 3 supports joint fleet batches with a pinned algorithm. See the
[simulation README](../roadrunner-simulation/README.md) for schema, event ordering,
replay commands and output populations.

A completed infeasible search is a valid result with no new assignment. A partial-budget
search cannot publish. Required unavailable predictions appear as typed nonpublication
records while committed execution continues. Invalid input/policy or structural failures
are command errors. Predicted feasibility and realized readiness/traffic errors are
reported separately.

## Tests and API docs

```bash
cargo +1.99.0 test -p roadrunner-cli --all-features --locked
cargo +1.99.0 test -p roadrunner-cli --test traffic_cli --locked
cargo +1.99.0 test -p roadrunner-cli --test simulation_cli --locked
cargo +1.99.0 doc -p roadrunner-cli --no-deps --locked
```

The package is a binary adapter, with no public integration library or Criterion target.
The underlying APIs live in core/OSM/dispatch/simulation. External CLI benchmark collectors
and the optional Docker/OSRM comparison are listed in the
[root benchmark guide](../../README.md#benchmarks-and-replay-evidence).

## Phase 18 fleet batches

Schema 3 `fleet_batch` jointly allocates current new work and resequences healthy editable
suffixes, preserving committed owners, accepted terms and active execution. Read
[fleet contracts](../../docs/fleet-optimization.md) for exact API, isolation and heuristic
coverage. Run the pooled fleet example from the repository root:

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-18/greedy-trap.json --json
```

## Phase 19 dynamic recovery

Schema 4 `dynamic` runs committed recovery before a separate Phase 18 admission
publication. Configure `RecoveryPolicy` version 1 with explicit reroute/stability
penalties, minimum improvement and cooldown. Active execution and custody stay
pinned; unstarted pickup/dropoff pairs may change owner without resetting terms.
Availability, forecast, cancellation and observed road-delay inputs use logical
timestamps; traffic, creation, actual readiness and stop completion also trigger
recovery. Refused cancellation and incomplete recovery preserve mandatory work.

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-19/offline-recovery.json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-19/budget-exhaustion.json --json
cargo +1.99.0 test -p roadrunner-dispatch --test recovery --locked -- --nocapture
cargo +1.99.0 test -p roadrunner-simulation --test recovery --locked
```

Dispatch APIs: `RecoveryPolicy`, `RecoveryContext`, `recover_fleet`,
`RecoveryDecision`, `RecoveryEvidence`, `RecoveryTermination`, `evaluate_recovery_plan`,
`World::commit_recovery`, `World::cancel_order`, `CancellationRefusal` and
`FulfillmentState::Cancelled`. Simulation adds `DynamicEvent`, `DynamicChange`,
`DispatchPolicy::Dynamic` and `SimulationRecoveryRecord`.
See [full recovery contracts](../../docs/dynamic-redispatch.md),
[scenario fixtures](../../data/fixtures/phase-19/README.md) and
[completion evidence](../../docs/phase-19-completion.md).

## Offline ETA dataset

`roadrunner dataset build <request.json> <new-directory>` captures verified synthetic
sources, code/lockfile/toolchain and all started legs. `roadrunner dataset verify
<directory>` checks digests, replays retained sources and rebuilds all derived fields.
See [usage and evidence contracts](../../docs/eta-dataset.md).

## Original-B0 baseline

`roadrunner baseline evaluate <dataset-directory> <new-result-directory>` verifies a
frozen Phase 23 dataset and publishes action-paired B0 errors, coverage and segments.
`roadrunner baseline verify <result-directory>` checks/recomputes the self-contained
result. See the [Phase 24 guide](../../docs/eta-baseline.md).
