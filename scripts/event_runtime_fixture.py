"""Owned disposable API/event test processes. Requires local PostgreSQL/PostGIS."""
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


class Fixture:
    def __init__(self, url, api_binary, event_binary, graph):
        self.url = url
        self.api_binary = str(Path(api_binary).resolve())
        self.event_binary = str(Path(event_binary).resolve())
        self.graph_path = graph
        self.key = "ev22-" + uuid.uuid4().hex[:20]
        self.directory = tempfile.TemporaryDirectory(prefix="roadrunner-event-anchors-")
        self.log = tempfile.TemporaryFile()
        self.token = secrets.token_hex(32)
        self.env = dict(os.environ, ROADRUNNER_DATABASE_URL=url, ROADRUNNER_API_TOKEN=self.token,
                        ROADRUNNER_DURABILITY_ANCHOR=str(Path(self.directory.name) / "world.head"),
                        ROADRUNNER_EVENT_ANCHORS=str(Path(self.directory.name) / "events"))
        self.processes = []
        subprocess.run([self.api_binary, "--migrate"], env=self.env, check=True, capture_output=True)
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        self.base = f"http://127.0.0.1:{port}"
        self.api = subprocess.Popen([self.api_binary, "--trusted-local", graph, "--postgres", self.key],
                                    env=dict(self.env, ROADRUNNER_API_BIND=f"127.0.0.1:{port}"), stdout=self.log, stderr=self.log)
        self.processes.append(self.api)
        self.wait(lambda: self.ready())
        self.namespace = self.request("GET", "/v1/state")["namespace"]
        self.graph = json.loads(json.loads(Path(graph).read_text())["payload_json"])
        self.created = 0

    def ready(self):
        if self.api.poll() is not None:
            self.log.seek(0)
            raise RuntimeError(self.log.read().decode())
        try:
            return self.request("GET", "/v1/state")
        except (OSError, urllib.error.URLError):
            return False

    def request(self, method, path, payload=None, key=None):
        headers = {"Authorization": "Bearer " + self.token, "Content-Type": "application/json"}
        if key:
            headers["Idempotency-Key"] = key
        body = None if payload is None else json.dumps(payload).encode()
        with urllib.request.urlopen(urllib.request.Request(self.base + path, body, headers, method=method), timeout=40) as response:
            return json.load(response)

    def rider(self):
        self.created += 1
        p = self.graph["nodes"][0]
        location = {"graph_digest": self.graph["snapshot_digest"], "node": 0,
                    "coordinate": {"latitude": p["latitude_e7"] / 1e7, "longitude": p["longitude_e7"] / 1e7}}
        return self.request("POST", "/v1/riders", {"namespace": self.namespace, "location": location,
                                                   "capacity": 1, "available": True}, f"rider-{self.created}")

    def sql(self, statement):
        result = subprocess.run(["psql", "-X", "-v", "ON_ERROR_STOP=1", "-At", "-d", self.url, "-c", statement], text=True, capture_output=True)
        if result.returncode:
            raise RuntimeError(result.stderr)
        return result.stdout.strip()

    @staticmethod
    def wait(test, timeout=15):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            result = test()
            if result:
                return result
            time.sleep(.04)
        raise AssertionError("controlled boundary did not become visible")

    def events(self, mode="--once", extra=(), environment=None):
        p = subprocess.Popen([self.event_binary, mode, self.key, *extra], env=dict(self.env, **(environment or {})),
                             stdout=subprocess.PIPE, stderr=self.log, text=True)
        self.processes.append(p)
        return p

    def run(self, mode="--once", extra=(), environment=None):
        p = self.events(mode, extra, environment)
        out, _ = p.communicate(timeout=60)
        if p.returncode:
            self.log.seek(0)
            raise RuntimeError(self.log.read().decode())
        return json.loads(out)

    def pause(self, table, consumer=None):
        assert table in ("rr_stream", "rr_publish_ack", "rr_consumer_effects", "rr_delivery_ack")
        # Only owned UUID namespace and fixed table/consumer names enter test SQL.
        if table == "rr_publish_ack":
            condition = f"NEW.event_id LIKE '{self.namespace}:%'"
        else:
            condition = f"NEW.namespace='{self.namespace}'"
        if consumer:
            assert consumer == "projection-v1"
            condition += f" AND NEW.consumer='{consumer}'"
        self.sql(f"CREATE FUNCTION rr22_pause() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF {condition} THEN PERFORM pg_sleep(8); END IF; RETURN NEW; END $$; CREATE TRIGGER rr22_pause AFTER INSERT ON {table} FOR EACH ROW EXECUTE FUNCTION rr22_pause()")

    def barrier(self):
        self.wait(lambda: self.sql(f"SELECT count(*) FROM pg_stat_activity WHERE application_name LIKE 'roadrunner-events/{self.key}/%' AND wait_event='PgSleep'") != "0")

    def cleanup_hook(self):
        for table in ("rr_stream", "rr_publish_ack", "rr_consumer_effects", "rr_delivery_ack"):
            self.sql(f"DROP TRIGGER IF EXISTS rr22_pause ON {table}")
        self.sql("DROP FUNCTION IF EXISTS rr22_pause()")

    def kill(self, p):
        p.kill()
        p.wait(timeout=10)
        self.wait(lambda: self.sql(f"SELECT count(*) FROM pg_stat_activity WHERE application_name LIKE 'roadrunner-events/{self.key}/%'") == "0")

    def count(self, table, where=""):
        assert table in ("rr_stream", "rr_publish_ack", "rr_consumer_effects", "rr_inbox", "rr_notifications", "rr_delivery_ack", "rr_outbox", "rr_projection", "rr_event_attempts")
        ns = f"event_id LIKE '{self.namespace}:%'" if table == "rr_publish_ack" else f"namespace='{self.namespace}'"
        return int(self.sql(f"SELECT count(*) FROM {table} WHERE {ns} {where}"))

    def close(self):
        for p in self.processes:
            if p.poll() is None:
                p.kill()
                p.wait(timeout=10)
            if p.stdout:
                p.stdout.close()
        self.cleanup_hook()
        self.directory.cleanup()
        self.log.close()

    def identity(self):
        return {"world_key": self.key, "namespace": self.namespace,
                "database": self.sql("SELECT version()"), "postgis": self.sql("SELECT PostGIS_Lib_Version()"),
                "graph_sha256": hashlib.sha256(Path(self.graph_path).read_bytes()).hexdigest(),
                "event_binary_sha256": hashlib.sha256(Path(self.event_binary).read_bytes()).hexdigest()}
