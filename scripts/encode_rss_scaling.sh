#!/usr/bin/env bash
# encode_rss_scaling.sh — measure `djvu encode`'s peak resident memory (max
# RSS) as a function of page count, and report the marginal MB/page slope.
#
# Formalizes the ad-hoc procedure used in the encoder peak-memory
# investigation (2026-09-02): render a multi-page fixture's pages to PNG
# once, assemble N-page input directories for a configurable list of N, run
# `djvu encode` on each under the platform's `/usr/bin/time` peak-RSS
# measurement, and report a table of N -> peak RSS (MB) plus the marginal
# slope. See PERF_EXPERIMENTS.md, "Encoder peak-memory measurement harness".
#
# Usage:
#   scripts/encode_rss_scaling.sh [options]
#
# Options (env var | flag):
#   FIXTURE=path       | --fixture PATH       DjVu file to render pages from.
#                                              Default: auto-discovered
#                                              tests/corpus/watchmaker.djvu.
#   PAGE_COUNTS="6 12 24" | --page-counts "N..."  Space-separated page counts
#                                              to test. Default: "6 12 24".
#   QUALITY=quality     | --quality NAME      Encode profile passed to
#                                              `djvu encode --quality`.
#                                              Default: quality.
#   OUT_DIR=path         | --out-dir PATH     Scratch dir for rendered PNGs
#                                              and assembled page dirs.
#                                              Default: a mktemp dir, removed
#                                              on exit unless --keep is given.
#   RENDER_DPI=300        | --render-dpi N    DPI passed to `djvu render`.
#                                              Default: 300 (watchmaker's
#                                              native page DPI, so encode
#                                              re-ingests native-resolution
#                                              pixmaps as PNG, matching the
#                                              investigation's numbers).
#                          --keep             Keep OUT_DIR instead of
#                                              deleting it on exit (still
#                                              re-renders are skipped if a
#                                              previous run's PNGs are found
#                                              there, so a repeat run with
#                                              the same --out-dir is cheap).
#                          --djvu-bin PATH    Path to the `djvu` CLI binary.
#                                              Default: build via `cargo
#                                              build --release --features cli`
#                                              and use target/release/djvu.
#                          -h, --help         Show this help and exit.
#
# Requires the platform peak-RSS measurement tool:
#   macOS:  /usr/bin/time -l   (reports "peak memory footprint" in bytes)
#   Linux:  /usr/bin/time -v   (reports "Maximum resident set size" in KB),
#           falling back to `/usr/bin/time -f %M` (KB) if -v is unsupported.
#
# Writes nothing into the repo working tree — all scratch output goes under
# OUT_DIR (a temp dir by default).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

usage() {
  sed -n '2,45p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

FIXTURE="${FIXTURE:-}"
PAGE_COUNTS="${PAGE_COUNTS:-6 12 24}"
QUALITY="${QUALITY:-quality}"
OUT_DIR="${OUT_DIR:-}"
RENDER_DPI="${RENDER_DPI:-300}"
KEEP=0
DJVU_BIN="${DJVU_BIN:-}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --fixture) FIXTURE="$2"; shift 2 ;;
    --page-counts) PAGE_COUNTS="$2"; shift 2 ;;
    --quality) QUALITY="$2"; shift 2 ;;
    --out-dir) OUT_DIR="$2"; shift 2 ;;
    --render-dpi) RENDER_DPI="$2"; shift 2 ;;
    --keep) KEEP=1; shift ;;
    --djvu-bin) DJVU_BIN="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "encode_rss_scaling: unknown argument: $1" >&2; usage >&2; exit 1 ;;
  esac
done

# ── locate the fixture ───────────────────────────────────────────────────────

if [[ -z "$FIXTURE" ]]; then
  candidates=(
    "$REPO_ROOT/tests/corpus/watchmaker.djvu"
  )
  # Fall back to a repo-wide search in case corpus layout moves.
  found=""
  for c in "${candidates[@]}"; do
    if [[ -f "$c" ]]; then found="$c"; break; fi
  done
  if [[ -z "$found" ]]; then
    found="$(find "$REPO_ROOT/tests" -type f -name 'watchmaker.djvu' -print -quit 2>/dev/null || true)"
  fi
  FIXTURE="$found"
fi

if [[ -z "$FIXTURE" || ! -f "$FIXTURE" ]]; then
  echo "encode_rss_scaling: could not find a fixture (looked for tests/corpus/watchmaker.djvu)." >&2
  echo "Pass --fixture PATH or set FIXTURE=path to an existing multi-page .djvu file." >&2
  exit 1
fi
FIXTURE="$(cd "$(dirname "$FIXTURE")" && pwd)/$(basename "$FIXTURE")"

# ── locate / build the djvu CLI (release, cli feature) ──────────────────────

if [[ -z "$DJVU_BIN" ]]; then
  DJVU_BIN="$REPO_ROOT/target/release/djvu"
  echo "==> building djvu CLI (release, cli feature)"
  ( cd "$REPO_ROOT" && cargo build --release --features cli --bin djvu )
fi
if [[ ! -x "$DJVU_BIN" ]]; then
  echo "encode_rss_scaling: djvu binary not found/executable at $DJVU_BIN" >&2
  exit 1
fi

# ── platform peak-RSS measurement ────────────────────────────────────────────

OS="$(uname -s)"
TIME_MODE=""
case "$OS" in
  Darwin)
    TIME_MODE="darwin"
    ;;
  Linux)
    if /usr/bin/time -v true >/dev/null 2>&1; then
      TIME_MODE="gnu-v"
    else
      TIME_MODE="gnu-f"
    fi
    ;;
  *)
    echo "encode_rss_scaling: unsupported OS '$OS' (need macOS or Linux /usr/bin/time)" >&2
    exit 1
    ;;
esac

# Runs "$@" under the platform's peak-RSS instrumentation. Prints one line
# "PEAK_KB=<n>" to stdout (the caller captures this) and passes the command's
# own stdout/stderr through unfiltered otherwise.
run_measured() {
  local time_log
  time_log="$(mktemp)"
  local status=0
  case "$TIME_MODE" in
    darwin)
      /usr/bin/time -l "$@" > /dev/null 2> "$time_log" || status=$?
      # "peak memory footprint" line, in bytes.
      local bytes
      bytes="$(grep 'peak memory footprint' "$time_log" | awk '{print $1}')"
      echo "PEAK_KB=$(( bytes / 1024 ))"
      ;;
    gnu-v)
      /usr/bin/time -v "$@" > /dev/null 2> "$time_log" || status=$?
      local kb
      kb="$(grep 'Maximum resident set size' "$time_log" | awk '{print $NF}')"
      echo "PEAK_KB=$kb"
      ;;
    gnu-f)
      /usr/bin/time -f '%M' -o "$time_log" "$@" > /dev/null 2>/dev/null || status=$?
      echo "PEAK_KB=$(cat "$time_log")"
      ;;
  esac
  cat "$time_log" >&2
  rm -f "$time_log"
  return "$status"
}

# ── scratch dir setup ────────────────────────────────────────────────────────

CLEANUP_OUT_DIR=0
if [[ -z "$OUT_DIR" ]]; then
  OUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/encode_rss_scaling.XXXXXX")"
  CLEANUP_OUT_DIR=1
else
  mkdir -p "$OUT_DIR"
fi
PAGES_DIR="$OUT_DIR/pages"
mkdir -p "$PAGES_DIR"

cleanup() {
  if [[ "$KEEP" -eq 0 && "$CLEANUP_OUT_DIR" -eq 1 ]]; then
    rm -rf "$OUT_DIR"
  fi
}
trap cleanup EXIT

echo "==> fixture: $FIXTURE"
echo "==> scratch dir: $OUT_DIR"

# ── render once, cached across runs ──────────────────────────────────────────

PAGE_COUNT_ALL="$("$DJVU_BIN" info "$FIXTURE" | head -1 | awk '{print $2}')"
if [[ -z "$PAGE_COUNT_ALL" || "$PAGE_COUNT_ALL" -lt 1 ]]; then
  echo "encode_rss_scaling: could not determine page count of $FIXTURE" >&2
  exit 1
fi

RENDER_SENTINEL="$PAGES_DIR/.rendered-dpi-$RENDER_DPI"
if [[ -f "$RENDER_SENTINEL" ]]; then
  echo "==> reusing cached render at $PAGES_DIR (dpi=$RENDER_DPI)"
else
  echo "==> rendering $PAGE_COUNT_ALL page(s) of $(basename "$FIXTURE") to PNG (dpi=$RENDER_DPI)"
  rm -f "$PAGES_DIR"/page-*.png
  "$DJVU_BIN" render "$FIXTURE" --all --dpi "$RENDER_DPI" --format png --output "$PAGES_DIR"
  touch "$RENDER_SENTINEL"
fi

# Portable across bash 3.2 (macOS system /bin/bash has no `mapfile`) and
# bash 4+.
RENDERED_PNGS=()
while IFS= read -r line; do
  RENDERED_PNGS+=("$line")
done < <(find "$PAGES_DIR" -maxdepth 1 -name '*.png' | sort)
NUM_RENDERED="${#RENDERED_PNGS[@]}"
if [[ "$NUM_RENDERED" -lt 1 ]]; then
  echo "encode_rss_scaling: rendering produced no PNGs in $PAGES_DIR" >&2
  exit 1
fi
echo "==> rendered $NUM_RENDERED page(s)"

# ── assemble N-page directories and run the measurement ─────────────────────

declare -a RESULT_N
declare -a RESULT_MB
declare -a RESULT_WALL

for N in $PAGE_COUNTS; do
  DIR="$OUT_DIR/pages_${N}"
  rm -rf "$DIR"
  mkdir -p "$DIR"
  # Cycle through the rendered pages (with a distinguishing name suffix) to
  # reach N inputs even when N exceeds the number of distinct source pages —
  # this duplicates per-page cost, which is exactly what we want for a pure
  # page-count scaling test (see the plan's 24-page case over a 12-page
  # fixture).
  for ((i = 0; i < N; i++)); do
    src="${RENDERED_PNGS[$((i % NUM_RENDERED))]}"
    printf -v idx '%04d' "$i"
    ln -f "$src" "$DIR/page-${idx}.png" 2>/dev/null || cp "$src" "$DIR/page-${idx}.png"
  done

  OUT_FILE="$OUT_DIR/out_${N}.djvu"
  rm -f "$OUT_FILE"

  echo "==> encoding $N page(s) (quality=$QUALITY)"
  t0=$(date +%s.%N)
  measured_out="$(run_measured "$DJVU_BIN" encode "$DIR" -o "$OUT_FILE" --quality "$QUALITY")"
  rc=$?
  t1=$(date +%s.%N)
  if [[ $rc -ne 0 ]]; then
    echo "encode_rss_scaling: djvu encode failed for N=$N (exit $rc)" >&2
    exit "$rc"
  fi
  peak_kb="$(echo "$measured_out" | grep '^PEAK_KB=' | tail -1 | cut -d= -f2)"
  peak_mb="$(awk -v kb="$peak_kb" 'BEGIN { printf "%.1f", kb / 1024 }')"
  wall_s="$(awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.2f", b - a }')"

  RESULT_N+=("$N")
  RESULT_MB+=("$peak_mb")
  RESULT_WALL+=("$wall_s")
done

# ── report ────────────────────────────────────────────────────────────────

echo
echo "== Peak RSS scaling: $(basename "$FIXTURE"), quality=$QUALITY, dpi=$RENDER_DPI =="
printf '%-8s %-12s %-16s %-10s\n' "Pages" "Peak RSS" "Marginal MB/pg" "Wall (s)"
prev_n=""
prev_mb=""
for i in "${!RESULT_N[@]}"; do
  n="${RESULT_N[$i]}"
  mb="${RESULT_MB[$i]}"
  wall="${RESULT_WALL[$i]}"
  if [[ -n "$prev_n" ]]; then
    marginal="$(awk -v mb="$mb" -v pmb="$prev_mb" -v n="$n" -v pn="$prev_n" 'BEGIN { printf "%.1f", (mb - pmb) / (n - pn) }')"
  else
    marginal="—"
  fi
  printf '%-8s %-12s %-16s %-10s\n' "$n" "${mb} MB" "$marginal" "$wall"
  prev_n="$n"
  prev_mb="$mb"
done

# Overall linear-fit slope (least squares) across all N as a cross-check
# against the successive-difference marginal above.
echo
awk -v ns="${RESULT_N[*]}" -v mbs="${RESULT_MB[*]}" '
BEGIN {
  split(ns, na, " "); split(mbs, ma, " ");
  n = length(na);
  if (n < 2) { exit }
  sx=0; sy=0; sxy=0; sxx=0;
  for (i = 1; i <= n; i++) {
    x = na[i] + 0; y = ma[i] + 0;
    sx += x; sy += y; sxy += x*y; sxx += x*x;
  }
  slope = (n*sxy - sx*sy) / (n*sxx - sx*sx);
  intercept = (sy - slope*sx) / n;
  printf "Linear fit: peak RSS (MB) ~= %.2f * pages + %.1f  (slope %.2f MB/page)\n", slope, intercept, slope;
}
'
