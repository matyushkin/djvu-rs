#!/usr/bin/env python3
"""Unit tests for scripts/djvulibre_compare.py's Criterion lookup.

The native-resolution rows prefer the reused-buffer stage bench and fall back
to the older `render_corpus_*` bench (RENDER_VS_DJVULIBRE_ALLOC in
PERF_EXPERIMENTS.md).

Run with: python3 -m unittest scripts/test_djvulibre_compare.py
"""

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "djvulibre_compare", Path(__file__).with_name("djvulibre_compare.py")
)
assert SPEC and SPEC.loader
dc = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(dc)

REUSE = "render_native_stages/render_into_reuse_buffer/watchmaker_color"
PIXMAP = "render_corpus_color"


def _write(root: Path, rel: str, mean_ns: float) -> None:
    d = root / rel / "new"
    d.mkdir(parents=True)
    (d / "estimates.json").write_text(json.dumps({"mean": {"point_estimate": mean_ns}}))


class FirstCriterionMs(unittest.TestCase):
    def test_prefers_first_present_path(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _write(root, REUSE, 55e6)
            _write(root, PIXMAP, 58e6)
            self.assertEqual(dc.load_first_criterion_ms(root, (REUSE, PIXMAP)), 55.0)

    def test_falls_back_when_stage_bench_missing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _write(root, PIXMAP, 58e6)
            self.assertEqual(dc.load_first_criterion_ms(root, (REUSE, PIXMAP)), 58.0)

    def test_none_when_nothing_present(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertIsNone(dc.load_first_criterion_ms(Path(tmp), (REUSE, PIXMAP)))

    def test_plain_string_path(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _write(root, PIXMAP, 2e6)
            self.assertEqual(dc.load_first_criterion_ms(root, PIXMAP), 2.0)

    def test_native_rows_prefer_reused_buffer(self):
        rels = {label: crit for label, crit, _, _ in dc.ROWS}
        native = [r for label, r in rels.items() if "native" in label]
        self.assertEqual(len(native), 2)
        for r in native:
            self.assertIsInstance(r, tuple)
            self.assertTrue(r[0].startswith("render_native_stages/render_into_reuse_buffer/"))


if __name__ == "__main__":
    unittest.main()
