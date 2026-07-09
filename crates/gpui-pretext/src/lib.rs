/// gpui-pretext: High-performance text measurement and multiline layout.
///
/// Rust port of [chenglou/pretext](https://github.com/chenglou/pretext).
///
/// # Architecture
///
/// Two-phase approach for efficient text layout:
///
/// 1. **Prepare phase** ([`prepare`] / [`prepare_with_segments`]):
///    Segments text, measures segment widths via a [`TextMeasure`] implementation,
///    and caches all width data. Run once per text block.
///
/// 2. **Layout phase** ([`layout`] / [`layout_with_lines`] / [`layout_next_line`]):
///    Pure arithmetic line breaking using cached widths. Fast enough to run on
///    every resize.
///
/// # Usage
///
/// ```rust,no_run
/// use gpui_pretext::{prepare, layout, TextMeasure, EngineProfile, PrepareOptions};
///
/// struct MyMeasure; // Backed by your text rendering system
/// impl TextMeasure for MyMeasure {
///     fn measure_width(&self, text: &str) -> f64 {
///         text.len() as f64 * 8.0 // placeholder
///     }
/// }
///
/// let measure = MyMeasure;
/// let profile = EngineProfile::default();
/// let options = PrepareOptions::default();
///
/// let prepared = prepare("Hello, world!", &measure, &profile, &options);
/// let result = layout(&prepared, 100.0, 20.0, &profile);
/// println!("Lines: {}, Height: {}", result.line_count, result.height);
/// ```
pub mod analysis;
pub mod bidi;
pub mod language_support;
pub mod layout;
#[allow(unused_assignments, unused_variables, unused_mut)]
pub mod line_break;
pub mod measurement;
pub mod rich_text;

// Re-export the main public API at the crate root
pub use analysis::{SegmentBreakKind, WhiteSpaceMode};
pub use language_support::{
    BENCHMARK_BASELINE_REPORT_TYPE, BENCHMARK_BASELINE_SCHEMA_VERSION, BenchmarkBaselineCase,
    BenchmarkBaselineReport, LANGUAGE_SUPPORT_REPORT_TYPE, LANGUAGE_SUPPORT_SCHEMA_VERSION,
    LOCALE_GOLDEN_REPORT_TYPE, LOCALE_GOLDEN_SCHEMA_VERSION, LanguageSupportLevel,
    LanguageSupportNote, LanguageSupportReport, LocaleGoldenCase, LocaleGoldenReport,
    PlatformTextComparator, benchmark_baseline_cases, benchmark_baseline_report,
    language_support_notes, language_support_report, locale_golden_cases, locale_golden_report,
};
pub use layout::{
    LayoutCursor, LayoutLine, LayoutLineRange, LayoutLinesResult, LayoutResult, PrepareOptions,
    PrepareProfile, PreparedText, PreparedTextWithSegments, layout, layout_next_line,
    layout_optimal, layout_with_lines, layout_with_lines_and_strategy, layout_with_lines_optimal,
    layout_with_strategy, prepare, prepare_with_segments, profile_prepare, walk_line_ranges,
    walk_line_ranges_optimal,
};
pub use line_break::{KnuthPlassParams, LineBreakStrategy};
pub use measurement::{EngineProfile, TextMeasure};
pub use rich_text::{
    AccessibleTextRole, AccessibleTextRun, FontVariationSettings, RichTextSpan, RichTextStyle,
    TextBenchmarkCase, VariableFontAxis, accessibility_runs_for_spans, parse_inline_markdown,
};
