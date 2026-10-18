#!/usr/bin/env python3
"""Versioned Phase 34 generation and isolated, externally bounded headless execution."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import subprocess
import time

VERSION = 'splitmix64-modulo-anchors/v1'
MASK = (1 << 64) - 1


def sha(data):
    return hashlib.sha256(data).hexdigest()


def encode(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False) + '\n').encode()


class Rng:
    def __init__(self, seed):
        self.state = seed

    def next(self):
        self.state = (self.state + 0x9e3779b97f4a7c15) & MASK
        v = self.state
        v = ((v ^ (v >> 30)) * 0xbf58476d1ce4e5b9) & MASK
        v = ((v ^ (v >> 27)) * 0x94d049bb133111eb) & MASK
        return v ^ (v >> 31)


def scenario(manifest, graph):
    required = {'schema_version', 'case_id', 'seed', 'riders', 'available_riders', 'orders',
                'capacity', 'arrival', 'geography', 'horizon_seconds', 'readiness_seconds',
                'policy', 'scenario_schema', 'admission', 'traffic_changes', 'grid'}
    if set(manifest) != required or manifest['schema_version'] != 1:
        raise ValueError('unknown or missing manifest fields/version')
    for key in ['seed', 'riders', 'available_riders', 'orders', 'capacity', 'scenario_schema']:
        if type(manifest[key]) is not int or manifest[key] < 0:
            raise ValueError(f'invalid {key}')
    if not 1 <= manifest['riders'] <= 10_000 or not 0 <= manifest['orders'] <= 100_000:
        raise ValueError('generator populations outside declared A/B/C ceiling')
    if manifest['available_riders'] > manifest['riders'] or manifest['capacity'] < 1 or manifest['seed'] > MASK:
        raise ValueError('invalid population/seed')
    if manifest['geography'] not in ['hotspot', 'dispersed']:
        raise ValueError('unsupported geography')
    for key in ['horizon_seconds', 'readiness_seconds']:
        if type(manifest[key]) not in [int, float] or not 0 <= manifest[key] < 1e9:
            raise ValueError(f'invalid {key}')
    arrival = manifest['arrival']
    if set(arrival) != {'kind', 'interval_seconds', 'burst_size', 'burst_interval_seconds'}:
        raise ValueError('invalid arrival schema')
    if arrival['kind'] not in ['sustained', 'burst', 'backlog_drain', 'mass']:
        raise ValueError('unsupported arrival process')
    if any(type(arrival[k]) not in [int, float] or not 0 <= arrival[k] < 1e9 for k in ['interval_seconds', 'burst_interval_seconds']):
        raise ValueError('invalid arrival durations')
    if type(arrival['burst_size']) is not int or arrival['burst_size'] < 1:
        raise ValueError('invalid burst size')
    if graph['nodes'] < 2:
        raise ValueError('insufficient anchors')
    rng = Rng(manifest['seed'])
    def anchor():
        # Uniform modulo dense nodes or first 2x2-node prefix. Not uniform geographic population.
        return rng.next() % (min(4, graph['nodes']) if manifest['geography'] == 'hotspot' else graph['nodes'])
    riders = [dict(id=i, node=anchor(), capacity=manifest['capacity'], available=i < manifest['available_riders']) for i in range(manifest['riders'])]
    orders = []
    for i in range(manifest['orders']):
        if arrival['kind'] == 'mass':
            at = 0
        elif arrival['kind'] == 'burst':
            at = (i // arrival['burst_size']) * arrival['burst_interval_seconds'] + (i % arrival['burst_size']) * arrival['interval_seconds']
        else:  # backlog_drain explicitly uses fast arrivals followed by the declared drain horizon.
            at = i * arrival['interval_seconds']
        ready = at + manifest['readiness_seconds']
        orders.append(dict(id=i, pickup_node=anchor(), dropoff_node=anchor(), created_at_seconds=at,
                           expected_ready_at_seconds=ready, actual_readiness=dict(kind='fixed', at_seconds=ready),
                           deadline_seconds=None, demand=1, admission=manifest['admission']))
    return dict(schema_version=manifest['scenario_schema'], scenario_id=manifest['case_id'], seed=manifest['seed'],
                start_seconds=0, end_seconds=manifest['horizon_seconds'], routing_epoch_seconds=0,
                graph_snapshot_digest=graph['graph_digest'], dispatch=manifest['policy'], riders=riders,
                orders=orders, initial_traffic=[], traffic_changes=manifest['traffic_changes'], dynamic_events=[])


def generate(manifest_path, bundle, binary, artifact=None):
    began = time.perf_counter_ns()
    manifest_bytes = manifest_path.read_bytes()
    manifest = json.loads(manifest_bytes)
    # Validate before expensive graph generation, then validate against actual qualified node count.
    scenario(manifest, dict(nodes=4, graph_digest='validation-only'))
    bundle.mkdir()
    grid = manifest['grid']
    if (grid is None) == (artifact is None):
        raise ValueError('declare exactly one grid or external artifact')
    if grid is not None:
        if set(grid) != {'side', 'spacing_e7'}:
            raise ValueError('invalid grid declaration')
        graph = json.loads(subprocess.check_output([str(binary.resolve()), 'grid', str(grid['side']), str(grid['spacing_e7']), str(bundle/'graph.rrgraph')]))
    else:
        import shutil
        shutil.copyfile(artifact, bundle/'graph.rrgraph')
        graph = json.loads(subprocess.check_output([str(binary.resolve()), 'inspect', str(bundle/'graph.rrgraph')]))
    materialized = scenario(manifest, graph)
    data = encode(materialized)
    (bundle/'scenario.json').write_bytes(data)
    (bundle/'manifest.json').write_bytes(manifest_bytes)
    metadata = dict(schema_version=1, manifest_sha256=sha(manifest_bytes), scenario_sha256=sha(data), **graph)
    metadata['generator'] = VERSION + ('+grid-e7/v1' if grid is not None else '+external-artifact')
    (bundle/'bundle.json').write_bytes(encode(metadata))
    return dict(bundle=str(bundle), generation_ns=time.perf_counter_ns()-began, scenario_bytes=len(data), **metadata)


def last_progress(path):
    if not path.exists():
        return None
    # Bounded tail; torn final lines are ignored without inventing a latest boundary.
    with path.open('rb') as file:
        file.seek(max(0, path.stat().st_size-16384))
        for line in reversed(file.read().splitlines()):
            try:
                return json.loads(line)
            except (ValueError, UnicodeDecodeError):
                continue
    return None


def run(bundle, output, binary, source_revision, seconds, rss_mib, disk_mib, observed=True, routing=None):
    if seconds <= 0 or rss_mib <= 0 or disk_mib <= 0:
        raise ValueError('positive external safety envelopes required')
    output.mkdir()
    binary = binary.resolve()
    env = dict(os.environ)
    config = dict(seconds=seconds, sampled_rss_mib=rss_mib, output_mib=disk_mib, poll_seconds=.1, diagnostics=observed)
    began = time.monotonic()
    samples = []
    stop = None
    engine_output = output/'engine'
    command = [str(binary), 'run' if observed else 'unobserved', str(bundle.resolve()), str(engine_output.resolve())]
    if routing is not None:
        command = [str(binary), 'routes', str(bundle.resolve()), str(engine_output.resolve()), str(routing['queries']), str(routing['concurrency']), routing['algorithm']]
        config['routing'] = routing
    timer = ['/usr/bin/time', '-l' if platform.system() == 'Darwin' else '-v', '-o', str((output/'resources.txt').resolve())]
    with (output/'stdout.log').open('wb') as stdout, (output/'stderr.log').open('wb') as stderr:
        child = subprocess.Popen(timer+command, stdout=stdout, stderr=stderr, start_new_session=True, env=env)
        while child.poll() is None:
            # Process group is isolated. ps reports resident KiB; sampling may miss/overshoot peaks.
            table = subprocess.run(['ps', '-axo', 'pgid=,rss='], capture_output=True, text=True, check=True)
            rss = sum(int(parts[1])*1024 for line in table.stdout.splitlines() if len(parts := line.split()) == 2 and int(parts[0]) == child.pid)
            disk = sum(p.stat().st_size for p in output.rglob('*') if p.is_file())
            elapsed = time.monotonic()-began
            samples.append(dict(elapsed_seconds=elapsed, process_group_rss_bytes=rss, output_bytes=disk))
            if elapsed >= seconds:
                stop = 'harness_wall_timeout'
            elif rss > rss_mib*1024*1024:
                stop = 'harness_sampled_rss_stop'
            elif disk > disk_mib*1024*1024:
                stop = 'harness_output_stop'
            if stop:
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                break
            time.sleep(.1)
        code = child.wait()
    report_path = engine_output/'report.json'
    report = json.loads(report_path.read_text()) if report_path.exists() and code == 0 and stop is None else None
    outcome = 'completed' if report is not None else stop or 'engine_failure'
    record = dict(schema_version=1, plane='routing' if routing is not None else 'simulation', outcome=outcome, exit_code=code, configuration=config,
                  source_revision=source_revision, compiler=subprocess.check_output(['rustc', '--version', '--verbose'], text=True).strip(), harness_sha256=sha(Path(__file__).read_bytes()), binary_sha256=sha(binary.read_bytes()), bundle=json.loads((bundle/'bundle.json').read_text()),
                  hardware=dict(os=platform.platform(), machine=platform.machine(), logical_cpus=os.cpu_count()),
                  wall_seconds=time.monotonic()-began, resource_samples=samples, final=report,
                  last_observed_boundary=last_progress(engine_output/'progress.jsonl'),
                  command=command, cache='fresh process; uncontrolled warm OS/filesystem; no route-result cache',
                  limits='External sampled stops are not engine admission guarantees. Failed runs have no inferred final counts. Last boundary may lag computation. RSS is process-group sampling, not allocations. Full evidence retained for completed runs.')
    (output/'run.json').write_text(json.dumps(record, indent=2)+'\n')
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    gen = commands.add_parser('generate')
    gen.add_argument('manifest', type=Path)
    gen.add_argument('bundle', type=Path)
    gen.add_argument('--binary', type=Path, required=True)
    gen.add_argument('--artifact', type=Path)
    execute = commands.add_parser('run')
    execute.add_argument('bundle', type=Path)
    execute.add_argument('output', type=Path)
    execute.add_argument('--binary', type=Path, required=True)
    execute.add_argument('--source-revision', required=True)
    execute.add_argument('--seconds', type=float, required=True)
    execute.add_argument('--rss-mib', type=int, required=True)
    execute.add_argument('--disk-mib', type=int, required=True)
    execute.add_argument('--unobserved', action='store_true')
    execute.add_argument('--routing', choices=['dijkstra', 'astar'])
    execute.add_argument('--queries', type=int, default=128)
    execute.add_argument('--concurrency', type=int, default=1)
    args = parser.parse_args()
    if args.command == 'generate':
        result = generate(args.manifest, args.bundle, args.binary, args.artifact)
    else:
        result = run(args.bundle, args.output, args.binary, args.source_revision, args.seconds, args.rss_mib, args.disk_mib, not args.unobserved, None if args.routing is None else dict(algorithm=args.routing, queries=args.queries, concurrency=args.concurrency))
    print(json.dumps({k:v for k,v in result.items() if k not in ['resource_samples', 'final']}, indent=2))
    if args.command == 'run' and result['outcome'] != 'completed':
        raise SystemExit(2)


if __name__ == '__main__':
    main()
