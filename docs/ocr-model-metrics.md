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

### Results

One run on 2026-11-17, with the set, the harness and the rule above
unchanged since they were committed.

| Item | Value |
|------|-------|
| Machine | Apple M1 Max, 10 cores, 64 GiB, macOS 27.2.1 |
| Rust | 1.100.0, release profile |
| `ocr-onnx` | pinned models of `docs/ocr-model-manifest.toml`, `tract-onnx` 0.23.8 |
| `ocr-tesseract` | Tesseract 5.5.4 (Leptonica 1.87.0), crate `tesseract` 0.15.2 |
| `rus.traineddata` | 15,301,764 bytes, SHA-256 `b617eb6830ffabaaa795dd87ea7fd251adfe9cf0efe05eb9a2e8128b7728d6b6` |
| `eng.traineddata` | 15,400,601 bytes, SHA-256 `8280aed0782fe27257a68ea10fe7ef324ca0f8d85bd2fd145d1c2b560bcb66ba` |

The Tesseract language files are the ones that Homebrew installed. They are
not pinned by this repository, so the two hashes identify them.

#### Per script (the decision is made on these rows)

| Script | Pages | Reference chars | Backend | CER | WER | Median s/page | Mean s/page |
|--------|-------|-----------------|---------|-----|-----|---------------|-------------|
| Cyrillic | 10 | 12462 | `ocr-onnx` | 0.0300 | 0.1354 | 6.97 | 6.57 |
| Cyrillic | 10 | 12462 | `ocr-tesseract` | 0.0561 | 0.2388 | 2.45 | 2.37 |
| Latin | 10 | 19712 | `ocr-onnx` | 0.0633 | 0.1155 | 6.60 | 7.41 |
| Latin | 10 | 19712 | `ocr-tesseract` | 0.0128 | 0.0289 | 2.62 | 2.97 |

#### Per scan

| Scan | Pages | Reference chars | Backend | CER | WER | Median s/page | Mean s/page |
|------|-------|-----------------|---------|-----|-----|---------------|-------------|
| bulgakov-rasskazy-1926 | 5 | 3667 | `ocr-onnx` | 0.0409 | 0.1328 | 5.21 | 5.55 |
| bulgakov-rasskazy-1926 | 5 | 3667 | `ocr-tesseract` | 0.0349 | 0.1309 | 1.32 | 1.36 |
| bulgakov-diavoliada-1925 | 5 | 8795 | `ocr-onnx` | 0.0255 | 0.1365 | 7.25 | 7.59 |
| bulgakov-diavoliada-1925 | 5 | 8795 | `ocr-tesseract` | 0.0649 | 0.2841 | 3.43 | 3.38 |
| twain-stormfield-1909 | 5 | 3607 | `ocr-onnx` | 0.0055 | 0.0146 | 2.94 | 2.84 |
| twain-stormfield-1909 | 5 | 3607 | `ocr-tesseract` | 0.0036 | 0.0219 | 1.33 | 1.30 |
| herzl-jewish-state-1917 | 5 | 16105 | `ocr-onnx` | 0.0762 | 0.1410 | 11.26 | 11.98 |
| herzl-jewish-state-1917 | 5 | 16105 | `ocr-tesseract` | 0.0148 | 0.0306 | 4.80 | 4.64 |

#### Per page

| Scan | Page | Reference chars | onnx CER | onnx WER | onnx s | tesseract CER | tesseract WER | tesseract s |
|------|------|-----------------|----------|----------|--------|---------------|---------------|-------------|
| bulgakov-rasskazy-1926 | 12 | 518 | 0.1042 | 0.1944 | 5.21 | 0.0985 | 0.2917 | 1.14 |
| bulgakov-rasskazy-1926 | 22 | 1126 | 0.0107 | 0.0667 | 7.33 | 0.0195 | 0.0909 | 1.95 |
| bulgakov-rasskazy-1926 | 34 | 537 | 0.0093 | 0.0435 | 3.37 | 0.0391 | 0.1087 | 0.95 |
| bulgakov-rasskazy-1926 | 45 | 766 | 0.0940 | 0.4176 | 6.91 | 0.0313 | 0.1868 | 1.44 |
| bulgakov-rasskazy-1926 | 55 | 720 | 0.0097 | 0.0280 | 4.95 | 0.0139 | 0.0561 | 1.32 |
| bulgakov-diavoliada-1925 | 20 | 1544 | 0.0272 | 0.1528 | 8.47 | 0.0376 | 0.1667 | 2.96 |
| bulgakov-diavoliada-1925 | 52 | 1732 | 0.0191 | 0.0977 | 7.03 | 0.0803 | 0.3320 | 3.43 |
| bulgakov-diavoliada-1925 | 82 | 2019 | 0.0317 | 0.1756 | 6.33 | 0.0857 | 0.3835 | 3.82 |
| bulgakov-diavoliada-1925 | 112 | 1661 | 0.0355 | 0.1717 | 7.25 | 0.0554 | 0.2403 | 3.10 |
| bulgakov-diavoliada-1925 | 145 | 1839 | 0.0141 | 0.0892 | 8.89 | 0.0593 | 0.2677 | 3.60 |
| twain-stormfield-1909 | 24 | 686 | 0.0044 | 0.0370 | 2.94 | 0.0087 | 0.0444 | 1.22 |
| twain-stormfield-1909 | 48 | 741 | 0.0189 | 0.0138 | 2.67 | 0.0027 | 0.0138 | 1.37 |
| twain-stormfield-1909 | 73 | 750 | 0.0013 | 0.0065 | 2.96 | 0.0027 | 0.0130 | 1.35 |
| twain-stormfield-1909 | 97 | 687 | 0.0000 | 0.0000 | 2.98 | 0.0015 | 0.0165 | 1.24 |
| twain-stormfield-1909 | 121 | 743 | 0.0027 | 0.0153 | 2.65 | 0.0027 | 0.0229 | 1.33 |
| herzl-jewish-state-1917 | 13 | 2612 | 0.0490 | 0.0820 | 10.23 | 0.0069 | 0.0310 | 3.88 |
| herzl-jewish-state-1917 | 25 | 3695 | 0.0947 | 0.1364 | 14.42 | 0.0097 | 0.0292 | 5.39 |
| herzl-jewish-state-1917 | 35 | 3412 | 0.0507 | 0.0922 | 12.78 | 0.0196 | 0.0271 | 4.83 |
| herzl-jewish-state-1917 | 45 | 3363 | 0.0782 | 0.1894 | 11.26 | 0.0113 | 0.0230 | 4.80 |
| herzl-jewish-state-1917 | 55 | 3023 | 0.1039 | 0.1966 | 11.23 | 0.0265 | 0.0439 | 4.30 |

### Outcome

`ocr-onnx` stays experimental. The rule needs both scripts and the Latin
script fails it:

- Cyrillic: CER 0.0300 against 0.0561. `ocr-onnx` makes about half the
  character errors of Tesseract.
- Latin: CER 0.0633 against 0.0128. `ocr-onnx` makes about five times the
  character errors of Tesseract.
- Time: `ocr-onnx` needs 2.5 to 2.8 times longer per page (median 6.97 s
  against 2.45 s on Cyrillic, 6.60 s against 2.62 s on Latin).

Limits of this result:

- The set is small: four books and twenty pages. The Cyrillic figure rests
  mostly on one book. `bulgakov-diavoliada-1925` holds 8795 of the 12462
  Cyrillic characters, and `ocr-onnx` is better on all five of its pages.
  On `bulgakov-rasskazy-1926` Tesseract is slightly better (0.0349 against
  0.0409).
- The Latin figure rests mostly on `herzl-jewish-state-1917` (16105 of
  19712 characters). Its pages are dense: about 3200 characters each,
  against about 720 for `twain-stormfield-1909`. On the Twain pages the two
  backends are close (0.0055 against 0.0036).
- CER counts a line in the wrong place as errors, because the reference is
  one text in reading order. So the figures measure reading order together
  with character recognition.
- Time is one pass on one machine.

What the per-page texts show. These are observations from the dumped texts
and from an approximate alignment (Python `difflib`), not a second
measurement:

- The Latin gap is mostly missing and misplaced text, not wrong letters. On
  `herzl-jewish-state-1917` pages 25, 45 and 55 the alignment gives
  `ocr-onnx` 40 to 58 substituted characters per page and 250 to 360
  characters that are missing or in the wrong place. Tesseract has 23 to 38
  substituted and 13 to 44 missing or added characters on the same pages.
- `ocr-onnx` sometimes splits one printed line into pieces and emits the
  pieces out of order. On `twain-stormfield-1909` page 48 its only error is
  one word in the wrong place, and that gives CER 0.0189.
- On `herzl-jewish-state-1917` page 45 `ocr-onnx` returns several lines as
  unreadable text. Tesseract reads the same render with CER 0.0113.
- `ocr-onnx` loses the letters of the `fi` and `fl` ligatures (`trifing`,
  `superfuous`) and puts Latin letters into Cyrillic words (`Bce`).
- On `bulgakov-diavoliada-1925` Tesseract makes two kinds of errors in
  similar amounts: wrong letters (`попос` for `понос`), and added
  characters where it reads the page border as text (3 to 16 `|` marks per
  page). `ocr-onnx` adds no such marks.

Open next steps, which this run does not measure:

- Line detection and line order of `ocr-onnx` on dense Latin pages.
- The separate Latin recognition model. The manifest pins it, but the
  pipeline uses the Cyrillic model for every page. The dictionary of the
  Cyrillic model also covers Latin letters.
