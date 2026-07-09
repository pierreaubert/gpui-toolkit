# gpui-pretext

High-performance text measurement and multiline layout without DOM reflows.

Rust port of [chenglou/pretext](https://github.com/chenglou/pretext). Zero framework dependencies — works with any text rendering backend.

## Architecture

Two-phase approach for efficient text layout:

1. **Prepare phase** (`prepare` / `prepare_with_segments`): Segments text, measures segment widths via a `TextMeasure` implementation, and caches all width data. Run once per text block.

2. **Layout phase** (`layout` / `layout_with_lines` / `layout_next_line`): Pure arithmetic line breaking using cached widths. Fast enough to run on every resize.

## Usage

```rust
use gpui_pretext::{prepare, layout, TextMeasure, EngineProfile, PrepareOptions};

struct MyMeasure;
impl TextMeasure for MyMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        text.len() as f64 * 8.0 // replace with real measurement
    }
}

let measure = MyMeasure;
let profile = EngineProfile::default();
let options = PrepareOptions::default();

let prepared = prepare("Hello world, this is a long paragraph.", &measure, &profile, &options);
let lines = layout(&prepared, 200.0, 20.0, &profile); // wrap at 200px
```

## Line-Breaking Algorithms

- **Greedy** (default) — fast, good for most UI text
- **Knuth-Plass** — optimal paragraph layout minimizing raggedness, inspired by TeX

Select via `layout_with_strategy`:

```rust
use gpui_pretext::{layout_with_strategy, LineBreakStrategy};

let lines = layout_with_strategy(
    &prepared,
    200.0,
    20.0,
    &profile,
    LineBreakStrategy::Optimal,
);
```

## Language and Script Support

`gpui-pretext` owns text segmentation, measurement caching, and line-breaking decisions. It intentionally delegates final glyph shaping, fallback fonts, and platform rendering to the caller's `TextMeasure` implementation, so release notes should distinguish built-in layout behavior from backend-dependent language coverage.

Use `language_support_report()` for a stable, schema-versioned summary of supported and limited areas. The report currently covers Latin and whitespace-separated text, CJK line breaking, emoji and grapheme clusters, RTL and bidi ordering, complex shaping scripts, rich text and variable fonts, and untrusted or very large text input policy.
Use `locale_golden_report()` for deterministic locale/script regression cases and `benchmark_baseline_report()` for the Criterion benchmarks, locale samples, and platform text-renderer comparator artifacts that release QA should attach before claiming platform parity.

```rust
use gpui_pretext::{benchmark_baseline_report, language_support_report};

let report = language_support_report();
assert_eq!(report.report_type, "gpui-pretext-language-support");
println!("{}", report.to_markdown());

let baselines = benchmark_baseline_report();
assert_eq!(baselines.report_type, "gpui-pretext-benchmark-baselines");
```

## Integration with gpui-builder

The `gpui-builder` crate uses `TextMeasure` for text-measured slot sizing (`Sizing::text()`), enabling layout slots whose size is determined by their text content.

## Testing

```bash
cargo test -p gpui-pretext --lib
```
