# DjVu Test Corpus

Public domain DjVu files used for benchmarks and integration tests.
All files are in the public domain in the United States and most other
jurisdictions, either because they were published before 1928 or because
they are US government works.

## Files

| File | Source | Type | Pages | Notes |
|------|--------|------|-------|-------|
| `watchmaker.djvu` | https://archive.org/details/Watchmaker2001 | Color IW44 | 1 | Single-page color DjVu, ~183 KB |
| `cable_1973_100133.djvu` | https://archive.org/details/State-Dept-cable-1973-100133 | JB2 bilevel | 1 | US State Dept cable, US gov't work (PD), ~15 KB |
| `conquete_paix.djvu` | https://archive.org/details/TriompheSagesseValeur | Mixed IW44+JB2 | multi | "La conquête de la paix" — pre-1928 French book (PD), ~1.7 MB |

## Public domain basis

- **US State Department cables (1973)**: Works of the US federal government
  are in the public domain under 17 U.S.C. § 105.
- **Watchmaker (2001)**: Identified by Internet Archive as public domain.
  Confirm before redistribution.
- **La conquête de la paix**: Published before 1928; public domain in the US.
  Original work published in France, also public domain there (author died > 70 years ago).

## License note

The corpus files are used for testing only and are NOT included in the
distributed crate (`cargo publish` excludes `tests/corpus/`).
