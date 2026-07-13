# DjVu conformance dashboard

The conformance job compares native-resolution page renders with a pinned
DjVuLibre release and publishes both machine-readable results and a static
dashboard at <https://matyushkin.github.io/djvu-rs/dev/conformance/>.

The corpus and thresholds are versioned in `conformance/corpus.json`. Do not
add or remove a fixture only in the workflow: the manifest is the coverage
contract used to detect silently skipped pages.

## Reproduce locally

Install DjVuLibre so that `ddjvu` is on `PATH`, then build the harness:

```sh
cargo build --release --features cli --example diff_djvulibre
cargo build --release --features cli --example conformance_semantic
cargo build --release --features cli --example interop_encode
```

Run the manifest entries and collect JSONL using the same command generator as
CI:

```sh
python3 - <<'PY' > conformance_commands.sh
import json, shlex
manifest = json.load(open("conformance/corpus.json"))
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
bash conformance_commands.sh > diff_results.jsonl
./target/release/examples/conformance_semantic \
  tests/fixtures/boy.djvu \
  tests/fixtures/boy_jb2.djvu \
  tests/fixtures/chicken.djvu \
  tests/fixtures/ccitt_2.djvu \
  tests/fixtures/links.djvu \
  tests/fixtures/problem_page.djvu \
  tests/fixtures/big-scanned-page.djvu \
  tests/fixtures/navm_fgbz.djvu > semantic_results.jsonl
./target/release/examples/conformance_semantic --max-pages 1 \
  tests/fixtures/colorbook.djvu >> semantic_results.jsonl
./target/release/examples/interop_encode \
  tests/fixtures/boy.djvu tests/fixtures/chicken.djvu > writer_results.txt
cargo test --lib djvu_mut
printf 'pass\n' > writer_status.txt
python3 scripts/conformance_report.py \
  --results diff_results.jsonl \
  --semantic-results semantic_results.jsonl \
  --writer-status writer_status.txt \
  --output-dir conformance_site
```

Open `conformance_site/index.html` to inspect the generated dashboard. The
report command exits non-zero when coverage is incomplete, a result is
duplicated or malformed, or any metric exceeds the manifest threshold.

For directly comparable historical results use the DjVuLibre identity shown
in `summary.json`. A local run with a different DjVuLibre release remains a
useful diagnostic, but it is not the pinned CI baseline.

## Artifact contract

`summary.json` is schema-versioned and records:

- Git commit and DjVuLibre identity;
- SHA-256 for every input plus a digest for the complete corpus definition;
- render policy and thresholds;
- every per-page render result, semantic-plane result, and validation failure.

`history.json` retains the latest 100 published run summaries. CI restores the
previous published history before appending the current run. Missing or
malformed current results always fail; inability to download old history does
not make the current conformance result invalid.
