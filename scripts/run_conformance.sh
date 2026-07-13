#!/usr/bin/env bash
# Reproduce the CI DjVu conformance gate from a clean checkout (#682).
# Requires: ddjvu/djvused on PATH, Rust toolchain, Python 3.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

OUT_DIR="${CONFORMANCE_OUT_DIR:-conformance_site}"
HISTORY_INPUT="${CONFORMANCE_HISTORY_INPUT:-}"
DJVULIBRE_ID="${DJVULIBRE_ID:-}"
MANIFEST="${CONFORMANCE_MANIFEST:-conformance/corpus.json}"

if ! command -v ddjvu >/dev/null 2>&1; then
  echo "conformance: ddjvu not found on PATH (install DjVuLibre)" >&2
  exit 2
fi
if ! command -v djvused >/dev/null 2>&1; then
  echo "conformance: djvused not found on PATH (install DjVuLibre)" >&2
  exit 2
fi

run() { printf '\n==> %s\n' "$*"; "$@"; }

export PYTHONDONTWRITEBYTECODE=1
run python3 -m unittest scripts/test_conformance_report.py
run cargo build --release --features cli --example diff_djvulibre
run cargo build --release --features cli --example conformance_semantic
run cargo build --release --features cli --example interop_encode

python3 - <<PY > conformance_commands.sh
import json, shlex
manifest = json.load(open("${MANIFEST}"))
base = [
    "./target/release/examples/diff_djvulibre",
    "--width", str(manifest["render"]["width"]),
    "--tolerance", str(manifest["render"]["channel_tolerance"]),
]
for doc in manifest["documents"]:
    command = list(base)
    if "max_pages" in doc:
        command += ["--max-pages", str(doc["max_pages"])]
    command.append(doc["path"])
    print(" ".join(shlex.quote(part) for part in command))
PY
set -o pipefail
run bash -c 'set -o pipefail; bash conformance_commands.sh | tee diff_results.jsonl'

python3 - <<PY > conformance_semantic_commands.sh
import json, shlex
manifest = json.load(open("${MANIFEST}"))
binary = "./target/release/examples/conformance_semantic"
for doc in manifest["documents"]:
    command = [binary]
    if "max_pages" in doc:
        command += ["--max-pages", str(doc["max_pages"])]
    command.append(doc["path"])
    print(" ".join(shlex.quote(part) for part in command))
PY
run bash -c 'set -o pipefail; bash conformance_semantic_commands.sh | tee semantic_results.jsonl'

set -o pipefail
./target/release/examples/interop_encode \
  tests/fixtures/boy.djvu \
  tests/fixtures/chicken.djvu | tee writer_results.txt
run cargo test --lib djvu_mut
# Writer status is derived from interop_encode exit (pipefail) + mutation tests.
printf 'pass\n' > writer_status.txt

history_args=()
if test -n "${HISTORY_INPUT}" && test -s "${HISTORY_INPUT}"; then
  history_args=(--history-input "${HISTORY_INPUT}")
fi
djvu_args=()
if test -n "${DJVULIBRE_ID}"; then
  djvu_args=(--djvulibre-version "${DJVULIBRE_ID}")
fi

run python3 scripts/conformance_report.py \
  --manifest "${MANIFEST}" \
  --results diff_results.jsonl \
  --semantic-results semantic_results.jsonl \
  --writer-status writer_status.txt \
  --writer-results writer_results.txt \
  --accepted-differences conformance/accepted_differences.json \
  --diff-fuzz-registry fuzz/corpus-regressions/diff_fuzz \
  --output-dir "${OUT_DIR}" \
  ${djvu_args[@]+"${djvu_args[@]}"} \
  ${history_args[@]+"${history_args[@]}"}

printf '\nconformance dashboard: %s/index.html\n' "${OUT_DIR}"
