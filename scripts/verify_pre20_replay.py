#!/usr/bin/env python3
"""Compare archived Phase 15–19 behavior and exactly replay current versioned evidence.

Only declared graph/traffic identity changes, the new execution-evidence version,
and frozen-prefix logical ActionId representation may differ. Every other leaf,
including timing, terms, ownership, stop order, work counts and metrics, must match.
"""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path


def differences(old, new, path=""):
    if type(old) is not type(new):
        raise AssertionError((path, type(old), type(new)))
    result = []
    if isinstance(old, dict):
        for key in sorted(old.keys() | new.keys()):
            where = f"{path}/{key}"
            if key not in old or key not in new:
                assert key == "runtime_semantics" and key in new
                assert new[key] == "shared-execution/v2"
                result.append(where)
            else:
                result.extend(differences(old[key], new[key], where))
    elif isinstance(old, list):
        assert len(old) == len(new), path
        for i, (left, right) in enumerate(zip(old, new)):
            result.extend(differences(left, right, f"{path}/{i}"))
    elif old != new:
        leaf = path.rsplit("/", 1)[-1]
        semantic_digest = leaf in ["graph_digest", "graph_snapshot_digest"] or path.endswith(("/traffic/Static", "/traffic/TimeDependent"))
        frozen_identity = leaf == "execution_id" and ("/frozen/" in path or "/unavailable_projections/" in path)
        assert semantic_digest or frozen_identity, (path, old, new)
        if semantic_digest:
            assert all(isinstance(v, str) and len(v) == 64 and all(c in "0123456789abcdef" for c in v) for v in [old, new])
        else:
            assert isinstance(old, int) and isinstance(new, int)
        result.append(path)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/roadrunner"))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    rows = []
    for phase in range(15, 20):
        for fixture in sorted(Path(f"data/fixtures/phase-{phase}").glob("*.json")):
            tail = ["simulate", str(fixture), "--json"]
            old = subprocess.check_output([str(args.baseline.resolve()), *tail])
            new = subprocess.check_output([str(args.binary.resolve()), *tail])
            replay = subprocess.check_output([str(args.binary.resolve()), *tail])
            assert new == replay, fixture
            changed = differences(json.loads(old), json.loads(new))
            rows.append({"fixture": str(fixture), "fixture_sha256": hashlib.sha256(fixture.read_bytes()).hexdigest(),
                         "baseline_evidence_sha256": hashlib.sha256(old).hexdigest(), "current_evidence_sha256": hashlib.sha256(new).hexdigest(),
                         "exact_current_replay": True, "all_other_semantic_fields_equal": True, "declared_changed_paths": changed})
    record = {"schema_version": 1, "baseline_commit": "4271ddd", "baseline_binary_sha256": hashlib.sha256(args.baseline.read_bytes()).hexdigest(),
              "current_binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
              "normalization_contract": "graph semantic identity v2, derived traffic identities, shared-execution/v2 envelope and immutable frozen ActionId only; no timing/assignment/protection/objective/metric normalization",
              "fixtures": rows}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2) + "\n")
    print(f"Verified {len(rows)} baseline comparisons and exact current replays")


if __name__ == "__main__":
    main()
