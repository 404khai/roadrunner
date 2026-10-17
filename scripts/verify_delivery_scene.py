#!/usr/bin/env python3
"""Reproduce Phase 30 scenes against a local API; never modifies operational state.

ROADRUNNER_API_URL (default http://127.0.0.1:3110) and ROADRUNNER_API_TOKEN
must point to the trusted-local API using the checked-in Phase 10 graph.
"""
import json
import os
from pathlib import Path
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
BASE = os.environ.get("ROADRUNNER_API_URL", "http://127.0.0.1:3110").rstrip("/")
TOKEN = os.environ["ROADRUNNER_API_TOKEN"]


def request(path, body=None):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(BASE + path, data=data, headers={
        "Authorization": "Bearer " + TOKEN, "Content-Type": "application/json"
    })
    with urllib.request.urlopen(req, timeout=60) as response:
        return json.load(response)


def authority(state):
    return {key: state[key] for key in [
        "namespace", "revision", "world", "execution", "plan_revisions", "publications"
    ]}


def main():
    before = authority(request("/v1/state"))
    captures = []
    for kind in ["solo", "fleet", "delayed", "traffic", "unfinished", "distributed"]:
        scenario = json.loads((ROOT / f"docs/evidence/phase-30/{kind}-scenario.json").read_text())
        result = request("/v1/visualization/deliveries", scenario)
        expected = json.loads((ROOT / f"web/tests/delivery-{kind}.json").read_text())
        assert result == expected, f"{kind}: engine output differs from recorded source fixture"
        captures.append({"scenario": kind, "frames": len(result["frames"]),
                         "actions": len(result["routes"]),
                         "schedule_changes": len(result["schedules"]),
                         "accepted_effects": len(result["effects"]),
                         "delivered": result["summary"]["delivered_orders"],
                         "run_identity": result["run_identity"]})
    assert before == authority(request("/v1/state")), "visualization changed operational authority"
    proof = {"semantics": "phase30-engine-reproduction/v1", "scenes": captures,
             "live_authority_unchanged": True, "physical_gps": False,
             "source": "real local Roadrunner API over checked-in phase-10 graph; synthetic scenarios"}
    target = ROOT / "docs/evidence/phase-30/reproduction.json"
    target.write_text(json.dumps(proof, indent=2) + "\n")
    print(json.dumps(proof, indent=2))


if __name__ == "__main__":
    main()
