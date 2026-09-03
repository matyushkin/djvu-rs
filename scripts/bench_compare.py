#!/usr/bin/env python3
"""Compare two sets of Criterion benchmark results and report regressions.

Usage:
    python3 scripts/bench_compare.py <baseline_criterion_dir> <current_criterion_dir> \
        [--suspects-out FILE] [--restrict FILE] [--title TEXT]

Exit codes:
    0   no regressions above threshold (or every flagged regression was
        subsumed by a drift-suspect run — see below)
    1   real regression(s) detected — the run does NOT look like uniform
        cross-runner drift, so this should fail CI
    2   bad or missing input (no current results, or nothing left to
        compare after --restrict)
    3   regression(s) detected, but the run is a drift suspect: fail-soft —
        report only, do not fail the job

Outputs a Markdown table suitable for posting as a PR comment.

`--suspects-out FILE` writes the regressed bench names (one per line) so CI can
re-measure just those. `--restrict FILE` limits the comparison to the bench
names listed in FILE — used for the same-runner re-check pass, where the
baseline was rebuilt from the merge base on the *same* machine (the artifact
baseline comes from a different runner, whose CPU generation can differ enough
to flag phantom regressions on micro benches — observed +11%/+34% on
`bzz_decode`/`bzz_encode` across PRs that do not touch BZZ at all).

── Cross-runner drift detection ─────────────────────────────────────────────
Two Benchmark CI runs (PR #779, PR #787 — see PERF_EXPERIMENTS.md) reported
"regressions" that were pure runner noise: local A/B testing reproduced none
of them. Both shared a tell visible right in the comparison table: almost
EVERY benchmark, including ones the diff could not possibly touch, moved in
the same direction by a similar amount (#787: roughly -8% across
render/IW44/PDF export while the diff touched only djvu-bzz). A few benches
moving further the *other* way, on top of that floor, is exactly what you'd
expect from ordinary measurement variance riding a faster-or-slower runner
that run — not a code-caused regression.

The heuristic: compute the median delta across every benchmark that exists in
both baseline and current (`machine_drift`). Call that the *machine drift* —
a runner-wide speed change has nothing to do with the diff. If a large
majority of benchmarks move together, the median tracks that shared move even
though a handful of outliers (the flagged "regressions") would drag a mean
around. If |drift| exceeds `DRIFT_THRESHOLD` (3%) across at least
`MIN_BENCHES_FOR_DRIFT` (5) overlapping benchmarks, the run is a *drift
suspect*: any regressions found are still listed in full (raw numbers,
unmodified threshold — transparency over cleverness), but the comparison
exits 3 instead of 1 so CI can report the result without failing the job. A
single real regression against an otherwise-flat table (|drift| ~ 0) still
exits 1 and still fails, exactly as before. Deltas printed in the table are
always raw (baseline vs current, unmodified); when a run is a drift suspect,
a `Corrected` column is added showing `delta - drift`, so a reader can see
the same numbers re-centered on the runner's own floor — that column is
informational only and never changes the exit code.

This deliberately does *not* try to map the diff's changed files to the
benches it could plausibly touch and auto-clear only those — that mapping is
brittle (crate boundaries, shared decoder code, `bzz_encode` calling
`bzz_decode` as setup outside its own timed loop, etc. — see the #787 entry
in PERF_EXPERIMENTS.md for exactly this trap) and easy to get quietly wrong.
Instead the comment lists every benchmark that moved, raw numbers included,
and lets a human weigh which ones plausibly relate to the diff.
"""

import json
import statistics
import sys
from pathlib import Path

REGRESSION_THRESHOLD = 0.05  # 5 %
DRIFT_THRESHOLD = 0.03  # 3 % median delta across all benches → suspect run
MIN_BENCHES_FOR_DRIFT = 5  # below this, a median is too noisy to trust


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


def machine_drift(deltas: list[float]) -> float | None:
    """Median delta across all overlapping benchmarks, or None if too few.

    A runner-wide speed change (faster or slower CPU that run, thermal
    state, noisy-neighbour VM, ...) shifts almost every benchmark's delta by
    roughly the same amount, regardless of what the diff touched. The median
    tracks that shared shift without being dragged around by a handful of
    real (or noisy) outliers the way a plain mean would be.
    """
    if len(deltas) < MIN_BENCHES_FOR_DRIFT:
        return None
    return statistics.median(deltas)


class Comparison:
    """Pure comparison logic, factored out of main() so it is unit-testable
    without touching the filesystem or argv."""

    def __init__(
        self,
        baseline: dict[str, float],
        current: dict[str, float],
        restrict: set[str] | None = None,
        title: str = "Benchmark comparison",
    ):
        self.baseline = baseline
        self.current = current
        self.restrict = restrict
        self.title = title

        self.rows: list[str] = []
        self.regressions: list[tuple[str, float, float, float]] = []
        self.drift: float | None = None
        self.suspect = False

        self._compare()

    def _compare(self) -> None:
        all_names = sorted(set(self.baseline) | set(self.current))
        if self.restrict is not None:
            all_names = [n for n in all_names if n in self.restrict]

        # First pass: collect every overlapping (present in both baseline
        # and current) delta so the drift estimate isn't skewed by new or
        # removed benches. Drift detection only makes sense against the
        # full unrestricted table — a --restrict pass (the same-runner
        # suspect re-check) is already a biased handful of benches, so a
        # "median" over 2-6 cherry-picked entries would be meaningless.
        if self.restrict is None:
            deltas: list[float] = []
            for name in all_names:
                cur = self.current.get(name)
                base = self.baseline.get(name)
                if cur is None or base is None or base == 0:
                    continue
                deltas.append((cur - base) / base)
            self.drift = machine_drift(deltas)
            self.suspect = self.drift is not None and abs(self.drift) > DRIFT_THRESHOLD

        for name in all_names:
            cur = self.current.get(name)
            base = self.baseline.get(name)

            if cur is None:
                self.rows.append(f"| `{name}` | {fmt_ns(base)} | — | removed |")
                continue
            if base is None:
                self.rows.append(f"| `{name}` | — | {fmt_ns(cur)} | new |")
                continue

            delta = (cur - base) / base if base != 0 else 0.0
            sign = "+" if delta >= 0 else ""
            badge = ""
            if delta > REGRESSION_THRESHOLD:
                self.regressions.append((name, base, cur, delta))
                badge = " ⚠️"
            elif delta < -REGRESSION_THRESHOLD:
                badge = " ✅"

            row = f"| `{name}` | {fmt_ns(base)} | {fmt_ns(cur)} | {sign}{delta * 100:.1f}%{badge} |"
            if self.suspect:
                corrected = delta - self.drift
                csign = "+" if corrected >= 0 else ""
                row += f" {csign}{corrected * 100:.1f}% |"
            self.rows.append(row)

    @property
    def exit_code(self) -> int:
        if not self.regressions:
            return 0
        return 3 if self.suspect else 1

    def render(self) -> str:
        out: list[str] = []
        out.append(f"### {self.title}\n")

        if self.suspect:
            n_overlap = len(set(self.baseline) & set(self.current))
            out.append(
                f"> ⚠️ **Probable cross-runner drift**: the median delta across "
                f"all {n_overlap} overlapping benchmarks is "
                f"{'+' if self.drift >= 0 else ''}{self.drift * 100:.1f}%, "
                f"above the {DRIFT_THRESHOLD * 100:.0f}% suspect threshold — "
                f"this run's CPU/runner was probably just faster or slower "
                f"than the baseline's, not the code. Flagged regressions "
                f"below are reported but did **not** fail the job. Deltas in "
                f"the table are raw; the `Corrected` column subtracts the "
                f"drift (delta − {self.drift * 100:.1f}%) for reference — it "
                f"does not change the verdict.\n"
            )

        if self.regressions:
            qualifier = " (drift-suspect — not failing)" if self.suspect else ""
            out.append(
                f"> **{len(self.regressions)} regression(s) detected** "
                f"(threshold: {REGRESSION_THRESHOLD * 100:.0f}%){qualifier}\n"
            )

        header = "| Benchmark | Baseline | Current | Delta |"
        sep = "|-----------|----------|---------|-------|"
        if self.suspect:
            header += " Corrected |"
            sep += "-----------|"
        out.append(header)
        out.append(sep)
        out.extend(self.rows)

        if self.regressions:
            out.append("")
            for name, base, cur, delta in self.regressions:
                out.append(f"- `{name}`: {fmt_ns(base)} → {fmt_ns(cur)} (+{delta * 100:.1f}%)")
        else:
            out.append(f"\nNo regressions above {REGRESSION_THRESHOLD * 100:.0f}% threshold.")

        return "\n".join(out)


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

    if restrict is not None:
        overlap = (set(baseline) | set(current)) & restrict
        if not overlap:
            print("No overlapping benchmarks for the re-check.", file=sys.stderr)
            return 2

    comparison = Comparison(baseline, current, restrict=restrict, title=title)
    print(comparison.render())

    if comparison.regressions and suspects_out is not None:
        suspects_out.write_text(
            "\n".join(name for name, _, _, _ in comparison.regressions) + "\n"
        )

    return comparison.exit_code


if __name__ == "__main__":
    sys.exit(main())
