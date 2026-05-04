#!/usr/bin/env bash
# bench_djvulibre.sh — build DjVuLibre benchmark, run it alongside the
# already-completed Criterion run, and write results to OUTPUT_DIR.
#
# Usage:
#   bash scripts/bench_djvulibre.sh [output_dir]
#
# output_dir defaults to the current directory.
#
# Output files:
#   djvulibre_bench.txt  — djvulibre_bench output (72 / 144 / 300 dpi)
#   ddjvu_timing.txt     — ddjvu CLI mean wall-clock times (5 runs / DPI)
#
# The script is intentionally non-fatal: every failure prints a message and
# exits 0 so CI steps using it can set continue-on-error: true or just rely
# on the graceful exit.

set -uo pipefail

BENCH_BIN="scripts/djvulibre_bench_ci"
OUT="${1:-.}"

# path|page|repeats|target_dpi
#
# These cases mirror existing Criterion benchmarks where possible:
# - boy.djvu@72dpi -> render_page/dpi/72
# - colorbook.djvu@150dpi -> render_colorbook
# - watchmaker.djvu@300dpi -> render_corpus_color
# - cable_1973_100133.djvu@300dpi -> render_corpus_bilevel
#
# Avoid upscale cases in the libdjvulibre C API harness: ddjvu_page_render can
# return a zero buffer for output rectangles larger than the native page.
CASES=(
  "references/djvujs/library/assets/boy.djvu|1|50|72"
  "references/djvujs/library/assets/colorbook.djvu|1|20|150"
  "tests/corpus/watchmaker.djvu|1|10|300"
  "tests/corpus/cable_1973_100133.djvu|1|10|300"
)

# ── helpers ──────────────────────────────────────────────────────────────────

die() { echo "bench_djvulibre: $*" >&2; exit 0; }

mkdir -p "$OUT" || die "cannot create output dir: $OUT"

# ── 1. Install DjVuLibre if not present ──────────────────────────────────────

if ! pkg-config --exists ddjvuapi 2>/dev/null; then
  echo "→ DjVuLibre not found — installing…"
  case "$(uname)" in
    Darwin)
      brew install djvulibre || die "brew install djvulibre failed"
      ;;
    Linux)
      sudo apt-get install -y --no-install-recommends \
        libdjvulibre-dev djvulibre-bin \
        || die "apt-get install djvulibre failed"
      ;;
    *)
      die "unsupported OS $(uname)"
      ;;
  esac
fi

# ── 2. Compile djvulibre_bench ────────────────────────────────────────────────

echo "→ Compiling djvulibre_bench…"
# shellcheck disable=SC2046
cc -O2 -o "$BENCH_BIN" scripts/djvulibre_bench.c \
    $(pkg-config --cflags --libs ddjvuapi) \
  || die "compilation failed"
echo "   OK — $BENCH_BIN"

# ── 3. Guard: test file must exist ────────────────────────────────────────────

for case in "${CASES[@]}"; do
  IFS='|' read -r file _page _repeats _dpi <<< "$case"
  [ -f "$file" ] || die "test file not found: $file"
done

# ── 4. Library-level benchmark (djvulibre_bench) ─────────────────────────────

echo "→ Library benchmark (render_only matrix)"
{
  for case in "${CASES[@]}"; do
    IFS='|' read -r file page repeats dpi <<< "$case"
    "$BENCH_BIN" "$file" "$page" "$repeats" "$dpi"
  done
} | tee "$OUT/djvulibre_bench.txt"

# ── 5. ddjvu CLI timing ───────────────────────────────────────────────────────

if command -v ddjvu &>/dev/null; then
  echo "→ ddjvu CLI timing (5 runs per case, warm-up included)"
  python3 - "${CASES[@]}" <<'PYEOF' | tee "$OUT/ddjvu_timing.txt"
import subprocess, statistics, sys, time
for case in sys.argv[1:]:
    file, page, _repeats, dpi = case.split("|")
    basename = file.split("/")[-1]
    # warm-up
    subprocess.run(["ddjvu", f"-page={page}", f"-scale={dpi}", "-format=ppm", file, "/dev/null"],
                   capture_output=True)
    times = []
    for _ in range(5):
        t0 = time.perf_counter()
        subprocess.run(["ddjvu", f"-page={page}", f"-scale={dpi}", "-format=ppm", file, "/dev/null"],
                       capture_output=True)
        times.append((time.perf_counter() - t0) * 1000)
    print(f"{basename}@{dpi}dpi: {statistics.mean(times):.1f} ms")
PYEOF
else
  echo "ddjvu CLI not found — skipping CLI timing"
  echo "skipped" > "$OUT/ddjvu_timing.txt"
fi

echo "→ Done. Results written to $OUT/"
