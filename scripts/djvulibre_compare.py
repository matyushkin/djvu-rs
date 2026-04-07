#!/usr/bin/env python3
"""Append a djvu-rs vs DjVuLibre comparison section to the benchmark report.

Usage:
    python3 scripts/djvulibre_compare.py \\
        --criterion target/criterion          \\
        --djvulibre-bench djvulibre_bench.txt \\
        --ddjvu-timing    ddjvu_timing.txt

Reads:
  djvulibre_bench.txt — output of djvulibre_bench (one or more file blocks).
  ddjvu_timing.txt    — lines produced by the ddjvu timing shell loop:
                          "<basename>.djvu@<dpi>dpi: <mean_ms> ms"

Outputs a Markdown section to stdout.  Always exits 0 (never blocks CI).
"""

import argparse
import json
import re
import sys
from pathlib import Path


# ---------------------------------------------------------------------------
# Criterion helpers
# ---------------------------------------------------------------------------

def load_criterion_ms(criterion_dir: Path, rel: str) -> float | None:
    """Return mean time in milliseconds from a Criterion estimates.json, or None."""
    p = criterion_dir / rel / "new" / "estimates.json"
    if not p.exists():
        return None
    try:
        ns = json.loads(p.read_text())["mean"]["point_estimate"]
        return ns / 1_000_000
    except (KeyError, json.JSONDecodeError, OSError):
        return None


# ---------------------------------------------------------------------------
# djvulibre_bench output parser
# ---------------------------------------------------------------------------

def parse_djvulibre_bench(text: str) -> dict[str, dict]:
    """Return {key: {"open_ms": float, "render_ms": float}} where key = "name@DPIdpi"."""
    results: dict[str, dict] = {}
    cur_key = ""
    cur: dict = {}
    for line in text.splitlines():
        m = re.match(r"file\s+:\s+(\S+)", line)
        if m:
            # flush previous block
            if cur_key and cur:
                results[cur_key] = cur
            cur = {}
            # Extract filename and output DPI from size line we'll see next
            cur["_file"] = Path(m.group(1)).name
            cur_key = ""
        m = re.match(r"size\s+:.*->.*@\d+dpi->(\d+)dpi", line)
        if m and cur.get("_file"):
            out_dpi = m.group(1)
            cur_key = f"{cur['_file']}@{out_dpi}dpi"
        m = re.match(r"open\+decode\s+:\s+([\d.]+)\s+ms", line)
        if m:
            cur["open_ms"] = float(m.group(1))
        m = re.match(r"render_only\s+:\s+([\d.]+)\s+ms", line)
        if m:
            cur["render_ms"] = float(m.group(1))
    if cur_key and cur:
        results[cur_key] = cur
    return results


# ---------------------------------------------------------------------------
# ddjvu CLI timing parser
# ---------------------------------------------------------------------------

def parse_ddjvu_timing(text: str) -> dict[str, float]:
    """Return {"name@DPIdpi": mean_ms} from timing lines like 'boy.djvu@72dpi: 28.5 ms'."""
    results: dict[str, float] = {}
    for line in text.splitlines():
        m = re.match(r"(\S+@\d+dpi)\s*:\s*([\d.]+)\s*ms", line)
        if m:
            results[m.group(1)] = float(m.group(2))
    return results


# ---------------------------------------------------------------------------
# Formatting
# ---------------------------------------------------------------------------

def fmt_ms(ms: float | None) -> str:
    if ms is None:
        return "—"
    if ms >= 1000:
        return f"{ms / 1000:.3f} s"
    if ms >= 1:
        return f"{ms:.2f} ms"
    return f"{ms * 1000:.0f} µs"


def ratio_cell(djvurs_ms: float | None, lib_ms: float | None) -> str:
    if djvurs_ms is None or lib_ms is None:
        return "—"
    r = lib_ms / djvurs_ms
    if r >= 1.05:
        return f"**{r:.1f}× faster**"
    if r < 0.952:   # 1/0.952 ≈ 1.05
        return f"{1/r:.1f}× slower"
    return "≈ equal"


# ---------------------------------------------------------------------------
# Comparison table definition
# ---------------------------------------------------------------------------
# Each row: (label, criterion_rel_path, lib_bench_key, ddjvu_key)
ROWS = [
    (
        "boy.djvu @ 72 dpi",
        "render_page/dpi/72",
        "boy.djvu@72dpi",
        "boy.djvu@72dpi",
    ),
    (
        "boy.djvu @ 144 dpi",
        "render_page/dpi/144",
        "boy.djvu@144dpi",
        "boy.djvu@144dpi",
    ),
    (
        "boy.djvu @ 300 dpi",
        "render_page/dpi/300",
        "boy.djvu@300dpi",
        "boy.djvu@300dpi",
    ),
]


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--criterion", type=Path, default=Path("target/criterion"),
                        help="Criterion output directory (default: target/criterion)")
    parser.add_argument("--djvulibre-bench", dest="lib_bench", type=Path, default=None,
                        help="djvulibre_bench output file")
    parser.add_argument("--ddjvu-timing", dest="ddjvu", type=Path, default=None,
                        help="ddjvu CLI timing file")
    args = parser.parse_args()

    lib = (parse_djvulibre_bench(args.lib_bench.read_text())
           if args.lib_bench and args.lib_bench.exists() else {})
    ddjvu = (parse_ddjvu_timing(args.ddjvu.read_text())
             if args.ddjvu and args.ddjvu.exists() else {})

    # Collect rows that have at least one non-None value
    table_rows = []
    for label, crit_rel, lib_key, ddjvu_key in ROWS:
        djvurs_ms = load_criterion_ms(args.criterion, crit_rel)
        lib_ms = lib.get(lib_key, {}).get("render_ms")
        ddjvu_ms = ddjvu.get(ddjvu_key)
        table_rows.append((label, djvurs_ms, lib_ms, ddjvu_ms))

    has_data = any(row[2] is not None or row[3] is not None for row in table_rows)
    if not has_data:
        return 0  # nothing to show; skip section silently

    print("\n### vs DjVuLibre 3.5.29\n")
    print("> **libdjvulibre (C API)**: render-only, page already decoded in memory.")
    print("> **ddjvu CLI**: includes process startup (~7 ms) and PPM output to `/dev/null`.")
    print("> Test file: `references/djvujs/library/assets/boy.djvu` (192×256 px, native 100 dpi).\n")

    print("| Benchmark | djvu-rs | libdjvulibre C API | ddjvu CLI | Ratio |")
    print("|-----------|---------|-------------------|-----------|-------|")
    for label, djvurs_ms, lib_ms, ddjvu_ms in table_rows:
        r = ratio_cell(djvurs_ms, lib_ms)
        print(f"| {label} | {fmt_ms(djvurs_ms)} | {fmt_ms(lib_ms)} | {fmt_ms(ddjvu_ms)} | {r} |")

    return 0


if __name__ == "__main__":
    sys.exit(main())
