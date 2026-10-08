#!/usr/bin/env python3
"""Measure Phase 18 fleet heuristics and verify canonical replay/publication coverage."""
import argparse
import hashlib
import json
import statistics
import subprocess
import tempfile
import time
from datetime import date
from pathlib import Path

from collect_simulation_benchmark import hardware, percentile

ALGORITHMS = ["Greedy", "LocalSearch", "MultiStartLocal"]


def validate(result):
    records = result["fleets"]
    for record in records:
        decision = record["decision"]
        e = decision["evidence"]
        assert e["riders_complete"]
        assert e["batch"] == sorted(set(e["batch"]))
        assert e["work"]["construction"] + e["work"]["neighborhood"] <= e["context"]["pooling"]["inputs"]["work_budget"]
        if not e["search_complete"]:
            assert e["termination"] == "SearchIncomplete"
            assert not record["committed"] and decision["proposal"] is None
        if record["committed"]:
            assert e["search_complete"] and decision["proposal"] is not None
            for rider in e["riders"]:
                if rider["isolation"] is not None:
                    assert str(rider["rider"]) not in decision["proposal"]["plans"]
    return {
        "attempts": len(records),
        "commits": sum(r["committed"] for r in records),
        "candidate_submissions": sum(r["decision"]["evidence"]["work"]["construction"] + r["decision"]["evidence"]["work"]["neighborhood"] for r in records),
        "precedence_exclusions": sum(r["decision"]["evidence"]["work"]["precedence_exclusions"] for r in records),
        "incomplete_input_attempts": sum(not r["decision"]["evidence"]["input_complete"] for r in records),
        "terminations": [r["decision"]["evidence"]["termination"] for r in records],
        "first_batch_objective": records[0]["decision"]["evidence"]["selected_objective"],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/roadrunner"))
    parser.add_argument("--fixtures", type=Path, default=Path("data/fixtures/phase-18"))
    parser.add_argument("--runs", type=int, default=11)
    parser.add_argument("--oracle-log", type=Path, required=True, help="cargo test -p roadrunner-dispatch --test fleet -- --nocapture output")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.runs < 2:
        parser.error("at least two measured runs required")
    marker = "FLEET_ORACLE_EVIDENCE="
    lines = [line for line in args.oracle_log.read_text().splitlines() if line.startswith(marker)]
    assert len(lines) == 1, "one passing exact-oracle evidence record required"
    assert "test result: ok." in args.oracle_log.read_text() and "FAILED" not in args.oracle_log.read_text()
    oracle = json.loads(lines[0][len(marker):])
    local_marker = "FLEET_LOCAL_MINIMUM_EVIDENCE="
    local_lines = [line for line in args.oracle_log.read_text().splitlines() if line.startswith(local_marker)]
    assert len(local_lines) == 1
    local_minimum = json.loads(local_lines[0][len(local_marker):])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    replays = args.output.parent / "phase-18-replays"
    replays.mkdir(exist_ok=True)
    datasets = []
    for path in sorted(args.fixtures.glob("*.json")):
        source_bytes = path.read_bytes()
        for algorithm in ALGORITHMS:
            source = json.loads(source_bytes)
            source["scenario"]["dispatch"]["algorithm"] = algorithm
            # Every Phase18 fixture is inline; no path-relative graph artifact is relocated.
            assert source["graph"]["kind"] == "inline"
            configuration = json.dumps(source, sort_keys=True).encode()
            with tempfile.NamedTemporaryFile(suffix=".json") as temporary:
                temporary.write(configuration)
                temporary.flush()
                command = [str(args.binary.resolve()), "simulate", temporary.name, "--json"]
                warmup = subprocess.check_output(command)
                result = json.loads(warmup)
                work = validate(result)
                samples = []
                for _ in range(args.runs):
                    start = time.perf_counter_ns()
                    output = subprocess.check_output(command)
                    samples.append(time.perf_counter_ns() - start)
                    assert output == warmup, (path, algorithm)
            semantic = replays / f"{path.stem}-{algorithm}.json"
            semantic.write_bytes(warmup)
            if path.stem == "greedy-trap":
                assert work["first_batch_objective"]["admitted_orders"] == (1 if algorithm == "Greedy" else 2)
            ordered = sorted(samples)
            datasets.append({"path": str(path), "sha256": hashlib.sha256(source_bytes).hexdigest(),
                "scenario_id": source["scenario"]["scenario_id"], "configuration": source["scenario"]["dispatch"],
                "configured_input_sha256": hashlib.sha256(configuration).hexdigest(),
                "graph_digest": result["graph_snapshot_digest"], "graph_nodes": result["graph_nodes"], "directed_edges": result["graph_edges"],
                "riders": len(result["riders"]), "orders": len(result["orders"]), "work": work,
                "summary": result["summary"], "semantic_artifact": str(semantic), "semantic_sha256": hashlib.sha256(warmup).hexdigest(),
                "timing": {"raw_ns": samples, "median_ns": statistics.median(ordered), "p95_ns": percentile(ordered, .95), "p99_ns": percentile(ordered, .99)}})
    record = {"schema_version": 1, "date": date.today().isoformat(), "hardware": hardware(),
        "purpose": "Small synthetic fleet heuristic/greedy quality and process measurements; no global optimum or production scalability claim",
        "algorithms": ALGORITHMS, "compiler": subprocess.check_output(["rustc", "+1.99.0", "--version"], text=True).strip(),
        "build": "cargo +1.99.0 build -p roadrunner-cli --release --locked", "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
        "warmups_per_configuration": 1, "measured_runs_per_configuration": args.runs,
        "measurement": "process launch + graph loading/validation + simulation + JSON capture", "percentiles": "linear interpolation",
        "concurrent_workspace_validation": False, "memory": "unmeasured", "oracle": oracle, "local_minimum": local_minimum,
        "oracle_test_sha256": hashlib.sha256(Path("crates/roadrunner-dispatch/tests/fleet.rs").read_bytes()).hexdigest(), "datasets": datasets}
    args.output.write_text(json.dumps(record, indent=2) + "\n")
    print(f"Verified {len(datasets)} configurations and {len(oracle['cases'])} exact-oracle comparisons; wrote {args.output}")


if __name__ == "__main__":
    main()
