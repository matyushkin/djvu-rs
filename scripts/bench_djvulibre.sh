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

OUT="${1:-.}"
# colorbook.djvu: 2260×3669 px, 400 dpi, color IW44 — representative document scan.
# Rendered at 150 dpi → 848×1377 px output, comparable to watchmaker.djvu native.
# boy.djvu (192×256) is too small to show djvu-rs's advantage over libdjvulibre.
FILE="references/djvujs/library/assets/colorbook.djvu"
BENCH_BIN="scripts/djvulibre_bench_ci"
DPIS=(150)

# ── helpers ──────────────────────────────────────────────────────────────────

die() { echo "bench_djvulibre: $*" >&2; exit 0; }

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

[ -f "$FILE" ] || die "test file not found: $FILE"

# ── 4. Library-level benchmark (djvulibre_bench) ─────────────────────────────

echo "→ Library benchmark (render_only, 20 runs per DPI): ${DPIS[*]} dpi"
{
  for dpi in "${DPIS[@]}"; do
    "$BENCH_BIN" "$FILE" 1 20 "$dpi"
  done
} | tee "$OUT/djvulibre_bench.txt"

# ── 5. ddjvu CLI timing ───────────────────────────────────────────────────────

if command -v ddjvu &>/dev/null; then
  echo "→ ddjvu CLI timing (5 runs per DPI, warm-up included): ${DPIS[*]} dpi"
  python3 - "$FILE" "${DPIS[@]}" | tee "$OUT/ddjvu_timing.txt" <<'PYEOF'
import subprocess, statistics, sys, time
file, *dpis = sys.argv[1], *[int(x) for x in sys.argv[2:]]
for dpi in dpis:
    # warm-up
    subprocess.run(["ddjvu", f"-scale={dpi}", "-format=ppm", file, "/dev/null"],
                   capture_output=True)
    times = []
    for _ in range(5):
        t0 = time.perf_counter()
        subprocess.run(["ddjvu", f"-scale={dpi}", "-format=ppm", file, "/dev/null"],
                       capture_output=True)
        times.append((time.perf_counter() - t0) * 1000)
    print(f"boy.djvu@{dpi}dpi: {statistics.mean(times):.1f} ms")
PYEOF
else
  echo "ddjvu CLI not found — skipping CLI timing"
  echo "skipped" > "$OUT/ddjvu_timing.txt"
fi

echo "→ Done. Results written to $OUT/"
