"""Checks comparison semantics without requiring Docker or network access."""

import unittest
from unittest.mock import patch

import run


class ComparisonTests(unittest.TestCase):
    def test_identical_geometry_has_zero_deviation(self):
        line = [[3.0, 6.0], [3.0001, 6.0], [3.0002, 6.0001]]
        self.assertEqual(run.geometry_difference(line, line)["symmetric_max_deviation_meters"], 0)

    def test_geometry_deviation_is_symmetric(self):
        first = [[3.0, 6.0], [3.001, 6.0]]
        second = [[3.0, 6.0001], [3.001, 6.0001]]
        self.assertEqual(run.geometry_difference(first, second), run.geometry_difference(second, first))
        self.assertGreater(run.geometry_difference(first, second)["symmetric_max_deviation_meters"], 10)

    def test_reference_no_route_is_distinct_from_reference_error(self):
        query = {"origin": [3.0, 6.0], "destination": [3.001, 6.001]}
        with patch.object(run, "request", return_value={"code": "NoRoute"}):
            self.assertEqual(run.reference_route("http://localhost", query)["status"], "unreachable")
        with patch.object(run, "request", return_value={"code": "NoSegment", "message": "missing road"}):
            self.assertEqual(run.reference_route("http://localhost", query)["status"], "reference_error")


if __name__ == "__main__":
    unittest.main()
