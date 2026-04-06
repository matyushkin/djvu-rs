#!/usr/bin/env python3
"""Compare two sets of Criterion benchmark results and report regressions.

Usage:
    python3 scripts/bench_compare.py <baseline_criterion_dir> <current_criterion_dir>

Exits with code 0 when no regressions exceed the threshold, 1 otherwise.
Outputs a Markdown table suitable for posting as a PR comment.
"""

import json
import sys
from pathlib import Path

REGRESSION_THRESHOLD = 0.05  # 5 %


def load_results(criterion_dir: Path) -> dict[str, float]:
    """Return {bench_path: mean_ns} for every estimates.json under criterion_dir."""
    results: dict[str, float] = {}
    for f in sorted(criterion_dir.rglob("*/new/estimates.json")):
        rel = f.relative_to(criterion_dir)
        # rel parts: <group>/<bench>/new/estimates.json  → drop last two
        bench_name = "/".join(rel.parts[:-2])
        try:
            data = json.loads(f.read_text())
            results[bench_name] = data["mean"]["point_estimate"]
        except (KeyError, json.JSONDecodeError):
            continue
    return results


def fmt_ns(ns: float) -> str:
    if ns >= 1_000_000_000:
        return f"{ns / 1_000_000_000:.3f} s"
    if ns >= 1_000_000:
        return f"{ns / 1_000_000:.2f} ms"
    if ns >= 1_000:
        return f"{ns / 1_000:.1f} µs"
    return f"{ns:.0f} ns"


def main() -> int:
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} <baseline_dir> <current_dir>", file=sys.stderr)
        return 2

    baseline_dir = Path(sys.argv[1])
    current_dir = Path(sys.argv[2])

    baseline = load_results(baseline_dir) if baseline_dir.exists() else {}
    current = load_results(current_dir) if current_dir.exists() else {}

    if not current:
        print("No benchmark results found in current run.")
        return 0

    if not baseline:
        print("### Benchmark results (no baseline for comparison)\n")
        print("| Benchmark | Current |")
        print("|-----------|---------|")
        for name, cur in sorted(current.items()):
            print(f"| `{name}` | {fmt_ns(cur)} |")
        return 0

    regressions: list[tuple[str, float, float, float]] = []
    rows: list[str] = []

    all_names = sorted(set(baseline) | set(current))
    for name in all_names:
        cur = current.get(name)
        base = baseline.get(name)

        if cur is None:
            rows.append(f"| `{name}` | {fmt_ns(base)} | — | removed |")
            continue
        if base is None:
            rows.append(f"| `{name}` | — | {fmt_ns(cur)} | new |")
            continue

        delta = (cur - base) / base
        sign = "+" if delta >= 0 else ""
        badge = ""
        if delta > REGRESSION_THRESHOLD:
            regressions.append((name, base, cur, delta))
            badge = " ⚠️"
        elif delta < -REGRESSION_THRESHOLD:
            badge = " ✅"
        rows.append(
            f"| `{name}` | {fmt_ns(base)} | {fmt_ns(cur)} | {sign}{delta * 100:.1f}%{badge} |"
        )

    print("### Benchmark comparison\n")
    if regressions:
        print(
            f"> **{len(regressions)} regression(s) detected** "
            f"(threshold: {REGRESSION_THRESHOLD * 100:.0f}%)\n"
        )
    print("| Benchmark | Baseline | Current | Delta |")
    print("|-----------|----------|---------|-------|")
    for row in rows:
        print(row)

    if regressions:
        print()
        for name, base, cur, delta in regressions:
            print(f"- `{name}`: {fmt_ns(base)} → {fmt_ns(cur)} (+{delta * 100:.1f}%)")
        return 1

    print(f"\nNo regressions above {REGRESSION_THRESHOLD * 100:.0f}% threshold.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
