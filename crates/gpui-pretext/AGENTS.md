# gpui-pretext

High-performance text measurement and multiline layout — Rust port of chenglou/pretext.

## Architecture

Two-phase approach: prepare (measure once) then layout (pure arithmetic, fast enough for every resize).

- `measurement.rs` — `TextMeasure` trait (implement to provide text width measurement)
- `layout.rs` — Core API: `prepare()`, `layout()`, `layout_with_lines()`, `layout_next_line()`, `layout_optimal()`. Also `PreparedText`, `LayoutResult`, `LayoutLine`, `PrepareOptions`, `EngineProfile`
- `line_break.rs` — Line breaking algorithms (greedy + Knuth-Plass)
- `analysis.rs` — Text segmentation, `SegmentBreakKind`, `WhiteSpaceMode`
- `bidi.rs` — Bidirectional text support

## Key Public API

- `prepare(text, measure, profile, options) -> PreparedText` — measure and cache all segment widths
- `layout(prepared, max_width, line_height, profile) -> LayoutResult` — line break using cached widths
- `layout_with_lines(prepared, max_width, line_height, profile) -> LayoutLinesResult` — includes per-line data
- `layout_optimal(prepared, max_width, line_height, profile) -> LayoutResult` — Knuth-Plass optimal layout
- `TextMeasure` trait — `fn measure_width(&self, text: &str) -> f64`
- `EngineProfile` — algorithm selection (Greedy/KnuthPlass), tolerances
- `PrepareOptions` — whitespace handling, segment break behavior

## Testing

```bash
cargo test -p gpui-pretext --lib
```

## Important Notes

- Zero framework dependencies — only depends on `unicode-segmentation`
- `gpui-builder` uses `TextMeasure` for text-measured slot sizing (`Sizing::text()`)
- Knuth-Plass algorithm minimizes paragraph raggedness (TeX-inspired), but greedy is faster for most UI text
- Prepare phase is expensive (measures text); layout phase is cheap (pure arithmetic)
