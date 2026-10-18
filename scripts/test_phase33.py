"""Contract tests for semantic comparison using retained real probe artifacts."""
import copy
import json
from pathlib import Path
import unittest
from compare_phase33 import compare

ROOT = Path(__file__).resolve().parents[1]


class ComparisonContracts(unittest.TestCase):
    def setUp(self):
        self.before = json.loads((ROOT/'benchmarks/results/phase33-control.json').read_text())
        self.after = json.loads((ROOT/'benchmarks/results/phase33-candidate.json').read_text())

    def test_matching_complete_evidence_is_accepted(self):
        self.assertEqual(len(compare(self.before, self.after)['cases']), len(self.before['cases']))

    def test_missing_workload_or_changed_context_is_rejected(self):
        for field in ['semantic_hash', 'manifest_sha256', 'graph_digest']:
            changed = copy.deepcopy(self.after)
            changed['cases'][0][field] = 'changed'
            with self.assertRaises(ValueError):
                compare(self.before, changed)
        changed = copy.deepcopy(self.after)
        changed['configuration'] = {}
        with self.assertRaises(ValueError):
            compare(self.before, changed)
        changed = copy.deepcopy(self.after)
        changed['cases'].append(changed['cases'][0])
        with self.assertRaises(ValueError):
            compare(self.before, changed)
        changed = copy.deepcopy(self.after)
        changed['cases'].pop()
        with self.assertRaises(ValueError):
            compare(self.before, changed)

    def test_tampered_sample_cannot_hide_behind_header_hash(self):
        self.after['cases'][0]['samples'][0]['probe']['result']['semantic_hash'] = 'changed'
        with self.assertRaises(ValueError):
            compare(self.before, self.after)


if __name__ == '__main__':
    unittest.main()
