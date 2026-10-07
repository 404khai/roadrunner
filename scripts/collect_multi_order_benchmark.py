#!/usr/bin/env python3
"""Measure Phase 17 CLI scenarios and verify deterministic coverage and publication."""
import argparse
import hashlib
import json
import statistics
import subprocess
import time
from datetime import date
from pathlib import Path

from collect_simulation_benchmark import hardware, percentile


def coverage(result):
    records = result["insertions"]
    for record in records:
        evidence = record["decision"]["evidence"]
        assert evidence["input_complete"] and evidence["riders_complete"]
        assert evidence["work"] == sum(r["evaluated"] for r in evidence["riders"])
        if evidence["search_complete"]:
            assert all(r["evaluated"] + r["pruned"] == r["pairs_total"] for r in evidence["riders"])
        if evidence["termination"] == "SearchIncomplete":
            assert not record["committed"] and record["decision"]["proposal"] is None
        for rider in evidence["riders"]:
            n = rider["suffix_length"]
            assert rider["pairs_total"] == (n + 1) * (n + 2) // 2
    return {
        "attempts": len(records), "committed": sum(r["committed"] for r in records),
        "candidate_submissions": sum(r["decision"]["evidence"]["work"] for r in records),
        "pairs_total": sum(r["pairs_total"] for a in records for r in a["decision"]["evidence"]["riders"]),
        "policy_excluded_pairs": sum(r["pruned"] for a in records for r in a["decision"]["evidence"]["riders"]),
        "terminations": [r["decision"]["evidence"]["termination"] for r in records],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/roadrunner"))
    parser.add_argument("--fixtures", type=Path, default=Path("data/fixtures/phase-17"))
    parser.add_argument("--runs", type=int, default=11)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.runs < 2:
        parser.error("at least two measured runs required")
    datasets = []
    args.output.parent.mkdir(parents=True, exist_ok=True)
    evidence_dir = args.output.parent / "phase-17-replays"
    evidence_dir.mkdir(exist_ok=True)
    for path in sorted(args.fixtures.glob("*.json")):
        command = [str(args.binary.resolve()), "simulate", str(path.resolve()), "--json"]
        warmup = subprocess.run(command, capture_output=True, check=False)
        source = json.loads(path.read_bytes())
        common = {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                  "scenario_id": source["scenario"]["scenario_id"], "configuration": source["scenario"]["dispatch"]}
        assert warmup.returncode == 0, warmup.stderr.decode()
        result = json.loads(warmup.stdout)
        work = coverage(result)
        if path.stem == "unavailable-forecast":
            assert result["prediction_failures"]
            assert all(not f["committed"] and not f["coverage"]["input_complete"] for f in result["prediction_failures"])
        work["prediction_failures"] = len(result.get("prediction_failures", []))
        samples = []
        for _ in range(args.runs):
            start = time.perf_counter_ns()
            output = subprocess.check_output(command)
            samples.append(time.perf_counter_ns() - start)
            assert output == warmup.stdout, path
        ordered = sorted(samples)
        artifact = evidence_dir / path.name
        artifact.write_bytes(warmup.stdout)
        datasets.append({**common, "graph_digest": result["graph_snapshot_digest"],
            "graph_nodes": result["graph_nodes"], "directed_edges": result["graph_edges"],
            "riders": len(result["riders"]), "orders": len(result["orders"]), "work": work,
            "summary": result["summary"], "semantic_artifact": str(artifact),
            "semantic_sha256": hashlib.sha256(warmup.stdout).hexdigest(),
            "timing": {"raw_ns": samples, "median_ns": statistics.median(ordered),
                       "p95_ns": percentile(ordered, .95), "p99_ns": percentile(ordered, .99)}})
    record = {"schema_version": 1, "date": date.today().isoformat(), "hardware": hardware(),
        "purpose": "Phase 17 small synthetic end-to-end replay and measurement; no scalability or quality threshold claim",
        "algorithm": "exhaustive order-preserving one-order insertion; Roadrunner Dijkstra per propagated leg",
        "compiler": subprocess.check_output(["rustc", "+1.99.0", "--version"], text=True).strip(),
        "build": "cargo +1.99.0 build -p roadrunner-cli --release --locked",
        "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
        "warmups_per_dataset": 1, "measured_runs_per_success_dataset": args.runs,
        "measurement": "process launch + load/validate graph + simulation + JSON output capture",
        "percentiles": "linear interpolation of sorted elapsed process samples",
        "concurrent_workspace_validation": False, "memory": "unmeasured", "datasets": datasets}
    args.output.write_text(json.dumps(record, indent=2) + "\n")
    print(f"Verified {len(datasets)} scenarios; wrote {args.output}")


if __name__ == "__main__":
    main()
