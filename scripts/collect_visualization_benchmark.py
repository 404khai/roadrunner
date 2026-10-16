#!/usr/bin/env python3
"""Measure Phase 26 portable export and loopback HTTP; build release targets first."""
import argparse
import concurrent.futures
import hashlib
import json
import math
import os
import platform
import re
import secrets
import socket
import statistics
import subprocess
import tempfile
import threading
import time
import urllib.request
from pathlib import Path


def stats(values):
    ordered = sorted(values)
    return dict(raw=values, median=statistics.median(values),
                p95=ordered[math.ceil(.95 * len(values)) - 1],
                p99=ordered[math.ceil(.99 * len(values)) - 1])


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', default='benchmarks/results/phase-26-visualization.json')
    p.add_argument('--runs', type=int, default=31)
    a = p.parse_args()
    if a.runs < 3:
        p.error('at least three HTTP runs required')
    cpu = subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip() if platform.system() == 'Darwin' else platform.processor()
    with tempfile.TemporaryDirectory(prefix='roadrunner-p26-') as tmp:
        graph = Path(tmp) / 'graph.json'
        example = ['target/release/examples/visualization_evidence', str(graph)]
        # macOS time reports actual peak resident size and process CPU across all example workloads.
        if platform.system() == 'Darwin':
            completed = subprocess.run(['/usr/bin/time', '-l', *example], check=True, capture_output=True, text=True)
        else:
            completed = subprocess.run(example, check=True, capture_output=True, text=True)
        core = json.loads(completed.stdout)
        payload = json.loads(json.loads(graph.read_bytes())['payload_json'])
        def location(node):
            point = payload['nodes'][node]
            return dict(graph_digest=payload['snapshot_digest'], node=node,
                        coordinate=dict(latitude=point['latitude_e7'] / 1e7, longitude=point['longitude_e7'] / 1e7))
        with socket.socket() as probe:
            probe.bind(('127.0.0.1', 0))
            port = probe.getsockname()[1]
        token = secrets.token_hex(32)
        env = dict(os.environ, ROADRUNNER_API_TOKEN=token, ROADRUNNER_API_BIND=f'127.0.0.1:{port}', RUST_LOG='roadrunner::visualization=debug')
        log_path = Path(tmp) / 'server.log'
        def request(path, body=None, key=None):
            raw = None if body is None else json.dumps(body).encode()
            headers = {'Authorization': f'Bearer {token}', 'Content-Type': 'application/json'}
            if key:
                headers['Idempotency-Key'] = key
            started = time.perf_counter_ns()
            with urllib.request.urlopen(urllib.request.Request(f'http://127.0.0.1:{port}{path}', raw, headers, method='GET' if body is None else 'POST'), timeout=30) as r:
                data = r.read()
            return (time.perf_counter_ns() - started) / 1e6, len(data), json.loads(data)
        with log_path.open('wb') as log:
            server = subprocess.Popen(['target/release/roadrunner-api', '--trusted-local', str(graph)], env=env, stdout=log, stderr=log)
            try:
                for _ in range(100):
                    try:
                        _, _, state = request('/v1/state')
                        break
                    except OSError:
                        if server.poll() is not None:
                            raise RuntimeError(log_path.read_text())
                        time.sleep(.05)
                else:
                    raise RuntimeError('API startup timed out')
                query = dict(origin=location(0), destination=location(len(payload['nodes']) - 1), departure_time='2026-01-01T00:00:00Z', objective='travel_time')
                results = {}
                for algorithm in ['dijkstra', 'astar']:
                    query['algorithm'] = algorithm
                    samples, sizes = [], []
                    for i in range(a.runs + 3):
                        elapsed, size, result = request('/v1/visualization/routes', query)
                        assert len(result['routes'][0]['geometry']['occurrences']) == 5000
                        if i >= 3:
                            samples.append(elapsed)
                            sizes.append(size)
                    results[algorithm] = dict(milliseconds=stats(samples), payload_bytes=sorted(set(sizes)))
                request('/v1/traffic', dict(namespace=state['namespace'], graph_digest=payload['snapshot_digest'], overrides=[dict(edge=i, multiplier=1.5) for i in range(1000)]), 'benchmark-traffic')
                query['objective'] = 'traffic_aware'
                samples, sizes = [], []
                for i in range(a.runs + 3):
                    elapsed, size, result = request('/v1/visualization/routes', query)
                    assert result['provenance']['traffic_digest']
                    if i >= 3:
                        samples.append(elapsed)
                        sizes.append(size)
                results['astar_traffic_1000_overrides'] = dict(milliseconds=stats(samples), payload_bytes=sorted(set(sizes)))
                intent = dict(namespace=state['namespace'], location=location(0), capacity=1, available=True)
                for concurrent in [False, True]:
                    stop = threading.Event()
                    background_count = [0]
                    def exports():
                        while not stop.is_set():
                            request('/v1/visualization/routes', query)
                            background_count[0] += 1
                    with concurrent_futures() as executor:
                        future = executor.submit(exports) if concurrent else None
                        latencies = []
                        for i in range(a.runs):
                            elapsed, _, result = request('/v1/riders', intent, f'authority-{concurrent}-{i}')
                            assert result['published'] is True
                            latencies.append(elapsed)
                        stop.set()
                        if future:
                            future.result(timeout=30)
                    results['authority_with_export' if concurrent else 'authority_without_export'] = dict(milliseconds=stats(latencies), concurrent_exports=background_count[0])
                _, _, final = request('/v1/state')
                assert len(final['world']['riders']) == a.runs * 2
            finally:
                server.terminate()
                server.wait(timeout=10)
        # ANSI stripping permits both terminal and redirected tracing format.
        text = re.sub(r'\x1b\[[0-9;]*m', '', log_path.read_text())
        waits = [int(v) for v in re.findall(r'lock_wait_ns=(\d+)', text)]
        holds = [int(v) for v in re.findall(r'lock_hold_ns=(\d+)', text)]
        assert holds and len(holds) == len(waits)
        result = dict(schema_version=1, hardware=dict(platform=platform.platform(), cpu=cpu, logical_cpus=os.cpu_count()),
                      build=dict(profile='release', rustc=subprocess.check_output(['rustc','--version'], text=True).strip(), binary_sha256=hashlib.sha256(Path('target/release/roadrunner-api').read_bytes()).hexdigest()),
                      dataset=dict(generator='crates/roadrunner-core/examples/visualization_evidence.rs', graph_sha256=hashlib.sha256(graph.read_bytes()).hexdigest(), graph_digest=payload['snapshot_digest'], nodes=len(payload['nodes']), edges=len(payload['edges']), points=160000, riders=f'0 to {a.runs*2}', orders=0, execution=0, traffic='free flow routes then adopted 1000 static edge overrides at 1.5; concurrent authority uses adopted traffic'),
                      configuration=dict(http_runs=a.runs, warmup=3, workers=4, client='Python urllib loopback new connection; body transfer included; JSON parse excluded from timer', concurrency='1 route requester; authority comparison adds 1 requester', logging='capture-lock debug enabled', core_runs=31),
                      core=core, core_process_resource_report=completed.stderr, http=results,
                      capture_lock=dict(samples=len(holds), wait_nanoseconds=stats(waits), hold_nanoseconds=stats(holds)),
                      limits='Controlled synthetic geometry/export workload only. Core build/routing excluded from core timings. HTTP includes route computation, JSON and transfer. Core process RSS is whole-example peak, not exporter allocation or API RSS. Client and server share host; concurrent authority samples are not causal attribution or a production SLO. Full state snapshot scaling and rendering are outside scope.')
        Path(a.output).parent.mkdir(parents=True, exist_ok=True)
        Path(a.output).write_text(json.dumps(result, indent=2) + '\n')
        print(a.output)


def concurrent_futures():
    return concurrent.futures.ThreadPoolExecutor(max_workers=1)


if __name__ == '__main__':
    main()
