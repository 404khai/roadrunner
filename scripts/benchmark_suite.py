#!/usr/bin/env python3
"""Collect or verify matched Phase 37 comparisons; no engine timing thresholds."""
import argparse
import copy
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone

from collect_simulation_benchmark import hardware

ROOT = Path(__file__).resolve().parents[1]
VARIANTS = {"astar_free_flow", "dijkstra_free_flow", "dijkstra_traffic"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()


def read(path):
    return json.loads(Path(path).read_bytes())


def bounded_path(base, name):
    path = (base / name).resolve()
    require(path.is_relative_to(base.resolve()), f"path escapes scope: {name}")
    return path


def stats(samples):
    require(len(samples) >= 2 and all(isinstance(n, (int, float)) and math.isfinite(n) and n > 0 for n in samples), "invalid timing samples")
    ordered = sorted(samples)
    def quantile(p):
        position = (len(ordered) - 1) * p
        lower = int(position)
        upper = min(lower + 1, len(ordered) - 1)
        return ordered[lower] + (ordered[upper] - ordered[lower]) * (position - lower)
    return {"raw_ns": samples, "runs": len(samples), "median_ns": statistics.median(samples), "p95_ns": quantile(.95), "p99_ns": quantile(.99)}


def routing_semantics(result, definition):
    require(result["schema_version"] == 1, "routing schema mismatch")
    rows = {v["id"]: v for v in result["variants"]}
    require(len(result["variants"]) == 3 and set(rows) == VARIANTS, "routing variants missing/duplicated")
    require(result["from_osm"] == definition["from_osm"] and result["to_osm"] == definition["to_osm"], "routing endpoints changed")
    d, a, t = [rows[n] for n in ["dijkstra_free_flow", "astar_free_flow", "dijkstra_traffic"]]
    for row in rows.values():
        route = row["route"]
        require(route["graph_snapshot_digest"] == result["graph_snapshot_digest"], "route graph substitution")
        require(route["total_cost"]["kind"] == "travel_time", "unexpected route objective")
        require(len(route["path"]) == len(route["edges"]) + 1, "route reconstruction mismatch")
    require(d["route"]["algorithm"] == t["route"]["algorithm"] == "dijkstra" and a["route"]["algorithm"] == "a_star", "algorithm provenance mismatch")
    require(math.isclose(d["route"]["total_cost"]["value"], a["route"]["total_cost"]["value"], rel_tol=1e-12, abs_tol=1e-9), "A*/Dijkstra cost mismatch")
    require(t["traffic_adjusted_eta_seconds"] <= d["traffic_adjusted_eta_seconds"] + 1e-9, "traffic optimization is worse under the same traffic context")
    require(math.isclose(t["route"]["total_cost"]["value"], t["traffic_adjusted_eta_seconds"], rel_tol=1e-12, abs_tol=1e-9), "traffic evaluation mismatch")
    if definition["expect_traffic_path_change"]:
        require(d["route"]["edges"] != t["route"]["edges"], "fixture no longer exercises traffic path change")
    semantic = copy.deepcopy(result)
    for field in ["execution_order", "heuristic_preparation_ns"]:
        semantic.pop(field)
    for row in semantic["variants"]:
        row.pop("query_ns")
    return semantic


def materialize(source, case, policy):
    document = copy.deepcopy(source)
    document["scenario"]["dispatch"] = policy
    if policy["kind"] == "multi_order":
        document["scenario"]["schema_version"] = 2
    else:
        document["scenario"]["schema_version"] = 1
        if case.get("legacy_admission_adapter", False):
            for order in document["scenario"]["orders"]:
                order.pop("admission", None)
    return document


def simulation_facts(result, document):
    scenario = document["scenario"]
    require(result["dispatch"] == scenario["dispatch"] and result["seed"] == scenario["seed"], "policy/seed substitution")
    require(result.get("scenario_id") == scenario.get("scenario_id"), "scenario identity substitution")
    require(result["started_at"] == scenario["start_seconds"] and result["ended_at"] == scenario["end_seconds"], "simulation horizon changed")
    require(len(result["riders"]) == len(scenario["riders"]) and {r["rider"] for r in result["riders"]} == {r["id"] for r in scenario["riders"]}, "rider identities lost/duplicated")
    orders = result["orders"]
    require(len(orders) == len({o["order"] for o in orders}) and {o["order"] for o in orders} == {o["id"] for o in scenario["orders"]}, "order identities lost/duplicated")
    require(not any(o.get("cancelled_at") is not None for o in orders), "this fixture accounting does not support cancellation")
    counts = {"scheduled_orders": len(orders), "created_orders": sum(o["created_at"] is not None for o in orders), "assigned_orders": sum(o["assigned_at"] is not None for o in orders), "delivered_orders": sum(o["delivered_at"] is not None for o in orders), "unassigned_orders": sum(o["created_at"] is not None and o["assigned_at"] is None for o in orders), "assigned_unfinished_orders": sum(o["assigned_at"] is not None and o["delivered_at"] is None for o in orders), "outstanding_orders": sum(o["created_at"] is not None and o["delivered_at"] is None for o in orders)}
    for name, value in counts.items():
        require(result["summary"][name] == value, f"lifecycle accounting mismatch: {name}")
    work = 0
    for insertion in result.get("insertions", []):
        evidence = insertion["decision"]["evidence"]
        work += evidence["work"]
        if not evidence["search_complete"]:
            require(not insertion["committed"] and insertion["decision"]["proposal"] is None, "incomplete insertion published")
    return {"counts": counts, "processed_events": len(result["events"]), "future_events": result["future_events"], "insertion_submissions": work, "summary": result["summary"], "graph_snapshot_digest": result["graph_snapshot_digest"], "nodes": result["graph_nodes"], "directed_edges": result["graph_edges"], "riders": len(result["riders"]), "seed": result["seed"], "randomness": result["randomness"]}


def rates(facts, document, samples):
    scenario = document["scenario"]
    duration = scenario["end_seconds"] - scenario["start_seconds"]
    return {"simulation_seconds": duration, "completed_orders_per_wall_second": [facts["counts"]["delivered_orders"] * 1e9 / n for n in samples], "events_per_wall_second": [facts["processed_events"] * 1e9 / n for n in samples], "simulation_seconds_per_wall_second": [duration * 1e9 / n for n in samples], "scope": "whole-process synthetic run rates; not operational service throughput"}


def command_json(command, timeout):
    began = time.perf_counter_ns()
    process = subprocess.run(command, capture_output=True, timeout=timeout, check=False, cwd=ROOT)
    elapsed = time.perf_counter_ns() - began
    require(process.returncode == 0, f"command failed ({process.returncode}): {process.stderr.decode(errors='replace')[:2000]}")
    require(not process.stderr, "unexpected command diagnostics")
    return read_bytes(process.stdout), elapsed


def read_bytes(data):
    return json.loads(data)


def asset(directory, name, value):
    path = bounded_path(directory, name)
    path.write_bytes(canonical(value) + b"\n")
    return {"path": name, "sha256": sha(path)}


def load_asset(directory, reference):
    path = bounded_path(directory, reference["path"])
    require(sha(path) == reference["sha256"], f"artifact hash mismatch: {path}")
    return read(path)


def definitions(manifest):
    require(set(manifest) == {"schema_version", "suite_id", "description", "routing", "simulation"} and manifest["schema_version"] == 1, "unsupported suite schema")
    require([c["id"] for c in manifest["simulation"]] == ["nearest-vs-pickup-eta", "basic-vs-preparation", "single-vs-multi-order"], "required comparisons missing/reordered")
    for case in manifest["simulation"]:
        require(len(case["policies"]) == 2, "comparison must contain two declared policies")
        expected = {"nearest-vs-pickup-eta": ["nearest_rider", "lowest_pickup_eta"], "basic-vs-preparation": ["basic", "preparation_aware"], "single-vs-multi-order": ["basic", "multi_order"]}
        require([p["kind"] for p in case["policies"]] == expected[case["id"]], "required policy variants missing/duplicated")
        require(case.get("legacy_admission_adapter", False) == (case["id"] == "single-vs-multi-order"), "legacy adaptation must be explicit and scoped")
        source = read(bounded_path(ROOT, case["scenario"]))
        require(source["graph"]["kind"] == "inline", "suite v1 supports only inline simulation graphs")
        yield case, source


def verify(directory):
    record = read(directory / "index.json")
    require(record["schema_version"] == 1 and record["status"] == "completed", "run incomplete/unknown")
    manifest = load_asset(directory, record["definition"])
    cases = list(definitions(manifest))
    required_inputs = {"Cargo.lock", manifest["routing"]["traffic"]}
    required_inputs.update(str(Path(manifest["routing"]["snapshot"]) / name) for name in ["graph.rr-graph", "graph.rr-provenance", "manifest.json"])
    required_inputs.update(case["scenario"] for case, _ in cases)
    require(required_inputs <= {r["path"] for r in record["inputs"]}, "input fingerprints missing")
    for reference in record["inputs"]:
        require(sha(bounded_path(ROOT, reference["path"])) == reference["sha256"], "repository input changed")
    reference = load_asset(directory, record["routing"]["replay"])
    routing_semantics(reference, manifest["routing"])
    graph_manifest = read(bounded_path(ROOT, manifest["routing"]["snapshot"]) / "manifest.json")
    require(reference["graph_snapshot_digest"] == graph_manifest["graph_snapshot_digest"] and reference["nodes"] == graph_manifest["graph_node_count"] and reference["directed_edges"] == graph_manifest["graph_edge_count"], "routing dataset substitution")
    require(set(record["routing"]["query_timing"]) == VARIANTS, "timing variants missing")
    for timing in [*record["routing"]["query_timing"].values(), record["routing"]["heuristic_preparation_timing"]]:
        require(timing == stats(timing["raw_ns"]) and timing["runs"] == record["runs"], "timing summary corrupted")
    require(len(record["simulation"]) == len(cases), "simulation comparisons missing")
    for row, (case, source) in zip(record["simulation"], cases):
        require(row["id"] == case["id"] and len(row["variants"]) == 2, "simulation comparison mismatch")
        digests = set()
        for variant, policy in zip(row["variants"], case["policies"]):
            document = load_asset(directory, variant["input"])
            require(document == materialize(source, case, policy), "unmatched variant input")
            result = load_asset(directory, variant["replay"])
            require(variant["facts"] == simulation_facts(result, document), "simulation facts corrupted")
            timing = variant["process_timing"]
            require(timing == stats(timing["raw_ns"]) and timing["runs"] == record["runs"], "timing summary corrupted")
            require(variant["rates"] == rates(variant["facts"], document, timing["raw_ns"]), "throughput evidence corrupted")
            digests.add(result["graph_snapshot_digest"])
        require(len(digests) == 1, "paired graph substitution")
    return record


def collect(args):
    manifest = read(args.manifest)
    cases = list(definitions(manifest))
    directory = args.output.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    binary, probe = args.binary.resolve(), args.routing_binary.resolve()
    routing = manifest["routing"]
    inputs = [Path("Cargo.lock"), args.manifest.resolve().relative_to(ROOT), Path(routing["traffic"])]
    inputs += [Path(routing["snapshot"]) / name for name in ["graph.rr-graph", "graph.rr-provenance", "manifest.json"]]
    inputs += [Path(case["scenario"]) for case, _ in cases]
    record = {"schema_version": 1, "status": "running", "suite_id": manifest["suite_id"], "collected_at": datetime.now(timezone.utc).isoformat(), "hardware": {**hardware(), "logical_cpus": os.cpu_count()}, "python": platform.python_version(), "collection_environment": "developer workstation; no CPU pinning, isolation or frequency control; background load unmeasured", "source_files": [{"path": p, "sha256": sha(ROOT / p)} for p in ["scripts/benchmark_suite.py", "crates/roadrunner-cli/examples/routing_comparison.rs"]], "source_revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(), "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(), "build_profile": args.build_profile, "build_command": args.build_command, "build_metadata": "profile/command operator-declared; executable SHA-256 and source revision recorded independently", "binary_sha256": sha(binary), "routing_binary_sha256": sha(probe), "runs": args.runs, "warmups": "one excluded process per variant; routing probe additionally performs one excluded query before each measured query", "order": "alternate forward/reverse within each comparison on successive rounds", "timeout_seconds_per_process": args.timeout, "cache": "fresh processes, graph reused within routing probe; no result cache; OS filesystem cache uncontrolled, not a cold-cache experiment", "routing_measurement": "individual in-process search calls, including scratch allocation/reconstruction; excludes load/deep verification, heuristic preparation, traffic construction and serialization", "simulation_measurement": "whole CLI process: launch, inline graph construction, validation, simulation, JSON serialization and capture", "memory_allocations_cpu": "unmeasured; no capacity or concurrent service throughput claim", "percentiles": "linear interpolation of sorted samples; descriptive small-sample tails, no SLA", "definition": asset(directory, "definition.json", manifest), "inputs": [{"path": str(p), "sha256": sha(ROOT / p)} for p in sorted(set(inputs))], "simulation": []}
    route_command = [str(probe), str(bounded_path(ROOT, routing["snapshot"])), str(bounded_path(ROOT, routing["traffic"])), routing["from_osm"], routing["to_osm"]]
    try:
        reference, _ = command_json(route_command + ["forward"], args.timeout)
        semantic = routing_semantics(reference, routing)
        samples = {key: [] for key in VARIANTS}
        preparation = []
        for iteration in range(args.runs):
            current, _ = command_json(route_command + ["reverse" if iteration % 2 else "forward"], args.timeout)
            require(routing_semantics(current, routing) == semantic, "routing failed semantic replay")
            for variant in current["variants"]:
                samples[variant["id"]].append(variant["query_ns"])
            preparation.append(current["heuristic_preparation_ns"])
        record["routing"] = {"replay": asset(directory, "routing-replay.json", reference), "query_timing": {key: stats(value) for key, value in sorted(samples.items())}, "heuristic_preparation_timing": stats(preparation)}
        for case, source in cases:
            variants = []
            for policy in case["policies"]:
                document = materialize(source, case, policy)
                name = f"{case['id']}-{policy['kind']}"
                input_asset = asset(directory, name + "-input.json", document)
                command = [str(binary), "simulate", str(directory / input_asset["path"]), "--json"]
                result, _ = command_json(command, args.timeout)
                variants.append({"input": input_asset, "replay": asset(directory, name + "-replay.json", result), "facts": simulation_facts(result, document), "samples": [], "command": command, "reference": canonical(result)})
            for iteration in range(args.runs):
                for variant in (reversed(variants) if iteration % 2 else variants):
                    result, elapsed = command_json(variant["command"], args.timeout)
                    require(canonical(result) == variant["reference"], "simulation failed deterministic replay")
                    variant["samples"].append(elapsed)
            for variant in variants:
                variant["process_timing"] = stats(variant.pop("samples"))
                variant["rates"] = rates(variant["facts"], load_asset(directory, variant["input"]), variant["process_timing"]["raw_ns"])
                variant.pop("command")
                variant.pop("reference")
            record["simulation"].append({"id": case["id"], "variants": variants})
        record["status"] = "completed"
        (directory / "index.json").write_text(json.dumps(record, indent=2, allow_nan=False) + "\n")
        verify(directory)
    except Exception as error:
        record["status"] = "failed"
        record["failure"] = str(error)
        (directory / "index.json").write_text(json.dumps(record, indent=2, allow_nan=False) + "\n")
        (directory / "failure.json").write_text(json.dumps({"status": "failed", "error": str(error), "completed_comparisons": len(record["simulation"])}))
        raise
    return directory


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    run = commands.add_parser("collect")
    run.add_argument("--manifest", type=Path, default=ROOT / "benchmarks/suite/manifest.json")
    run.add_argument("--binary", type=Path, required=True)
    run.add_argument("--routing-binary", type=Path, required=True)
    run.add_argument("--build-profile", choices=["debug", "release"], required=True)
    run.add_argument("--build-command", required=True)
    run.add_argument("--runs", type=int, default=11)
    run.add_argument("--timeout", type=float, default=30)
    run.add_argument("--output", type=Path, required=True)
    check = commands.add_parser("verify")
    check.add_argument("directory", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "collect":
            require(args.runs >= 2 and math.isfinite(args.timeout) and args.timeout > 0, "runs >= 2 and finite positive timeout required")
            path = collect(args)
            print(f"Collected and verified five comparisons: {path}")
        else:
            verify(args.directory.resolve())
            print("Verified definitions, hashes, matched inputs, timing summaries and semantic evidence")
    except (ValueError, KeyError, OSError, subprocess.SubprocessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
