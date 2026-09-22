#!/usr/bin/env python3
"""Rebuild a pinned Roadrunner/OSRM comparison from the same OSM extract."""

import argparse
import hashlib
import json
import math
import platform
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "data/fixtures/phase-7/lagos-marina.osm.pbf"
CORPUS = ROOT / "data/fixtures/phase-7/route-corpus.json"
IMAGE = "ghcr.io/project-osrm/osrm-backend@sha256:855614a38f464b0558a2ad6eaa7cb8c139f39887da9b38b485ce453c6e6e6124"


def command(*args, capture=True):
    result = subprocess.run(args, cwd=ROOT, check=True, text=True,
                            stdout=subprocess.PIPE if capture else subprocess.DEVNULL,
                            stderr=subprocess.PIPE)
    return result.stdout.strip() if capture else None


def request(url):
    with urllib.request.urlopen(url, timeout=10) as response:
        return json.load(response)


def xy(point, latitude):
    scale = 111_195.0
    return (point[0] * scale * math.cos(math.radians(latitude)), point[1] * scale)


def point_segment_distance(point, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    norm = dx * dx + dy * dy
    fraction = 0.0 if norm == 0 else max(0.0, min(1.0, ((point[0] - a[0]) * dx + (point[1] - a[1]) * dy) / norm))
    return math.hypot(point[0] - a[0] - fraction * dx, point[1] - a[1] - fraction * dy)


def samples(line, latitude):
    points = [xy(point, latitude) for point in line]
    output = [points[0]]
    for a, b in zip(points, points[1:]):
        length = math.dist(a, b)
        steps = max(1, math.ceil(length / 10.0))
        output.extend((a[0] + (b[0] - a[0]) * i / steps,
                       a[1] + (b[1] - a[1]) * i / steps) for i in range(1, steps + 1))
    return output


def geometry_difference(first, second):
    latitude = (first[0][1] + second[0][1]) / 2
    a, b = samples(first, latitude), samples(second, latitude)
    def distances(points, line):
        segments = list(zip(line, line[1:])) or [(line[0], line[0])]
        return [min(point_segment_distance(point, x, y) for x, y in segments) for point in points]
    distances_both = distances(a, b) + distances(b, a)
    return {"symmetric_max_deviation_meters": round(max(distances_both), 3),
            "symmetric_mean_deviation_meters": round(sum(distances_both) / len(distances_both), 3),
            "sampling_step_meters": 10}


def reference_route(base_url, query):
    endpoints = ";".join(f"{point[0]:.7f},{point[1]:.7f}" for point in (query["origin"], query["destination"]))
    url = f"{base_url}/route/v1/driving/{endpoints}?overview=full&geometries=geojson&steps=false"
    try:
        result = request(url)
    except urllib.error.HTTPError as error:
        result = json.load(error)
    code = result.get("code")
    if code == "NoRoute":
        return {"status": "unreachable", "osrm_code": code}
    if code != "Ok" or len(result.get("routes", [])) != 1:
        return {"status": "reference_error", "osrm_code": code, "message": result.get("message")}
    route = result["routes"][0]
    return {"status": "reachable", "osrm_code": code,
            "distance_meters": route["distance"], "duration_seconds": route["duration"],
            "geometry": route["geometry"]["coordinates"],
            "snapped_origin": result["waypoints"][0]["location"],
            "snapped_destination": result["waypoints"][1]["location"],
            "origin_snap_meters": round(math.dist(xy(query["origin"], query["origin"][1]), xy(result["waypoints"][0]["location"], query["origin"][1])), 3),
            "destination_snap_meters": round(math.dist(xy(query["destination"], query["destination"][1]), xy(result["waypoints"][1]["location"], query["destination"][1])), 3)}


def compare(base_url, exported):
    rows = []
    for query in exported["queries"]:
        reference = reference_route(base_url, query)
        row = {"id": query["id"], "from_osm_node": query["from_osm_node"],
               "to_osm_node": query["to_osm_node"], "origin": query["origin"],
               "destination": query["destination"],
               "expected_status": query["expected_status"],
               "roadrunner_matches_expected": query["status"] == query["expected_status"],
               "roadrunner": {key: query[key] for key in ("status", "distance_meters", "duration_seconds")},
               "reference": {key: value for key, value in reference.items() if key != "geometry"},
               "status_agrees": query["status"] == reference["status"]}
        if query["status"] == reference["status"] == "reachable":
            row["distance_difference_meters"] = round(query["distance_meters"] - reference["distance_meters"], 3)
            row["duration_difference_seconds"] = round(query["duration_seconds"] - reference["duration_seconds"], 3)
            row["geometry_difference"] = geometry_difference(query["geometry"], reference["geometry"])
        rows.append(row)
    reachable_both = [r for r in rows if r["roadrunner"]["status"] == r["reference"]["status"] == "reachable"]
    return {"query_count": len(rows), "status_agreement_count": sum(r["status_agrees"] for r in rows),
            "roadrunner_expected_status_count": sum(r["roadrunner_matches_expected"] for r in rows),
            "both_reachable_count": len(reachable_both),
            "reference_failure_count": sum(r["reference"]["status"] == "reference_error" for r in rows),
            "median_absolute_distance_difference_meters": median([abs(r["distance_difference_meters"]) for r in reachable_both]),
            "median_absolute_duration_difference_seconds": median([abs(r["duration_difference_seconds"]) for r in reachable_both]),
            "median_symmetric_max_geometry_deviation_meters": median([r["geometry_difference"]["symmetric_max_deviation_meters"] for r in reachable_both]),
            "queries": rows}


def median(values):
    if not values:
        return None
    values = sorted(values)
    center = len(values) // 2
    return round(values[center] if len(values) % 2 else (values[center - 1] + values[center]) / 2, 3)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source_hash = hashlib.sha256(FIXTURE.read_bytes()).hexdigest()
    with tempfile.TemporaryDirectory(prefix="roadrunner-phase8-") as directory:
        work = Path(directory)
        pbf = work / "map.osm.pbf"
        pbf.write_bytes(FIXTURE.read_bytes())
        command("cargo", "run", "--locked", "-q", "-p", "roadrunner-cli", "--", "osm", "extract", str(pbf), str(work / "map.rr-osm"), "--source-id", "phase8-lagos-marina")
        command("cargo", "run", "--locked", "-q", "-p", "roadrunner-cli", "--", "osm", "compile", str(work / "map.rr-osm"), str(work / "snapshot"))
        exported = json.loads(command("cargo", "run", "--locked", "-q", "-p", "roadrunner-cli", "--", "route", "corpus", str(work / "snapshot"), str(CORPUS)))
        if exported["source_sha256"] != source_hash or exported["fixture"] != FIXTURE.name:
            raise ValueError("Roadrunner export does not match the reference fixture")
        mount = f"{work}:/data"
        command("docker", "run", "--rm", "-v", mount, IMAGE, "osrm-extract", "-p", "/opt/car.lua", "/data/map.osm.pbf", capture=False)
        command("docker", "run", "--rm", "-v", mount, IMAGE, "osrm-contract", "/data/map.osrm", capture=False)
        container = command("docker", "run", "--rm", "-d", "-p", "127.0.0.1::5000", "-v", mount, IMAGE, "osrm-routed", "--algorithm", "ch", "/data/map.osrm")
        try:
            port = command("docker", "port", container, "5000/tcp").rsplit(":", 1)[-1]
            base_url = f"http://127.0.0.1:{port}"
            for attempt in range(30):
                try:
                    request(f"{base_url}/nearest/v1/driving/3.3792,6.5244")
                    break
                except (urllib.error.URLError, TimeoutError):
                    time.sleep(0.2)
            else:
                raise RuntimeError("OSRM server did not start")
            comparison = compare(base_url, exported)
        finally:
            command("docker", "stop", container)
    output = {"schema_version": 1, "dataset": {"fixture": str(FIXTURE.relative_to(ROOT)),
              "pbf_sha256": source_hash, "corpus": str(CORPUS.relative_to(ROOT)),
              "corpus_sha256": hashlib.sha256(CORPUS.read_bytes()).hexdigest(),
              "graph_snapshot_digest": exported["graph_snapshot_digest"],
              "graph_node_count": exported["graph_node_count"],
              "graph_edge_count": exported["graph_edge_count"]},
              "roadrunner": {"routing_profile": exported["routing_profile"],
                             "algorithm": "dijkstra", "cost_model": "travel_time"},
              "reference": {"engine": "OSRM", "version": "5.27.1", "image": IMAGE,
                            "profile": "stock car.lua", "algorithm": "CH"},
              "source_files_sha256": {
                  str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
                  for path in (ROOT / "benchmarks/reference-comparison/run.py",
                               ROOT / "crates/roadrunner-cli/src/reference_comparison.rs",
                               ROOT / "Cargo.lock")},
              "environment": {"platform": platform.platform(), "machine": platform.machine()},
              "comparison": comparison}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps({"output": str(args.output), **{k: v for k, v in comparison.items() if k != "queries"}}, indent=2))


if __name__ == "__main__":
    main()
