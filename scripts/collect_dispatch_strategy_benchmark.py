#!/usr/bin/env python3
"""Collect paired Phase 16 outcomes and per-strategy process timings, checking exact replay."""
import argparse
import copy
import hashlib
import json
import statistics
import subprocess
import tempfile
import time
from datetime import date
from pathlib import Path

from collect_simulation_benchmark import hardware, percentile


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/roadrunner"))
    parser.add_argument("--scenario", type=Path, default=Path("data/fixtures/phase-16/paired-strategies.json"))
    parser.add_argument("--runs", type=int, default=11)
    parser.add_argument("--idle-penalty-weight", type=float, default=1.0)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.runs < 2:
        parser.error("at least two measured runs are required")
    binary = str(args.binary.resolve())
    scenario_path = args.scenario.resolve()
    source = scenario_path.read_bytes()
    document = json.loads(source)
    command = [binary, "benchmark", "dispatch", str(scenario_path), "--idle-penalty-weight", str(args.idle_penalty_weight), "--json"]
    comparison_bytes = subprocess.check_output(command)
    if subprocess.check_output(command) != comparison_bytes:
        raise ValueError("comparison did not replay byte-for-byte")
    comparison = json.loads(comparison_bytes)
    # Bind artifacts before moving variant documents into a temporary directory.
    graph_artifact = None
    if document["graph"]["kind"] == "artifact":
        artifact_path = scenario_path.parent / document["graph"]["path"]
        graph_artifact = {"path": str(artifact_path.resolve()), "sha256": digest(artifact_path.read_bytes())}
        document["graph"]["path"] = str(artifact_path.resolve())
    records = []
    with tempfile.TemporaryDirectory(prefix="roadrunner-phase16-") as directory:
        for expected in comparison["runs"]:
            variant = copy.deepcopy(document)
            variant["scenario"]["dispatch"] = expected["dispatch"]
            path = Path(directory) / "scenario.json"
            path.write_text(json.dumps(variant))
            run_command = [binary, "simulate", str(path), "--json"]
            reference = subprocess.check_output(run_command)  # One excluded warmup per policy.
            if json.loads(reference) != expected:
                raise ValueError("standalone policy result differs from paired comparison")
            samples = []
            for _ in range(args.runs):
                began = time.perf_counter_ns()
                output = subprocess.check_output(run_command)
                samples.append(time.perf_counter_ns() - began)
                if output != reference:
                    raise ValueError("standalone simulation did not replay byte-for-byte")
            ordered = sorted(samples)
            records.append({
                "dispatch": expected["dispatch"],
                "summary": expected["summary"],
                "orders": expected["orders"],
                "riders": expected["riders"],
                "processed_events": len(expected["events"]),
                "decision_count": len(expected["decisions"]),
                "replay_sha256": digest(reference),
                "timing": {"samples_ns": samples, "median_ns": statistics.median(ordered),
                           "p95_ns": percentile(ordered, .95), "p99_ns": percentile(ordered, .99)},
            })
    first = comparison["runs"][0]
    record = {
        "schema_version": 1,
        "date": date.today().isoformat(),
        "purpose": "Paired synthetic dispatch strategy comparison; no production scalability claim",
        "hardware": hardware(),
        "dataset": {"path": str(args.scenario), "sha256": digest(source),
                    "kind": "synthetic" if graph_artifact is None else "scenario on graph artifact; inspect source provenance",
                    "graph_artifact": graph_artifact, "seed": first["seed"], "randomness": first["randomness"],
                    "graph_digest": first["graph_snapshot_digest"], "graph_nodes": first["graph_nodes"],
                    "graph_directed_edges": first["graph_edges"], "riders": len(first["riders"]),
                    "scheduled_orders": first["summary"]["scheduled_orders"]},
        "algorithm": {"routing": "Roadrunner Dijkstra with static traffic overlays",
                      "events": "BinaryHeap ordered by logical time and insertion sequence",
                      "coverage": "exhaustive eligible road-feasible riders", "tie_break": "lowest RiderId"},
        "configuration": {"build": "cargo +1.99.0 build -p roadrunner-cli --release --locked",
                          "rustc": subprocess.check_output(["rustc", "+1.99.0", "--version"], text=True).strip(),
                          "binary_sha256": digest(Path(binary).read_bytes()), "profile": "release",
                          "comparison_command": command, "idle_penalty_weight": args.idle_penalty_weight,
                          "warmup_runs_per_strategy": 1, "measured_runs_per_strategy": args.runs,
                          "start_seconds": first["started_at"], "end_seconds": first["ended_at"],
                          "includes": "process startup, graph/scenario loading and validation, simulation, JSON serialization and capture",
                          "timing_percentiles": "linear interpolation of ordered elapsed process samples",
                          "outcome_percentiles": "nearest rank; even median midpoint",
                          "memory": "not measured", "concurrent_workspace_validation": False},
        "comparison_replay_sha256": digest(comparison_bytes),
        "strategies": records,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2) + "\n")


if __name__ == "__main__":
    main()
