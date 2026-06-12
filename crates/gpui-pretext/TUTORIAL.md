# gpui-pretext Tutorial

`gpui-pretext` provides high-performance text measurement and multiline layout.

## 1. Add the crate

```toml
[dependencies]
gpui-pretext = { workspace = true }
```

## 2. Choose a line-break strategy

Use the exported layout and line-break types when building custom text surfaces:

```rust
use gpui_pretext::{LineBreakStrategy, TextMeasure};
```

## 3. Prepare text input

1. Normalize or segment text before layout if your editor needs custom rules.
2. Pick whitespace behavior through `WhiteSpaceMode`.
3. Pick a line break strategy, such as Knuth-Plass parameters for paragraph
   layout.
4. Measure text through a `TextMeasure` implementation.
5. Feed the produced runs into your GPUI renderer.

## 4. Use with gpui-builder

`gpui-builder` re-exports useful pretext pieces for layout solving. Use this
when text measurement affects responsive layout decisions.

## 5. Verify

```bash
cargo test -p gpui-pretext
```

When adding new Unicode behavior, include examples with combining marks,
bidirectional text, and long unbroken words.
