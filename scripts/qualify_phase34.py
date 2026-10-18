#!/usr/bin/env python3
"""Run a staged qualification; fail closed on small-case failure and stop tier escalation."""
import argparse
import json
from pathlib import Path
from phase34 import generate, run
from summarize_phase34 import summarize


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--source-revision', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--repetitions', type=int, default=3)
    parser.add_argument('--seconds', type=float, required=True)
    parser.add_argument('--rss-mib', type=int, required=True)
    parser.add_argument('--disk-mib', type=int, required=True)
    args = parser.parse_args()
    if not 2 <= args.repetitions <= 20:
        parser.error('2..20 measured repetitions required')
    args.output.mkdir()
    records = []
    generation = []
    tier_blocked = False
    skipped = []
    for manifest in sorted(Path('benchmarks/phase34').glob('*.json')):
        name = manifest.stem
        bundle = args.output/(name+'-bundle')
        generation.append(generate(manifest, bundle, args.binary))
        if name.startswith('tier-') and tier_blocked:
            skipped.append(dict(case=name, reason='previous tier failed external qualification; no escalation'))
            continue
        # One excluded warmup. A failure is still retained, never discarded or regenerated.
        case = []
        for i in range(args.repetitions+1):
            record = run(bundle, args.output/f'{name}-run-{i}', args.binary, args.source_revision,
                         args.seconds, args.rss_mib, args.disk_mib)
            case.append(record)
            print(f'{name} repetition {i}: {record["outcome"]}', flush=True)
            if record['outcome'] != 'completed':
                tier_blocked = True
                break
        if all(r['outcome'] == 'completed' for r in case):
            records.extend(case[1:])
        else:
            records.extend(case)  # retain failed warmup; no success-only selection.
            if not name.startswith('tier-'):
                raise RuntimeError(f'small-case qualification failed: {name}; inspect retained artifacts')
        partial = dict(generation=generation, skipped=skipped, summary=summarize(records))
        (args.output/'qualification.json').write_text(json.dumps(partial, indent=2)+'\n')
    (args.output/'qualification.json').write_text(json.dumps(dict(generation=generation, skipped=skipped, summary=summarize(records)), indent=2)+'\n')
    # Matching diagnostics-off execution is a distinct instrumentation comparison.
    smoke = args.output/'smoke-bundle'
    for i in range(args.repetitions):
        record = run(smoke, args.output/f'smoke-unobserved-{i}', args.binary, args.source_revision,
                     args.seconds, args.rss_mib, args.disk_mib, observed=False)
        if record['outcome'] != 'completed':
            raise RuntimeError('unobserved smoke failed; inspect retained record')
        observed_hash = next(r['final']['result_sha256'] for r in records if r['bundle']['scenario_sha256'] == record['bundle']['scenario_sha256'] and r['final'] is not None)
        if record['final']['result_sha256'] != observed_hash:
            raise RuntimeError('instrumentation changed full evidence')
    for algorithm in ['dijkstra', 'astar']:
        for concurrency in [1, 4]:
            for i in range(args.repetitions):
                record = run(smoke, args.output/f'routes-{algorithm}-{concurrency}-{i}', args.binary, args.source_revision,
                    args.seconds, args.rss_mib, args.disk_mib,
                    routing=dict(algorithm=algorithm, queries=128, concurrency=concurrency))
                if record['outcome'] != 'completed':
                    raise RuntimeError('routing qualification failed; inspect retained record')


if __name__ == '__main__':
    main()
