#!/usr/bin/env python3
"""Measure the proved volatile operational boundary without HTTP/storage claims."""
import argparse
import hashlib
import json
import statistics
import re
from datetime import date
import subprocess
from pathlib import Path
from collect_simulation_benchmark import hardware, percentile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    command = ["cargo", "+1.99.0", "test", "-p", "roadrunner-dispatch", "--release", "--test", "operational", "--locked", "operational_boundary_measurement", "--", "--ignored", "--nocapture"]
    process = subprocess.run(command, text=True, capture_output=True, check=True)
    marker = "OPERATIONAL_BOUNDARY_MEASUREMENT="
    lines = [line for line in process.stdout.splitlines() if line.startswith(marker)]
    assert len(lines) == 1 and "test result: ok." in process.stdout
    record = json.loads(lines[0][len(marker):])
    for phase in ["evaluation", "certification", "publication"]:
        raw = record[f"{phase}_raw_ns"]
        assert len(raw) == record["runs"]
        ordered = sorted(raw)
        record[f"{phase}_aggregate_ns"] = {"median": statistics.median(raw), "p95": percentile(ordered, .95), "p99": percentile(ordered, .99)}
    executable = re.search(r"Running tests/operational\.rs \(([^)]+)\)", process.stderr)
    assert executable is not None
    record.update({"date": date.today().isoformat(), "binary_sha256": hashlib.sha256(Path(executable.group(1)).read_bytes()).hexdigest(),
                   "implementation_sha256": {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in [Path("crates/roadrunner-dispatch/src/operational.rs"), Path("crates/roadrunner-dispatch/src/temporal.rs")]},
                   "hardware": hardware(), "compiler": subprocess.check_output(["rustc", "+1.99.0", "--version"], text=True).strip(),
                   "command": command, "build": "release; Cargo.lock; default target features",
                   "fixture_sha256": hashlib.sha256(Path("crates/roadrunner-dispatch/tests/support/operational_fixture.rs").read_bytes()).hexdigest(),
                   "measurement": "Instant timers around separate evaluation, interval certification, complete staging/publication; fixture setup and assertions outside timers",
                   "memory": "unmeasured", "concurrent_workspace_validation": False,
                   "purpose": "small synthetic volatile correctness-boundary overhead; no HTTP latency, storage durability, contention or scalability claim"})
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2) + "\n")
    args.output.with_suffix(".log").write_text(process.stderr + process.stdout)
    print(f"Measured {record['runs']} boundary samples; wrote {args.output}")


if __name__ == "__main__":
    main()
