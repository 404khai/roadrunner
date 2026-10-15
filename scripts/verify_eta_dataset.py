#!/usr/bin/env python3
"""Independent source/authority oracle for Phase 23 bundles (no error metrics)."""
import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path


def verify(directory):
    manifest = json.loads((directory / 'manifest.json').read_text())
    raw = (directory / 'bundle.json').read_bytes()
    assert hashlib.sha256(raw).hexdigest() == manifest['packaging_sha256']
    bundle = json.loads(raw)
    rows = bundle['rows']
    seen = set()
    outcome_counts = Counter()
    details = []
    for source in bundle['sources']:
        for name in ['graph', 'scenario', 'result']:
            evidence = source[name]
            assert hashlib.sha256(evidence['utf8'].encode()).hexdigest() == evidence['sha256']
        result = json.loads(source['result']['utf8'])
        trace = result['trace']
        starts = trace['starts']
        assert len(starts) == trace['committed_start_count']
        assert [s['original']['action_id'] for s in starts] == list(range(len(starts)))
        assert trace['horizon'] == result['ended_at']
        selected = [r for r in rows if r['key']['run'] == source['run']]
        assert len(selected) == len(starts)
        by_id = {r['key']['action']: r for r in selected}
        assert len(by_id) == len(starts)
        for start in starts:
            original = start['original']
            row = by_id[original['action_id']]
            key = (source['run'], tuple(row['key']['namespace']), row['key']['action'])
            assert key not in seen
            seen.add(key)
            assert row['rider'] == start['rider']
            assert row['features']['original_route'] == original['route']
            assert row['features']['b0_seconds'] == original['route']['travel']
            assert row['features']['distance_meters'] == original['route']['distance']
            assert row['features']['witness']['start_revision'] == start['committed_revision']
            arrivals = [e for e in trace['effects'] if e['action'] == original['action_id'] and e['effect'] == 'Arrival']
            label = row['label']
            if arrivals:
                assert len({(e['at'], e['rider'], e['generation']) for e in arrivals}) == 1
                observation = next(a for a in trace['arrivals'] if a['action'] == original['action_id'])
                movement = next(e for e in result['events'] if e['sequence'] == observation['event_sequence'])
                assert movement['event']['type'] == 'RIDER_MOVED'
                assert movement['event']['rider'] == observation['rider'] == start['rider']
                assert movement['at'] == observation['reported_at'] == observation['processing_sample_at']
                at = arrivals[0]['at']
                assert all(e['rider'] == start['rider'] for e in arrivals)
                assert label['outcome'] == 'Observed'
                assert label['duration_seconds'] == at - original['route']['departed_at']
                assert label['lower_bound_seconds'] is None
            else:
                active = trace['final_active'][str(start['rider'])]
                assert active['action_id'] == original['action_id']
                assert any(p['action']['kind'] == 'Move' and p['action']['data']['execution_id'] == active['id'] and p['at'] > trace['horizon'] for p in trace['pending'])
                assert label['outcome'] == 'RightCensored'
                assert label['duration_seconds'] is None
                assert label['lower_bound_seconds'] == trace['horizon'] - original['route']['departed_at']
            assert label['evidence']['measurement'] == 'SyntheticControlled'
            outcome_counts[label['outcome']] += 1
        details.append({'run': source['run'], 'authority_starts': trace['committed_start_count'], 'rows': len(selected),
                        'horizon': trace['horizon'], 'pending_events': len(trace['pending']), 'schedules': len(trace['schedules'])})
    assert len(seen) == len(rows) == bundle['quality']['committed_starts']
    assert sum(bundle['quality']['outcomes'].values()) == len(rows)
    assert {k: v for k, v in bundle['quality']['outcomes'].items() if v} == dict(outcome_counts)
    assert bundle['quality']['exact_labels'] == sum(r['label']['duration_seconds'] is not None for r in rows)
    assert len(set(bundle['splits']['groups'].values())) == bundle['quality']['dependency_groups']
    assert 'EvaluationUnavailable' in bundle['evaluation']['physical']
    for value in bundle['reproduction']['files'].values():
        assert hashlib.sha256(value['utf8'].encode()).hexdigest() == value['sha256']
    return {'schema_version': 1, 'oracle': 'independent-python-source-authority/v1', 'dataset': str(directory),
            'packaging_sha256': manifest['packaging_sha256'], 'census': len(rows), 'outcomes': dict(outcome_counts),
            'sources': details, 'result': 'PASS', 'claim': 'synthetic dataset engineering only; no predictive error metrics'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    report = verify(args.directory)
    text = json.dumps(report, indent=2) + '\n'
    if args.output:
        args.output.write_text(text)
    print(text)


if __name__ == '__main__':
    main()
