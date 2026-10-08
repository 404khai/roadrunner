#!/usr/bin/env python3
"""Measure schema 4 recovery scenarios and validate exact deterministic replays."""
import argparse
import hashlib
import json
import statistics
import subprocess
import time
from datetime import date
from pathlib import Path

from collect_simulation_benchmark import hardware, percentile


def validate(result, name):
    records = result.get("recoveries", [])
    assert records
    for record in records:
        d = record["decision"]
        if d is None:
            assert record["failure"] and not record["committed"]
            continue
        assert d["evaluated"] <= d["context"]["pooling"]["inputs"]["work_budget"]
        if not record["committed"]:
            assert record["world_version_after"] == d["world_version"]
        if d["termination"] == "SearchIncomplete":
            assert d["proposal"] is None and not record["committed"]
        if record["committed"]:
            assert d["termination"] == "LocalMinimum" and d["proposal"]
            assert record["world_version_after"] == d["world_version"] + 1
            for order in d["locked_orders"]:
                assert d["expected"]["assignments"][str(order)] == d["proposal"]["assignments"][str(order)]
    accepted = {}
    for record in result.get("fleets", []):
        if record["committed"]:
            accepted.update(record["decision"]["proposal"]["accepted"])
    assert result["final_state"]["accepted"] == accepted
    if name in ["offline-recovery", "frozen-wait", "frozen-service"]:
        assert any(r["committed"] and r["decision"]["baseline_requires_repair"] for r in records)
        assert result["summary"]["delivered_orders"] == 2
    if name == "optional-recovery":
        assert any(r["committed"] and not r["decision"]["baseline_requires_repair"] for r in records)
    if name == "churn-suppression":
        assert not any(r["committed"] for r in records)
    if name == "cancellation":
        assert result["summary"]["delivered_orders"] == 1 and result["summary"]["cancelled_orders"] == 1
    if name == "budget-exhaustion":
        assert any(r["decision"] and r["decision"]["termination"] == "SearchIncomplete" for r in records)
    if name == "unavailable-readiness":
        assert any(r["failure"] and "PredictionUnavailable" in r["failure"] for r in records)
    if name in ["observed-road-delay", "hard-deadline-breach"]:
        assert any(r["decision"] and r["decision"]["termination"] == "NoRecovery" for r in records)
        assert len([e for e in result["events"] if e["event"]["type"] == "RIDER_MOVED"]) == 2
    return {"attempts": len(records), "commits": sum(r["committed"] for r in records),
            "work": sum(r["decision"]["evaluated"] for r in records if r["decision"]),
            "feasible": sum(r["decision"]["feasible"] for r in records if r["decision"]),
            "precedence_exclusions": sum(r["decision"]["precedence_exclusions"] for r in records if r["decision"]),
            "terminations": [r["decision"]["termination"] if r["decision"] else r["failure"] for r in records]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/roadrunner"))
    parser.add_argument("--fixtures", type=Path, default=Path("data/fixtures/phase-19"))
    parser.add_argument("--runs", type=int, default=11)
    parser.add_argument("--oracle-log", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.runs < 2:
        parser.error("at least two measured runs required")
    log = args.oracle_log.read_text()
    lines = [line for line in log.splitlines() if line.startswith("RECOVERY_ORACLE_EVIDENCE=")]
    assert len(lines) == 1 and "test result: ok." in log and "FAILED" not in log
    oracle = json.loads(lines[0].split("=", 1)[1])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    replays = args.output.parent / "phase-19-replays"
    replays.mkdir(exist_ok=True)
    datasets = []
    for path in sorted(args.fixtures.glob("*.json")):
        command = [str(args.binary.resolve()), "simulate", str(path), "--json"]
        warmup = subprocess.check_output(command)
        result = json.loads(warmup)
        work = validate(result, path.stem)
        samples = []
        for _ in range(args.runs):
            started = time.perf_counter_ns()
            output = subprocess.check_output(command)
            samples.append(time.perf_counter_ns() - started)
            assert output == warmup, path
        semantic = replays / path.name
        semantic.write_bytes(warmup)
        source = json.loads(path.read_bytes())
        datasets.append({"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                         "scenario_id": result["scenario_id"], "configuration": source["scenario"]["dispatch"],
                         "graph_digest": result["graph_snapshot_digest"], "graph_nodes": result["graph_nodes"],
                         "directed_edges": result["graph_edges"], "riders": len(result["riders"]), "orders": len(result["orders"]),
                         "work": work, "summary": result["summary"], "semantic_artifact": str(semantic),
                         "semantic_sha256": hashlib.sha256(warmup).hexdigest(),
                         "timing": {"raw_ns": samples, "median_ns": statistics.median(samples),
                                    "p95_ns": percentile(sorted(samples), .95), "p99_ns": percentile(sorted(samples), .99)}})
    record = {"schema_version": 1, "date": date.today().isoformat(), "hardware": hardware(),
              "purpose": "Tiny synthetic committed recovery and process measurements; no global optimum or scalability claim",
              "compiler": subprocess.check_output(["rustc", "+1.99.0", "--version"], text=True).strip(),
              "build": "cargo +1.99.0 build -p roadrunner-cli --release --locked",
              "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
              "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
              "source_diff_sha256": hashlib.sha256(subprocess.check_output(["git", "diff", "HEAD"])).hexdigest(),
              "warmups_per_configuration": 1, "measured_runs_per_configuration": args.runs,
              "measurement": "process launch + graph loading/validation + simulation + JSON capture",
              "percentiles": "linear interpolation", "concurrent_workspace_validation": False,
              "memory": "unmeasured", "oracle": oracle,
              "oracle_test_sha256": hashlib.sha256(Path("crates/roadrunner-dispatch/tests/recovery.rs").read_bytes()).hexdigest(),
              "datasets": datasets}
    args.output.write_text(json.dumps(record, indent=2) + "\n")
    print(f"Verified {len(datasets)} scenarios with {args.runs} exact measured replays each; wrote {args.output}")


if __name__ == "__main__":
    main()
