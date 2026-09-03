#!/usr/bin/env python3
"""Unit tests for scripts/bench_compare.py's Comparison logic.

Covers the three cases from the cross-runner drift investigation
(PERF_EXPERIMENTS.md, "PR #779" and "PR #787" bench-failure triage
entries):

  (a) one bench regresses 20%, everything else flat        → must still fail
  (b) everything moves -8%, three benches move +40%         → must be
      reported as drift-suspect, must NOT fail the job
  (c) everything flat                                       → must pass

Run with: python3 -m unittest scripts/test_bench_compare.py
"""

import importlib.util
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "bench_compare", Path(__file__).with_name("bench_compare.py")
)
assert SPEC and SPEC.loader
bc = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(bc)


def _flat_suite(n: int, base_ns: float = 1000.0) -> dict[str, float]:
    """n unrelated benchmarks, all identical baseline values."""
    return {f"group/bench_{i}": base_ns for i in range(n)}


class RealRegressionTests(unittest.TestCase):
    """Case (a): a single genuine regression against an otherwise-flat table."""

    def test_single_regression_still_fails(self):
        baseline = _flat_suite(30)
        current = dict(baseline)
        current["group/bench_0"] = baseline["group/bench_0"] * 1.20  # +20%

        cmp = bc.Comparison(baseline, current)

        self.assertEqual(len(cmp.regressions), 1)
        self.assertEqual(cmp.regressions[0][0], "group/bench_0")
        self.assertFalse(cmp.suspect, "flat table should not look like drift")
        self.assertEqual(cmp.exit_code, 1, "real regression must still fail CI")

    def test_render_mentions_regression_and_no_drift_banner(self):
        baseline = _flat_suite(10)
        current = dict(baseline)
        current["group/bench_0"] *= 1.20
        cmp = bc.Comparison(baseline, current)
        text = cmp.render()
        self.assertIn("1 regression(s) detected", text)
        self.assertNotIn("drift", text.lower())


class DriftSuspectTests(unittest.TestCase):
    """Case (b): the #787 signature — a uniform floor shift plus a few
    benches moving further the other way."""

    def test_uniform_drift_with_outliers_is_suspect_and_fail_soft(self):
        baseline = _flat_suite(40)
        current = {}
        outliers = {"group/bench_0", "group/bench_1", "group/bench_2"}
        for name, base in baseline.items():
            if name in outliers:
                current[name] = base * 1.40  # +40%
            else:
                current[name] = base * 0.92  # -8%

        cmp = bc.Comparison(baseline, current)

        self.assertIsNotNone(cmp.drift)
        self.assertLess(cmp.drift, -0.03, "median should track the -8% floor")
        self.assertTrue(cmp.suspect)
        self.assertEqual(
            {name for name, *_ in cmp.regressions}, outliers, "outliers still listed"
        )
        self.assertEqual(
            cmp.exit_code, 3, "drift-suspect regressions must not hard-fail (exit 3)"
        )

    def test_render_shows_drift_banner_and_corrected_column(self):
        baseline = _flat_suite(40)
        current = {}
        for i, (name, base) in enumerate(baseline.items()):
            current[name] = base * (1.40 if i < 3 else 0.92)
        cmp = bc.Comparison(baseline, current)
        text = cmp.render()
        self.assertIn("Probable cross-runner drift", text)
        self.assertIn("Corrected", text)
        self.assertIn("did **not** fail the job", text)

    def test_few_benchmarks_never_trigger_drift(self):
        # Below MIN_BENCHES_FOR_DRIFT: even a uniform shift shouldn't be
        # trusted as "drift" — too small a sample.
        baseline = _flat_suite(3)
        current = {name: base * 0.80 for name, base in baseline.items()}
        cmp = bc.Comparison(baseline, current)
        self.assertIsNone(cmp.drift)
        self.assertFalse(cmp.suspect)


class FlatTableTests(unittest.TestCase):
    """Case (c): nothing moved."""

    def test_flat_table_passes(self):
        baseline = _flat_suite(25)
        current = dict(baseline)
        cmp = bc.Comparison(baseline, current)
        self.assertEqual(cmp.regressions, [])
        self.assertFalse(cmp.suspect)
        self.assertEqual(cmp.exit_code, 0)

    def test_small_noise_under_threshold_passes(self):
        baseline = _flat_suite(25)
        current = {name: base * 1.01 for name, base in baseline.items()}  # +1%
        cmp = bc.Comparison(baseline, current)
        self.assertEqual(cmp.regressions, [])
        self.assertEqual(cmp.exit_code, 0)


class RestrictModeTests(unittest.TestCase):
    """--restrict (same-runner suspect re-check) must never compute drift
    over its own biased subset."""

    def test_restrict_disables_drift_detection(self):
        baseline = _flat_suite(40)
        current = {}
        for i, (name, base) in enumerate(baseline.items()):
            current[name] = base * (1.40 if i < 3 else 0.92)
        restrict = {"group/bench_0", "group/bench_1", "group/bench_2"}

        cmp = bc.Comparison(baseline, current, restrict=restrict)

        self.assertIsNone(cmp.drift)
        self.assertFalse(cmp.suspect)
        self.assertEqual(len(cmp.regressions), 3)
        self.assertEqual(cmp.exit_code, 1, "restricted re-check still fails on real deltas")


class NewAndRemovedBenchTests(unittest.TestCase):
    def test_new_and_removed_benches_do_not_skew_drift_or_crash(self):
        baseline = _flat_suite(20)
        baseline["group/only_in_baseline"] = 500.0
        current = dict(baseline)
        del current["group/only_in_baseline"]
        current["group/only_in_current"] = 500.0

        cmp = bc.Comparison(baseline, current)
        self.assertEqual(cmp.exit_code, 0)
        self.assertTrue(any("removed" in row for row in cmp.rows))
        self.assertTrue(any("new" in row for row in cmp.rows))


if __name__ == "__main__":
    unittest.main()
