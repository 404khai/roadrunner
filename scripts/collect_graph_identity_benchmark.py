#!/usr/bin/env python3
"""Record Criterion graph publication/verified loading samples after the documented run."""
import argparse
import hashlib
import json
import statistics
import subprocess
from pathlib import Path
from collect_simulation_benchmark import hardware, percentile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path("target/criterion/graph_lifecycle")
    samples = []
    source = Path("crates/roadrunner-core/benches/support/mod.rs")
    bench = Path("crates/roadrunner-core/benches/graph_lifecycle.rs")
    binaries = [p for p in Path("target/release/deps").glob("graph_lifecycle-*") if p.is_file() and p.suffix != ".d"]
    binary = max(binaries, key=lambda p: p.stat().st_mtime)
    for path in sorted(root.glob("**/new/sample.json")):
        data = json.loads(path.read_text())
        raw = [elapsed / count for elapsed, count in zip(data["times"], data["iters"])]
        assert len(raw) == 20
        configuration = path.parent.parent.name
        nodes = int(configuration.split("_")[0])
        ordered = sorted(raw)
        samples.append({"operation": path.relative_to(root).parts[0], "configuration": configuration,
                        "nodes": nodes, "segments": 3 * nodes - 6, "directed_edges": 3 * nodes - 6,
                        "criterion_raw": data, "per_iteration_raw_ns": raw, "median_ns": statistics.median(raw),
                        "p95_ns": percentile(ordered, .95), "p99_ns": percentile(ordered, .99)})
    assert len(samples) == 8
    record = {"schema_version": 1, "hardware": hardware(), "compiler": subprocess.check_output(["rustc", "+1.99.0", "--version"], text=True).strip(),
              "algorithm": "canonical builder finalization + graph-semantics.v2 SHA256 sealing; artifact schema4 serialization and verified decoding",
              "dataset": "forward_banded_v1; fan-out3; E7 coordinates; generated-seed0; 36kph; General access; directed forward edges",
              "dataset_definition_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
              "benchmark_definition_sha256": hashlib.sha256(bench.read_bytes()).hexdigest(),
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "build": "Rust1.99 release/bench; Cargo.lock; default target features",
              "command": "cargo +1.99.0 bench -p roadrunner-core --bench graph_lifecycle --locked -- --noplot --warm-up-time 1 --measurement-time 2 --sample-size 20",
              "warmup_seconds": 1, "target_measurement_seconds": 2, "samples_per_configuration": 20,
              "percentiles": "linear interpolation over per-iteration Criterion samples; no SLO claim",
              "memory": "unmeasured", "concurrent_workspace_validation": False, "samples": samples}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2) + "\n")
    print(f"Recorded {len(samples)} graph configurations")


if __name__ == "__main__":
    main()
