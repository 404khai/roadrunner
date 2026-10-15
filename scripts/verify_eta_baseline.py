#!/usr/bin/env python3
"""Independent source-paired Phase 24 coverage/metric oracle, using Decimal arithmetic."""
import argparse
from collections import Counter
from decimal import Decimal, localcontext
import hashlib
import json
import math
from pathlib import Path


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(value):
    # Used only for metadata objects without binary64 number formatting ambiguity.
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()


def key(value):
    return value['run'], tuple(value['namespace']), value['action']


def mean(values):
    with localcontext() as ctx:
        ctx.prec = 80
        return float(sum(map(Decimal.from_float, values)) / len(values))


def quantiles(values):
    ordered = sorted(values)
    return {f'p{p}': ordered[(len(ordered)*p+99)//100-1] for p in [50, 95, 99]}


def metrics(residuals):
    if not residuals:
        return None
    ordered = sorted(residuals)
    positive = [v for v in ordered if v > 0]
    middle = len(ordered)//2
    with localcontext() as ctx:
        ctx.prec = 80
        squares = [Decimal.from_float(v)**2 for v in residuals]
        rms = float((sum(squares)/len(squares)).sqrt())
        median = ordered[middle] if len(ordered) % 2 else float((Decimal.from_float(ordered[middle-1])+Decimal.from_float(ordered[middle]))/2)
    return {'samples': len(residuals), 'mae_seconds': mean([abs(v) for v in residuals]), 'rmse_seconds': rms,
            'mean_signed_residual_seconds': mean(residuals), 'median_signed_residual_seconds': median,
            'absolute_error_quantiles_seconds': quantiles([abs(v) for v in residuals]),
            'positive_residual_quantiles_seconds': quantiles(positive) if positive else None,
            'underprediction_count': len(positive), 'underprediction_frequency': len(positive)/len(residuals),
            'underprediction_severity_seconds': mean(positive) if positive else None}


def equivalent(actual, expected, path=''):
    if isinstance(expected, dict):
        assert actual.keys() == expected.keys(), (path, actual.keys(), expected.keys())
        for name, value in expected.items():
            equivalent(actual[name], value, path+'/'+name)
    elif isinstance(expected, float):
        assert math.isfinite(actual) and math.isclose(actual, expected, rel_tol=2e-14, abs_tol=1e-12), (path, actual, expected)
    else:
        assert actual == expected, (path, actual, expected)


def fraction(numerator, denominator):
    return {'numerator': numerator, 'denominator': denominator, 'ratio': numerator/denominator if denominator else None}


def slice_report(actions):
    outcomes = dict.fromkeys(['Observed', 'Pending', 'RightCensored', 'MissingArrival', 'DisputedOrInvalid'], 0)
    reasons = Counter()
    residuals = []
    groups, paired_groups = set(), set()
    excluded = exact = available = eligible = 0
    for action in actions:
        outcomes[action['outcome']] += 1
        reasons.update(action['reasons'])
        groups.add(action['dependency_group'])
        excluded += action['partition'] == 'Excluded'
        exact += action['observed_seconds'] is not None
        available += action['b0_seconds'] is not None
        eligible += action['label_eligible']
        if action['label_eligible'] and action['b0_seconds'] is not None:
            residuals.append(action['observed_seconds']-action['b0_seconds'])
            paired_groups.add(action['dependency_group'])
    n = len(actions)
    return {'coverage': {'committed_starts': n, 'excluded_starts': excluded, 'outcomes': outcomes, 'reasons': dict(reasons),
                         'exact_label_coverage': fraction(exact, n), 'prediction_coverage': fraction(available, n),
                         'eligible_label_coverage': fraction(eligible, n-excluded), 'prediction_on_eligible_coverage': fraction(len(residuals), eligible),
                         'census_groups': len(groups), 'paired_groups': len(paired_groups)},
            'metrics': metrics(residuals), 'metrics_unavailable_reason': None if residuals else 'NoEligibleExactPredictionLabelPairs',
            'full_eligible_population': 'Available' if eligible and eligible == len(residuals) else {'EvaluationUnavailable': 'NoEligibleExactLabels' if not eligible else 'MissingPredictionsAndNoAuthorizedFallback'}}


def verify(directory, dataset_directory, fixture):
    manifest = json.loads((directory/'manifest.json').read_text())
    assert manifest['schema'] == 'eta-b0-publication/v1' and manifest['artifact_file'] == 'evaluation.json'
    raw = (directory/'evaluation.json').read_bytes()
    assert sha(raw) == manifest['packaging_sha256']
    artifact = json.loads(raw)
    dataset, report = artifact['dataset'], artifact['report']
    assert report['schema'] == 'eta-b0-report/v1' and report['protocol']['version'] == 'b0-evaluation/v1'
    original = json.loads((dataset_directory/'bundle.json').read_text())
    source_manifest = json.loads((dataset_directory/'manifest.json').read_text())
    assert original == dataset and sha((dataset_directory/'bundle.json').read_bytes()) == source_manifest['packaging_sha256']
    assert report['dataset_semantic_sha256'] == source_manifest['semantic_sha256']
    assert report['frozen_package_sha256'] == sha(canonical(dataset['evaluation']))
    assert report['split_plan_sha256'] == sha(canonical(dataset['splits']))
    actions = {key(a['key']): a for a in report['actions']}
    assert len(actions) == len(report['actions']) == len(dataset['rows'])
    expected_actions = []
    segments = {'distance_band': {v: [] for v in ['under_1km', '1_to_5km', 'at_least_5km']},
                'estimate_band': {v: [] for v in ['under_60s', '60_to_300s', 'at_least_300s']},
                'partition': {json.dumps(v): [] for v in ['Descriptive', 'Train', 'Validation', 'Test', 'Excluded']}}
    raw_sources = {}
    all_source_keys = set()
    for source in dataset['sources']:
        for name in ['graph', 'scenario', 'result']:
            assert sha(source[name]['utf8'].encode()) == source[name]['sha256']
        trace = json.loads(source['result']['utf8'])['trace']
        assert [s['original']['action_id'] for s in trace['starts']] == list(range(trace['committed_start_count']))
        raw_sources[source['run']] = source, trace
        all_source_keys.update((source['run'], tuple(trace['final_revision']['namespace']), start['original']['action_id']) for start in trace['starts'])
        for dimension, value in [('source', source['run']), ('family', source['lineage']['family']), ('generator', source['lineage']['generator']), ('policy', source['lineage']['policy']), ('dependency_group', dataset['splits']['groups'][source['run']])]:
            segments.setdefault(dimension, {}).setdefault(value, [])
    assert all_source_keys == set(actions) == {key(row['key']) for row in dataset['rows']}
    for row in dataset['rows']:
        action = actions[key(row['key'])]
        source, trace = raw_sources[row['key']['run']]
        original = next(s for s in trace['starts'] if s['original']['action_id'] == row['key']['action'])
        route = original['original']['route']
        arrival = next((e for e in trace['effects'] if e['action'] == row['key']['action'] and e['effect'] == 'Arrival'), None)
        observed = arrival['at']-route['departed_at'] if arrival else None
        assert action['b0_seconds'] == route['travel'] == row['features']['b0_seconds']
        assert action['observed_seconds'] == observed == row['label']['duration_seconds']
        assert action['rider'] == row['rider'] == original['rider']
        assert action['start_revision'] == original['committed_revision']
        assert action['start_effective_at'] == route['departed_at']
        assert action['source_content'] == row['source_content'] == sha(canonical(source))
        partition = dataset['splits']['request']['assignments'][source['run']]
        eligible = observed is not None and partition != 'Excluded'
        outcome = 'Observed' if arrival else 'RightCensored'
        reasons = [] if arrival else ['NoExactArrival']
        if partition == 'Excluded':
            reasons.append('FrozenSplitExcluded')
        memberships = {
            'distance_band': 'under_1km' if route['distance'] < 1000 else '1_to_5km' if route['distance'] < 5000 else 'at_least_5km',
            'estimate_band': 'under_60s' if route['travel'] < 60 else '60_to_300s' if route['travel'] < 300 else 'at_least_300s',
            'cohort': json.dumps('SyntheticControlled'), 'profile': route['routing']['profile'], 'graph': route['routing']['graph_digest'],
            'traffic': canonical(route['routing']['traffic']).decode(), 'family': source['lineage']['family'], 'generator': source['lineage']['generator'],
            'policy': source['lineage']['policy'], 'source': source['run'], 'dependency_group': dataset['splits']['groups'][source['run']],
            'partition': json.dumps(partition), 'intervention_diagnostic': 'RoadDelayObserved' if any(s['execution']['action_id'] == row['key']['action'] for s in trace['schedules']) else 'NoRoadDelayObserved'}
        assert action['segments'] == memberships and action['partition'] == partition and action['dependency_group'] == memberships['dependency_group']
        assert action['measurement'] == 'SyntheticControlled' and action['outcome'] == outcome and action['label_eligible'] == eligible
        assert action['reasons'] == reasons and action['residual_seconds'] == (observed-route['travel'] if eligible else None)
        # Construct expected scoring inputs from raw source endpoints, not reported residuals/metrics.
        expected = dict(action, observed_seconds=observed, b0_seconds=route['travel'], label_eligible=eligible, reasons=reasons, outcome=outcome)
        expected_actions.append(expected)
        for dimension, value in memberships.items():
            segments.setdefault(dimension, {}).setdefault(value, []).append(expected)
    equivalent(report['descriptive_all_included'], slice_report(expected_actions))
    expected_segments = {d: {v: slice_report(items) for v, items in groups.items()} for d, groups in segments.items()}
    equivalent(report['segments'], expected_segments)
    assert report['independent_holdout'] == dataset['splits']['evaluation']
    assert report['physical_generalization'] == dataset['evaluation']['physical']
    for name in ['metric_uncertainty', 'prediction_uncertainty', 'causal_policy_effect']:
        assert 'EvaluationUnavailable' in report[name]
    for value in artifact['evaluator']['files'].values():
        assert sha(value['utf8'].encode()) == value['sha256']
    checked = 0
    for case in json.loads(fixture.read_text())['cases']:
        residuals = [float(label)-float(prediction) for label, prediction in zip(case['observed_seconds'], case['predictions_seconds'])]
        expected = case['expected'].copy()
        expected['rmse_seconds'] = math.sqrt(expected.pop('mean_squared_error_seconds_squared'))
        expected['samples'] = len(residuals)
        equivalent(metrics(residuals), expected)
        checked += 1
    return {'schema_version': 1, 'result': 'PASS', 'oracle': 'independent-source-endpoint-pairing-and-Decimal80-metrics/v1',
            'packaging_sha256': manifest['packaging_sha256'], 'actions': len(actions), 'exact_pairs': len([a for a in expected_actions if a['label_eligible']]),
            'segment_slices': sum(map(len, segments.values())), 'hand_calculated_metric_fixtures': checked,
            'metric_comparison_tolerance': 'relative 2e-14, absolute 1e-12; identities, outcomes, counts and unavailability exact',
            'physical_generalization': report['physical_generalization'], 'independent_holdout': report['independent_holdout']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--dataset', type=Path, default=Path('ml/data/phase23-representative-v1'))
    parser.add_argument('--fixture', type=Path, default=Path('data/fixtures/phase-24/metric-oracles.json'))
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    report = verify(args.directory, args.dataset, args.fixture)
    text = json.dumps(report, indent=2)+'\n'
    if args.output:
        args.output.write_text(text)
    print(text)


if __name__ == '__main__':
    main()
