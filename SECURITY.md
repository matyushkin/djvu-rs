# Security Policy

## Supported versions

| Version | Gets fixes | Until |
|---------|------------|-------|
| newest `1.x` | security and other bugs | — |
| `1.0.0-rc.N` | none: upgrade to the next candidate or to `1.0.0` | — |
| `0.46.x` | security and critical bugs (a panic, wrong output or lost data on some input) | the `1.0.0` release |
| `0.46.x` | security only | 2027-03-31 |
| older `0.x` | none: upgrade to `0.46.x` | — |

Fixes are not backported to an older `1.x` minor release: upgrade to the
newest `1.x`. A `1.x` release does not break the stable API
([`docs/api-compatibility.md`](docs/api-compatibility.md)), so a Cargo
requirement of `djvu-rs = "1"` selects each fix on `cargo update`.

Moving from `0.46` to `1.0` needs code changes, described in
[`docs/migration.md`](docs/migration.md). The `0.46.x` line gets fixes for a
while so that this move can wait for a planned moment: `djvu-rs = "0.46"`
selects each `0.46.x` fix on `cargo update`. After 2027-03-31 the `0.x` line
gets no more releases.

## Scope

djvu-rs is a parser for an untrusted binary format. The following are in scope:

- Panics or aborts when parsing malformed DjVu input
- Out-of-bounds memory access (the crate forbids `unsafe`, but logic bugs can still
  cause index panics)
- Integer overflow leading to incorrect output or resource exhaustion
- Denial-of-service via crafted input (excessive allocation, infinite loops)

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Report privately via GitHub's built-in mechanism:
**Security → Report a vulnerability** on the repository page.

Include:
- A minimal reproducer (a byte sequence or `#[test]` that triggers the issue)
- Rust version and OS
- What you expected vs. what happened

You will receive a response within 7 days. If the issue is confirmed, a patched
release will be published and you will be credited in the changelog unless you
prefer otherwise.

## Fuzzing

The `fuzz/` directory contains libFuzzer targets for the decoders, the encoders,
the document parser and the metadata/validation paths (one per file in
`fuzz/fuzz_targets/`). CI (`.github/workflows/fuzz.yml`) runs every target for
60 seconds on each push to `main` and every Monday.

djvu-rs is **not fuzzed on OSS-Fuzz**. An OSS-Fuzz maintainer declined the
submission ([google/oss-fuzz#15331](https://github.com/google/oss-fuzz/pull/15331))
on 2026-11-16: the service takes projects that already have a wide user base.
The integration files stay in [`oss-fuzz/`](oss-fuzz/) for a later submission.
[`oss-fuzz/README.md`](oss-fuzz/README.md) has the details.

If you find a crash via fuzzing, please include the minimized corpus input.

## Decode-time resource ceilings

A malicious file can cost only bounded memory and work before decoding errors
out. Every allocation or loop whose size is driven by untrusted bytes is capped
against an explicit constant or the input length (#589 inventory). The load-
bearing bounds:

| Axis | Bound | Constant |
|------|-------|----------|
| JB2 symbol pixels (per symbol) | 16 MP | `MAX_SYMBOL_PIXELS` |
| JB2 symbol pixels (per stream) | 256 MP | `MAX_TOTAL_SYMBOL_PIXELS` |
| JB2 page / blit pixels | 16 MP / 256 MP | `MAX_PAGE_SYMBOL_PIXELS` / `MAX_TOTAL_BLIT_PIXELS` |
| JB2 record count | 65 536 | `MAX_RECORDS` |
| IW44 declared image | 64 MP | checked in `decode_chunk` |
| BZZ block / total output | 4 MiB / 256 MB | `MAX_BLOCK_SIZE` / `MAX_OUTPUT_SIZE` |
| IFF / FORM nesting depth | 64 | `MAX_IFF_DEPTH` |
| NAVM bookmark depth | 256 | `MAX_NAVM_DEPTH` |
| S-expression (ANTz) depth | bounded | `MAX_SEXPR_DEPTH` |
| **Text-zone (TXTz) depth** | **64** | **`MAX_ZONE_DEPTH`** (#589) |
| **Text-zone child reservation** | **≤ remaining ÷ 17 bytes** | **`MIN_ZONE_RECORD_BYTES`** (#589) |

DIRM component/name tables, ANTz maparea lists, and IFF chunk payloads allocate
only in proportion to bytes actually present in the file (each is length-checked
against the buffer before allocating), so they are bounded by construction.

The `#589` additions close the one unbounded axis the inventory found: a crafted
`TXTz` zone could declare up to ~16.7M children (a 3-byte `i24`) and recurse with
no depth limit. The child count no longer drives an up-front reservation beyond
what the remaining bytes could encode, and the zone tree cannot nest past
`MAX_ZONE_DEPTH`.
