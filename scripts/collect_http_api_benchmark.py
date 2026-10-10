#!/usr/bin/env python3
"""Reproduce loopback HTTP adapter measurements; build the release binary first."""
import argparse
import hashlib
import json
import math
import os
import platform
import secrets
import socket
import statistics
import subprocess
import tempfile
import time
import urllib.request
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/roadrunner-api")
    parser.add_argument("--graph", default="data/fixtures/phase-10/snapshot.semantic-v2/graph.rr-graph")
    parser.add_argument("--runs", type=int, default=30)
    parser.add_argument("--output", default="benchmarks/results/phase-20-http.json")
    args = parser.parse_args()
    if args.runs < 2:
        parser.error("runs must be at least 2")
    graph_path = Path(args.graph)
    data = graph_path.read_bytes()
    payload = json.loads(json.loads(data)["payload_json"])
    edge = next(e for e in payload["edges"] if e["access"] == "general")
    def location(node):
        point = payload["nodes"][node]
        return {"graph_digest": payload["snapshot_digest"], "node": node,
                "coordinate": {"latitude": point["latitude_e7"] / 1e7,
                               "longitude": point["longitude_e7"] / 1e7}}
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    token = secrets.token_hex(32)
    env = dict(os.environ, ROADRUNNER_API_TOKEN=token,
               ROADRUNNER_API_BIND=f"127.0.0.1:{port}")
    base = f"http://127.0.0.1:{port}"
    def request(method, path, body=None, key=None, namespace=None):
        headers = {"Authorization": f"Bearer {token}", "Content-Type": "application/json"}
        if key:
            headers["Idempotency-Key"] = key
        if namespace:
            headers["X-Roadrunner-Namespace"] = namespace
        raw = None if body is None else json.dumps(body).encode()
        started = time.perf_counter_ns()
        with urllib.request.urlopen(urllib.request.Request(base + path, raw, headers, method=method), timeout=10) as response:
            result = json.load(response)
        return (time.perf_counter_ns() - started) / 1e6, result
    with tempfile.TemporaryFile() as log:
        process = subprocess.Popen([args.binary, "--trusted-local", str(graph_path)], env=env, stdout=log, stderr=log)
        try:
            state = None
            for _ in range(100):
                if process.poll() is not None:
                    log.seek(0)
                    raise RuntimeError(log.read().decode())
                try:
                    _, state = request("GET", "/v1/state")
                    break
                except (OSError, urllib.error.URLError):
                    time.sleep(.05)
            if state is None:
                raise RuntimeError("server did not become ready")
            namespace = state["namespace"]
            query = {"origin": location(edge["from"]), "destination": location(edge["to"]),
                     "departure_time": "2026-10-10T00:00:00Z", "objective": "travel_time"}
            results = {}
            for algorithm in ("dijkstra", "astar"):
                samples = []
                query["algorithm"] = algorithm
                for _ in range(5):
                    request("POST", "/v1/routes", query)
                expanded = []
                for _ in range(args.runs):
                    elapsed, result = request("POST", "/v1/routes", query)
                    samples.append(elapsed)
                    expanded.append(result["routes"][0]["expanded_states"])
                results[f"route_{algorithm}"] = {"raw_ms": samples, "expanded_states": expanded}
            samples, retries = [], []
            for index in range(args.runs):
                intent = {"namespace": namespace, "location": location(edge["from"]), "capacity": 1, "available": True}
                key = f"benchmark-rider-{index}"
                elapsed, original = request("POST", "/v1/riders", intent, key)
                repeat_elapsed, repeated = request("POST", "/v1/riders", intent, key)
                if original != repeated:
                    raise AssertionError("retry changed original result")
                samples.append(elapsed)
                retries.append(repeat_elapsed)
            results["create_rider"] = {"raw_ms": samples}
            results["recognized_retry"] = {"raw_ms": retries}
            _, final = request("GET", "/v1/state")
            if len(final["world"]["riders"]) != args.runs:
                raise AssertionError("unexpected command effects")
            for result in results.values():
                ordered = sorted(result["raw_ms"])
                result.update(median_ms=statistics.median(ordered),
                              p95_ms=ordered[math.ceil(.95 * len(ordered)) - 1],
                              p99_ms=ordered[math.ceil(.99 * len(ordered)) - 1])
            cpu = platform.processor()
            if platform.system() == "Darwin":
                cpu = subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip()
            artifact = {
                "schema_version": 1, "hardware": {"platform": platform.platform(), "cpu": cpu, "logical_cpus": os.cpu_count()},
                "dataset": {"path": str(graph_path), "sha256": hashlib.sha256(data).hexdigest(),
                            "graph_digest": payload["snapshot_digest"], "nodes": len(payload["nodes"]), "edges": len(payload["edges"])},
                "build": {"profile": "release", "rustc": subprocess.check_output(["rustc", "+1.99.0", "--version"], text=True).strip(),
                          "binary_sha256": hashlib.sha256(Path(args.binary).read_bytes()).hexdigest()},
                "configuration": {"transport": "loopback HTTP/1.1; new Python urllib connection per request", "concurrency": 1,
                                  "routing_pair": [edge["from"], edge["to"]], "objective": "travel_time", "warmup_routes": 5,
                                  "workers": 4, "work_budget": 50000, "certification_window_seconds": 2,
                                  "creation_state": "empty to runs riders; no orders"},
                "runs": args.runs, "results": results,
                "resources": {"cpu_usage": "unmeasured", "peak_memory": "unmeasured"},
                "limits": "Tiny fixed pair/graph and local client+HTTP+serialization overhead; not routing scalability or an SLO. No durable/process-restart guarantees."
            }
            output = Path(args.output)
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_text(json.dumps(artifact, indent=2) + "\n")
            print(output)
        finally:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


if __name__ == "__main__":
    main()
