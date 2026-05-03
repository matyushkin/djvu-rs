# Differential testing tolerances vs DjVuLibre

This document records the per-codec tolerance thresholds used by the
`diff_djvulibre` harness (`examples/diff_djvulibre.rs`) and the CI
workflow `.github/workflows/diff.yml`. Tracked under #192.

## Harness

```
cargo run --release --features cli --example diff_djvulibre -- \
    [--width N] [--tolerance T] [--max-pages M] <file.djvu> [...]
```

For each page, both `djvu_rs` and `ddjvu` are asked to render at the
same target size; the resulting RGBA / RGB pixmaps are compared per
pixel. A pixel is "mismatched" if `max(|Δr|, |Δg|, |Δb|) > tolerance`.

## Why tolerance > 0 is acceptable

A small per-channel difference is **not** a decoder bug:

* **IW44**: lossy wavelet, both implementations carry intermediate
  rounding errors that diverge by ±1 LSB.
* **Resampling**: when target render size < native page size, both
  pipelines apply a downsampling filter. They are not identical
  (different kernels, different colour-space conversions). At small
  widths this dominates the diff.
* **YCbCr ↔ RGB**: the BT.601 / BT.709 matrices and rounding bias
  differ subtly between implementations.

Differences > a few LSB or > 5% of pixels are **decoder bugs** and
should be filed as separate issues.

## Per-codec ceilings (`PAGE_CEILING_PCT`, `MEAN_DELTA_CEILING`)

The CI gate enforces global ceilings; per-codec breakdown is
informational for triage.

| Codec / page kind | Tolerance | Page mismatch% | Mean abs Δ |
|-------------------|-----------|----------------|------------|
| **JB2 only (bilevel)**         | 0 | < 0.5% | < 0.5 |
| **IW44 only (photo)**          | 4 | < 5%   | < 1.5 |
| **Mixed JB2 + IW44 (scan)**    | 4 | < 5%   | < 1.5 |
| **Native-resolution render**   | 4 | < 0.5% | < 0.2 |
| **Downsampled render < 600px** | 8 | < 14%  | < 3.0 |

Empirical results, `--width 600 --tolerance 4`:

| File | Codec | Mismatch% | Max Δ | Mean Δ |
|------|-------|-----------|-------|--------|
| `boy.djvu`           | JB2+IW44 | 0.000% | 0  | 0.00 |
| `chicken.djvu`       | JB2+IW44 | 0.000% | 0  | 0.00 |
| `colorbook.djvu`     | IW44     | < 1%   | 22 | 0.20 |
| `watchmaker.djvu` †  | IW44     | 0.03%  | 22 | 0.05 |

† `watchmaker.djvu` not in fixtures; see `references/djvujs/library/assets/`.

## CI ceilings (currently enforced)

`.github/workflows/diff.yml` runs weekly + on dispatch and fails if any
page in the *bit-perfect-baseline* corpus exceeds:

* `PAGE_CEILING_PCT = 0.5`
* `MEAN_DELTA_CEILING = 0.2`

The CI corpus is the subset of `tests/fixtures/*.djvu` that is
bit-for-bit identical to ddjvu at native resolution today (see the
empirical table above). Any future regression — even a tiny mean Δ
shift — fails the gate.

## Known divergences (excluded from CI gate, tracked separately)

Tracked under **#250**:

* `colorbook.djvu` page 0 — residual IW44 native-resolution divergence
  after the #199 compositor coordinate fix. Reproducer:
  `cargo run --release --features cli --example diff_djvulibre -- --width 99999 --tolerance 4 --max-pages 1 tests/fixtures/colorbook.djvu`.
  Current result at 2260x3669: `mismatch_pct = 3.4482%`,
  `max Δ = 97`, `mean Δ = 0.659`.
  A nearest-neighbor FG44 cell-mapping experiment regressed this to 10.93%,
  so the remaining drift is not explained by simple FG44 cell assignment.
  Keep this page excluded from the strict native-resolution CI corpus unless
  a future IW44 interpolation/color-ordering fix brings it below 0.5%.
* `navm_fgbz.djvu` pages 3 + 4 — FGbz divergence
  (`mismatch_pct = 0.96%–2.2%`, `mean Δ ≈ 1–2` at width 2550).
  Small enough that single-LSB rounding can't explain it.

These are excluded from the CI corpus until their tracking issues land.

## Filing divergences

If the CI gate fails or local development surfaces a page with
`mismatch_pct > 5%` at native resolution:

1. Save the failing JSON line from `diff_results.jsonl`.
2. Reproduce locally with the same tolerance + width.
3. Attempt to localise the codec: re-run with the same args but a
   single `tests/fixtures/<file>.djvu` to isolate.
4. File a new issue under the `bug` label, link to #192, attach:
   - Source `.djvu`
   - djvu-rs render PNG
   - ddjvu render PPM
   - Diff visualisation (`magick compare`)
