#!/usr/bin/env python3
"""Actual-process crash/restart and SQL fault verification on a disposable database."""
import argparse
import concurrent.futures
import copy
import hashlib
import json
import os
import secrets
import signal
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/debug/roadrunner-api")
    parser.add_argument("--graph", default="data/fixtures/phase-10/snapshot.semantic-v2/graph.rr-graph")
    parser.add_argument("--output", default="docs/evidence/phase-21-runtime.json")
    parser.add_argument("--database-url", default=os.environ.get("ROADRUNNER_TEST_DATABASE_URL"))
    args = parser.parse_args()
    if not args.database_url:
        parser.error("disposable --database-url or ROADRUNNER_TEST_DATABASE_URL required")
    binary = str(Path(args.binary).resolve())
    key = "process-verify-" + uuid.uuid4().hex
    token = secrets.token_hex(32)
    anchor_directory=tempfile.TemporaryDirectory(prefix="roadrunner-independent-anchor-")
    env = dict(os.environ, ROADRUNNER_API_TOKEN=token, ROADRUNNER_DATABASE_URL=args.database_url,ROADRUNNER_DURABILITY_ANCHOR=str(Path(anchor_directory.name)/"world.head"))
    subprocess.run([binary, "--migrate"], env=env, check=True, capture_output=True)
    graph = json.loads(json.loads(Path(args.graph).read_text())["payload_json"])
    def location(node):
        p = graph["nodes"][node]
        return {"graph_digest": graph["snapshot_digest"], "node": node,
                "coordinate": {"latitude": p["latitude_e7"] / 1e7, "longitude": p["longitude_e7"] / 1e7}}
    def sql(statement, expect_failure=False):
        result = subprocess.run(["psql", "-X", "-v", "ON_ERROR_STOP=1", "-At", "-d", args.database_url, "-c", statement], text=True, capture_output=True)
        if expect_failure:
            assert result.returncode != 0, "SQL mutation unexpectedly succeeded"
            return result.stderr
        if result.returncode:
            raise RuntimeError(result.stderr)
        return result.stdout.strip()
    process = None
    base = ""
    namespace = ""
    results = []
    log = tempfile.TemporaryFile()
    def request(method, path, payload=None, idem=None, lookup=False):
        headers = {"Authorization": "Bearer " + token, "Content-Type": "application/json"}
        if idem:
            headers["Idempotency-Key"] = idem
        if lookup:
            headers["X-Roadrunner-Namespace"] = namespace
        body = None if payload is None else json.dumps(payload).encode()
        try:
            response = urllib.request.urlopen(urllib.request.Request(base + path, body, headers, method=method), timeout=40)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            return response.status, json.load(response)
    def get_state():
        status, state = request("GET", "/v1/state")
        assert status == 200, (status, state)
        return state
    def start():
        nonlocal process, base, namespace
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        base = f"http://127.0.0.1:{port}"
        for _ in range(80):
            process = subprocess.Popen([binary, "--trusted-local", args.graph, "--postgres", key], env=dict(env, ROADRUNNER_API_BIND=f"127.0.0.1:{port}"), stdout=log, stderr=log)
            for _ in range(100):
                if process.poll() is not None:
                    break
                try:
                    state = get_state()
                    if namespace:
                        assert state["namespace"] == namespace
                    namespace = state["namespace"]
                    return state
                except (OSError, urllib.error.URLError):
                    time.sleep(.03)
            if process.poll() is None:
                process.kill()
                process.wait()
            time.sleep(.05)
        log.seek(0)
        raise RuntimeError("server startup failed: " + log.read().decode())
    def crash():
        process.send_signal(signal.SIGKILL)
        process.wait(timeout=5)
    def command(path, idem, intent):
        status, result = request("POST", path, dict(intent, namespace=namespace), idem)
        assert status == 200, (path, status, result)
        return result
    def observation(identity, sequence, at):
        return {"source": "trusted-local", "observation_id": identity, "sequence": sequence, "observed_at_seconds": at}
    def at():
        return get_state()["now_seconds"]
    def state_facts(state):
        return {k: state[k] for k in ("namespace", "revision", "time_domain", "world", "plan_revisions", "execution", "contexts", "history", "publications")}
    try:
        start()
        rider_intent = {"location": location(0), "capacity": 3, "available": True}
        original = command("/v1/riders", "created-rider", rider_intent)
        rider = original["result"]["rider"]
        ready = at()
        order_result = command("/v1/orders", "created-order", {"pickup": location(0), "dropoff": location(1), "demand": 1,
                                                              "policy": "local", "expected_ready_seconds": ready,
                                                              "forecast_valid_until_seconds": ready + 600})
        order = order_result["result"]["order"]
        command("/v1/traffic", "traffic-original", {"graph_digest": graph["snapshot_digest"], "overrides": [{"edge": 0, "multiplier": 2}]})
        dispatch = command("/v1/dispatch", "dispatch", {"order": order})
        assert dispatch["published"] is True
        command(f"/v1/orders/{order}/ready", "ready", {"observation": observation("ready", 1, at())})
        _, rider_state = request("GET", f"/v1/riders/{rider}")
        started = command(f"/v1/riders/{rider}/actions/start", "start-pickup", {"plan_revision": rider_state["plan_revision"]})
        command("/v1/traffic", "traffic-new", {"graph_digest": graph["snapshot_digest"], "overrides": [{"edge": 0, "multiplier": 3}]})
        frozen = state_facts(get_state())
        crash()
        token=secrets.token_hex(32)
        env["ROADRUNNER_API_TOKEN"]=token  # Same stable principal, rotated credential.
        assert state_facts(start()) == frozen, "crash restoration changed complete authority/frozen execution"
        assert command("/v1/riders", "created-rider", rider_intent) == original
        _, lookup = request("GET", "/v1/commands/dispatch", lookup=True)
        assert lookup == dispatch, "lost-response retry did not return authentic original publication"
        results.append({"case": "SIGKILL and frozen restoration", "result": "PASS", "namespace": namespace,
                        "revision": frozen["revision"], "action": started["result"]["action"], "generation": started["result"]["generation"],
                        "preserved": ["world", "execution", "terms", "contexts", "history", "publications", "command outcomes"]})
        last_completion = None
        for index in range(2):
            if index:
                _, rider_state = request("GET", f"/v1/riders/{rider}")
                started = command(f"/v1/riders/{rider}/actions/start", "start-dropoff", {"plan_revision": rider_state["plan_revision"]})
            effect = {"action": started["result"]["action"], "generation": started["result"]["generation"],
                      "effect_id": namespace.replace("world:0", f"effect:{index*10+1}"),
                      "observation": observation(f"arrival-{index}", index*2+1, at())}
            command(f"/v1/riders/{rider}/actions/arrival", f"arrival-{index}", effect)
            command(f"/v1/riders/{rider}/actions/service", f"service-{index}", {})
            effect = dict(effect, effect_id=namespace.replace("world:0", f"effect:{index*10+2}"),
                          observation=observation(f"completion-{index}", index*2+2, at()))
            command(f"/v1/riders/{rider}/actions/completion", f"completion-{index}", effect)
            last_completion = effect
        completed = state_facts(get_state())
        assert "Delivered" in completed["world"]["fulfillment"]["1"]
        crash()
        assert state_facts(start()) == completed
        alias = copy.deepcopy(last_completion)
        alias["effect_id"] = namespace.replace("world:0", "effect:13")
        alias["observation"] = observation("completion-alias-after-restart", 5, last_completion["observation"]["observed_at_seconds"])
        repeated = command(f"/v1/riders/{rider}/actions/completion", "completion-alias", alias)
        assert repeated["published"] is False and repeated["result"]["effect"]["effect"] == "already_applied"
        assert get_state()["revision"] == completed["revision"]
        results.append({"case": "cross-command effect recognition after restart", "result": "PASS", "revision": completed["revision"], "effect": "already_applied"})
        route_status, route = request("POST", "/v1/routes", {"origin": location(0), "destination": location(1), "algorithm": "astar"})
        assert route_status == 200 and route["route_records"]
        assert sql(f"SELECT bool_and(ST_SRID(geometry)=4326 AND ST_IsValid(geometry)) FROM rr_routes WHERE namespace='{namespace}'") == "t"
        assert sql(f"SELECT bool_and(ST_IsValid(geometry)) FROM rr_execution_routes WHERE namespace='{namespace}'") == "t"
        live_before = get_state()["world"]
        scenario = {"schema_version": 1, "seed": 7, "start_seconds": 0, "end_seconds": 300, "routing_epoch_seconds": 0,
                    "graph_snapshot_digest": graph["snapshot_digest"], "dispatch": {"kind": "basic"},
                    "riders": [{"id": 1, "node": 0, "capacity": 1, "available": True}],
                    "orders": [{"id": 1, "pickup_node": 0, "dropoff_node": 1, "created_at_seconds": 0, "demand": 1,
                                "actual_readiness": {"kind": "fixed", "at_seconds": 0}}]}
        simulation = command("/v1/simulations", "simulation", {"scenario": scenario})
        assert simulation["result"]["summary"]["delivered_orders"] == 1
        assert get_state()["world"] == live_before
        assert sql(f"SELECT count(*) FROM rr_simulations WHERE namespace='{namespace}'") == "1"
        results.append({"case": "spatial routes and isolated simulation persistence", "result": "PASS", "route_records": route["route_records"]})
        # Pause a real transaction after its committed reservation and partial relational write.
        sql("CREATE OR REPLACE FUNCTION rr_test_pause() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_sleep(8); RETURN NEW; END $$; CREATE TRIGGER rr_test_pause AFTER INSERT ON rr_riders FOR EACH ROW EXECUTE FUNCTION rr_test_pause();")
        with concurrent.futures.ThreadPoolExecutor(max_workers=1) as executor:
            lost = executor.submit(request, "POST", "/v1/riders", dict(rider_intent, namespace=namespace), "interrupted-rider")
            for _ in range(100):
                if sql(f"SELECT count(*) FROM pg_stat_activity WHERE application_name='roadrunner/{key}' AND wait_event='PgSleep'") == "1":
                    break
                time.sleep(.02)
            else:
                raise AssertionError("fault injection did not reach SQL write")
            assert sql(f"SELECT body#>>'{{envelope,outcome}}' FROM rr_commands WHERE namespace='{namespace}' AND key='interrupted-rider'") == "pending"
            crash()
            try:
                lost.result(timeout=15)
            except (OSError, urllib.error.URLError):
                pass
            sql("DROP TRIGGER rr_test_pause ON rr_riders; DROP FUNCTION rr_test_pause();")
        restored = start()
        _, interrupted = request("GET", "/v1/commands/interrupted-rider", lookup=True)
        assert interrupted["outcome"] == "interrupted_before_publication" and interrupted["published"] is False
        assert len(restored["world"]["riders"]) == 1
        results.append({"case": "SIGKILL after reservation and partial SQL writes", "result": "PASS",
                        "outcome": interrupted["outcome"], "riders": 1})
        # Discard the actual HTTP response, then kill after SQL terminal publication.
        lost_intent=dict(rider_intent,namespace=namespace,available=False)
        raw=json.dumps(lost_intent).encode()
        port=int(base.rsplit(":",1)[1])
        with socket.create_connection(("127.0.0.1",port),timeout=5) as stream:
            headers=(f"POST /v1/riders HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nIdempotency-Key: lost-http-response\r\nContent-Type: application/json\r\nContent-Length: {len(raw)}\r\nConnection: close\r\n\r\n").encode()
            stream.sendall(headers+raw)
            for _ in range(100):
                if sql(f"SELECT count(*) FROM rr_commands WHERE namespace='{namespace}' AND key='lost-http-response'")=="1":
                    break
                time.sleep(.02)
            else:
                raise AssertionError("request was not reserved before disconnect")
        for _ in range(100):
            if sql(f"SELECT body#>>'{{envelope,outcome}}' FROM rr_commands WHERE namespace='{namespace}' AND key='lost-http-response'")=="committed":
                break
            time.sleep(.02)
        else:
            raise AssertionError("disconnected command did not finish")
        expected_lost=json.loads(sql(f"SELECT body->'envelope' FROM rr_commands WHERE namespace='{namespace}' AND key='lost-http-response'"))
        crash()
        start()
        status,resolved=request("POST","/v1/riders",lost_intent,"lost-http-response")
        assert status==200 and resolved==expected_lost
        assert len(get_state()["world"]["riders"])==2
        results.append({"case":"lost HTTP response after SQL commit and SIGKILL","result":"PASS","command":resolved["command"],"one_effect":True})
        # Delay the deferred COMMIT boundary until the certified proposal expires.
        sql("CREATE OR REPLACE FUNCTION rr_test_expire() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.certified_until IS NOT NULL THEN PERFORM pg_sleep(3); END IF; RETURN NEW; END $$; CREATE CONSTRAINT TRIGGER aa_test_expire AFTER UPDATE ON rr_worlds DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION rr_test_expire();")
        order2 = command("/v1/orders", "expiry-order", {"pickup": location(0), "dropoff": location(1), "demand": 1, "policy": "local",
                                                       "expected_ready_seconds": at(), "forecast_valid_until_seconds": at()+600})["result"]["order"]
        status, expired = request("POST", "/v1/dispatch", {"namespace": namespace, "order": order2}, "expiry-dispatch")
        sql("DROP TRIGGER aa_test_expire ON rr_worlds; DROP FUNCTION rr_test_expire();")
        assert status in (409, 503), (status, expired)
        assert not get_state()["world"]["assignments"], "expired SQL publication leaked assignment"
        assert "2" not in get_state()["world"]["accepted"], "expired publication leaked accepted terms"
        _, original_expiry = request("GET", "/v1/commands/expiry-dispatch", lookup=True)
        assert original_expiry["outcome"] == "rejected" and not original_expiry["published"]
        results.append({"case": "certificate expiry during deferred SQL COMMIT", "result": "PASS", "http_status": status,
                        "original_outcome": original_expiry["outcome"]})
        # Retain a caller watermark, and prove that a changed watermark cannot restore.
        head = json.loads(sql(f"SELECT row_to_json(t) FROM (SELECT storage_revision AS revision,content_sha256 AS digest FROM rr_worlds WHERE world_key='{key}') t"))
        subprocess_result = subprocess.run([binary, "--trusted-local", args.graph, "--postgres", key], env=dict(env, ROADRUNNER_EXPECTED_HEAD=json.dumps(head)), capture_output=True, timeout=10)
        assert subprocess_result.returncode != 0, "second live writer was accepted"
        assert sql(f"SELECT count(*) FROM rr_outbox WHERE namespace='{namespace}'") != "0"
        sql(f"UPDATE rr_outbox SET body='{{}}' WHERE namespace='{namespace}'", expect_failure=True)
        results.append({"case": "writer exclusion and immutable outbox", "result": "PASS", "head": head})
        document = {"schema_version": 1, "world_key": key, "database": sql("SELECT version()"),
                    "postgis": sql("SELECT PostGIS_Full_Version()"), "dataset_sha256": hashlib.sha256(Path(args.graph).read_bytes()).hexdigest(),
                    "binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(), "cases": results,
                    "observations": "Explicit synthetic trusted-local test observations; no physical-source integration claimed."}
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(document, indent=2)+"\n")
        print(output)
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        sql("DROP TRIGGER IF EXISTS rr_test_pause ON rr_riders; DROP TRIGGER IF EXISTS aa_test_expire ON rr_worlds; DROP FUNCTION IF EXISTS rr_test_pause(); DROP FUNCTION IF EXISTS rr_test_expire();")
        log.close()
        anchor_directory.cleanup()


if __name__ == "__main__":
    main()
