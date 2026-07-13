#!/usr/bin/env python3

import importlib.util
import json
from unittest import mock
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "conformance_report", Path(__file__).with_name("conformance_report.py")
)
assert SPEC and SPEC.loader
report = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(report)


class ValidateTests(unittest.TestCase):
    def setUp(self):
        self.manifest = {
            "render": {"max_mismatch_pct": 0.5, "max_mean_abs_diff": 0.2},
            "documents": [{"path": "a.djvu", "pages": 2}],
        }
        self.row = {
            "file": "a.djvu",
            "page": 0,
            "width": 10,
            "height": 20,
            "total_px": 200,
            "mismatched_px": 0,
            "mismatch_pct": 0.0,
            "max_abs_diff": 0,
            "mean_abs_diff": 0.0,
        }

    def test_complete_results_pass(self):
        second = {**self.row, "page": 1}
        self.assertEqual(report.validate(self.manifest, [self.row, second]), [])

    def test_missing_page_fails_closed(self):
        failures = report.validate(self.manifest, [self.row])
        self.assertIn("missing result for a.djvu page 1", failures)

    def test_duplicate_and_threshold_regression_fail(self):
        bad = {**self.row, "mismatch_pct": 0.6, "mean_abs_diff": 0.3}
        failures = report.validate(self.manifest, [bad, bad])
        self.assertTrue(any("duplicate" in item for item in failures))
        self.assertTrue(any("mismatch" in item for item in failures))
        self.assertTrue(any("mean delta" in item for item in failures))

    def test_non_finite_and_impossible_metrics_fail(self):
        second = {**self.row, "page": 1, "mismatch_pct": float("nan")}
        failures = report.validate(self.manifest, [self.row, second])
        self.assertTrue(any("invalid metric" in item for item in failures))
        impossible = {
            **self.row,
            "page": 1,
            "total_px": 199,
            "mismatched_px": 200,
        }
        failures = report.validate(self.manifest, [self.row, impossible])
        self.assertTrue(any("invalid dimensions" in item for item in failures))

    def test_semantic_results_are_complete_and_matching(self):
        rows = []
        for page in range(2):
            for plane in ("text", "annotations"):
                rows.append(
                    {
                        "file": "a.djvu",
                        "page": page,
                        "plane": plane,
                        "status": "match",
                        "ours": "x",
                        "djvulibre": "x",
                    }
                )
        rows.append(
            {
                "file": "a.djvu",
                "page": 0,
                "plane": "bookmarks",
                "status": "match",
                "ours": "",
                "djvulibre": "",
            }
        )
        self.assertEqual(report.validate_semantic(self.manifest, rows), [])
        rows[0]["status"] = "diverge"
        failures = report.validate_semantic(self.manifest, rows)
        self.assertTrue(any("semantic divergence" in item for item in failures))

    def test_semantic_status_must_match_payload(self):
        row = {
            "file": "a.djvu",
            "page": 0,
            "plane": "text",
            "status": "match",
            "ours": "a",
            "djvulibre": "b",
        }
        failures = report.validate_semantic(self.manifest, [row])
        self.assertTrue(any("contradiction" in item for item in failures))

    def test_history_must_be_array(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "history.json"
            path.write_text(json.dumps({"not": "an array"}))
            with self.assertRaises(RuntimeError):
                report.load_history(path)

    @mock.patch.object(report.subprocess, "run")
    def test_ddjvu_nonzero_help_still_reports_banner(self, run):
        run.return_value = mock.Mock(
            returncode=10, stdout="", stderr="DDJVU --- DjVuLibre-3.5.29\nUsage"
        )
        self.assertEqual(report.djvulibre_version(), "DDJVU --- DjVuLibre-3.5.29")


if __name__ == "__main__":
    unittest.main()
