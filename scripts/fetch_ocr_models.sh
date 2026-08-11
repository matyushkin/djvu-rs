#!/usr/bin/env bash
# Downloads / verifies the pinned OCR model weights (#693).
#
# Usage: bash scripts/fetch_ocr_models.sh
#
# Driven by docs/ocr-model-manifest.toml — the single source of truth for
# URLs and SHA-256 hashes (the Rust loader verifies against the same file).
# Weights land in models/ocr/ (gitignored, excluded from the published crate).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST="$SCRIPT_DIR/../docs/ocr-model-manifest.toml"
MODELS_DIR="${DJVU_OCR_MODELS_DIR:-$SCRIPT_DIR/../models/ocr}"
mkdir -p "$MODELS_DIR"

echo "Models directory: $MODELS_DIR"

verify_sha() {
  local file="$1" expect="$2"
  local got
  if command -v shasum >/dev/null 2>&1; then
    got=$(shasum -a 256 "$file" | awk '{print $1}')
  else
    got=$(sha256sum "$file" | awk '{print $1}')
  fi
  if [[ "$got" != "$expect" ]]; then
    echo "  ERROR: checksum mismatch for $(basename "$file")" >&2
    echo "    expected $expect" >&2
    echo "    got      $got" >&2
    return 1
  fi
}

# Flatten the manifest's [[model]] tables into "file|sha256|url" lines.
parse_manifest() {
  awk -F' *= *' '
    /^\[\[model\]\]/ { if (file != "") print file "|" sha "|" url; file=""; sha=""; url="" }
    $1 == "file"   { gsub(/"/, "", $2); file=$2 }
    $1 == "sha256" { gsub(/"/, "", $2); sha=$2 }
    $1 == "url"    { gsub(/"/, "", $2); url=$2 }
    END { if (file != "") print file "|" sha "|" url }
  ' "$MANIFEST"
}

fail=0
while IFS='|' read -r filename sha url; do
  [[ -z "$filename" ]] && continue
  dest="$MODELS_DIR/$filename"
  if [[ -f "$dest" ]]; then
    if verify_sha "$dest" "$sha"; then
      echo "  ok: $filename"
      continue
    fi
    echo "  re-fetching $filename (checksum failed)"
    rm -f "$dest"
  fi
  echo "  downloading: $filename"
  if ! curl --silent --show-error --fail --location --output "$dest" "$url"; then
    echo "  ERROR: failed to download $filename" >&2
    rm -f "$dest"
    fail=1
    continue
  fi
  if ! verify_sha "$dest" "$sha"; then
    rm -f "$dest"
    fail=1
  fi
done < <(parse_manifest)

if [[ "$fail" -ne 0 ]]; then
  echo "Some models failed to download or verify." >&2
  exit 1
fi

echo
echo "Done. Model files:"
ls -lh "$MODELS_DIR"
