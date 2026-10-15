#!/usr/bin/env python3
"""Compare the audited Phase 22 binary with Phase 23 on every Phase 15–19 fixture.

Only the additive top-level trace is removed; all preexisting fields must match exactly.
"""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path


def verify_started_trace(trace, result):
    starts = trace['starts']
    assert [s['original']['action_id'] for s in starts] == list(range(trace['committed_start_count']))
    by_action = {s['original']['action_id']: s for s in starts}
    for effect in trace['effects']:
        assert effect['rider'] == by_action[effect['action']]['rider']
    arrived = set()
    for observation in trace['arrivals']:
        assert observation['action'] not in arrived
        arrived.add(observation['action'])
        start = by_action[observation['action']]
        movement = next(e for e in result['events'] if e['sequence'] == observation['event_sequence'])
        assert movement['event']['type'] == 'RIDER_MOVED'
        assert movement['event']['rider'] == start['rider'] == observation['rider']
        assert movement['at'] == observation['reported_at'] == observation['processing_sample_at']
        assert movement['event']['leg']['routing'] == start['original']['route']['routing']
        assert any(e['effect'] == 'Arrival' and e['action'] == observation['action'] and e['at'] == observation['reported_at'] and e['generation'] == observation['generation'] for e in trace['effects'])
    assert len(arrived) == sum(e['event']['type'] == 'RIDER_MOVED' for e in result['events'])
    for action, start in by_action.items():
        if action not in arrived:
            active = trace['final_active'][str(start['rider'])]
            assert active['action_id'] == action and active['route'] == start['original']['route']
            assert any(p['at'] > trace['horizon'] and p['action']['kind'] == 'Move' and p['action']['data']['rider'] == start['rider'] and p['action']['data']['execution_id'] == active['id'] for p in trace['pending'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--binary', type=Path, default=Path('target/debug/roadrunner'))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    rows = []
    for phase in range(15, 20):
        for fixture in sorted(Path(f'data/fixtures/phase-{phase}').glob('*.json')):
            tail = ['simulate', str(fixture), '--json']
            old = subprocess.check_output([str(args.baseline.resolve()), *tail])
            new = subprocess.check_output([str(args.binary.resolve()), *tail])
            again = subprocess.check_output([str(args.binary.resolve()), *tail])
            assert new == again, fixture
            before, after = json.loads(old), json.loads(new)
            trace = after.pop('trace')
            assert before == after, fixture
            verify_started_trace(trace, after)
            rows.append({'fixture': str(fixture), 'fixture_sha256': hashlib.sha256(fixture.read_bytes()).hexdigest(),
                         'baseline_sha256': hashlib.sha256(old).hexdigest(), 'current_sha256': hashlib.sha256(new).hexdigest(),
                         'all_preexisting_fields_equal': True, 'exact_new_replay': True, 'independent_started_trace_oracle': True, 'committed_starts': trace['committed_start_count']})
    report = {'schema_version': 1, 'baseline_commit': 'acfd80900dcf02c4358b2f18d731f2e72020968e',
              'baseline_binary_sha256': hashlib.sha256(args.baseline.read_bytes()).hexdigest(),
              'current_binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
              'normalization': 'remove only new top-level trace; no route, metric, timing, terms, identity or decision normalization',
              'fixtures': rows}
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(f'PASS: {len(rows)} unchanged baseline histories and exact instrumented replays')


if __name__ == '__main__':
    main()
