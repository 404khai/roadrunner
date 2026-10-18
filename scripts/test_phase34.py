"""Generator, isolation and observed/unobserved execution contracts."""
import copy
import json
import os
from pathlib import Path
import tempfile
import sys
import unittest
from phase34 import Rng, encode, generate, run, scenario, sha

ROOT = Path(__file__).resolve().parents[1]


class GeneratorContracts(unittest.TestCase):
    def setUp(self):
        self.manifest = json.loads((ROOT/'benchmarks/phase34/smoke.json').read_text())
        self.graph = dict(nodes=64, graph_digest='exact-test-graph')

    def test_reference_rng_and_byte_determinism(self):
        self.assertEqual(Rng(0).next(), 0xe220a8397b1dcdaf)
        first = encode(scenario(self.manifest, self.graph))
        self.assertEqual(first, encode(scenario(self.manifest, self.graph)))
        self.manifest['seed'] += 1
        self.assertNotEqual(first, encode(scenario(self.manifest, self.graph)))

    def test_independent_populations_and_future_arrivals_retained(self):
        self.manifest['available_riders'] = 1
        self.manifest['horizon_seconds'] = 0
        s = scenario(self.manifest, self.graph)
        self.assertEqual(sum(r['available'] for r in s['riders']), 1)
        self.assertEqual(len(s['orders']), self.manifest['orders'])
        self.assertGreater(s['orders'][-1]['created_at_seconds'], s['end_seconds'])
        self.assertEqual(s['graph_snapshot_digest'], self.graph['graph_digest'])
        self.assertTrue(all(0 <= o['pickup_node'] < 64 and 0 <= o['dropoff_node'] < 64 for o in s['orders']))

    def test_unknown_or_invalid_configuration_fails_closed(self):
        changes = [dict(unknown=True), dict(available_riders=100), dict(seed=-1), dict(orders=100001), dict(geography='made_up')]
        for change in changes:
            with self.assertRaises(ValueError):
                scenario(self.manifest | change, self.graph)

    def test_arrival_families_are_exogenous(self):
        for kind in ['mass', 'sustained', 'burst', 'backlog_drain']:
            m = copy.deepcopy(self.manifest)
            m['arrival']['kind'] = kind
            if kind == 'burst':
                m['arrival']['interval_seconds'] = 0
            s = scenario(m, self.graph)
            times = [o['created_at_seconds'] for o in s['orders']]
            self.assertEqual(times, sorted(times))
            self.assertEqual(len(times), m['orders'])
            if kind == 'mass':
                self.assertEqual(set(times), {0})


class ExternalBoundaryContracts(unittest.TestCase):
    def test_sampled_memory_and_output_stops_are_external(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bundle = root/'bundle'
            bundle.mkdir()
            (bundle/'bundle.json').write_text('{"scenario_sha256":"fake-child-input"}')
            fake = root/'isolated-child'
            fake.write_text('#!'+sys.executable+"\nimport pathlib,sys,time\noutput=pathlib.Path(sys.argv[3]); output.mkdir()\npayload=bytearray(16*1024*1024)\n(output/'payload').write_bytes(payload)\ntime.sleep(30)\n")
            fake.chmod(0o700)
            memory = run(bundle, root/'memory', fake, 'test-fake-child', 10, 1, 128)
            self.assertEqual(memory['outcome'], 'harness_sampled_rss_stop')
            self.assertIsNone(memory['final'])
            disk = run(bundle, root/'disk', fake, 'test-fake-child', 10, 1024, 1)
            self.assertEqual(disk['outcome'], 'harness_output_stop')
            self.assertIsNone(disk['final'])


@unittest.skipUnless(os.environ.get('PHASE34_BINARY'), 'requires built Phase 34 example; CI invokes explicitly')
class EngineContracts(unittest.TestCase):
    def test_complete_evidence_equivalence_and_tamper_refusal(self):
        binary = Path(os.environ['PHASE34_BINARY']).resolve()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            generate(ROOT/'benchmarks/phase34/smoke.json', root/'bundle', binary)
            a = run(root/'bundle', root/'a', binary, 'test-build', 30, 1024, 128)
            b = run(root/'bundle', root/'b', binary, 'test-build', 30, 1024, 128, observed=False)
            self.assertEqual(a['outcome'], 'completed')
            self.assertEqual(b['outcome'], 'completed')
            self.assertEqual(a['final']['result_sha256'], b['final']['result_sha256'])
            self.assertEqual(a['final']['summary']['delivered_orders'], 12)
            self.assertEqual(sha((root/'a/engine/result.json').read_bytes()), a['final']['result_sha256'])
            self.assertGreater(a['final']['processed_entries'], 12)
            path = root/'bundle/scenario.json'
            path.write_text(path.read_text().replace('"seed":34', '"seed":35'))
            bad = run(root/'bundle', root/'bad', binary, 'test-build', 30, 1024, 128)
            self.assertEqual(bad['outcome'], 'engine_failure')
            self.assertIsNone(bad['final'])

    def test_planning_policies_preserve_observed_result_and_fail_closed_budget(self):
        binary = Path(os.environ['PHASE34_BINARY']).resolve()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name in ['insertion', 'fleet', 'recovery', 'budget-incomplete']:
                generate(ROOT/f'benchmarks/phase34/{name}.json', root/name, binary)
                a = run(root/name, root/f'{name}-a', binary, 'test-build', 30, 1024, 128)
                b = run(root/name, root/f'{name}-b', binary, 'test-build', 30, 1024, 128, observed=False)
                self.assertEqual(a['outcome'], 'completed', name)
                self.assertEqual(b['outcome'], 'completed', name)
                self.assertEqual(a['final']['result_sha256'], b['final']['result_sha256'], name)
                if name == 'budget-incomplete':
                    self.assertEqual(a['final']['summary']['assigned_orders'], 0)
                    self.assertGreater(a['final']['summary']['outstanding_orders'], 0)

    def test_concurrent_routing_preserves_algorithm_results_and_cost_oracle(self):
        binary = Path(os.environ['PHASE34_BINARY']).resolve()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            generate(ROOT/'benchmarks/phase34/smoke.json', root/'bundle', binary)
            routes = {}
            for algorithm in ['dijkstra', 'astar']:
                for concurrency in [1, 4]:
                    destination = root/f'{algorithm}-{concurrency}'
                    record = run(root/'bundle', destination, binary, 'test-build', 30, 1024, 128,
                                 routing=dict(algorithm=algorithm, queries=24, concurrency=concurrency))
                    self.assertEqual(record['outcome'], 'completed')
                    self.assertEqual(record['final']['successful'], 24)
                    routes[(algorithm, concurrency)] = json.loads((destination/'engine/routes.json').read_text())
                self.assertEqual(routes[(algorithm, 1)], routes[(algorithm, 4)])
            self.assertEqual(len(routes[('dijkstra', 1)]), len(routes[('astar', 1)]))
            for d, a in zip(routes[('dijkstra', 1)], routes[('astar', 1)]):
                self.assertEqual(d['result']['route']['total_cost'], a['result']['route']['total_cost'])

    def test_harness_timeout_does_not_invent_engine_result(self):
        binary = Path(os.environ['PHASE34_BINARY']).resolve()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            generate(ROOT/'benchmarks/phase34/tier-c.json', root/'bundle', binary)
            stopped = run(root/'bundle', root/'stopped', binary, 'test-build', .01, 1024, 128)
            self.assertEqual(stopped['outcome'], 'harness_wall_timeout')
            self.assertIsNone(stopped['final'])
            self.assertFalse((root/'stopped/engine/report.json').exists())


class SummaryContracts(unittest.TestCase):
    def test_summary_retains_failure_without_fabricating_counts(self):
        from summarize_phase34 import summarize
        stopped = dict(bundle=dict(scenario_sha256='input'), configuration=dict(diagnostics=True),
                       hardware={}, binary_sha256='binary', source_revision='source', compiler='rustc',
                       outcome='harness_wall_timeout', final=None, resource_samples=[], last_observed_boundary=None)
        case = summarize([stopped])['cases'][0]
        self.assertEqual(case['completed'], 0)
        self.assertIsNone(case['execution_ns'])
        self.assertEqual(case['outcomes'], ['harness_wall_timeout'])
        self.assertEqual(case['final_counts'], [])

    def test_histogram_upper_bounds_and_empty_population(self):
        from summarize_phase34 import histogram_quantiles
        self.assertIsNone(histogram_quantiles([0]*65))
        buckets = [0]*65
        buckets[3] = 100
        self.assertEqual(histogram_quantiles(buckets), dict(samples=100, p50_upper_ns=7, p95_upper_ns=7, p99_upper_ns=7))


if __name__ == '__main__':
    unittest.main()
