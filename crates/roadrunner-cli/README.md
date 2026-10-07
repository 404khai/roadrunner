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

Cargo's package name is `roadrunner-cli`; the binary name is `roadrunner`. No arguments,
`help` and `--help` display the command list. Invalid arguments print an error to stderr
and exit nonzero. Use the explicit `--` between Cargo flags and program arguments.

## Complete command reference

| Command | Input / output |
| --- | --- |
| `osm extract <input.osm.pbf> <output.rr-osm> --source-id <identity>` | Normalize PBF to a versioned artifact; readable count summary |
| `osm compile <input.rr-osm> <snapshot-directory>` | Compile motorcycle graph/provenance/manifest; atomic directory publication and count summary |
| `graph verify <snapshot-directory> --deep` | Validate bundle and provenance against graph; readable identity/count summary |
| `route corpus <snapshot-directory> <route-corpus.json>` | Resolve source OSM endpoints and route each pair; JSON including reachable/unreachable cases |
| `route alternatives <snapshot-directory> <from-osm-node> <to-osm-node> --count <n>` | Diverse static free-flow routes; JSON with ranks, metrics, overlap, work and termination |
| `route traffic <snapshot-directory> <from-osm-node> <to-osm-node> --scenario <traffic.json>` | Compare distance, free-flow and static-traffic routes; JSON |
| `route schedule <snapshot-directory> <from-osm-node> <to-osm-node> --scenario <profile.json> --depart <seconds>` | Compare free-flow and FIFO time-dependent routes; JSON |
| `simulate <scenario.json> [--json]` | Readable or structured simulation/planning/publication/actual outcome evidence |
| `benchmark dispatch <scenario.json> [--idle-penalty-weight <weight>] [--json]` | Four-policy schema 1 comparison; readable or JSON summaries |

Route commands already emit JSON; they do not accept an extra `--json`. Routing endpoints
are source **OSM node identities**, not local graph IDs or geographic coordinate strings.
Departure seconds are logical scenario time, not ISO timestamps. CLI parsing uses the
listed positional order. Planned `serve`, generic `route --from/--to`, `graph stats`,
interactive map and fleet-optimization commands do not exist yet.

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

For traffic/profile routing, first build snapshots using the source IDs pinned by their
scenario digests: `phase10-lagos-marina` for the static fixture and
`phase11-lagos-marina` for the time-dependent fixture. The exact extraction/compilation
commands and runnable routes are in the [root walkthrough](../../README.md#static-traffic),
[traffic guide](../../docs/traffic.md) and [profile guide](../../docs/time-dependent-routing.md).
Using a generic snapshot here with those bound overlays produces a digest mismatch.

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
admission with named scenario/policies and forecast validity. See the
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
