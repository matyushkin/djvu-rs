# CLAUDE.md — агентная память проекта

Этот файл — лабораторный журнал для Claude. Обновляй его ПЕРЕД коммитом каждого
значимого эксперимента. Цель: не тратить токены на повтор уже пройденных путей.

---

## Архитектура горячих путей

```
DjVu decode pipeline:
  IFF parse → chunk dispatch
    ├─ JB2  (bilevel):  ZpDecoder → jb2.rs → bitmap
    ├─ IW44 (color):   ZpDecoder → iw44_new.rs → YCbCr tiles → RGB
    └─ BZZ  (text):    ZpDecoder → MTF/Huffman → UTF-8

ZpDecoder (src/zp/mod.rs) — самый горячий путь:
  decode_bit() вызывается миллионы раз на страницу
  поля: a (interval), c (code), fence (cached bound), bit_buf, bit_count
  renormalize() — вызывается при каждом LPS событии

Composite pipeline (src/djvu_render.rs):
  JB2 bitmap + IW44 background → final pixmap
  горячий путь: composite_bilevel(), composite_color()
```

**Профилировщик:** `cargo bench --bench codecs` (Criterion, ~2 мин)  
**Сравнение с DjVuLibre:** `bash scripts/bench_djvulibre.sh .`

---

## Базовые метрики (Apple M1 Max, 2026-04-15, после ZP u16→u32)

| Benchmark | Результат | vs BENCHMARKS.md (v0.4.1) |
|-----------|-----------|---------------------------|
| `jb2_decode` | **131.8 µs** | −42% (было 228 µs) |
| `iw44_decode_first_chunk` | **725 µs** | −1.2% (было 734 µs) |
| `iw44_decode_corpus_color` | **2.30 ms** | — |
| `jb2_decode_corpus_bilevel` | **421 µs** | — |
| `jb2_encode` | **182 µs** | — |
| `iw44_encode_color` | **2.16 ms** | — |
| `render_page/dpi/72` | 1.21 ms | (из BENCHMARKS.md) |
| `render_page/dpi/300` | 4.02 ms | (из BENCHMARKS.md) |

> Числа из Criterion на M1 Max. Полная таблица с x86_64 и DjVuLibre → BENCHMARKS.md

---

## Журнал экспериментов

Формат: `дата | компонент | изменение | результат | вердикт`

### ✓ Оставлено

| Дата | Компонент | Изменение | Эффект |
|------|-----------|-----------|--------|
| 2026-04 | ZP/JB2 | local-copy ZP state (register alloc) + hardware CLZ | −15% JB2 |
| 2026-04 | ZP/JB2 | устранение bounds checks в горячих циклах JB2 + ZP renormalize | значимо |
| 2026-04 | ZP | a/c/fence: u16→u32, убраны все as u16 касты в hot loop | jb2 −2%, iw44_color −1.8%, jb2_encode −2.2% |
| 2026-04 | IW44 | row_pass SIMD обобщён на s=2/4/8 (было только s=1) | sub2_decode −3.1% (p=0.00); sub1 noise |
| 2026-04 | BZZ | inline ZP state locals в MTF decode | значимо |
| 2026-04 | render | downsampled mask pyramid для composite | 8ms→23ms на 150dpi |
| 2026-04 | render | partial BG44 decode для sub=4 | пропуск высоких частот |
| 2026-04 | render | chunks_exact_mut → убраны per-pixel bounds checks | небольшой |
| 2026-04 | render | x86_64 SSE2/SSSE3 fast paths (alpha fill, RGB→RGBA) | значимо на x86_64 |

### ✗ Отменено / Откатано

| Дата | Компонент | Что пробовали | Почему откатили |
|------|-----------|---------------|-----------------|
| 2026-04 | render | bilevel composite fast path (#165) | регрессия — восстановлено в #169 |
| 2026-04 | ZP | `#[cold] #[inline(never)]` для LPS-ветки + cmov-friendly context update | iw44 +4%, jb2_encode +2% — function call overhead > I-cache выигрыш; LPS 10-15% слишком часто для out-of-line |

> **Важно:** если что-то откатываешь — записывай сюда с причиной, иначе это будет попробовано снова.

### → Гипотезы (не измеряли)

| Компонент | Идея | Ожидание | Риск |
|-----------|------|----------|------|
| ZP | SIMD decode нескольких символов за раз (8-wide) | большой | сложно, breaking |
| ZP | branch-free decode_bit с cmov (#179) | ✗ отменено — см. журнал | LPS function call overhead хуже чем inline |
| IW44 | column_pass SIMD при s=2 (stride-2 gather, #180 продолжение) | небольшой | нужен load8_strided (vld2q_s16 на NEON) |
| JB2 | битовая упаковка bitmap → меньше памяти/cache | средний | сложно |
| render | предвычисление JB2 bitmap на отдельном потоке | средний | требует Arc |
| ZP | LUT для частых состояний (#181) | небольшой | cache pressure |

---

## Правила ведения журнала

1. После отката — **сразу** добавь строку в "Отменено" с причиной
2. После замера — обнови числа в "Базовых метриках" если изменились >5%
3. Перед началом эксперимента — проверь "Гипотезы" и "Отменено" чтобы не дублировать
4. Гипотезу после реализации — перемести в "Оставлено" или "Отменено"
