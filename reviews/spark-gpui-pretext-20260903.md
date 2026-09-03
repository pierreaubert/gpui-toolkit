# Code Review: gpui-pretext — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-pretext` (38 files, ~9.4k LOC)

## 1. Purpose / role
High-perf text measurement + Knuth-Plass/greedy line breaking, bidi, analysis, rich-text spans; `TextMeasure` trait lets GPUI/host supply shaping. Largest: `line_break/tests/misc.rs` (1580), `line_break/types.rs` (878), `line_break/walk.rs` (749), `language_support.rs` (686), `line_break/layout.rs` (463). Benches: `layout_temporaries.rs`; tests: `allocation_contracts.rs`, `fuzz.rs`.

Public API: `prepare/prepare_with_budget/prepare_with_segments/profile_prepare`, `PrepareOptions/PreparedText/TextBudget/TextPrepareError`, `layout/layout_optimal/layout_next_line/layout_with_lines/walk_line_ranges`, `TextMeasure/EngineProfile/MeasureCache`, `KnuthPlassParams/LineBreakStrategy`, `SegmentBreakKind/WhiteSpaceMode`, `RichTextSpan/Style/parse_inline_markdown/accessibility_runs_for_spans`, `LanguageSupportReport/PlatformTextComparator`.

## 2. SOTA gap analysis (vs HarfBuzz, Skia Paragraph, TextKit)
1. **No real shaping** — `measure_width(&str)->f64` only; no glyph advances, ligatures, kerning.
2. **Skeletal bidi** (`bidi/mod.rs:85 classify_char`, `:116 compute_bidi_levels`) — no UAX#9 embeddings/isolates vs SheenBidi/rustybuzz.
3. **No font fallback / coverage / fontconfig stack.**
4. **No hyphenation dictionaries** (Knuth-Liang) despite `KnuthPlassParams`.
5. **No UAX#14 full tailoring + CJK/JL handling.**
6. **Variable-font axes are data-only** (`rich_text.rs`).
7. **No paragraph justification** (inter-word expansion/kashida) — Skia `ParagraphStyle` parity absent.
8. **Unbounded `MeasureCache`** — no LRU/byte budget tied to `TextBudget`; miss-path benches missing.

## 3. Performance evaluation
- Line-walk god-functions: `walk.rs:243 walk_prepared_lines` (371 lines/cyclo 54/nesting 8), `layout.rs:24 layout_next_line_range` (292 lines/cyclo 54), `types.rs:298 build_kp_items` (271 lines). Tested but MI 10–19.
- Per-line clone: `walk.rs:636 Vec::new() pending_lines` + `:643,:684 line.clone()` per candidate.
- Untested analysis core: `layout/types.rs:109 measure_analysis` (292 lines/risk 648), `types.rs:746 breakpoints_to_lines` (risk 354), `analysis_profile.rs:40 build_merged_segmentation` (176 lines/risk 240). Coverage 21% (33/157) — best in viz group but low where it matters.
- `measurement.rs:148 get_grapheme_widths` fan-in 14; benches cover hits only, not CJK/emoji misses.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Push indices/`Arc<Line>` instead of `line.clone()` (`walk.rs:643,684`); extend allocation contracts | S | walk allocs → ~0 |
| 2 | Split walk/emit loops by strategy (`_simple` vs `_optimal` at `walk.rs:25,619`, `layout.rs:317`) | M | readability + testability |
| 3 | Bounded LRU `MeasureCache` + miss-path benches (CJK/emoji) | M | worst-case latency |
| 4 | Fuzz `measure_analysis/breakpoints/build_merged_segmentation/bidi` before shaping work | S | correctness gate |
| 5 | Define `shape_run()` seam returning advances/clusters so HarfBuzz/rustybuzz plugs in | M | shaping without breaking `PrepareOptions` |

## 5. Verdict
Best-tested layout crate here; the gap is shaping/bidi/fallback, not architecture. Keep the allocation-contract discipline while adding the shaping seam.
