#!/usr/bin/env python3
"""Measure a fixed simulation CLI replay, including process/loading/JSON overhead."""
import argparse
import hashlib
import json
import platform
import statistics
import subprocess
import time
from datetime import date
from pathlib import Path


def percentile(values, fraction):
    position = (len(values) - 1) * fraction
    lower = int(position)
    upper = min(lower + 1, len(values) - 1)
    return values[lower] + (values[upper] - values[lower]) * (position - lower)


def hardware():
    info = {"operating_system": platform.platform(), "architecture": platform.machine()}
    if platform.system() == "Darwin":
        for key, field in [("machdep.cpu.brand_string", "cpu"), ("hw.memsize", "memory_bytes")]:
            value = subprocess.check_output(["sysctl", "-n", key], text=True).strip()
            info[field] = int(value) if field == "memory_bytes" else value
    else:
        info["cpu"] = platform.processor() or "unavailable"
    return info


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/roadrunner"))
    parser.add_argument("--scenario", type=Path, default=Path("data/fixtures/phase-15/seeded-deliveries.json"))
    parser.add_argument("--runs", type=int, default=11)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.runs < 2:
        parser.error("at least two measured runs are required")
    command = [str(args.binary.resolve()), "simulate", str(args.scenario.resolve()), "--json"]
    reference = subprocess.check_output(command)  # One excluded warmup.
    result = json.loads(reference)
    measurements = []
    for _ in range(args.runs):
        began = time.perf_counter_ns()
        output = subprocess.check_output(command)
        measurements.append(time.perf_counter_ns() - began)
        if output != reference:
            raise ValueError("simulation output did not replay byte-for-byte")
    ordered = sorted(measurements)
    record = {
        "schema_version": 1,
        "date": date.today().isoformat(),
        "purpose": "Phase 15 small-fixture wall-clock replay validation, not a scalability claim",
        "hardware": hardware(),
        "dataset": {"path": str(args.scenario), "sha256": hashlib.sha256(args.scenario.read_bytes()).hexdigest(),
                    "kind": "seeded synthetic", "seed": result["seed"], "randomness": result["randomness"],
                    "graph_digest": result["graph_snapshot_digest"], "graph_nodes": result["graph_nodes"],
                    "graph_directed_edges": result["graph_edges"], "scheduled_orders": result["summary"]["scheduled_orders"],
                    "created_orders": result["summary"]["created_orders"], "riders": len(result["riders"])},
        "algorithm": {"events": "BinaryHeap ordered by logical time then insertion sequence",
                      "routing": "Roadrunner Dijkstra", "dispatch": result["dispatch"], "coverage": "exhaustive eligible riders"},
        "configuration": {"binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(), "profile": "release", "build": "cargo +1.99.0 build -p roadrunner-cli --release --locked",
                          "rustc": subprocess.check_output(["rustc", "+1.99.0", "--version"], text=True).strip(),
                          "command": "target/release/roadrunner simulate " + str(args.scenario) + " --json",
                          "warmup_runs": 1, "measured_runs": args.runs, "concurrent_workspace_validation": False,
                          "includes": "process launch, graph loading/validation, simulation, JSON serialization and output capture",
                          "percentiles": "linear interpolation of ordered elapsed process samples", "memory": "not measured"},
        "observations": {"logical_window_seconds": result["ended_at"] - result["started_at"],
                         "processed_events": len(result["events"]), "delivered_orders": result["summary"]["delivered_orders"],
                         "future_events": result["future_events"], "replay_sha256": hashlib.sha256(reference).hexdigest()},
        "timing": {"samples_ns": measurements, "median_ns": statistics.median(ordered),
                   "p95_ns": percentile(ordered, .95), "p99_ns": percentile(ordered, .99)},
    }
    args.output.write_text(json.dumps(record, indent=2) + "\n")


if __name__ == "__main__":
    main()
