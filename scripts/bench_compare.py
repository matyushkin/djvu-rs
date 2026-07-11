#!/usr/bin/env python3
"""Compare two sets of Criterion benchmark results and report regressions.

Usage:
    python3 scripts/bench_compare.py <baseline_criterion_dir> <current_criterion_dir> \
        [--suspects-out FILE] [--restrict FILE] [--title TEXT]

Exits with code 0 when no regressions exceed the threshold, 1 otherwise.
Outputs a Markdown table suitable for posting as a PR comment.

`--suspects-out FILE` writes the regressed bench names (one per line) so CI can
re-measure just those. `--restrict FILE` limits the comparison to the bench
names listed in FILE — used for the same-runner re-check pass, where the
baseline was rebuilt from the merge base on the *same* machine (the artifact
baseline comes from a different runner, whose CPU generation can differ enough
to flag phantom regressions on micro benches — observed +11%/+34% on
`bzz_decode`/`bzz_encode` across PRs that do not touch BZZ at all).
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
    args = sys.argv[1:]
    suspects_out = None
    restrict_file = None
    title = "Benchmark comparison"
    positional: list[str] = []
    i = 0
    while i < len(args):
        if args[i] == "--suspects-out" and i + 1 < len(args):
            suspects_out = Path(args[i + 1])
            i += 2
        elif args[i] == "--restrict" and i + 1 < len(args):
            restrict_file = Path(args[i + 1])
            i += 2
        elif args[i] == "--title" and i + 1 < len(args):
            title = args[i + 1]
            i += 2
        else:
            positional.append(args[i])
            i += 1
    if len(positional) != 2:
        print(
            f"usage: {sys.argv[0]} <baseline_dir> <current_dir> "
            "[--suspects-out FILE] [--restrict FILE] [--title TEXT]",
            file=sys.stderr,
        )
        return 2

    baseline_dir = Path(positional[0])
    current_dir = Path(positional[1])

    restrict: set[str] | None = None
    if restrict_file is not None:
        restrict = {
            line.strip()
            for line in restrict_file.read_text().splitlines()
            if line.strip()
        }

    baseline = load_results(baseline_dir) if baseline_dir.exists() else {}
    current = load_results(current_dir) if current_dir.exists() else {}

    if not current:
        print("No benchmark results found in current run.", file=sys.stderr)
        return 2

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
    if restrict is not None:
        all_names = [n for n in all_names if n in restrict]
        if not all_names:
            print("No overlapping benchmarks for the re-check.", file=sys.stderr)
            return 2
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

    print(f"### {title}\n")
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
        if suspects_out is not None:
            suspects_out.write_text(
                "\n".join(name for name, _, _, _ in regressions) + "\n"
            )
        return 1

    print(f"\nNo regressions above {REGRESSION_THRESHOLD * 100:.0f}% threshold.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
