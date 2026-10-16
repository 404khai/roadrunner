#!/usr/bin/env python3
"""Independent retained-source and Decimal80 oracle for offline route ETA research.

Does not import the trainer. Reconstructs labels/B0 and dependency closure from
raw captures, verifies ridge normal equations with Decimal Gaussian elimination,
and checks every comparison slice against independently paired predictions.
"""
import argparse
from collections import Counter
from decimal import Decimal, localcontext
import hashlib
import json
import math
from pathlib import Path
from verify_eta_baseline import equivalent, fraction, metrics


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, allow_nan=False, separators=(',', ':')).encode()


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def key(value):
    return value['run'], tuple(value['namespace']), value['action']


def decimal_solve(matrix, rhs):
    # Independent pivoted elimination, not the production Cholesky solver.
    augmented = [list(row) + [y] for row, y in zip(matrix, rhs)]
    for col in range(len(rhs)):
        pivot = max(range(col, len(rhs)), key=lambda i: abs(augmented[i][col]))
        augmented[col], augmented[pivot] = augmented[pivot], augmented[col]
        divisor = augmented[col][col]
        assert divisor != 0
        augmented[col] = [v / divisor for v in augmented[col]]
        for i in range(len(rhs)):
            if i != col:
                factor = augmented[i][col]
                augmented[i] = [a - factor*b for a, b in zip(augmented[i], augmented[col])]
    return [row[-1] for row in augmented]


def dependency_oracle(dataset):
    sources = dataset['sources']
    groups = {s['run']: s['run'] for s in sources}
    tags = []
    for source in sources:
        lineage = source['lineage']
        tags.append(set(lineage['dependencies']) | {
            'graph:' + source['graph']['sha256'], 'family:' + lineage['family'],
            'world:' + lineage['base_world'], 'realization:' + lineage['realization'],
            'replay:' + source['graph']['sha256'] + ':' + source['scenario']['sha256']})
    while True:
        old = groups.copy()
        for i, left in enumerate(sources):
            for j, right in enumerate(sources[i+1:], i+1):
                if tags[i] & tags[j]:
                    a, b = groups[left['run']], groups[right['run']]
                    groups = {run: min(a, b) if group in (a, b) else group for run, group in groups.items()}
        if old == groups:
            break
    assert groups == dataset['splits']['groups']
    roles = {}
    for run, group in groups.items():
        role = dataset['splits']['request']['assignments'][run]
        assert roles.setdefault(group, role) == role
    return groups


def raw_actions(dataset, export):
    groups = dependency_oracle(dataset)
    starts = {}
    for source in dataset['sources']:
        for field in ('graph', 'scenario', 'result'):
            assert sha(source[field]['utf8'].encode()) == source[field]['sha256']
        trace = json.loads(source['result']['utf8'])['trace']
        assert [s['original']['action_id'] for s in trace['starts']] == list(range(trace['committed_start_count']))
        for start in trace['starts']:
            route = start['original']['route']
            action = start['original']['action_id']
            namespace = tuple(trace['final_revision']['namespace'])
            arrival = next((e for e in trace['effects'] if e['action'] == action and e['effect'] == 'Arrival'), None)
            label = arrival['at'] - route['departed_at'] if arrival else None
            partition = dataset['splits']['request']['assignments'][source['run']]
            starts[(source['run'], namespace, action)] = {
                'source': source, 'route': route, 'start': start, 'observed_seconds': label,
                'b0_seconds': route['travel'], 'partition': partition, 'outcome': 'Observed' if arrival else 'RightCensored',
                'label_eligible': label is not None and partition != 'Excluded', 'dependency_group': groups[source['run']]}
    assert len(starts) == len(export['inputs']) == len(export['baseline']['actions']) == len(dataset['rows'])
    assert set(starts) == {key(a['key']) for a in export['baseline']['actions']}
    actions = []
    for action, request in zip(export['baseline']['actions'], export['inputs']):
        raw = starts[key(action['key'])]
        assert key(request['key']) == key(action['key'])
        for name in ('observed_seconds', 'b0_seconds', 'partition', 'outcome', 'label_eligible', 'dependency_group'):
            assert action[name] == raw[name], name
        assert request['route'] == raw['route']
        assert request['values'] == [raw['route']['travel'], raw['route']['distance']]
        assert request['reference_seconds'] == raw['route']['departed_at']
        assert request['witness']['start_revision'] == raw['start']['committed_revision']
        assert set(request) == {'key', 'route', 'route_sha256', 'reference_seconds', 'witness', 'values'}
        source, route = raw['source'], raw['route']
        # Independently reconstruct diagnostic memberships; never numeric predictors.
        memberships = {
            'distance_band': 'under_1km' if route['distance'] < 1000 else '1_to_5km' if route['distance'] < 5000 else 'at_least_5km',
            'estimate_band': 'under_60s' if route['travel'] < 60 else '60_to_300s' if route['travel'] < 300 else 'at_least_300s',
            'cohort': json.dumps('SyntheticControlled'), 'profile': route['routing']['profile'], 'graph': route['routing']['graph_digest'],
            'traffic': canonical(route['routing']['traffic']).decode(), 'family': source['lineage']['family'], 'generator': source['lineage']['generator'],
            'policy': source['lineage']['policy'], 'source': source['run'], 'dependency_group': raw['dependency_group'],
            'partition': json.dumps(raw['partition']),
            'intervention_diagnostic': 'RoadDelayObserved' if any(s['execution']['action_id'] == action['key']['action'] for s in json.loads(source['result']['utf8'])['trace']['schedules']) else 'NoRoadDelayObserved'}
        assert memberships == action['segments']
        actions.append(dict(action, **{k: raw[k] for k in ('observed_seconds', 'b0_seconds', 'label_eligible')}))
    return actions


def check_fit(export, experiment):
    model = experiment['model']
    if model is None:
        assert export['baseline']['independent_holdout'] != 'Available' or any(
            not any(a['partition'] == role and a['label_eligible'] for a in export['baseline']['actions']) for role in ('Train', 'Validation', 'Test'))
        assert experiment['status'].startswith('EvaluationUnavailable')
        return 0
    params = model['parameters']
    assert sha(canonical(params)) == model['model_id']
    pairs = [(a, x) for a, x in zip(export['baseline']['actions'], export['inputs']) if a['partition'] == 'Train' and a['label_eligible']]
    assert params['training_keys'] == [a['key'] for a, _ in pairs]
    assert params['training_input_sha256'] == sha(canonical([x for _, x in pairs]))
    assert params['training_labels_sha256'] == sha(canonical([a['observed_seconds'] for a, _ in pairs]))
    assert params['dataset_sha256'] == export['baseline']['dataset_semantic_sha256']
    assert params['split_plan_sha256'] == export['baseline']['split_plan_sha256']
    with localcontext() as ctx:
        ctx.prec = 80
        D = Decimal.from_float
        for j in range(2):
            column = [float(x['values'][j]) for _, x in pairs]
            center = sum(map(D, column)) / len(column)
            scale = (sum((D(v)-center)**2 for v in column)/len(column)).sqrt() or Decimal(1)
            assert math.isclose(params['center'][j], float(center), rel_tol=2e-14, abs_tol=1e-12)
            assert math.isclose(params['scale'][j], float(scale), rel_tol=2e-14, abs_tol=1e-12)
            assert params['minimum'][j] == min(column) and params['maximum'][j] == max(column)
        design = [[Decimal(1)] + [D((v-c)/s) for v, c, s in zip(x['values'], params['center'], params['scale'])] for _, x in pairs]
        residuals = [D(a['observed_seconds'] - x['values'][0]) for a, x in pairs]
        matrix = [[sum(row[i]*row[j] for row in design) + Decimal(i == j) for j in range(3)] for i in range(3)]
        rhs = [sum(row[j]*y for row, y in zip(design, residuals)) for j in range(3)]
        expected = decimal_solve(matrix, rhs)
        for actual, value in zip(params['coefficients'], expected):
            assert math.isclose(actual, float(value), rel_tol=2e-14, abs_tol=1e-12)
    return len(pairs)


def check_predictions(export, experiment):
    assert len(experiment['predictions']) == len(export['inputs'])
    model = experiment['model']
    for request, prediction in zip(export['inputs'], experiment['predictions']):
        assert prediction['action_key'] == request['key']
        assert prediction['request_sha256'] == sha(canonical(request))
        if model is None:
            assert prediction['failure'] == 'ModelUnavailable' and prediction['duration_seconds'] is None
            continue
        params = model['parameters']
        inside = all(lo <= v <= hi for v, lo, hi in zip(request['values'], params['minimum'], params['maximum']))
        if not inside:
            assert prediction['failure'] == 'OutsideTrainingSupport' and prediction['duration_seconds'] is None
            continue
        effective = [1.0] + [(v-c)/s for v, c, s in zip(request['values'], params['center'], params['scale'])]
        assert prediction['effective_values'] == effective
        assert prediction['effective_binary64_hex'] == [v.hex() for v in effective]
        with localcontext() as ctx:
            ctx.prec = 80
            correction = sum(Decimal.from_float(c)*Decimal.from_float(v) for c, v in zip(params['coefficients'], effective))
        expected = request['values'][0] + float(correction)
        assert prediction['failure'] is None and prediction['model_id'] == model['model_id']
        assert math.isclose(prediction['duration_seconds'], expected, rel_tol=2e-14, abs_tol=1e-12)


def check_slice(actions, predictions, actual):
    included = [(a, p) for a, p in zip(actions, predictions) if a['partition'] != 'Excluded']
    eligible = [(a, p) for a, p in included if a['label_eligible']]
    paired = [(a, p) for a, p in eligible if p['duration_seconds'] is not None]
    fallback = any(p['failure'] for _, p in included)
    coverage = {
        'committed_starts': len(actions), 'excluded_starts': len(actions)-len(included),
        'outcomes': {s: sum(a['outcome'] == s for a in actions) for s in ('Observed', 'Pending', 'RightCensored', 'MissingArrival', 'DisputedOrInvalid')},
        'exact_label_coverage': fraction(sum(a['observed_seconds'] is not None for a in actions), len(actions)),
        'eligible_label_coverage': fraction(len(eligible), len(included)),
        'prediction_coverage': fraction(sum(p['duration_seconds'] is not None for _, p in included), len(included)),
        'prediction_on_eligible_coverage': fraction(len(paired), len(eligible)),
        'census_groups': len({a['dependency_group'] for a in actions}),
        'paired_groups': len({a['dependency_group'] for a, _ in paired}),
        'failures': dict(Counter(p['failure'] for _, p in included if p['failure'])),
        'label_exclusion_reasons': dict(Counter(r for a in actions for r in a['reasons']))}
    expected = {'coverage': coverage, 'b0_full_eligible': metrics([a['observed_seconds']-a['b0_seconds'] for a, _ in eligible]),
                'b0_matched_candidate': metrics([a['observed_seconds']-a['b0_seconds'] for a, _ in paired]),
                'candidate_matched': metrics([a['observed_seconds']-p['duration_seconds'] for a, p in paired]),
                'full_eligible_with_coherent_b0_fallback': metrics([a['observed_seconds']-(a['b0_seconds'] if fallback else p['duration_seconds']) for a, p in eligible]),
                'coherent_batch_fallback': bool(fallback), 'metrics_unavailable_reason': None if eligible else 'NoEligibleExactLabels'}
    equivalent(actual, expected)


def verify(directory):
    manifest = json.loads((directory/'manifest.json').read_text())
    for name, digest in manifest['files'].items():
        assert sha((directory/name).read_bytes()) == digest
    publication = json.loads((directory/'research.json').read_text())
    dataset = json.loads((directory/'dataset/bundle.json').read_text())
    dataset_manifest = json.loads((directory/'dataset/manifest.json').read_text())
    assert dataset_manifest['packaging_sha256'] == sha((directory/'dataset/bundle.json').read_bytes())
    export, result = publication['export'], publication['experiment']
    assert export['baseline']['dataset_semantic_sha256'] == dataset_manifest['semantic_sha256']
    actions = raw_actions(dataset, export)
    trained = check_fit(export, result)
    check_predictions(export, result)
    predictions = result['predictions']
    check_slice(actions, predictions, result['slices']['all_included_descriptive'])
    expected_names = {'all_included_descriptive'}
    for dimension, values in export['baseline']['segments'].items():
        for value in values:
            name = dimension + ':' + value
            expected_names.add(name)
            indices = [i for i, a in enumerate(actions) if a['segments'][dimension] == value]
            check_slice([actions[i] for i in indices], [predictions[i] for i in indices], result['slices'][name])
    assert set(result['slices']) == expected_names
    test = [i for i, a in enumerate(actions) if a['partition'] == 'Test']
    check_slice([actions[i] for i in test], [predictions[i] for i in test], result['protected_test'])
    baseline, candidate = result['protected_test']['b0_full_eligible'], result['protected_test']['candidate_matched']
    gain = bool(result['model'] and baseline and candidate and not result['protected_test']['coherent_batch_fallback']
                and baseline['samples'] == candidate['samples'] and baseline['mae_seconds']-candidate['mae_seconds'] > 1e-9
                and candidate['rmse_seconds'] <= baseline['rmse_seconds'])
    assert result['supported_offline_improvement'] == gain
    assert result['operational_admission'].startswith('EvaluationUnavailable')
    return {'result': 'PASS', 'oracle': 'raw-start-arrival-census; dependency-closure; Decimal80-pivoted-ridge-and-metrics/v1',
            'publication_sha256': sha((directory/'research.json').read_bytes()), 'starts': len(actions),
            'train_exact_actions': trained, 'dependency_groups': len(set(dataset['splits']['groups'].values())),
            'comparison_slices': len(expected_names)+1, 'supported_offline_improvement': gain,
            'physical_generalization': 'EvaluationUnavailable',
            'tolerance': 'identities/counts/coverage exact; numeric relative 2e-14, absolute 1e-12; no metric clamping'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    report = json.dumps(verify(args.directory), indent=2) + '\n'
    if args.output:
        args.output.write_text(report)
    print(report)
