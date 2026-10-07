# roadrunner-osm

Deterministic extraction and compilation of OpenStreetMap `.osm.pbf` into Roadrunner
road graphs. This crate owns source filtering, policy compilation, OSM provenance,
restriction diagnostics and validated snapshot bundles; routing itself belongs to core.

See the [project setup](../../README.md#setup). All commands run from the **repository root**.
Committed PBFs are available offline after checkout; normal ingestion needs no Docker,
Python `osmium`, database or external routing service.

## Pipeline and API

```text
.osm.pbf → extract_pbf → NormalizedOsmDataset → .rr-osm
         → compile_motorcycle_graph → FrozenGraph + provenance + manifest
         → atomic snapshot publication → validated snapshot load
```

| API | Purpose |
| --- | --- |
| `extract_pbf` | Read PBF with caller-supplied source identity and normalize supported source facts |
| `encode_dataset_artifact`, `decode_dataset_artifact`, `write_dataset_artifact_atomic` | Versioned normalized `.rr-osm` artifacts |
| `compile_motorcycle_graph` | Compile the supported `delivery_motorcycle_v2` / `ng_v2` policy |
| `CompiledGraph`, `BuildManifest`, `CompilationDiagnostics`, `GraphProvenance` | Graph output, source mappings, policies, component/restriction diagnostics |
| `write_snapshot_bundle_atomic` | Publish graph, provenance and manifest as one snapshot directory |
| `load_snapshot_bundle` | Validate a snapshot; optional deep verification checks provenance/graph consistency |

A bundle contains `graph.rr-graph`, `graph.rr-provenance` and `manifest.json`.
Source identity participates in provenance/snapshot identity. Scenario overlays are
bound to exact graph digests; a different source ID may invalidate a pinned fixture.
Supported node-via turn restrictions affect routing. Unsupported conditional, via-way
and other restriction forms remain diagnosable rather than silently claiming support.
See [supported OSM semantics and artifact contract](../../docs/osm-ingestion.md),
[turn-restriction completion](../../docs/phase-7.5-completion.md), and
[fixture provenance](../../data/fixtures/phase-7/README.md).

## Run ingestion and verification

This is a library with CLI adapters. Build the `roadrunner` binary first:

```bash
cargo +1.99.0 build -p roadrunner-cli --release --locked
mkdir -p target/osm-readme
./target/release/roadrunner osm extract \
  data/fixtures/phase-7/lagos-marina.osm.pbf \
  target/osm-readme/marina.rr-osm --source-id readme-osm-marina
./target/release/roadrunner osm compile \
  target/osm-readme/marina.rr-osm target/osm-readme/marina-snapshot
./target/release/roadrunner graph verify target/osm-readme/marina-snapshot --deep
./target/release/roadrunner route corpus \
  target/osm-readme/marina-snapshot data/fixtures/phase-7/route-corpus.json
```

Choose a fresh snapshot directory if repeating publication. For static/time-dependent
traffic fixtures use the exact source IDs in the [root routing walkthrough](../../README.md#static-traffic).
The larger `lagos-island-engineering.osm.pbf` can replace the input for ingestion
measurements; its size does not establish production-scale behavior.

## Build, tests, docs and benchmark

```bash
cargo +1.99.0 build -p roadrunner-osm --locked
cargo +1.99.0 test -p roadrunner-osm --all-features --locked
cargo +1.99.0 test -p roadrunner-osm --test pipeline --locked
cargo +1.99.0 doc -p roadrunner-osm --no-deps --locked
cargo +1.99.0 bench -p roadrunner-osm --bench osm_pipeline --locked
```

API docs: `target/doc/roadrunner_osm/index.html`. Add `-- --test` to the benchmark for
one-pass smoke validation. The pipeline benchmark covers extraction, normalization,
compilation, serialization/loading, deep verification and routing on committed fixtures.
Criterion results go to `target/criterion`.

## Optional fixture tools

The Python conversion/generation helpers require `osmium`; this is separate from the
Rust `osmpbf` dependency. Write generated files outside the committed fixture paths:

```bash
mkdir -p target/osm-readme
python3 -m venv target/osm-tools-venv
target/osm-tools-venv/bin/python -m pip install osmium
target/osm-tools-venv/bin/python scripts/generate_phase7_semantics_fixture.py target/osm-readme/source-semantics.osm.pbf
# Supply an existing XML input file:
target/osm-tools-venv/bin/python scripts/convert_osm_to_pbf.py /path/to/input.osm target/osm-readme/converted.osm.pbf
```

Both helpers replace their output file if present. Fixture data is © OpenStreetMap
contributors, distributed under ODbL 1.0; the fixture guide records dataset hashes/sources.

## Historical Phase 7 report assembler

`scripts/collect_phase7_benchmark.py` expects `<snapshot-root>/small.rr-osm`,
`small-snapshot/`, `engineering.rr-osm` and `engineering-snapshot/`, plus measured
`osm_*` samples in `target/criterion`. Prepare the input files with:

```bash
mkdir -p target/phase7-report-input
./target/release/roadrunner osm extract data/fixtures/phase-7/lagos-marina.osm.pbf target/phase7-report-input/small.rr-osm --source-id phase7-small
./target/release/roadrunner osm compile target/phase7-report-input/small.rr-osm target/phase7-report-input/small-snapshot
./target/release/roadrunner osm extract data/fixtures/phase-7/lagos-island-engineering.osm.pbf target/phase7-report-input/engineering.rr-osm --source-id phase7-engineering
./target/release/roadrunner osm compile target/phase7-report-input/engineering.rr-osm target/phase7-report-input/engineering-snapshot
python3 scripts/collect_phase7_benchmark.py target/phase7-report-input target/phase7-report-input/historical-report.json
```

The script embeds historical hardware, compiler, date and memory values. It is not a
fresh memory/hardware collector; do not present those embedded values as measurements
of this run. For new experiments independently record the machine, compiler/config,
raw runs and memory (or explicitly unmeasured), following [benchmark requirements](../../docs/benchmarks.md).
