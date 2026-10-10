# Roadrunner

Roadrunner is a Rust routing, dispatch, and delivery simulation engine for last-mile
logistics. It builds road networks from OpenStreetMap, computes legal routes with its
own algorithms, assigns orders to riders, and runs reproducible delivery simulations.

The project focuses on the optimization infrastructure a delivery platform would place
on top of a mapping system: who should carry an order, when it can be picked up, how
traffic changes the route, and whether another delivery fits an existing rider plan.

## Current capabilities

Implemented through **Phase 19 — Dynamic Re-dispatch**:

- Deterministic graph construction, geographic units, Haversine distance, validated
  graph artifacts, and provenance-addressed OSM ingestion.
- Manually implemented Dijkstra and A*, maneuver-aware routing for the supported
  node-via turn restriction subset, and diverse alternative routes.
- Distance/free-flow/static-traffic costs and FIFO time-dependent traffic profiles.
- Linear and indexed nearby-rider lookup, basic and preparation-aware assignment,
  and four comparable single-order dispatch strategies.
- Whole-plan multi-order insertion with capacity checks at every stop, explicit
  readiness/service timing, hard admission policies, immutable acceptance protections,
  frozen execution preservation, and atomic publication.
- Joint fleet batch allocation, editable suffix resequencing, fixed committed owners,
  deterministic greedy/multi-start/local search, and all-or-nothing fleet publication.
- Explicit dynamic recovery of unstarted committed orders, immutable custody/frozen
  execution, churn penalties/cooldown, pre-pickup cancellation and observed road delays.
- Seeded discrete-event delivery simulation, deterministic replay, CLI workflows,
  correctness oracles, and reproducible benchmark artifacts.

[Phase 17 completion](docs/phase-17-completion.md), [Phase 18 completion](docs/phase-18-completion.md)
and [Phase 19 completion](docs/phase-19-completion.md) record implementation gates. Phase 20 adds an [HTTP API with Swagger UI](crates/roadrunner-api/README.md). Databases,
Redis/Kafka, live traffic, machine learning, and a map UI are future phases. The current
workspace runs locally without those services; there is no `serve` or `demo` command.

## Workspace

| Crate | Role | Entry points |
| --- | --- | --- |
| [roadrunner-api](crates/roadrunner-api/README.md) | Authorized HTTP commands, routing, isolated simulation, OpenAPI and Swagger UI | `roadrunner-api` binary and embeddable Axum router |
| [roadrunner-core](crates/roadrunner-core/README.md) | Graphs, geography, costs and routing | Rust library, tests, four Criterion targets |
| [roadrunner-osm](crates/roadrunner-osm/README.md) | OSM extraction, motorcycle graph compilation, provenance and snapshot validation | Rust library, CLI adapters, pipeline benchmark |
| [roadrunner-dispatch](crates/roadrunner-dispatch/README.md) | Rider lookup, world/plan state, timing, assignment and insertion | Rust library, two examples, tests, lookup benchmark |
| [roadrunner-simulation](crates/roadrunner-simulation/README.md) | Logical clock, event queue, actual observations, execution and metrics | Rust library and CLI scenarios |
| [roadrunner-cli](crates/roadrunner-cli/README.md) | File loading and command-line adapters | `roadrunner` binary |

Core does not depend on delivery orders or rider plans. OSM compiles source data into
core graphs. Dispatch owns planning and domain transitions; simulation drives those
transitions with logical time. The CLI composes the libraries.

## Setup

Requirements:

- Git, [rustup](https://rustup.rs/), and your platform's native linker/build tools.
- Rust **1.99.0** for the reproducible commands below. `rust-toolchain.toml` selects
  `stable` for unqualified commands; the explicit version avoids a locally older stable.
- Python **3.10+** for benchmark collectors and reference tooling.
- Docker with a running daemon only for the optional OSRM comparison.
- Python `osmium` only when generating/converting PBF fixtures; committed PBFs need no
  Python OSM package to load or route.

Shell examples use POSIX paths (on Windows, use a suitable environment such as WSL).
Run every command in this README and the crate READMEs from the repository root:

```bash
git clone https://github.com/404khai/roadrunner.git
cd roadrunner
rustup toolchain install 1.99.0 --profile minimal --component clippy --component rustfmt
cargo +1.99.0 build --workspace --locked
cargo +1.99.0 test --workspace --all-features --locked
```

The first Cargo build downloads dependencies. Fixtures and tests use committed data;
no production mapping, traffic, or delivery service credentials are required.

## Quick start: deliveries and pooled orders

Run the self-contained single-order simulation, then a pooled delivery scenario:

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-15/seeded-deliveries.json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-17/successful-pooling.json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-17/successful-pooling.json --json
```

Readable output contains delivery populations, waiting, utilization and distance; pooled
runs also show insertion coverage, rejections and selected plans. JSON distinguishes
predicted plans, successful publication, actual execution and realized policy violations.
Schema 3 enables joint fleet batches; schema 1 preserves legacy single-order semantics; schema 2 requires a scenario identity
and explicit per-order admission policies. See [simulation](docs/simulation.md) and
[multi-order contracts](docs/multi-order.md) before authoring a scenario.

Compare nearest rider, pickup ETA, completion time and preparation-aware dispatch on
identical generated inputs:

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- benchmark dispatch data/fixtures/phase-16/paired-strategies.json
cargo +1.99.0 run -p roadrunner-cli --locked -- benchmark dispatch data/fixtures/phase-16/paired-strategies.json --idle-penalty-weight 1 --json
```

The four-policy comparison is a schema 1 workflow, separate from Phase 17 pooling.
[Phase 17 fixtures](data/fixtures/phase-17/README.md) also exercise capacity, hard deadlines,
cumulative delay, unavailable forecasts, budget exhaustion, and frozen wait/service.


Jointly allocate a fleet batch and compare against greedy/local baselines:

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-18/greedy-trap.json --json
cargo +1.99.0 test -p roadrunner-dispatch --test fleet --locked -- --nocapture > target/phase18-oracle.log
PYTHONDONTWRITEBYTECODE=1 python3 scripts/collect_fleet_benchmark.py --oracle-log target/phase18-oracle.log --output target/readme-results/fleet.json
```

The collector needs the release CLI built below. [Fleet contracts](docs/fleet-optimization.md)
define heuristic/local coverage and isolation; [Phase 18 fixtures](data/fixtures/phase-18/README.md)
cover joint publication, greedy traps and rejection. No global VRP optimum is claimed.

## Build the CLI once

```bash
cargo +1.99.0 build -p roadrunner-cli --release --locked
./target/release/roadrunner --help
```

Use this binary for the remaining CLI commands. If you set `CARGO_TARGET_DIR`, replace
`./target/release/roadrunner` and collector `--binary` paths with your actual target path.
The binary name is `roadrunner`, while the Cargo package is `roadrunner-cli`.

## Import OSM, validate a snapshot, and route

Create an ignored local output directory. Snapshot publication requires an output path
that does not already exist; use a new directory name when repeating a compilation.

```bash
mkdir -p target/readme-data
./target/release/roadrunner osm extract \
  data/fixtures/phase-7/lagos-marina.osm.pbf \
  target/readme-data/marina.rr-osm --source-id readme-lagos-marina
./target/release/roadrunner osm compile \
  target/readme-data/marina.rr-osm target/readme-data/marina-snapshot
./target/release/roadrunner graph verify target/readme-data/marina-snapshot --deep
./target/release/roadrunner route corpus \
  target/readme-data/marina-snapshot data/fixtures/phase-7/route-corpus.json
./target/release/roadrunner route alternatives \
  target/readme-data/marina-snapshot 5602610872 5594385916 --count 3
```

Route arguments are **source OSM node IDs**, resolved through graph provenance; they
are not latitude/longitude strings or snapshot-local `NodeId`s. Corpus and alternatives
emit JSON directly without `--json`. There is no generic `route --from ...` command yet.
The compiler uses the supported motorcycle/jurisdiction policy; unsupported OSM restriction
forms remain diagnosable. [OSM ingestion](docs/osm-ingestion.md) describes the supported
subset and snapshot contents; [alternatives](docs/alternative-routes.md) explains diversity
and search-budget termination.

### Static traffic

Use the published bundle paired with the pinned overlay. Recompiling the PBF on
another platform can change floating-point distance bits and therefore the digest.

```bash
./target/release/roadrunner graph verify data/fixtures/phase-10/snapshot.semantic-v2 --deep
./target/release/roadrunner route traffic \
  data/fixtures/phase-10/snapshot.semantic-v2 5602610872 5594385916 \
  --scenario data/fixtures/phase-10/lagos-marina-severe.semantic-v2.json
```

This compares shortest distance, free-flow fastest, and traffic-adjusted fastest routes.
The overlay affects directed edges independently. See [traffic policies](docs/traffic.md).

### Time-dependent traffic

Use the separately published snapshot pinned by the profile:

```bash
./target/release/roadrunner graph verify data/fixtures/phase-11/snapshot.semantic-v2 --deep
./target/release/roadrunner route schedule \
  data/fixtures/phase-11/snapshot.semantic-v2 5602610872 5594385916 \
  --scenario data/fixtures/phase-11/lagos-marina-profile.semantic-v2.json --depart 0
./target/release/roadrunner route schedule \
  data/fixtures/phase-11/snapshot.semantic-v2 5602610872 5594385916 \
  --scenario data/fixtures/phase-11/lagos-marina-profile.semantic-v2.json --depart 600
```

Departure values are logical seconds since the declared scenario epoch. Profiles must
satisfy FIFO; traffic is sampled at entry to each edge. See [time-dependent routing](docs/time-dependent-routing.md).

## Dispatch library examples

These examples construct synthetic roads and validated worlds, evaluate assignments,
and demonstrate shared fulfillment transitions or predicted-versus-actual preparation:

```bash
cargo +1.99.0 run -p roadrunner-dispatch --example basic_dispatch --locked
cargo +1.99.0 run -p roadrunner-dispatch --example preparation_aware_dispatch --locked
```

For a Rust API integration, start with the [dispatch README](crates/roadrunner-dispatch/README.md)
and [API contracts](docs/api.md). Libraries are workspace path dependencies, not published
crates or standalone services.

## Tests, linting, and API documentation

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.99.0 test --workspace --all-features --locked
cargo +1.99.0 doc --workspace --no-deps --locked
```

`cargo fmt --all` applies formatting. Generated API docs start at
`target/doc/roadrunner_core/index.html`. To run a focused package or test suite:

```bash
cargo +1.99.0 test -p roadrunner-core --test routing_correctness --locked
cargo +1.99.0 test -p roadrunner-osm --test pipeline --locked
cargo +1.99.0 test -p roadrunner-dispatch --test pooling --locked
cargo +1.99.0 test -p roadrunner-simulation --test pooling --locked
cargo +1.99.0 test -p roadrunner-cli --test simulation_cli --locked
python3 -m unittest discover -s benchmarks/reference-comparison -p 'test_*.py'
```

The reference comparison's Python tests do not start Docker. Rust tests include
hand-worked routing/timing cases, independent tiny insertion oracles, seeded invariants,
CLI validation and canonical replay; tests do not rely on live OSM downloads.

## Benchmarks and replay evidence

Criterion targets (full measurements may take minutes):

```bash
cargo +1.99.0 bench -p roadrunner-core --bench graph_traversal --locked
cargo +1.99.0 bench -p roadrunner-core --bench graph_lifecycle --locked
cargo +1.99.0 bench -p roadrunner-core --bench dijkstra --locked
cargo +1.99.0 bench -p roadrunner-core --bench astar --locked
cargo +1.99.0 bench -p roadrunner-osm --bench osm_pipeline --locked
cargo +1.99.0 bench -p roadrunner-dispatch --bench rider_lookup --locked -- --sample-size 30 --measurement-time 1
```

Criterion writes to `target/criterion`. Add `-- --test` to a specific benchmark command
for one-pass smoke validation rather than a statistical measurement. Build the release CLI
first for the process-level collectors, then write fresh evidence outside committed results:

```bash
mkdir -p target/readme-results
python3 scripts/collect_simulation_benchmark.py --output target/readme-results/simulation.json
python3 scripts/collect_dispatch_strategy_benchmark.py --output target/readme-results/dispatch.json
python3 scripts/collect_multi_order_benchmark.py --output target/readme-results/multi-order.json
RUSTUP_TOOLCHAIN=1.99.0 python3 scripts/collect_rider_lookup_benchmark.py --output target/readme-results/rider-lookup.json
```

The rider collector requires a completed measured `rider_lookup` Criterion run; smoke
runs do not create measurement samples. Process collectors default to
`target/release/roadrunner`, have `--binary`/`--runs` options, and verify byte-identical replay.
The multi-order collector also checks coverage and nonpublication of incomplete searches.
The rider collector reads its compiler identity from the active rustup toolchain;
`RUSTUP_TOOLCHAIN=1.99.0` keeps that metadata aligned with the benchmark build.
Benchmark measurements are meaningful only with their hardware, dataset/hash, graph size,
build/configuration, run count and raw samples. Some artifacts explicitly leave memory
unmeasured. Consult [benchmark methodology](docs/benchmarks.md),
[benchmark inventory](benchmarks/README.md), and the [Phase 17 report](docs/phase-17-completion.md).

### Optional OSRM comparison

With a running Docker daemon and network access to pull the pinned image:

```bash
python3 benchmarks/reference-comparison/run.py --output target/readme-results/osrm.json
```

The script builds Roadrunner snapshots, extracts/contracts the same PBF with OSRM,
starts a temporary localhost server, queries the route corpus, and stops the container.
See [reference comparison](benchmarks/reference-comparison/README.md) for profile differences
and limitations. OSRM is a validation reference; Roadrunner's routing remains its own.

## Fixture maintenance and historical tooling

Normal operation uses committed PBFs. To generate or convert a fixture, use an optional
Python environment (the conversion helper overwrites its output path):

```bash
python3 -m venv target/osm-tools-venv
target/osm-tools-venv/bin/python -m pip install osmium
target/osm-tools-venv/bin/python scripts/generate_phase7_semantics_fixture.py target/readme-data/source-semantics.osm.pbf
# Replace the input with your existing OSM XML file:
target/osm-tools-venv/bin/python scripts/convert_osm_to_pbf.py /path/to/input.osm target/readme-data/converted.osm.pbf
```

`scripts/collect_phase7_benchmark.py` is a historical report assembler, **not a fresh
hardware/memory measurement collector**: it embeds the old machine/compiler/date and
memory values. It reads Criterion samples and prebuilt `small`/`engineering` artifacts.
Its invocation is `python3 scripts/collect_phase7_benchmark.py <snapshot-root> <output.json>`;
see the [OSM crate README](crates/roadrunner-osm/README.md) for required filenames. Do not
publish its embedded historical metadata as measurements from a new run.

## Documentation and development

- [Specification](docs/spec-v1.md), [architecture](docs/architecture.md), [glossary](docs/glossary.md).
- [OSM ingestion](docs/osm-ingestion.md), [traffic](docs/traffic.md),
  [time-dependent routing](docs/time-dependent-routing.md), [alternative routes](docs/alternative-routes.md).
- [Rider index](docs/rider-spatial-index.md), [dispatch](docs/dispatch.md),
  [simulation](docs/simulation.md), [dispatch comparisons](docs/dispatch-strategy-benchmarks.md).
- [Multi-order contracts](docs/multi-order.md), [normative audit](docs/adr/0016-multi-order-normative-audit.md),
  [API index](docs/api.md), [architectural decisions](docs/adr/).
- [Data fixture provenance](data/fixtures/phase-7/README.md) and [benchmark artifacts](benchmarks/results/).
- [AGENTS.md](AGENTS.md) defines phase boundaries, correctness, benchmark discipline and contribution rules.

Keep changes scoped, implement core algorithms visibly, establish correctness before
optimization, and record reproducible evidence for quantitative claims. OpenStreetMap
fixture data is © OpenStreetMap contributors under ODbL 1.0; source provenance lives with
the fixtures. Cargo packages currently set `publish = false`; no project-wide license
file is present in this checkout.

## Dynamic recovery (Phase 19)

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-19/offline-recovery.json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-19/cancellation.json --json
cargo +1.99.0 test -p roadrunner-dispatch --test recovery --locked -- --nocapture > target/phase19-oracle.log
cargo +1.99.0 build -p roadrunner-cli --release --locked
PYTHONDONTWRITEBYTECODE=1 python3 scripts/collect_recovery_benchmark.py --oracle-log target/phase19-oracle.log --output target/readme-results/recovery.json
```

[Recovery contracts](docs/dynamic-redispatch.md) define movable commitments,
explicit churn policy, input failures and atomic publication. [Schema 4 fixtures](data/fixtures/phase-19/README.md)
cover successful recovery, pinned execution, cancellation refusals, incomplete
search and realized violations after valid admission. Custody handoffs, returns and
physically immobilized vehicles remain outside this runtime.

## Pre-Phase-20 foundation

Graph artifact schema 4/compiler v4 verify canonical compiled semantic identities;
older identity claims remain historical. Dispatch OperationalState owns coherent
volatile World/execution, distinct plan/action/schedule/effect identities and atomic
publication. Simulation drives shared validated transitions and declares its
`shared-execution/v2` evidence contract. Named static-road temporal certification
supports delayed publication; time-dependent operational certification is unsupported.
The [Phase 20 HTTP adapter](crates/roadrunner-api/README.md) is implemented over the volatile authority. Persistence and event transport remain future phases.

See [runtime boundary](docs/runtime-boundary.md) and
[prerequisite verification](docs/pre-phase-20-remediation-completion.md).

```bash
cargo +1.99.0 test -p roadrunner-core --test semantic_identity --locked
cargo +1.99.0 test -p roadrunner-dispatch --test operational --locked
```
