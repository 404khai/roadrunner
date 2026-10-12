#!/usr/bin/env python3
"""Actual publisher/consumer process crashes against the selected PostgreSQL transport."""
import argparse
import json
import os
import signal
from pathlib import Path
from event_runtime_fixture import Fixture


def facts(state):
    return {key: state[key] for key in ("namespace", "revision", "time_domain", "plan_revisions", "world", "execution", "contexts", "history", "publications")}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--database-url", default=os.environ.get("ROADRUNNER_TEST_DATABASE_URL"))
    p.add_argument("--api-binary", default="target/debug/roadrunner-api")
    p.add_argument("--event-binary", default="target/debug/roadrunner-events")
    p.add_argument("--graph", default="data/fixtures/phase-10/snapshot.semantic-v2/graph.rr-graph")
    p.add_argument("--output", default="docs/evidence/phase-22-runtime.json")
    a = p.parse_args()
    if not a.database_url:
        p.error("dedicated disposable database required")
    cases = []
    for table, consumer, point in [
        ("rr_stream", None, "publisher before append COMMIT"),
        ("rr_publish_ack", None, "publisher after append before transport ACK"),
        ("rr_consumer_effects", "projection-v1", "consumer after partial effect before inbox COMMIT"),
        ("rr_delivery_ack", "projection-v1", "consumer after effect/inbox COMMIT before delivery ACK"),
    ]:
        f = Fixture(a.database_url, a.api_binary, a.event_binary, a.graph)
        try:
            original_command=f.rider()
            origin_ids=json.loads(f.sql(f"SELECT json_agg(event_id ORDER BY storage_revision,ordinal) FROM rr_outbox WHERE namespace='{f.namespace}'"))
            initial = f.request("GET", "/v1/state")
            f.pause(table, consumer)
            worker = f.events()
            f.barrier()
            f.kill(worker)
            f.cleanup_hook()
            if table == "rr_stream":
                assert f.count("rr_stream") == 0 and f.count("rr_publish_ack") == 0
            if table == "rr_publish_ack":
                assert f.count("rr_stream") == 1 and f.count("rr_publish_ack") == 0
            if table == "rr_consumer_effects":
                assert f.count("rr_consumer_effects") == 0 and f.count("rr_inbox") == 0
            if table == "rr_delivery_ack":
                assert f.count("rr_consumer_effects") == 1 and f.count("rr_inbox") == 1 and f.count("rr_delivery_ack") == 0
            crash_visible={table:f.count(table) for table in ("rr_stream","rr_publish_ack","rr_consumer_effects","rr_inbox","rr_delivery_ack")}
            status = f.run()
            assert status["outbox_pending"] == 0 and status["quarantined"] == 0 and status["deferred"] == 0, status
            n = f.count("rr_outbox")
            assert n == 2 and f.count("rr_stream") == n and f.count("rr_publish_ack") == n
            assert f.count("rr_inbox") == 2*n and f.count("rr_consumer_effects") == 2*n and f.count("rr_notifications") == n and f.count("rr_delivery_ack") == 2*n
            assert facts(f.request("GET", "/v1/state")) == facts(initial), "event consumers changed operational authority"
            before = f.count("rr_event_attempts")
            replay = f.run("--replay", ("projection-v1",))
            assert replay["external_effects_suppressed"] and len(replay["projection"]) == 3
            assert f.count("rr_event_attempts") == before
            assert f.run()["outbox_pending"] == 0 and f.count("rr_consumer_effects") == 2*n
            cases.append({"case": "SIGKILL " + point, "result": "PASS", "namespace": f.namespace,
                          "initial_operational_revision":initial["revision"],"original_command":original_command["command"],"origin_event_ids":origin_ids,"after_SIGKILL_visible_rows":crash_visible,
                          "outbox": n, "stream": n, "consumer_effects": 2*n, "notifications": n,
                          "operational_state_unchanged": True, "replay_projection_digest": replay["projection_digest"]})
        finally:
            f.close()
    f = Fixture(a.database_url, a.api_binary, a.event_binary, a.graph)
    try:
        worker = f.events("--run")
        f.wait(lambda: f.sql(f"SELECT count(*) FROM rr_event_heads WHERE namespace='{f.namespace}'") == "3")
        competing = f.events("--once", environment={"ROADRUNNER_EVENT_ANCHORS": str(Path(f.directory.name)/"competing")})
        competing.communicate(timeout=10)
        assert competing.returncode != 0, "concurrent publisher was not excluded"
        for _ in range(3):
            f.rider()
        f.wait(lambda: f.count("rr_delivery_ack") == 12, timeout=30)
        state = f.request("GET", "/v1/state")
        assert f.run("--inspect")["outbox_pending"]==0
        worker.send_signal(signal.SIGINT)
        worker.communicate(timeout=20)
        if worker.returncode:
            f.log.seek(0)
            raise AssertionError(f.log.read().decode())
        replay = f.run("--replay", ("projection-v1",))
        for field in ("profiles", "riders", "plans"):
            for key, value in state["world"][field].items():
                assert replay["projection"][f"world/{field}/{key}"] == value
        assert facts(f.request("GET", "/v1/state")) == facts(state)
        cases.append({"case": "asynchronous live delivery, writer exclusion, isolated compatible replay", "result": "PASS", "riders": 3, "stream": 6, "delivery_acknowledgements": 12, "operational_revision": state["revision"]})
        artifact = dict(schema_version=1, transport="PostgreSQL immutable local stream/v1", cases=cases, **f.identity())
        Path(a.output).parent.mkdir(parents=True, exist_ok=True)
        Path(a.output).write_text(json.dumps(artifact, indent=2)+"\n")
        print(a.output)
    finally:
        f.close()


if __name__ == "__main__":
    main()
