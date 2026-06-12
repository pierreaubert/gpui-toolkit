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
let lines = layout(&prepared, 200.0); // wrap at 200px
```

## Line-Breaking Algorithms

- **Greedy** (default) — fast, good for most UI text
- **Knuth-Plass** — optimal paragraph layout minimizing raggedness, inspired by TeX

Select via `EngineProfile`:

```rust
let profile = EngineProfile {
    algorithm: Algorithm::KnuthPlass,
    ..Default::default()
};
```

## Integration with gpui-builder

The `gpui-builder` crate uses `TextMeasure` for text-measured slot sizing (`Sizing::text()`), enabling layout slots whose size is determined by their text content.

## Testing

```bash
cargo test -p gpui-pretext --lib
```
