#!/usr/bin/env bash
# Bisect x86 codegen features for ZP decode via Callgrind (#566).
# Intended for ubuntu-latest (x86_64) + valgrind.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
mkdir -p target/x86-feature-bisect
VARIANTS=(
  "default|"
  "bmi1|-C target-feature=+bmi1"
  "bmi2|-C target-feature=+bmi2"
  "lzcnt|-C target-feature=+lzcnt"
  "popcnt|-C target-feature=+popcnt"
  "bmi1_bmi2|-C target-feature=+bmi1,+bmi2"
  "v3|-C target-cpu=x86-64-v3"
)
echo "variant,workload,Ir" > target/x86-feature-bisect/summary.csv
for entry in "${VARIANTS[@]}"; do
  name="${entry%%|*}"
  flags="${entry#*|}"
  echo "==> $name  RUSTFLAGS='$flags'"
  export RUSTFLAGS="${flags}"
  cargo build --release --example callgrind_workload
  for workload in bzz jb2 iw44; do
    out="target/x86-feature-bisect/callgrind.${name}.${workload}"
    valgrind --tool=callgrind --callgrind-out-file="$out" \
      target/release/examples/callgrind_workload "$workload" >/dev/null 2>"${out}.log" || true
    ir=$(awk '/^summary:/{print $2; exit}' "$out" 2>/dev/null || echo NA)
    echo "$name,$workload,$ir" | tee -a target/x86-feature-bisect/summary.csv
  done
done
echo
column -t -s, target/x86-feature-bisect/summary.csv || cat target/x86-feature-bisect/summary.csv
