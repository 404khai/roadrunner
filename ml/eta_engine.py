#!/usr/bin/env python3
"""Offline whole-route residual ridge research. No live engine or ML dependencies.

Run: python3 ml/eta_engine.py run DATASET NEW_OUTPUT --verifier target/debug/roadrunner
Verify: python3 ml/eta_engine.py verify OUTPUT --verifier target/debug/roadrunner
Predict: python3 ml/eta_engine.py predict OUTPUT REQUEST.json --verifier ...

The only accepted requests are source-verified committed starts. Output is an
expected duration in seconds, never an edge cost or physical bound.
"""
import argparse
from collections import Counter
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile

SCHEMA = 'eta-route-research/v1; whole-started-leg; seconds,meters; binary64'
MODEL_SCHEMA = 'eta-route-residual-ridge/v1'
PROTOCOL = {
    'version': 'offline-route-ridge/v1',
    'target': 'whole-started-leg/synthetic-logical-seconds/v1',
    'features': ['original_b0_seconds', 'original_distance_meters'],
    'fit': 'eligible exact Train residual; Train population standardization; ridge lambda=1 including intercept; fixed model, no tuning',
    'arithmetic': 'Python binary64; math.fsum; deterministic Cholesky normal equations; no BLAS, randomization, clamping or pickle',
    'applicability': 'verified original started request; same Train profile; closed per-feature Train ranges; finite nonnegative prediction',
    'comparison': 'MAE primary; full Test coverage; improvement >1e-9 seconds; no RMSE deterioration; offline evidence only',
    'metrics': 'b0-evaluation/v1; label-prediction; equally weighted actions; midpoint median; nearest-rank p50/p95/p99; MAPE deferred',
    'fallback': 'each reported slice is one offline batch; any included request failure makes the entire batch original-B0; matched metrics separate',
    'uncertainty': 'EvaluationUnavailable: controlled design, no sampled population or physical measurement; no prediction bounds',
    'admission': 'Unavailable: no operational or shadow authorization; no routing, edge, future-departure or causal validation',
    'resources': 'offline one process, two features; maximum 100000 starts and 64 MiB export; no runtime SLA claim',
}
MAX_ROWS = 100000
MAX_BYTES = 64 * 1024 * 1024


class ContractError(ValueError):
    """Invalid evidence, artifact or unsupported applicability; no silent repair."""


def require(condition, reason):
    if not condition:
        raise ContractError(reason)


def encoded(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, allow_nan=False,
                      separators=(',', ':')).encode('utf-8')


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def identity(value):
    return sha(encoded(value))


def finite(value):
    return type(value) in (int, float) and math.isfinite(value)


def load_json(path):
    def pairs(items):
        result = {}
        for name, value in items:
            require(name not in result, 'DuplicateJsonKey:' + name)
            result[name] = value
        return result
    def invalid(value):
        raise ContractError('NonfiniteJson:' + value)
    def number(value):
        result = float(value)
        require(math.isfinite(result), 'NonfiniteJson:' + value)
        return result
    require(Path(path).stat().st_size <= MAX_BYTES, 'ArtifactSizeLimit')
    return json.loads(Path(path).read_text(), object_pairs_hook=pairs, parse_constant=invalid, parse_float=number)


def checked_export(dataset, verifier):
    """Rust validates manifests, source bytes, rerun, census, inputs and splits."""
    process = subprocess.run([str(Path(verifier).resolve()), 'learned', 'export', str(dataset)],
                             capture_output=True, check=False, timeout=120)
    require(process.returncode == 0, 'SourceVerificationFailed:' + process.stderr.decode(errors='replace')[:2000])
    require(len(process.stdout) <= MAX_BYTES, 'ExportSizeLimit')
    export = json.loads(process.stdout)
    validate_export(export)
    return export


def validate_request(request):
    require(isinstance(request, dict), 'RequestSchema')
    require(set(request) == {'key', 'route', 'route_sha256', 'reference_seconds', 'witness', 'values'}, 'RequestSchema')
    values = request['values']
    require(len(values) == 2 and all(finite(v) and v >= 0 for v in values), 'FeatureUnitsOrNonfinite')
    route = request['route']
    require(values == [route['travel'], route['distance']], 'OriginalB0OrDistanceMismatch')
    require(finite(request['reference_seconds']) and request['reference_seconds'] == route['departed_at'], 'PredictionReferenceMismatch')
    require(len(route['nodes']) == len(route['edges']) + 1, 'DirectedRouteShape')
    require(route['nodes'][0] == route['from'] and route['nodes'][-1] == route['to'], 'DirectedRouteEndpoints')
    require(request['witness']['start_revision']['namespace'] == request['key']['namespace'], 'NamespaceMismatch')
    require(request['witness']['rule'] == 'original-static-inputs-captured-at-start-commit/v1'
            and request['witness']['transform'] == 'allowlist-route-static/v1', 'AvailabilityWitness')
    require(set(route['routing']['traffic']) == {'Static'}, 'UnsupportedTrafficContext')


def validate_export(export):
    require(export['schema'] == SCHEMA, 'ExportSchemaOrUnits')
    actions, inputs = export['baseline']['actions'], export['inputs']
    require(len(actions) == len(inputs) <= MAX_ROWS, 'CensusMismatchOrLimit')
    require(len({identity(a['key']) for a in actions}) == len(actions), 'DuplicateAction')
    for action, request in zip(actions, inputs):
        validate_request(request)
        require(action['key'] == request['key'] and action['original_route_sha256'] == request['route_sha256'], 'RequestBindingMismatch')
        require(action['b0_seconds'] == request['values'][0] and action['start_effective_at'] == request['reference_seconds'], 'OriginalPredictionMismatch')
        require(action['start_revision'] == request['witness']['start_revision'], 'StartRevisionMismatch')
        require(action['measurement'] == 'SyntheticControlled', 'UnsupportedMeasurementCohort')
        if action['label_eligible']:
            require(action['outcome'] == 'Observed' and finite(action['observed_seconds'])
                    and action['observed_seconds'] >= 0 and action['partition'] != 'Excluded', 'InvalidExactLabel')
    groups = {}
    for action in actions:
        old = groups.setdefault(action['dependency_group'], action['partition'])
        require(old == action['partition'], 'DependencyCrossesPartitions')


def mean(values):
    scale = max(map(abs, values), default=0.0)
    return scale * (math.fsum(v / scale for v in values) / len(values)) if scale else 0.0


def metrics(residuals):
    if not residuals:
        return None
    require(all(finite(v) for v in residuals), 'NonfiniteResidual')
    ordered = sorted(residuals)
    absolute = sorted(map(abs, ordered))
    positive = [v for v in ordered if v > 0]
    def quantiles(values):
        return {f'p{p}': values[(len(values) * p + 99) // 100 - 1] for p in (50, 95, 99)}
    middle = len(ordered) // 2
    median = ordered[middle] if len(ordered) % 2 else ordered[middle-1] / 2 + ordered[middle] / 2
    scale = absolute[-1]
    result = {
        'samples': len(ordered), 'mae_seconds': mean(absolute),
        'rmse_seconds': scale * math.sqrt(mean([(v / scale)**2 for v in ordered])) if scale else 0.0,
        'mean_signed_residual_seconds': mean(ordered), 'median_signed_residual_seconds': median,
        'absolute_error_quantiles_seconds': quantiles(absolute),
        'positive_residual_quantiles_seconds': quantiles(positive) if positive else None,
        'underprediction_count': len(positive), 'underprediction_frequency': len(positive)/len(ordered),
        'underprediction_severity_seconds': mean(positive) if positive else None,
    }
    encoded(result)  # finite representability is mandatory, including arithmetic overflow
    return result


def solve_spd(matrix, rhs):
    """Three-dimensional Cholesky; ridge makes collinear designs positive definite."""
    size = len(rhs)
    lower = [[0.0] * size for _ in range(size)]
    for i in range(size):
        for j in range(i + 1):
            value = matrix[i][j] - math.fsum(lower[i][k] * lower[j][k] for k in range(j))
            require(finite(value), 'NonfiniteFit')
            if i == j:
                require(value > 0, 'NonPositiveDefiniteFit')
                lower[i][j] = math.sqrt(value)
            else:
                lower[i][j] = value / lower[j][j]
    forward = []
    for i in range(size):
        forward.append((rhs[i] - math.fsum(lower[i][j]*forward[j] for j in range(i))) / lower[i][i])
    result = [0.0] * size
    for i in reversed(range(size)):
        result[i] = (forward[i] - math.fsum(lower[j][i]*result[j] for j in range(i+1, size))) / lower[i][i]
    require(all(finite(v) for v in result), 'NonfiniteCoefficients')
    return result


def fit(export):
    validate_export(export)
    if export['baseline']['independent_holdout'] != 'Available':
        return None, 'EvaluationUnavailable:InsufficientProtectedIndependentComponents'
    actions, inputs = export['baseline']['actions'], export['inputs']
    for role in ('Train', 'Validation', 'Test'):
        if not any(a['partition'] == role and a['label_eligible'] for a in actions):
            return None, 'EvaluationUnavailable:NoEligibleExactLabels:' + role
    pairs = [(a, x) for a, x in zip(actions, inputs) if a['partition'] == 'Train' and a['label_eligible']]
    columns = [[x['values'][j] for _, x in pairs] for j in range(2)]
    center = [mean(v) for v in columns]
    scale = [math.sqrt(mean([(v - c)**2 for v in col])) or 1.0 for c, col in zip(center, columns)]
    require(all(finite(v) and v > 0 for v in scale), 'NonfinitePreprocessing')
    design = [[1.0] + [(v - c)/s for v, c, s in zip(x['values'], center, scale)] for _, x in pairs]
    residuals = [a['observed_seconds'] - x['values'][0] for a, x in pairs]
    matrix = [[math.fsum(row[i]*row[j] for row in design) + (1.0 if i == j else 0.0)
               for j in range(3)] for i in range(3)]
    rhs = [math.fsum(row[j]*y for row, y in zip(design, residuals)) for j in range(3)]
    model = {
        'schema': MODEL_SCHEMA, 'protocol_sha256': identity(PROTOCOL),
        'target': PROTOCOL['target'], 'features': PROTOCOL['features'], 'units': ['seconds', 'meters'],
        'center': center, 'scale': scale, 'coefficients': solve_spd(matrix, rhs), 'ridge_lambda': 1.0,
        'minimum': list(map(min, columns)), 'maximum': list(map(max, columns)),
        'profiles': sorted({x['route']['routing']['profile'] for _, x in pairs}),
        'training_keys': [a['key'] for a, _ in pairs],
        'training_input_sha256': identity([x for _, x in pairs]),
        'training_labels_sha256': identity([a['observed_seconds'] for a, _ in pairs]),
        'dataset_sha256': export['baseline']['dataset_semantic_sha256'],
        'split_plan_sha256': export['baseline']['split_plan_sha256'],
        'runtime': 'Python IEEE-754 binary64; stdlib math; source retained',
    }
    return {'model_id': identity(model), 'parameters': model}, 'FittedOfflineOnly'


def predict(model, request, expected_request_sha256):
    """Bound request -> duration and exact effective inputs, or a typed failure.

    The caller obtains expected_request_sha256 from the source-verified export,
    independently of an inference worker. This interface accepts no edge label.
    """
    try:
        require(model is not None, 'ModelUnavailable')
        params = model['parameters']
        require(identity(params) == model['model_id'], 'ModelDigestMismatch')
        require(params['schema'] == MODEL_SCHEMA and params['protocol_sha256'] == identity(PROTOCOL)
                and params['target'] == PROTOCOL['target'] and params['features'] == PROTOCOL['features']
                and params['units'] == ['seconds', 'meters'] and params['ridge_lambda'] == 1.0, 'ModelSchemaOrUnits')
        for name, length in (('center', 2), ('scale', 2), ('minimum', 2), ('maximum', 2), ('coefficients', 3)):
            require(len(params[name]) == length and all(finite(v) for v in params[name]), 'ModelNumericShape')
        require(all(v > 0 for v in params['scale']), 'ModelScale')
        validate_request(request)
        require(identity(request) == expected_request_sha256, 'RequestContextMismatch')
        require(request['route']['routing']['profile'] in params['profiles'], 'UnsupportedProfile')
        values = request['values']
        require(all(lo <= v <= hi for v, lo, hi in zip(values, params['minimum'], params['maximum'])), 'OutsideTrainingSupport')
        effective = [1.0] + [(v-c)/s for v, c, s in zip(values, params['center'], params['scale'])]
        correction = math.fsum(c*v for c, v in zip(params['coefficients'], effective))
        duration = values[0] + correction
        require(finite(duration) and duration >= 0, 'InvalidPredictedDuration')
        return {'action_key': request['key'], 'duration_seconds': duration, 'failure': None, 'model_id': model['model_id'],
                'request_sha256': expected_request_sha256, 'effective_binary64_hex': [v.hex() for v in effective],
                'effective_values': effective, 'correction_seconds': correction}
    except (ContractError, KeyError, TypeError, ValueError, OverflowError) as error:
        return {'action_key': request.get('key') if isinstance(request, dict) else None, 'duration_seconds': None, 'failure': str(error), 'model_id': model.get('model_id') if isinstance(model, dict) else None,
                'request_sha256': expected_request_sha256, 'effective_binary64_hex': None,
                'effective_values': None, 'correction_seconds': None}


def fraction(n, d):
    return {'numerator': n, 'denominator': d, 'ratio': n/d if d else None}


def score(actions, predictions):
    require(len(actions) == len(predictions), 'PredictionCensusMismatch')
    require(len({identity(a['key']) for a in actions}) == len(actions), 'DuplicateScoringAction')
    for action, prediction in zip(actions, predictions):
        require(action['key'] == prediction['action_key'], 'PredictionActionMismatch')
        value = prediction['duration_seconds']
        require((value is None) == (prediction['failure'] is not None), 'PredictionFailureMismatch')
        require(value is None or (finite(value) and value >= 0), 'InvalidScoringPrediction')
    eligible, matched, b0, raw, fallback = [], [], [], [], []
    included = [(a, p) for a, p in zip(actions, predictions) if a['partition'] != 'Excluded']
    batch_fallback = any(p['failure'] is not None for _, p in included)
    for action, prediction in included:
        if action['label_eligible']:
            eligible.append(action)
            label = action['observed_seconds']
            b0.append(label - action['b0_seconds'])
            if prediction['duration_seconds'] is not None:
                matched.append(label - action['b0_seconds'])
                raw.append(label - prediction['duration_seconds'])
            fallback.append(label - (action['b0_seconds'] if batch_fallback else prediction['duration_seconds']))
    successes = sum(p['duration_seconds'] is not None for _, p in included)
    exact = sum(a['observed_seconds'] is not None for a in actions)
    return {
        'coverage': {
            'committed_starts': len(actions), 'excluded_starts': len(actions)-len(included),
            'outcomes': {s: sum(a['outcome'] == s for a in actions) for s in ('Observed', 'Pending', 'RightCensored', 'MissingArrival', 'DisputedOrInvalid')},
            'exact_label_coverage': fraction(exact, len(actions)),
            'eligible_label_coverage': fraction(len(eligible), len(included)),
            'prediction_coverage': fraction(successes, len(included)),
            'prediction_on_eligible_coverage': fraction(len(raw), len(eligible)),
            'census_groups': len({a['dependency_group'] for a in actions}),
            'paired_groups': len({a['dependency_group'] for a, p in included if a['label_eligible'] and p['failure'] is None}),
            'failures': dict(sorted(Counter(p['failure'] for _, p in included if p['failure']).items())),
            'label_exclusion_reasons': dict(sorted(Counter(r for a in actions for r in a['reasons']).items())),
        },
        'b0_full_eligible': metrics(b0), 'b0_matched_candidate': metrics(matched),
        'candidate_matched': metrics(raw), 'full_eligible_with_coherent_b0_fallback': metrics(fallback),
        'coherent_batch_fallback': batch_fallback,
        'metrics_unavailable_reason': None if eligible else 'NoEligibleExactLabels',
    }


def experiment(export):
    model, status = fit(export)
    actions = export['baseline']['actions']
    predictions = [predict(model, request, identity(request)) for request in export['inputs']]
    slices = {'all_included_descriptive': score(actions, predictions)}
    for dimension, values in export['baseline']['segments'].items():
        for value in values:
            indexes = [i for i, a in enumerate(actions) if a['segments'].get(dimension) == value]
            slices[dimension + ':' + value] = score([actions[i] for i in indexes], [predictions[i] for i in indexes])
    test = score([a for a in actions if a['partition'] == 'Test'],
                 [p for a, p in zip(actions, predictions) if a['partition'] == 'Test'])
    baseline, candidate = test['b0_full_eligible'], test['candidate_matched']
    improvement = bool(model and baseline and candidate and not test['coherent_batch_fallback']
                       and candidate['samples'] == baseline['samples']
                       and baseline['mae_seconds'] - candidate['mae_seconds'] > 1e-9
                       and candidate['rmse_seconds'] <= baseline['rmse_seconds'])
    return {'protocol': PROTOCOL, 'status': status, 'model': model, 'predictions': predictions,
            'slices': slices, 'protected_test': test, 'supported_offline_improvement': improvement,
            'operational_admission': 'EvaluationUnavailable:NotAuthorizedOrValidated',
            'physical_generalization': export['baseline']['physical_generalization'],
            'metric_uncertainty': 'EvaluationUnavailable:NoSamplingDesign',
            'prediction_uncertainty': 'EvaluationUnavailable:PointEstimateOnly'}


def source_snapshot():
    return {Path(__file__).name: Path(__file__).read_text()}


def write_json(path, value):
    with Path(path).open('xb') as file:
        file.write(encoded(value) + b'\n')
        file.flush()
        os.fsync(file.fileno())


def publish(dataset, destination, verifier):
    destination = Path(destination)
    require(not destination.exists(), 'PublicationExists')
    export = checked_export(dataset, verifier)
    result = experiment(export)
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix='.' + destination.name + '-', dir=destination.parent))
    try:
        (temporary / 'dataset').mkdir()
        for name in ('bundle.json', 'manifest.json'):
            shutil.copyfile(Path(dataset) / name, temporary / 'dataset' / name)
        # Reverify the retained copy, not merely the paths read before fitting.
        require(checked_export(temporary / 'dataset', verifier) == export, 'DatasetChangedDuringCapture')
        write_json(temporary / 'research.json', {'schema': 'eta-offline-publication/v1', 'export': export,
                   'experiment': result, 'source': source_snapshot(),
                   'environment': {'python': sys.version, 'platform': platform.platform(), 'implementation': platform.python_implementation()}})
        files = {str(p.relative_to(temporary)): sha(p.read_bytes()) for p in sorted(temporary.rglob('*')) if p.is_file()}
        write_json(temporary / 'manifest.json', {'schema': 'eta-offline-manifest/v1', 'files': files})
        # No overwrite even when another publisher races us. mkdir is the claim.
        destination.mkdir()
        try:
            for path in sorted(temporary.iterdir(), key=lambda p: p.name == 'manifest.json'):
                path.rename(destination / path.name)
            descriptor = os.open(destination, os.O_RDONLY)
            try:
                os.fsync(descriptor)
            finally:
                os.close(descriptor)
        except BaseException:
            shutil.rmtree(destination)
            raise
    finally:
        shutil.rmtree(temporary)
    return result


def verify(directory, verifier):
    directory = Path(directory)
    manifest = load_json(directory / 'manifest.json')
    require(manifest['schema'] == 'eta-offline-manifest/v1', 'ManifestSchema')
    require(set(manifest['files']) == {'research.json', 'dataset/bundle.json', 'dataset/manifest.json'}, 'ManifestFileSet')
    for name, expected in manifest['files'].items():
        require(sha((directory / name).read_bytes()) == expected, 'ArtifactDigestMismatch:' + name)
    publication = load_json(directory / 'research.json')
    require(publication['schema'] == 'eta-offline-publication/v1', 'PublicationSchema')
    require(publication['source'] == source_snapshot(), 'TrainingSourceMismatch:restore retained source for reproduction')
    export = checked_export(directory / 'dataset', verifier)
    require(export == publication['export'], 'ExportReproductionMismatch')
    require(experiment(export) == publication['experiment'], 'TrainingInferenceMetricReproductionMismatch')
    return publication


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['run', 'verify', 'predict'])
    parser.add_argument('input', type=Path)
    parser.add_argument('output_or_request', type=Path, nargs='?')
    parser.add_argument('--verifier', type=Path, default=Path('target/debug/roadrunner'))
    args = parser.parse_args()
    try:
        if args.operation == 'run':
            require(args.output_or_request is not None, 'NewOutputDirectoryRequired')
            result = publish(args.input, args.output_or_request, args.verifier)
            print(json.dumps({'status': result['status'], 'supported_offline_improvement': result['supported_offline_improvement'],
                              'operational_admission': result['operational_admission']}))
        else:
            publication = verify(args.input, args.verifier)
            if args.operation == 'verify':
                print('verified source capture, protected fitting, inference, metrics and immutable publication')
            else:
                require(args.output_or_request is not None, 'RequestRequired')
                request = load_json(args.output_or_request)
                expected = next((identity(x) for x in publication['export']['inputs'] if x == request), None)
                require(expected is not None, 'UnsupportedUnverifiedOrCounterfactualRequest')
                print(json.dumps(predict(publication['experiment']['model'], request, expected), allow_nan=False))
    except (ContractError, OSError, ValueError, KeyError, TypeError, OverflowError, subprocess.TimeoutExpired) as error:
        parser.exit(1, 'error: ' + str(error) + '\n')


if __name__ == '__main__':
    main()
