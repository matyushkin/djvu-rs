#!/usr/bin/env bash
# Write SHA-256 checksums for packaging artifacts under a directory.
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <artifacts-dir> [output-file]" >&2
  exit 2
fi

ART_DIR="$(cd "$1" && pwd)"
OUT="${2:-$ART_DIR/SHA256SUMS}"

if [[ ! -d "$ART_DIR" ]]; then
  echo "not a directory: $ART_DIR" >&2
  exit 1
fi

tmp="$(mktemp)"
(
  cd "$ART_DIR"
  # shellcheck disable=SC2035
  find . -type f \
    ! -name SHA256SUMS \
    ! -name '*.attestation.json' \
    ! -name '.DS_Store' \
    -print0 \
    | sort -z \
    | while IFS= read -r -d '' f; do
        # Portable sha256
        if command -v sha256sum >/dev/null 2>&1; then
          sha256sum "$f"
        else
          shasum -a 256 "$f"
        fi
      done
) >"$tmp"

mv "$tmp" "$OUT"
echo "wrote $OUT"
wc -l <"$OUT" | tr -d ' ' | xargs -I{} echo "checksum entries: {}"
