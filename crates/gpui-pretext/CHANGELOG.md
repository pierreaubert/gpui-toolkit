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
