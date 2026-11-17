# OCR model metrics — recorded baseline (#693)

Baseline quality of the pinned neural OCR models on the synthetic corpus
(`src/ocr_onnx/corpus.rs`). The corpus is project-owned and deterministic:
pages are rendered with the pinned PT Sans font (manifest key
`metrics-font-pt-sans`) from versioned layout parameters, so no images are
committed and no third-party benchmark license applies.

**Policy.** A model bump (any change to `docs/ocr-model-manifest.toml`
affecting the pipeline), a font bump, or a preprocessing change must
re-measure and update this table in the same PR. The corpus test
(`ocr_onnx::corpus::tests::corpus_baseline_holds_for_pinned_models`) asserts
thresholds = baseline plus a safety margin, so normal cross-platform float
jitter passes while a silent regression fails. CI runs it in the main-only
`OCR (onnx models)` job; locally it skips unless models and font are fetched
(`scripts/fetch_ocr_models.sh`).

## Baseline

Pinned pipeline: `ppocr-v4-mobile-det` (detection) +
`ppocr-v5-cyrillic-rec` with its pinned dictionary (recognition),
tract-onnx 0.22.3. Re-measured 2026-10-02 under tract-onnx 0.23.8: the
same numbers to three decimals.

Measured 2026-08-12 on macOS arm64 (debug profile; metrics are
platform-stable by design — fixed-point preprocessing, deterministic
rasterization):

| Sample | CER | WER | Mean line IoU | Threshold (CER / WER / IoU) |
|--------|-----|-----|---------------|------------------------------|
| Cyrillic (4 lines, 2 pangram excerpts) | 0.000 | 0.000 | 0.937 | ≤ 0.05 / ≤ 0.10 / ≥ 0.80 |
| Latin + digits (4 lines) | 0.000 | 0.000 | 0.827 | ≤ 0.05 / ≤ 0.10 / ≥ 0.70 |

Notes:

- CER/WER of exactly 0.0 means the clean synthetic pages are recognized
  verbatim, including «ё», punctuation, and digits. Real scans will be
  worse; the corpus gates *regressions of the pinned artifacts*, not
  real-world accuracy.
- IoU compares ink-tight ground-truth line boxes against DBNet's unclipped
  boxes; values well below 1.0 are expected (the unclip ratio pads the
  detector's shrunken text cores). The Latin sample scores lower because its
  ascender/descender profile pads differently, not because detection is
  worse.

## Real scans (#989)

The synthetic corpus above cannot say how the backends read real book
pages. This section measures `ocr-onnx` and `ocr-tesseract` on the same
scanned pages and decides the status of `ocr-onnx` by a rule fixed before
the run.

### The set

Twenty pages from four scanned books, pinned in
[`ocr-eval-manifest.toml`](ocr-eval-manifest.toml). Nothing is committed:
`scripts/fetch_ocr_eval_set.sh` downloads the files into `models/ocr-eval/`
and checks each size and SHA-256.

| Scan | Script | Print | Page size, resolution |
|------|--------|-------|-----------------------|
| `bulgakov-rasskazy-1926` | Cyrillic | M. Bulgakov, *Rasskazy*, 1926 | 1283 × 1621 px, 300 dpi |
| `bulgakov-diavoliada-1925` | Cyrillic | M. Bulgakov, *Diavoliada*, 1925 | 1554 × 2480 px, 300 dpi |
| `twain-stormfield-1909` | Latin | M. Twain, *Extract from Captain Stormfield's Visit to Heaven*, 1909 | 1736 × 2924 px, 400 dpi |
| `herzl-jewish-state-1917` | Latin | T. Herzl, *A Jewish State*, 1917 | 1438 × 2466 px, 300 dpi |

- **Scans.** DjVu files from Wikimedia Commons, public domain (printed
  before 1928). The measurement decodes them with this crate, as `djvu ocr`
  does.
- **Reference text.** The Wikisource transcription of each page, at the
  "validated" level: two people proofread it against the scan. Each page is
  pinned to one revision. The transcriptions are CC BY-SA, so they are
  fetched and not committed.
- **Page choice.** Per scan: take the validated pages whose wikitext is at
  least 0.6 of the scan's median size, in page order, and pick the pages at
  positions 0.1, 0.3, 0.5, 0.7 and 0.9 of that list. The rule ran before
  any OCR; no page was swapped afterwards.

The four scans differ on purpose: clean bitonal print (Bulgakov 1926), a
grey low-contrast scan (Bulgakov 1925), large type with little text per
page (Twain) and small dense type with footnotes (Herzl).

### Protocol

The harness is `tests/ocr_real_scans.rs`:

```sh
bash scripts/fetch_ocr_models.sh
bash scripts/fetch_ocr_eval_set.sh
cargo test --release --features ocr-onnx,ocr-tesseract \
    --test ocr_real_scans -- --ignored --nocapture
```

- Each page is rendered once at its native resolution, as `djvu ocr`
  renders it. Both backends read the same pixels.
- `ocr-tesseract` gets the language `rus` for the Cyrillic scans and `eng`
  for the Latin scans, and the page resolution. Everything else is the
  Tesseract default.
- `ocr-onnx` runs the pinned models. It ignores the options.
- The reference is everything the proofreaders transcribed for the page:
  the body, the footnotes, and the running header and page number where
  the transcription has them (the two Latin scans). The Cyrillic
  transcriptions leave out the page number, so both backends pay for it
  equally. The wikitext converter rejects markup that it does not know.
- **Normalisation**, applied to the reference and to both recognized texts
  by one function:
  1. A word hyphenated at a line end is joined (a line that ends with a
     letter and a hyphen, followed by a line that starts with a lowercase
     letter). The transcription has such words joined.
  2. Typographic variants fold to one character: all dashes to `-`, all
     quotation marks to `'` or `"`, `…` to `...`, Latin ligatures to their
     letters.
  3. Spaces next to a dash are dropped. Old print sets dashes close; the
     transcription spaces them.
  4. Every run of whitespace, line breaks included, becomes one space.

  Case, punctuation and digits are compared as they are.
- **CER** (character error rate) is the edit distance in characters
  divided by the reference length. **WER** (word error rate) is the same
  over whitespace-separated words. Rates are pooled: total distance over
  total reference length, so a long page weighs more than a short one.
- **Time** is the wall time of `recognize` for one page, render excluded,
  release profile, one backend instance for all pages, one pass.

### Decision rule

Written and committed before the first run:

> `ocr-onnx` leaves the experimental status only if its pooled CER is no
> worse than the pooled CER of `ocr-tesseract` on the Cyrillic pages **and**
> on the Latin pages of this set. If it is worse on either script, it stays
> experimental and the README states the measured gap.

WER and time are reported. They do not enter the rule.
