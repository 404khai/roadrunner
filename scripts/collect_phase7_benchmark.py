"""Generate structured Phase 7 benchmark evidence from Criterion and snapshots."""

import argparse
import hashlib
import json
import math
import statistics
from pathlib import Path


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def percentiles(criterion: Path) -> dict[str, dict[str, float | int]]:
    results = {}
    for sample in sorted(criterion.glob("osm_*/new/sample.json")):
        data = json.loads(sample.read_text())
        values = sorted(
            elapsed / iterations
            for elapsed, iterations in zip(data["times"], data["iters"])
        )

        def percentile(fraction: float) -> float:
            return values[min(len(values) - 1, math.ceil(fraction * len(values)) - 1)]

        results[sample.parents[1].name] = {
            "sample_count": len(values),
            "median_ns": statistics.median(values),
            "p95_ns": percentile(0.95),
            "p99_ns": percentile(0.99),
        }
    return results


def snapshot(root: Path, name: str, pbf: Path) -> dict:
    directory = root / f"{name}-snapshot"
    manifest = json.loads((directory / "manifest.json").read_text())
    files = {
        "source_pbf": pbf.stat().st_size,
        "normalized": (root / f"{name}.rr-osm").stat().st_size,
        "graph": (directory / "graph.rr-graph").stat().st_size,
        "provenance": (directory / "graph.rr-provenance").stat().st_size,
        "manifest": (directory / "manifest.json").stat().st_size,
    }
    files["published_snapshot_total"] = files["graph"] + files["provenance"] + files["manifest"]
    return {
        "source_pbf_sha256": sha256(pbf),
        "graph_snapshot_digest": manifest["graph_snapshot_digest"],
        "sizes_bytes": files,
        "source_statistics": manifest["source_statistics"],
        "graph": {
            "normalized_nodes": manifest["normalized_node_count"],
            "normalized_ways": manifest["normalized_way_count"],
            "compiled_ways": manifest["compiled_way_count"],
            "excluded_ways": manifest["excluded_way_count"],
            "nodes": manifest["graph_node_count"],
            "segments": manifest["graph_segment_count"],
            "directed_edges": manifest["graph_edge_count"],
            "components": manifest["components"],
            "diagnostics": manifest["diagnostics"],
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("snapshot_root", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    fixtures = Path("data/fixtures/phase-7")
    result = {
        "schema_version": 1,
        "recorded_at": "2026-09-21",
        "methodology": {
            "hardware": "Apple M3, 16 GB",
            "os": "macOS 27.0 (26A428), arm64",
            "rust": "rustc 1.96.0 (ac68faa20 2026-05-25)",
            "build_profile": "release/criterion",
            "criterion": "20 samples; 1 s warm-up; 2 s target measurement",
            "memory": "/usr/bin/time -l maximum resident set size and peak memory footprint",
            "notes": "Current extractor retains candidate ways and requested nodes; no bounded-memory claim.",
        },
        "fixtures": {
            "small": snapshot(args.snapshot_root, "small", fixtures / "lagos-marina.osm.pbf"),
            "engineering": snapshot(
                args.snapshot_root,
                "engineering",
                fixtures / "lagos-island-engineering.osm.pbf",
            ),
        },
        "criterion": percentiles(Path("target/criterion")),
        "process_memory_bytes": {
            "engineering_extract": {"max_rss": 7_077_888, "peak_footprint": 5_882_216},
            "engineering_compile_publish": {"max_rss": 9_125_888, "peak_footprint": 7_782_760},
            "engineering_load_deep_verify": {"max_rss": 7_143_424, "peak_footprint": 5_767_528},
        },
    }
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
