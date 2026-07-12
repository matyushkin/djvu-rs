# DjVu Test Corpus

Public domain DjVu files used for benchmarks and integration tests.
All files are in the public domain in the United States and most other
jurisdictions, either because they were published before 1928 or because
they are US government works.

## Tier 1 (legacy Latin-script book scans)

| File | Source | Type | Pages | Notes |
|------|--------|------|-------|-------|
| `watchmaker.djvu` | https://archive.org/details/Watchmaker2001 | Color IW44 | 1 | Single-page color DjVu, ~183 KB |
| `cable_1973_100133.djvu` | https://archive.org/details/State-Dept-cable-1973-100133 | JB2 bilevel | 1 | US State Dept cable, US gov't work (PD), ~15 KB |
| `conquete_paix.djvu` | https://archive.org/details/TriompheSagesseValeur | Mixed IW44+JB2 | multi | "La conquête de la paix" — pre-1928 French book (PD), ~1.7 MB |
| `pathogenic_bacteria_1896.djvu` | https://archive.org/details/PathogenicBacteria | Mixed IW44+JB2 | 520 | "Pathogenic Bacteria" (1896) — medical textbook, text + microscopy photos (PD), ~25 MB |

## Tier 2 (content-class diversity, #558)

Checked in (small extracts / whole short docs). SHA-256 in
`scripts/fetch_corpus.sh` for re-download verification.

| File | Class | Source | Pages | Size | Notes |
|------|-------|--------|-------|------|-------|
| `war_1812.djvu` | newspaper / photo-heavy scan | https://archive.org/details/warv1n2wood (*The War*, 1812) | 8 | ~898 KB | Early newspaper; p1 is BG44-dominated (tiny Sjbz), later pages text+mask |
| `goody_twoshoes.djvu` | mixed layout / illustrated | https://archive.org/details/goodytwoshoes00newyiala (1888, NOT_IN_COPYRIGHT) | 16 | ~1.0 MB | Text + colour illustration pages |
| `map_atlas_sample.djvu` | map / line-art | pages 28–29 of https://archive.org/details/graphicatlasofwo00bart (*Graphic atlas of the world*, 1910, NOT_IN_COPYRIGHT) | 2 | ~403 KB | Dense JB2 line work (Sjbz ~145–150 KB/page) |
| `chinese_cookbook_sample.djvu` | CJK text | pages 1–3, 27, 233 of https://archive.org/details/chinesecookbook00chan (*The Chinese cook book*, 1917, LoC PD note) | 5 | ~214 KB | Han-script scan; OCR_QA multi-script feed |
| `cyrillic_simonovich_co2.djvu` | Cyrillic text | https://archive.org/details/20200630_simonovich_uglekislota (Симонович, 1905, PD mark) | 12 | ~168 KB | Pre-reform orthography (ѣ etc.); shared Djbz + TXTz |
| `big_scanned_page.djvu` | photo / maskless IW44 | djvu.js test asset (Unlicense; also `tests/fixtures/big-scanned-page.djvu`) | 1 | ~571 KB | Full-res BG44 only (6780×9148) — true photo page the tier-1 corpus lacked |

### Tier-2 substitutions vs the issue wishlist

- **Second "chroma-half" (IW44 v2):** superseded by #561 (`CARTE_CHROMA_HEADER`). Valid IW44 v1.2 streams always use full-resolution chroma; `carte.djvu` remains the unusual short-INFO / v1.2 control in `tests/fixtures/`. Tier 2 ships `big_scanned_page.djvu` instead as the missing photo class.
- **Born-digital:** not found as a clearly licensed small DjVu; `watchmaker.djvu` (tier 1) and US-gov `cable_1973_100133.djvu` remain the closest anchors.

## Public domain basis

- **US State Department cables (1973)**: Works of the US federal government
  are in the public domain under 17 U.S.C. § 105.
- **Watchmaker (2001)**: Identified by Internet Archive as public domain.
  Confirm before redistribution.
- **Pathogenic Bacteria (1896)**: Published 1896 (pre-1928 US); public domain.
- **La conquête de la paix**: Published before 1928; public domain in the US.
  Original work published in France, also public domain there (author died > 70 years ago).
- **The War (1812), Goody Two-Shoes (1888), Graphic atlas (1910), Chinese cook book (1917), Simonovich (1905)**: pre-1928 / PD-mark / LoC unrestricted as cited in the tier-2 table.

## License note

The corpus files are used for testing only and are NOT included in the
distributed crate (`cargo publish` excludes `tests/corpus/`).
