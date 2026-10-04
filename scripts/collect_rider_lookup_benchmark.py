#!/usr/bin/env python3
"""Collect the migrated Criterion rider lookup samples without inventing metrics."""
import argparse
import json
import platform
import statistics
import subprocess
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
    parser.add_argument("--criterion-root", type=Path, default=Path("target/criterion"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--concurrent-validation", action="store_true",
                        help="Record that workspace checks overlapped this verification run")
    args = parser.parse_args()
    results = []
    for count in [100, 1000, 10000, 100000]:
        for algorithm in ["linear", "indexed"]:
            path = args.criterion_root / "rider_lookup_5km_limit10" / algorithm / str(count) / "new/sample.json"
            sample = json.loads(path.read_text())
            if len(sample["iters"]) != len(sample["times"]) or len(sample["iters"]) != 30:
                raise ValueError(f"Expected 30 complete Criterion samples: {path}")
            values = sorted(t / n for t, n in zip(sample["times"], sample["iters"]))
            results.append({"riders": count, "algorithm": algorithm, "runs": len(values),
                            "median_ns": statistics.median(values), "p95_ns": percentile(values, .95),
                            "p99_ns": percentile(values, .99)})
    record = {"date": date.today().isoformat(), "purpose": "Phase 12 dispatch ownership migration validation",
              "hardware": hardware(), "graph_size": "not applicable: geographic rider lookup",
              "dataset": {"kind": "seeded synthetic", "seed": 12,
                          "region": "latitude 6.4..6.65, longitude 3.25..3.50",
                          "query": {"latitude": 6.5244, "longitude": 3.3792, "radius_meters": 5000, "limit": 10},
                          "rider_counts": [100, 1000, 10000, 100000]},
              "configuration": {"benchmark": "cargo bench -p roadrunner-dispatch --bench rider_lookup --locked -- --sample-size 30 --measurement-time 1",
                                "profile": "release", "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
                                "criterion_samples": 30, "construction_included": False,
                                "concurrent_workspace_validation": args.concurrent_validation,
                                "sample_statistic": "total sample nanoseconds / iterations; percentile by linear interpolation of sorted sample means"},
              "results": results}
    args.output.write_text(json.dumps(record, indent=2) + "\n")


if __name__ == "__main__":
    main()
