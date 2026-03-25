#!/usr/bin/env bash
# Downloads public-domain DjVu test corpus from Internet Archive.
# Files are CC0 / Public Domain.
#
# Usage: bash scripts/fetch_corpus.sh
#
# Downloaded files are placed in tests/corpus/. This directory is listed in
# .gitignore so the blobs are not committed to the repository. CI skips
# corpus-dependent tests unless the files are present (see the `corpus-tests`
# Cargo feature).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CORPUS_DIR="$SCRIPT_DIR/../tests/corpus"
mkdir -p "$CORPUS_DIR"

# ---------------------------------------------------------------------------
# Corpus manifest
# ---------------------------------------------------------------------------
# Each entry is:  <output-filename>  <URL>
#
# All files are small (< 500 KB) and sourced from publicly accessible
# repositories / Internet Archive. They cover the main codec variants:
#   - JB2-dominant  (text-heavy scanned pages)
#   - IW44-dominant (photo / colour pages)
#   - Mixed         (foreground mask + background wavelet)
#
# To add a new file, append a line here and re-run the script.
# ---------------------------------------------------------------------------

declare -A CORPUS=(
  # Text-heavy JB2 page from Internet Archive (public domain)
  ["jb2_text.djvu"]="https://archive.org/download/sampledjvu/sample.djvu"

  # Small colour page (IW44 background) from the DjVuLibre test suite
  # hosted on a public mirror
  ["iw44_colour.djvu"]="https://sourceforge.net/p/djvu/djvulibre-git/ci/master/tree/test/samples/boy.djvu?format=raw"
)

# ---------------------------------------------------------------------------
# Download
# ---------------------------------------------------------------------------
echo "Fetching corpus into: $CORPUS_DIR"

for filename in "${!CORPUS[@]}"; do
  url="${CORPUS[$filename]}"
  dest="$CORPUS_DIR/$filename"

  if [[ -f "$dest" ]]; then
    echo "  already present: $filename"
    continue
  fi

  echo "  downloading: $filename"
  if ! curl --silent --show-error --fail --location --output "$dest" "$url"; then
    echo "  WARNING: failed to download $filename from $url — skipping" >&2
    rm -f "$dest"
  fi
done

echo "Done. Corpus contents:"
ls -lh "$CORPUS_DIR"
