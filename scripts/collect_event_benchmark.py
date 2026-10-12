#!/usr/bin/env python3
"""Measure real API commit -> asynchronous local consumer delivery acknowledgement."""
import argparse
import hashlib
import json
import math
import os
import platform
import signal
import statistics
import subprocess
import time
from pathlib import Path
from event_runtime_fixture import Fixture


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--database-url",default=os.environ.get("ROADRUNNER_TEST_DATABASE_URL"))
    p.add_argument("--api-binary",default="target/release/roadrunner-api")
    p.add_argument("--event-binary",default="target/release/roadrunner-events")
    p.add_argument("--graph",default="data/fixtures/phase-10/snapshot.semantic-v2/graph.rr-graph")
    p.add_argument("--runs",type=int,default=30)
    p.add_argument("--output",default="benchmarks/results/phase-22-event-delivery.json")
    a=p.parse_args()
    if not a.database_url or a.runs<1:p.error("disposable database and positive runs required")
    f=Fixture(a.database_url,a.api_binary,a.event_binary,a.graph)
    try:
        worker=f.events("--run")
        f.wait(lambda:f.sql(f"SELECT count(*) FROM rr_event_heads WHERE namespace='{f.namespace}'")=="3")
        raw=[]
        for index in range(a.runs+3):
            # Timer starts AFTER HTTP acknowledgement of the originating SQL commit.
            f.rider()
            started=time.perf_counter_ns()
            target=4*(index+1)
            f.wait(lambda:f.count("rr_delivery_ack")==target,timeout=30)
            elapsed=(time.perf_counter_ns()-started)/1e6
            if index>=3:raw.append(elapsed)
        worker.send_signal(signal.SIGINT);worker.communicate(timeout=20)
        assert worker.returncode==0
        status=f.run("--inspect")
        assert status["outbox_pending"]==0 and status["quarantined"]==0
        replay=f.run("--replay",("projection-v1",))
        assert len(replay["projection"])==3*(a.runs+3)
        ordered=sorted(raw)
        cpu=subprocess.run(["sysctl","-n","machdep.cpu.brand_string"],capture_output=True,text=True).stdout.strip() if platform.system()=="Darwin" else platform.processor()
        artifact={"schema_version":1,"hardware":{"platform":platform.platform(),"cpu":cpu,"logical_cpus":os.cpu_count()},
                  "dataset":{"path":a.graph,"sha256":f.identity()["graph_sha256"],"graph_digest":f.graph["snapshot_digest"],"nodes":len(f.graph["nodes"]),"edges":len(f.graph["edges"])},
                  "build":{"profile":"release","rustc":subprocess.run(["rustc","+1.99.0","--version"],capture_output=True,text=True,check=True).stdout.strip(),"event_binary_sha256":f.identity()["event_binary_sha256"],"api_binary_sha256":hashlib.sha256(Path(f.api_binary).read_bytes()).hexdigest()},
                  "configuration":{"transport":"PostgreSQL local immutable stream/v1","algorithm":"serial outbox publisher + projection/notification consumers v1","poll_interval_ms":100,"measurement_poll_ms":40,"warmups":3,"concurrency":1,"world":"empty to runs+warmups riders; no orders","events_per_creation":2,"consumers":2,"acknowledgements_per_creation":4,"synchronous_commit":"on","independent_anchor":"file+parent directory fsync for publisher and each consumer"},
                  "database":{"version":f.sql("SELECT version()"),"postgis":f.sql("SELECT PostGIS_Lib_Version()"),"fsync":f.sql("SHOW fsync"),"shared_buffers":f.sql("SHOW shared_buffers")},
                  "runs":a.runs,"results":{"commit_ack_to_both_consumer_acks":{"raw_ms":raw,"median_ms":statistics.median(raw),"p95_ms":ordered[math.ceil(.95*len(ordered))-1],"p99_ms":ordered[math.ceil(.99*len(ordered))-1]}},
                  "durable_counts":status,"verification":{"effects":f.count("rr_consumer_effects"),"notifications":f.count("rr_notifications"),"delivery_acknowledgements":f.count("rr_delivery_ack"),"projection_digest":replay["projection_digest"]},
                  "resources":{"cpu_usage":"unmeasured","peak_memory":"unmeasured"},
                  "limits":["Includes psql process launch and polling observation delay; upper-bound latency sample, not precise internal service time","Small sequential local fixture and full-evidence hashing; no production scalability/SLO claim","Local notification obligation only; no remote sending"]}
        Path(a.output).parent.mkdir(parents=True,exist_ok=True);Path(a.output).write_text(json.dumps(artifact,indent=2)+"\n");print(a.output)
    finally:f.close()


if __name__=="__main__":main()
