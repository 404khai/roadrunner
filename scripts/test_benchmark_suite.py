"""Correctness/reproducibility contracts, with no wall-clock performance thresholds."""
import argparse
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import benchmark_suite as suite


class SuiteContracts(unittest.TestCase):
    def test_statistics_and_invalid_samples(self):
        row = suite.stats([10, 20, 30])
        self.assertEqual(row["median_ns"], 20)
        self.assertEqual(row["p95_ns"], 29)
        self.assertEqual(row["p99_ns"], 29.8)
        for values in [[], [1], [1, float("nan")], [-1, 2]]:
            with self.assertRaises(ValueError):
                suite.stats(values)

    def test_explicit_legacy_adapter_changes_only_declared_contract_fields(self):
        manifest = suite.read(suite.ROOT / "benchmarks/suite/manifest.json")
        case, source = list(suite.definitions(manifest))[-1]
        original = copy.deepcopy(source)
        legacy = suite.materialize(source, case, case["policies"][0])
        pooled = suite.materialize(source, case, case["policies"][1])
        self.assertEqual(source, original)
        self.assertEqual(pooled, source)
        restored = copy.deepcopy(legacy)
        restored["scenario"]["schema_version"] = 2
        restored["scenario"]["dispatch"] = source["scenario"]["dispatch"]
        for order, original_order in zip(restored["scenario"]["orders"], source["scenario"]["orders"]):
            order["admission"] = original_order["admission"]
        self.assertEqual(restored, source)
        invalid = copy.deepcopy(manifest)
        invalid["simulation"].pop()
        with self.assertRaises(ValueError):
            list(suite.definitions(invalid))

    def test_required_policy_pair_cannot_be_replaced_by_duplicate(self):
        manifest = suite.read(suite.ROOT / "benchmarks/suite/manifest.json")
        manifest["simulation"][0]["policies"][1] = manifest["simulation"][0]["policies"][0]
        with self.assertRaisesRegex(ValueError, "policy variants"):
            list(suite.definitions(manifest))

    def test_path_escape_rejected(self):
        with self.assertRaises(ValueError):
            suite.bounded_path(suite.ROOT, "../outside.json")


class ExecutedSuite(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binary = suite.ROOT / "target/debug/roadrunner"
        cls.probe = suite.ROOT / "target/debug/examples/routing_comparison"
        if not cls.binary.is_file() or not cls.probe.is_file():
            raise unittest.SkipTest("build CLI and routing_comparison first")
        cls.temporary = tempfile.TemporaryDirectory(prefix="roadrunner-suite-tests-")
        cls.directory = Path(cls.temporary.name) / "valid"
        cls.args = argparse.Namespace(manifest=suite.ROOT / "benchmarks/suite/manifest.json", output=cls.directory, binary=cls.binary, routing_binary=cls.probe, runs=2, timeout=30, build_profile="debug", build_command="cargo build --locked -p roadrunner-cli --bin roadrunner --example routing_comparison")
        suite.collect(cls.args)

    @classmethod
    def tearDownClass(cls):
        cls.temporary.cleanup()

    def cloned_run(self, name):
        import shutil
        path = Path(self.temporary.name) / name
        shutil.copytree(self.directory, path)
        return path

    def test_real_suite_accounts_all_five_comparisons_and_refuses_overwrite(self):
        record = suite.verify(self.directory)
        self.assertEqual(record["status"], "completed")
        self.assertEqual(len(record["simulation"]), 3)
        self.assertEqual(set(record["routing"]["query_timing"]), suite.VARIANTS)
        with self.assertRaises(FileExistsError):
            suite.collect(self.args)

    def test_artifact_tampering_rejected(self):
        path = self.cloned_run("hash-tampering")
        (path / "routing-replay.json").write_text("{}")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            suite.verify(path)

    def test_false_completion_counts_rejected_even_with_updated_hash(self):
        path = self.cloned_run("accounting-tampering")
        index = suite.read(path / "index.json")
        reference = index["simulation"][0]["variants"][0]["replay"]
        result = suite.load_asset(path, reference)
        result["summary"]["delivered_orders"] += 1
        updated = suite.asset(path, reference["path"], result)
        reference.update(updated)
        (path / "index.json").write_text(json.dumps(index))
        with self.assertRaisesRegex(ValueError, "accounting mismatch"):
            suite.verify(path)

    def test_mismatched_variant_inputs_rejected_even_with_updated_hash(self):
        path = self.cloned_run("input-tampering")
        index = suite.read(path / "index.json")
        reference = index["simulation"][0]["variants"][0]["input"]
        document = suite.load_asset(path, reference)
        document["scenario"]["seed"] += 1
        reference.update(suite.asset(path, reference["path"], document))
        (path / "index.json").write_text(json.dumps(index))
        with self.assertRaisesRegex(ValueError, "unmatched variant"):
            suite.verify(path)

    def test_timeout_cannot_become_successful_run(self):
        args = copy.copy(self.args)
        args.output = Path(self.temporary.name) / "timed-out"
        with patch.object(suite, "command_json", side_effect=subprocess.TimeoutExpired("probe", 1)):
            with self.assertRaises(subprocess.TimeoutExpired):
                suite.collect(args)
        self.assertEqual(suite.read(args.output / "index.json")["status"], "failed")
        with self.assertRaisesRegex(ValueError, "incomplete"):
            suite.verify(args.output)


if __name__ == "__main__":
    unittest.main()
