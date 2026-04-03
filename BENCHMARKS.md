# cos-djvu Benchmark Suite

This document describes the benchmark methodology for `cos-djvu`, the pure-Rust
DjVu decoder. Benchmarks cover the three core codec paths and the full rendering
pipeline.

---

## What is measured

| Benchmark | File | What it exercises |
|-----------|------|-------------------|
| `bzz_decode` | `navm_fgbz.djvu` | BZZ arithmetic decompressor (DIRM / NAVM chunks) |
| `jb2_decode` | `boy_jb2.djvu` | JB2 bilevel image decoder (Sjbz foreground mask) |
| `iw44_decode_first_chunk` | `boy.djvu` | IW44 wavelet decoder, first BG44 chunk only |
| `render_page/dpi/72` | `boy.djvu` | Full page render at 72 DPI (screen) |
| `render_page/dpi/144` | `boy.djvu` | Full page render at 144 DPI (retina) |
| `render_page/dpi/300` | `boy.djvu` | Full page render at 300 DPI (print) |
| `render_coarse` | `boy.djvu` | Fast coarse render (first BG44 chunk only) |

All test assets live in `references/djvujs/library/assets/` and are bundled with
the repository. Benchmarks that require a file skip gracefully with an `eprintln!`
message if the file is absent — they do not fail the benchmark run.

---

## How to run

```bash
# Run all benchmarks for cos-djvu (uses Criterion, not nextest)
cargo bench -p cos-djvu

# Run a single benchmark group
cargo bench -p cos-djvu --bench codecs
cargo bench -p cos-djvu --bench render

# Compile benchmarks without running (fast CI smoke check)
cargo bench -p cos-djvu --no-run

# Save a baseline for comparison
cargo bench -p cos-djvu -- --save-baseline baseline-v1

# Compare against a saved baseline
cargo bench -p cos-djvu -- --baseline baseline-v1
```

Criterion writes HTML reports to `target/criterion/`. Open
`target/criterion/report/index.html` in a browser for interactive graphs.

---

## Expected performance ranges

Measured on an Apple M2 (single core, release build). These figures are
indicative; actual numbers depend on file size and hardware.

| Benchmark | Expected range | Notes |
|-----------|---------------|-------|
| `bzz_decode` | 5–30 ms | Depends on NAVM payload size |
| `jb2_decode` | 10–80 ms | Depends on symbol count and dictionary size |
| `iw44_decode_first_chunk` | 1–10 ms | Single chunk only |
| `render_page/dpi/72` | 20–100 ms | Full IW44 + JB2 composite |
| `render_page/dpi/144` | 50–200 ms | Scaling adds significant cost |
| `render_page/dpi/300` | 150–600 ms | Large output pixmap |
| `render_coarse` | 5–30 ms | Only decodes first wavelet slice |

**Fill in with actual numbers after running locally.** The ranges above are
rough estimates based on comparable Rust DjVu decoders.

---

## Corpus

The `tests/corpus/` directory is intentionally empty in the repository (only
`.gitkeep` is committed). To populate it with additional public-domain samples
from the Internet Archive, run:

```bash
bash crates/cos-djvu/scripts/fetch_corpus.sh
```

Benchmarks that use corpus files are gated behind `#[cfg(feature = "corpus-tests")]`
and are skipped automatically in CI unless the feature is explicitly enabled and
the files are present.

---

## Comparison with djvulibre (`ddjvu`)

`ddjvu` is the reference C implementation shipped with DjVuLibre.

### Setup

```bash
# macOS
brew install djvulibre

# Linux
apt-get install djvulibre-bin
```

### Measuring ddjvu throughput

```bash
# Render a page to PNM at 300 DPI, discard output, measure wall time
time ddjvu -format=pnm -page=1 -scale=300 file.djvu /dev/null

# Loop over a corpus for stable averages
for f in tests/corpus/*.djvu; do
  echo -n "$f: "
  time ddjvu -format=pnm -page=1 -scale=144 "$f" /dev/null 2>&1 | grep real
done
```

### Comparison methodology

1. Run `cargo bench -p cos-djvu -- --save-baseline cos-djvu-v1` on the same files.
2. Record the Criterion mean for `render_page/dpi/144` (closest to a retina
   screen render).
3. Run `ddjvu` at `-scale=144` on the same files and record wall time with
   `hyperfine` or `time`.
4. Divide `ddjvu` wall time by Criterion mean to get a relative speedup/slowdown
   ratio.

### Reference numbers (TBD)

| File | ddjvu 144 DPI | cos-djvu 144 DPI | Ratio |
|------|--------------|------------------|-------|
| boy.djvu | TBD | TBD | TBD |
| DjVu3Spec_bundled.djvu | TBD | TBD | TBD |

**Run locally and fill in this table.** The goal of `cos-djvu` for the initial
crates.io release is to be within 3× of djvulibre on the IW44 path and within
1.5× on the BZZ path.

---

## Comparison with djvu-rs

[`djvu-rs`](https://crates.io/crates/djvu) is the primary existing pure-Rust
alternative on crates.io.

### Setup

Add a temporary benchmark target in a scratch project:

```toml
[dev-dependencies]
djvu = "*"
cos-djvu = { path = "..." }
criterion = "0.5"
```

### Methodology

Use the same DjVu file and compare:
- `cos-djvu`: `DjVuDocument::parse` + `djvu_render::render_pixmap`
- `djvu-rs`: equivalent parse + render calls

Compare mean latency from Criterion output.

### Reference numbers (TBD)

| Operation | djvu-rs | cos-djvu | Ratio |
|-----------|---------|----------|-------|
| Parse document | TBD | TBD | TBD |
| Render page (144 DPI) | TBD | TBD | TBD |

---

## CI integration

Benchmarks are **not** run by `cargo nextest`. They only run via `cargo bench`.

The recommended CI step is:

```yaml
- name: Verify benchmarks compile
  run: cargo bench -p cos-djvu --no-run
```

This catches compilation errors without spending time on the actual measurements.
