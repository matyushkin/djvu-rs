#!/usr/bin/env bash
# Downloads / verifies the public-domain DjVu test corpus.
#
# Usage: bash scripts/fetch_corpus.sh
#
# Tier-1 and tier-2 blobs live in tests/corpus/. Cargo publish excludes that
# directory. CI skips corpus-dependent benches when files are absent.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CORPUS_DIR="$SCRIPT_DIR/../tests/corpus"
mkdir -p "$CORPUS_DIR"

# filename|sha256|url
# Tier-2 extracts that are checked in are listed so a wipe+refetch can rebuild
# them; map/chinese samples are multi-page extracts (see tests/corpus/README.md)
# and are NOT auto-rebuilt here — only whole-document sources are.
MANIFEST=$(cat <<'EOF'
watchmaker.djvu|PLACEHOLDER_TIER1|https://archive.org/download/Watchmaker2001/Watchmaker.djvu
cable_1973_100133.djvu|PLACEHOLDER_TIER1|https://archive.org/download/State-Dept-cable-1973-100133/State%20Dept%20cable%201973-100133.djvu
war_1812.djvu|fdcf488ba1d4c5ab66e788e429279473ed3d000fc067ca35edafe5c34c9f3092|https://archive.org/download/warv1n2wood/warv1n2wood.djvu
goody_twoshoes.djvu|20b594d3207d07b98b8ef0f8b6cd8744e94942575e71916cdac8684f5b5eeb97|https://archive.org/download/goodytwoshoes00newyiala/goodytwoshoes00newyiala.djvu
cyrillic_simonovich_co2.djvu|596258ef4c283fc33be1e0f77a5778378a5a95220b09776d0fdd05d1744d14ad|https://archive.org/download/20200630_simonovich_uglekislota/%D0%A1%D0%B8%D0%BC%D0%BE%D0%BD%D0%BE%D0%B2%D0%B8%D1%87.%20%D0%91%D0%B5%D1%81%D0%B5%D0%B4%D1%8B%20%D0%BF%D0%BE%20%D1%85%D0%B8%D0%BC%D0%B8%D0%B8.%20%D0%A0%D1%83%D0%BA%D0%BE%D0%B2%D0%BE%D0%B4%D1%81%D1%82%D0%B2%D0%BE%20%D0%B4%D0%BB%D1%8F%20%D1%83%D1%87%D0%B8%D1%82%D0%B5%D0%BB%D0%B5%D0%B9%20%D0%BF%D1%80%D0%B8%20%D0%BF%D1%80%D0%BE%D0%B8%D0%B7%D0%B2%D0%BE%D0%B4%D1%81%D1%82%D0%B2%D0%B5%20%D0%BE%D0%BF%D1%8B%D1%82%D0%BE%D0%B2%20%D1%81%20%D1%83%D0%B3%D0%BB%D0%B5%D0%BA%D0%B8%D1%81%D0%BB%D0%BE%D1%82%D0%BE%D0%B9.djvu
EOF
)

echo "Corpus directory: $CORPUS_DIR"

verify_sha() {
  local file="$1" expect="$2"
  if [[ "$expect" == "PLACEHOLDER_TIER1" ]]; then
    return 0
  fi
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

while IFS='|' read -r filename sha url; do
  [[ -z "$filename" ]] && continue
  dest="$CORPUS_DIR/$filename"
  if [[ -f "$dest" ]]; then
    if verify_sha "$dest" "$sha"; then
      echo "  ok: $filename"
    else
      echo "  re-fetching $filename (checksum failed)"
      rm -f "$dest"
    fi
  fi
  if [[ ! -f "$dest" ]]; then
    echo "  downloading: $filename"
    if ! curl --silent --show-error --fail --location --output "$dest" "$url"; then
      echo "  WARNING: failed to download $filename — skipping" >&2
      rm -f "$dest"
      continue
    fi
    verify_sha "$dest" "$sha" || rm -f "$dest"
  fi
done <<< "$MANIFEST"

echo
echo "Checked-in extracts (verify-only; rebuild notes in README):"
EXTRACTS=$(cat <<'EOF'
map_atlas_sample.djvu|6ed70710bf7650bc7b311dc08b1265e81f1e0a9eefd85c3965d01ec79c63d45b
chinese_cookbook_sample.djvu|ea6c0e80fc8c4f238dc5e4af1a5e9572cc585ab1a7b502f6e0caeb301c6d1ad1
big_scanned_page.djvu|200f9bcaf891f12014fe39bd9e119f1f4560566abd8bea9b67a21eb198567594
EOF
)
while IFS='|' read -r filename sha; do
  [[ -z "$filename" ]] && continue
  dest="$CORPUS_DIR/$filename"
  if [[ -f "$dest" ]]; then
    if verify_sha "$dest" "$sha"; then
      echo "  ok: $filename"
    else
      echo "  MISMATCH: $filename" >&2
    fi
  else
    echo "  MISSING: $filename"
  fi
done <<< "$EXTRACTS"

echo
echo "Done. Corpus contents:"
ls -lh "$CORPUS_DIR"
