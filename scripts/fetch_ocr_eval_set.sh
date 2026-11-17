#!/usr/bin/env bash
# Downloads / verifies the OCR real-scan evaluation set (#989).
#
# Usage: bash scripts/fetch_ocr_eval_set.sh
#
# Driven by docs/ocr-eval-manifest.toml — the single source of truth for
# URLs, sizes and SHA-256 hashes. Files land in models/ocr-eval/ (gitignored,
# excluded from the published crate): four DjVu scans from Wikimedia Commons
# (about 18 MB) and twenty proofread pages from Wikisource.
#
# Wikimedia asks automated clients to identify themselves and to go slowly:
# the script sends a User-Agent (override with DJVU_OCR_EVAL_USER_AGENT) and
# waits between downloads.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST="$SCRIPT_DIR/../docs/ocr-eval-manifest.toml"
EVAL_DIR="${DJVU_OCR_EVAL_DIR:-$SCRIPT_DIR/../models/ocr-eval}"
USER_AGENT="${DJVU_OCR_EVAL_USER_AGENT:-djvu-rs-ocr-eval/1 (+https://github.com/matyushkin/djvu-rs)}"
PAUSE_SECONDS=2
mkdir -p "$EVAL_DIR"

echo "Evaluation set directory: $EVAL_DIR"

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

# Flatten the manifest's [[scan]] and [[page]] tables into "file|sha256|url"
# lines. The value is everything after the first "=": page URLs contain "=".
parse_manifest() {
  awk '
    function value(line) { sub(/^[^=]*= */, "", line); gsub(/"/, "", line); return line }
    /^\[\[(scan|page)\]\]/ { if (file != "") print file "|" sha "|" url; file=""; sha=""; url="" }
    /^file *=/   { file = value($0) }
    /^sha256 *=/ { sha = value($0) }
    /^url *=/    { url = value($0) }
    END { if (file != "") print file "|" sha "|" url }
  ' "$MANIFEST"
}

fail=0
while IFS='|' read -r filename sha url; do
  [[ -z "$filename" ]] && continue
  dest="$EVAL_DIR/$filename"
  if [[ -f "$dest" ]]; then
    if verify_sha "$dest" "$sha"; then
      echo "  ok: $filename"
      continue
    fi
    echo "  re-fetching $filename (checksum failed)"
    rm -f "$dest"
  fi
  echo "  downloading: $filename"
  if ! curl --silent --show-error --fail --location --retry 3 --retry-delay 10 \
      --user-agent "$USER_AGENT" --output "$dest" "$url"; then
    echo "  ERROR: failed to download $filename" >&2
    rm -f "$dest"
    fail=1
    continue
  fi
  if ! verify_sha "$dest" "$sha"; then
    rm -f "$dest"
    fail=1
  fi
  sleep "$PAUSE_SECONDS"
done < <(parse_manifest)

if [[ "$fail" -ne 0 ]]; then
  echo "Some files failed to download or verify." >&2
  exit 1
fi

echo
echo "Done. $(find "$EVAL_DIR" -type f | wc -l | tr -d ' ') files in $EVAL_DIR"
