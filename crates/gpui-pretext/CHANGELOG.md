# Unreleased

# 0.11.1

## Added

- `MeasureCache::bound_to` / `bound_to_with_budgets` / `rebind`:
  construction-bound provider identity. Foreign providers on a bound
  cache are measured fresh, retain nothing, and cannot serve or evict
  the bound provider's entries.

## Fixed

- Segmentation corruption with chained escaped-quote clusters
  (`\'` repeated): a later cluster could resurrect a segment slot
  emptied earlier in the same merge pass, keeping the earlier start
  offset for later text and breaking the (start, text) storage
  invariant. Found by the new fuzz corpus replay; regression test
  `fuzz_crash_segmentation_starts_stay_in_storage`.

# 0.11.0

# 0.10.11

## Added

- Reworked text measurement caching and the line-break walk, and added
  allocation-contract tests, fuzz targets, and a temporaries bench.

## Refactored

- Normalized formatting in the line-break walk and exports; no behavior
  change.

## 0.9.9 - 2026-08-23

### Performance

- Reused segmentation, text-analysis, measurement, and line-breaking buffers to reduce repeated layout allocations.

## Performance

- Added bounded text preparation APIs with byte, grapheme, and segment limits
  plus cooperative cancellation checks around analysis and measurement.

# 0.9.7

## Fixes

- Replaced the bidi ratio heuristic with the `unicode-bidi` implementation of
  Unicode Bidirectional Algorithm (UAX #9).
- Corrected first-strong paragraph direction, explicit embeddings, isolates,
  and neutral-character handling.

# 0.7.4

## Performance

- `PreparedLineBreakData` now holds `Cow` views into `PreparedCore` and shares a
  Knuth-Plass item cache, reducing allocations during line breaking.
- `SegmentMetrics` widths are returned as `Arc<[f64]>` and reused across
  callers.
- `RichTextSpan` and `AccessibleTextRun` use `Cow<str>` to avoid cloning owned
  text during parsing and layout.

# 0.6.2

## New

- Started to migrate to new design/builder pattern
- Added Knuth-Plass algo to pretext (for fan of LaTeX)
- Add a gpui-pretext that does the same thing as pretext for js
