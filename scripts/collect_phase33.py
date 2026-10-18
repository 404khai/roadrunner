#!/usr/bin/env python3
"""Collect bounded Phase 33 probes; never asserts an SLO or Phase 34 capacity."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import tempfile
import time


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def summarize(values):
    ordered = sorted(values)
    def quantile(p):
        position = (len(ordered) - 1) * p
        lower = int(position)
        upper = min(lower + 1, len(ordered) - 1)
        return ordered[lower] + (ordered[upper] - ordered[lower]) * (position - lower)
    return dict(raw=values, median=statistics.median(values), p95=quantile(.95), p99=quantile(.99))


def collect(binary, manifest, timeout, environment):
    # Per-process RSS includes graph setup, simulation and serialization together.
    command = [str(binary), str(manifest)]
    timer = ['/usr/bin/time', '-l'] if platform.system() == 'Darwin' else ['/usr/bin/time', '-v']
    began = time.perf_counter_ns()
    run = subprocess.run(timer + command, capture_output=True, timeout=timeout, check=False, env=environment)
    elapsed = time.perf_counter_ns() - began
    if run.returncode:
        raise RuntimeError(f'{manifest}: exit {run.returncode}: {run.stderr.decode()[:4000]}')
    diagnostics = run.stderr.decode()
    match = re.search(r'(\d+)\s+maximum resident set size', diagnostics) if platform.system() == 'Darwin' else re.search(r'Maximum resident set size \(kbytes\):\s*(\d+)', diagnostics)
    rss = int(match[1]) * (1 if platform.system() == 'Darwin' else 1024) if match else None
    result = json.loads(run.stdout)
    return dict(process_ns=elapsed, peak_process_rss_bytes=rss, probe=result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--runs', type=int, default=7)
    parser.add_argument('--timeout', type=float, default=120)
    parser.add_argument('--cases', nargs='*', type=Path)
    parser.add_argument('--reference', type=Path, help='Reject any changed deterministic semantic hash')
    args = parser.parse_args()
    if args.runs < 2 or args.timeout <= 0:
        parser.error('runs >= 2 and timeout > 0 are required')
    binary = args.binary.resolve()
    cases = args.cases or sorted(Path('benchmarks/phase33').glob('*.json'))
    references = {}
    if args.reference:
        references = {r['manifest']: r for r in json.loads(args.reference.read_text())['cases']}
    rows = []
    environment = dict(os.environ)
    # Tools and execution mode are explicit; build is outside collected intervals.
    for manifest in cases:
        warmup = collect(binary, manifest, args.timeout, environment)
        expected = warmup['probe']['result']['semantic_hash']
        reference = references.get(str(manifest))
        if reference:
            if reference['manifest_sha256'] != digest(manifest) or reference['graph_digest'] != warmup['probe']['graph_digest']:
                raise RuntimeError(f'{manifest}: comparison identity differs')
            if reference['semantic_hash'] != expected:
                raise RuntimeError(f'{manifest}: baseline/candidate semantic regression')
        samples = []
        for _ in range(args.runs):
            sample = collect(binary, manifest, args.timeout, environment)
            if sample['probe']['result']['semantic_hash'] != expected:
                raise RuntimeError(f'{manifest}: nondeterministic output')
            samples.append(sample)
        rows.append(dict(manifest=str(manifest), manifest_sha256=digest(manifest), graph_digest=warmup['probe']['graph_digest'], semantic_hash=expected, workload=warmup['probe']['manifest'], graph_nodes=warmup['probe']['graph_nodes'], graph_edges=warmup['probe']['graph_edges'], samples=samples, process_ns=summarize([s['process_ns'] for s in samples]), peak_process_rss_bytes=summarize([s['peak_process_rss_bytes'] for s in samples]) if all(s['peak_process_rss_bytes'] is not None for s in samples) else None))
        print(f'{manifest}: verified {args.runs} samples', flush=True)
    def output(command):
        return subprocess.check_output(command, text=True).strip()
    hardware = dict(os=platform.platform(), architecture=platform.machine(), logical_cpus=os.cpu_count())
    if platform.system() == 'Darwin':
        hardware.update(cpu=output(['sysctl', '-n', 'machdep.cpu.brand_string']), memory_bytes=int(output(['sysctl', '-n', 'hw.memsize'])))
    record = dict(schema_version=1, baseline_revision=output(['git','rev-parse','HEAD']), tracked_diff_sha256=hashlib.sha256(subprocess.check_output(['git','diff','HEAD'])).hexdigest(), binary_sha256=digest(binary), compiler=output(['rustc','+1.99.0','--version']), hardware=hardware, configuration=dict(runs=args.runs, excluded_warmups=1, process_timeout_seconds=args.timeout, concurrency=1, cache='one excluded process warmup; OS/filesystem caches uncontrolled, no cold-cache claim; no route cache', build='cargo +1.99.0 build --release -p roadrunner-simulation --example phase33_probe', timing='stage ns and whole process ns separately; includes timer launcher for process time', rss='whole-process peak, includes setup and serialization; not per-operation allocated bytes', diagnostic_modes='manifest diagnostics flag controls post-execution retained-buffer inventory only; stage clocks always enabled', percentiles='linear interpolation across process/stage samples; small descriptive sample, not service-tail evidence'), unavailable=['per-query allocator attribution','CPU utilization sampling','coordinator/SQL/anchor attribution','operational open-loop throughput'], cases=rows)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2)+'\n')


if __name__ == '__main__':
    main()
