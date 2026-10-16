"""Adversarial math, leakage, applicability, fallback and publication proof cases."""
import copy
import importlib.util
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'ml'))
import eta_engine as engine


def fixture():
    actions, inputs = [], []
    for i, role in enumerate(['Train'] * 3 + ['Validation', 'Test']):
        key = {'run': str(i), 'namespace': [0] * 16, 'action': 2**64-1}
        witness = {'start_revision': {'namespace': [0] * 16, 'number': 4},
                   'rule': 'original-static-inputs-captured-at-start-commit/v1', 'transform': 'allowlist-route-static/v1'}
        request = {'key': key, 'route': {'from': 0, 'to': 1, 'departed_at': 0.0, 'travel': 10.0,
                   'distance': 100.0, 'nodes': [0, 1], 'edges': [0],
                   'routing': {'profile': 'fixture/v1', 'traffic': {'Static': 'traffic'}, 'graph_digest': 'graph'}},
                   'route_sha256': 'route', 'reference_seconds': 0.0, 'witness': witness, 'values': [10.0, 100.0]}
        inputs.append(request)
        actions.append({'key': key, 'original_route_sha256': 'route', 'b0_seconds': 10.0, 'start_effective_at': 0.0,
                        'start_revision': witness['start_revision'], 'measurement': 'SyntheticControlled',
                        'partition': role, 'dependency_group': str(i), 'label_eligible': True,
                        'outcome': 'Observed', 'observed_seconds': 12.0, 'segments': {'partition': role}, 'reasons': []})
    return {'schema': engine.SCHEMA, 'inputs': inputs, 'baseline': {'actions': actions,
            'independent_holdout': 'Available', 'dataset_semantic_sha256': 'dataset', 'split_plan_sha256': 'split',
            'physical_generalization': {'EvaluationUnavailable': 'synthetic'},
            'segments': {'partition': {p: {} for p in ['Train', 'Validation', 'Test', 'Excluded']}}}}


class RidgeContracts(unittest.TestCase):
    def setUp(self):
        self.export = fixture()
        self.model, self.status = engine.fit(self.export)
        self.request = self.export['inputs'][-1]

    def test_hand_solved_regularized_constant_correction(self):
        # Min sum_3 (2-intercept)^2 + intercept^2 => intercept=1.5.
        coefficients = self.model['parameters']['coefficients']
        self.assertEqual(coefficients, [1.5, 0.0, 0.0])
        result = engine.predict(self.model, self.request, engine.identity(self.request))
        self.assertEqual(result['duration_seconds'], 11.5)
        self.assertEqual(result['effective_binary64_hex'], ['0x1.0000000000000p+0', '0x0.0p+0', '0x0.0p+0'])

    def test_heldout_labels_never_change_fit_or_predictions(self):
        changed = copy.deepcopy(self.export)
        for action in changed['baseline']['actions'][3:]:
            action['observed_seconds'] = 99999.0
        self.assertEqual(engine.fit(changed), (self.model, self.status))
        self.assertEqual(engine.experiment(changed)['predictions'], engine.experiment(self.export)['predictions'])

    def test_heldout_values_do_not_change_scaling_coefficients_or_train_identity(self):
        changed = copy.deepcopy(self.export)
        changed['inputs'][-1]['values'][1] = 1000.0
        changed['inputs'][-1]['route']['distance'] = 1000.0
        self.assertEqual(engine.fit(changed), (self.model, self.status))
        self.assertEqual(engine.experiment(changed)['predictions'][-1]['failure'], 'OutsideTrainingSupport')

    def test_train_only_preprocessing_with_nonconstant_columns(self):
        for i in range(3):
            self.export['inputs'][i]['values'][1] = 100.0 * i
            self.export['inputs'][i]['route']['distance'] = 100.0 * i
        model, _ = engine.fit(self.export)
        self.assertEqual(model['parameters']['center'], [10.0, 100.0])
        self.assertAlmostEqual(model['parameters']['scale'][1], math.sqrt(20000/3))

    def test_no_holdout_and_empty_label_roles_are_unavailable(self):
        self.export['baseline']['independent_holdout'] = {'EvaluationUnavailable': 'one-group'}
        self.assertIsNone(engine.fit(self.export)[0])
        self.export['baseline']['independent_holdout'] = 'Available'
        self.export['baseline']['actions'][-1].update(label_eligible=False, observed_seconds=None, outcome='RightCensored')
        self.assertEqual(engine.fit(self.export)[1], 'EvaluationUnavailable:NoEligibleExactLabels:Test')

    def test_shared_dependency_cannot_cross_protected_partitions(self):
        self.export['baseline']['actions'][-1]['dependency_group'] = '0'
        with self.assertRaisesRegex(engine.ContractError, 'DependencyCrossesPartitions'):
            engine.fit(self.export)

    def test_census_reordering_duplicate_and_missing_requests_reject(self):
        for mutation in ('reorder', 'duplicate', 'missing'):
            changed = copy.deepcopy(self.export)
            if mutation == 'reorder':
                changed['inputs'].reverse()
            elif mutation == 'duplicate':
                changed['baseline']['actions'][-1]['key'] = changed['baseline']['actions'][0]['key']
            else:
                changed['inputs'].pop()
            with self.assertRaises(engine.ContractError):
                engine.fit(changed)

    def test_future_or_identity_predictor_fields_reject(self):
        for field in ('arrival', 'scenario', 'seed', 'policy', 'future_traffic', 'actual_ready'):
            changed = copy.deepcopy(self.request)
            changed[field] = 20
            result = engine.predict(self.model, changed, engine.identity(changed))
            self.assertEqual(result['failure'], 'RequestSchema')

    def test_wrong_context_reference_and_stale_binding_fail(self):
        for field in ('route_sha256', 'reference_seconds', 'values'):
            changed = copy.deepcopy(self.request)
            changed[field] = 'different' if field == 'route_sha256' else 1 if field == 'reference_seconds' else [10.0, 200.0]
            result = engine.predict(self.model, changed, engine.identity(self.request))
            self.assertIsNone(result['duration_seconds'])
        changed = copy.deepcopy(self.request)
        changed['route']['routing']['profile'] = 'unsupported'
        self.assertEqual(engine.predict(self.model, changed, engine.identity(changed))['failure'], 'UnsupportedProfile')

    def test_missing_corrupt_schema_units_and_nonfinite_models_fail(self):
        self.assertEqual(engine.predict(None, self.request, '')['failure'], 'ModelUnavailable')
        bad = copy.deepcopy(self.model)
        bad['parameters']['coefficients'][0] += 1
        self.assertEqual(engine.predict(bad, self.request, '')['failure'], 'ModelDigestMismatch')
        for field, value in [('schema', 'unknown'), ('units', ['meters', 'seconds']), ('scale', [0.0, 1.0]),
                             ('coefficients', [0.0]), ('coefficients', [float('inf'), 0.0, 0.0])]:
            bad = copy.deepcopy(self.model)
            bad['parameters'][field] = value
            try:
                bad['model_id'] = engine.identity(bad['parameters'])
            except ValueError:
                pass  # JSON cannot encode a nonfinite model even to forge its digest.
            self.assertIsNone(engine.predict(bad, self.request, engine.identity(self.request))['duration_seconds'])

    def test_negative_prediction_is_failure_not_zero_clamp(self):
        bad = copy.deepcopy(self.model)
        bad['parameters']['coefficients'] = [-100.0, 0.0, 0.0]
        bad['model_id'] = engine.identity(bad['parameters'])
        result = engine.predict(bad, self.request, engine.identity(self.request))
        self.assertEqual(result['failure'], 'InvalidPredictedDuration')
        self.assertIsNone(result['duration_seconds'])

    def test_one_failed_request_forces_whole_batch_b0_with_matched_comparison(self):
        predictions = engine.experiment(self.export)['predictions']
        predictions[-1] = engine.predict(None, self.request, '')
        report = engine.score(self.export['baseline']['actions'], predictions)
        self.assertEqual(report['candidate_matched']['samples'], 4)
        self.assertEqual(report['b0_matched_candidate']['samples'], 4)
        self.assertEqual(report['b0_full_eligible']['samples'], 5)
        self.assertEqual(report['full_eligible_with_coherent_b0_fallback'], report['b0_full_eligible'])
        self.assertEqual(report['coverage']['prediction_on_eligible_coverage']['ratio'], 0.8)

    def test_different_candidate_populations_and_reordered_batches_reject(self):
        actions = self.export['baseline']['actions']
        predictions = engine.experiment(self.export)['predictions']
        with self.assertRaisesRegex(engine.ContractError, 'PredictionActionMismatch'):
            engine.score(actions, list(reversed(predictions)))
        with self.assertRaisesRegex(engine.ContractError, 'PredictionCensusMismatch'):
            engine.score(actions, predictions[:-1])

    def test_zero_exact_labels_are_valid_censored_and_excluded_retained(self):
        self.export['baseline']['actions'][0]['observed_seconds'] = 0.0
        self.export['baseline']['actions'][1].update(observed_seconds=None, label_eligible=False, outcome='RightCensored')
        self.export['baseline']['actions'][2].update(label_eligible=False, partition='Excluded')
        result = engine.experiment(self.export)
        report = result['slices']['all_included_descriptive']
        self.assertEqual(report['coverage']['committed_starts'], 5)
        self.assertEqual(report['coverage']['excluded_starts'], 1)
        self.assertEqual(report['coverage']['outcomes']['RightCensored'], 1)
        self.assertEqual(report['b0_full_eligible']['samples'], 3)

    def test_negative_or_unsupported_labels_reject(self):
        for field, value in [('observed_seconds', -1), ('observed_seconds', float('nan')), ('measurement', 'PhysicalMeasurementSupported')]:
            changed = copy.deepcopy(self.export)
            changed['baseline']['actions'][0][field] = value
            with self.assertRaises(engine.ContractError):
                engine.fit(changed)

    def test_metrics_hand_oracle_and_zero_empty_extreme(self):
        report = engine.metrics([-2.0, 0.0, 4.0, 6.0])
        self.assertEqual(report['mae_seconds'], 3)
        self.assertAlmostEqual(report['rmse_seconds'], math.sqrt(14))
        self.assertEqual(report['mean_signed_residual_seconds'], 2)
        self.assertEqual(report['median_signed_residual_seconds'], 2)
        self.assertEqual(report['absolute_error_quantiles_seconds'], {'p50': 2, 'p95': 6, 'p99': 6})
        self.assertEqual(report['underprediction_frequency'], 0.5)
        self.assertEqual(report['underprediction_severity_seconds'], 5)
        self.assertIsNone(engine.metrics([]))
        self.assertEqual(engine.metrics([0.0])['mae_seconds'], 0)
        self.assertTrue(math.isfinite(engine.metrics([-1e308, 1e308])['rmse_seconds']))
        with self.assertRaises(engine.ContractError):
            engine.metrics([float('nan')])

    def test_repeatable_fit_and_negative_experiment_do_not_admit(self):
        self.assertEqual(engine.experiment(self.export), engine.experiment(copy.deepcopy(self.export)))
        self.assertTrue(engine.experiment(self.export)['supported_offline_improvement'])
        self.assertTrue(engine.experiment(self.export)['operational_admission'].startswith('EvaluationUnavailable'))
        for action in self.export['baseline']['actions']:
            action['observed_seconds'] = 10.0
        self.assertFalse(engine.experiment(self.export)['supported_offline_improvement'])

    def test_strict_json_rejects_duplicate_nan_and_large_artifact(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'bad.json'
            for value in ('{"a":1,"a":2}', '{"a":NaN}', '{"a":Infinity}', '{"a":1e999}'):
                path.write_text(value)
                with self.assertRaises(engine.ContractError):
                    engine.load_json(path)


class SourceBackedIntegration(unittest.TestCase):
    def test_rehashed_model_tampering_fails_training_reproduction(self):
        import shutil
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / 'result'
            shutil.copytree(ROOT/'ml/results/route-ridge-null-v1', destination)
            publication = engine.load_json(destination/'research.json')
            model = publication['experiment']['model']
            model['parameters']['coefficients'][0] = 1.0
            model['model_id'] = engine.identity(model['parameters'])
            (destination/'research.json').write_bytes(engine.encoded(publication))
            manifest = engine.load_json(destination/'manifest.json')
            manifest['files']['research.json'] = engine.sha((destination/'research.json').read_bytes())
            (destination/'manifest.json').write_bytes(engine.encoded(manifest))
            with self.assertRaisesRegex(engine.ContractError, 'TrainingInferenceMetricReproductionMismatch'):
                engine.verify(destination, ROOT/'target/debug/roadrunner')

    def test_bound_prediction_cli_returns_effective_values_and_identity(self):
        result = ROOT/'ml/results/route-ridge-null-v1'
        publication = engine.load_json(result/'research.json')
        original = publication['export']['inputs'][-1]
        with tempfile.TemporaryDirectory() as directory:
            request = Path(directory)/'request.json'
            request.write_bytes(engine.encoded(original))
            output = subprocess.run([sys.executable, str(ROOT/'ml/eta_engine.py'), 'predict', str(result),
                                     str(request), '--verifier', str(ROOT/'target/debug/roadrunner')],
                                    capture_output=True, check=True)
            prediction = json.loads(output.stdout)
            self.assertEqual(prediction['action_key'], original['key'])
            self.assertEqual(prediction['request_sha256'], engine.identity(original))
            self.assertEqual(prediction['model_id'], publication['experiment']['model']['model_id'])
            self.assertEqual(prediction['duration_seconds'], original['values'][0])
            self.assertIsNone(prediction['failure'])
            self.assertEqual(len(prediction['effective_binary64_hex']), 3)

    def test_verified_export_publication_corruption_and_counterfactual_refusal(self):
        verifier = ROOT / 'target/debug/roadrunner'
        self.assertTrue(verifier.exists(), 'build roadrunner-cli before running ML tests')
        dataset = ROOT / 'ml/data/phase23-representative-v1'
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'result'
            result = engine.publish(dataset, output, verifier)
            self.assertIsNone(result['model'])
            self.assertEqual(result['slices']['all_included_descriptive']['coverage']['committed_starts'], 5)
            self.assertFalse(result['supported_offline_improvement'])
            publication = engine.verify(output, verifier)
            self.assertEqual(result, publication['experiment'])
            with self.assertRaisesRegex(engine.ContractError, 'PublicationExists'):
                engine.publish(dataset, output, verifier)
            request = Path(directory) / 'request.json'
            request.write_text('{}')
            run = subprocess.run([sys.executable, str(ROOT/'ml/eta_engine.py'), 'predict', str(output), str(request),
                                  '--verifier', str(verifier)], capture_output=True)
            self.assertNotEqual(run.returncode, 0)
            self.assertIn(b'UnsupportedUnverifiedOrCounterfactualRequest', run.stderr)
            (output / 'research.json').write_text('{}')
            with self.assertRaisesRegex(engine.ContractError, 'ArtifactDigestMismatch'):
                engine.verify(output, verifier)

    def test_fixture_generator_exact_and_distinct_topologies(self):
        spec = importlib.util.spec_from_file_location('generator', ROOT/'scripts/generate_eta_research_fixtures.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            generated = Path(directory) / 'generated'
            module.generate(generated)
            expected = ROOT/'data/fixtures/phase-25'
            self.assertEqual({p.name: p.read_bytes() for p in generated.iterdir()},
                             {p.name: p.read_bytes() for p in expected.iterdir()})
            graphs = [json.loads((generated/(name+'.json')).read_text())['graph'] for name, *_ in module.DESIGNS]
            self.assertEqual(len({engine.identity(g) for g in graphs}), 6)


if __name__ == '__main__':
    unittest.main()
