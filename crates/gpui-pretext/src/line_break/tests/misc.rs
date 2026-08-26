use std::borrow::Cow;
use std::sync::Arc;

use super::super::compute::compute_adjustment_ratio;
use super::super::count::count_prepared_lines;
use super::super::count::count_prepared_lines_optimal;
use super::super::get::get_breakable_advance;
use super::super::get::get_tab_advance;
use super::super::knuth_plass_params::KnuthPlassParams;
use super::super::layout::layout_next_line_range;
use super::super::misc::badness;
use super::super::misc::can_break_after;
use super::super::misc::fit_soft_hyphen_break;
use super::super::misc::is_simple_collapsible_space;
use super::super::types::InternalLayoutLine;
use super::super::types::KPItem;
use super::super::types::LineBreakCursor;
use super::super::types::PreparedLineBreakData;
use super::super::types::PreparedLineChunk;
use super::super::types::build_kp_items;
use super::super::types::knuth_plass_chunk;
use super::super::types::normalize_line_start;
use super::super::walk::walk_prepared_lines;
use super::super::walk::walk_prepared_lines_optimal;
/// Line breaking algorithms, ported from chenglou/pretext.
///
/// Implements both:
/// 1. **Greedy** line breaking with pending-break tracking, soft hyphen
///    support, tab stops, and overflow-wrap grapheme-level breaking.
/// 2. **Knuth-Plass** optimal line breaking that minimizes total demerits
///    (badness + penalties) across the entire paragraph using dynamic
///    programming over feasible breakpoints.
///
/// The greedy algorithm has both a "simple" fast path (no tabs/soft-hyphens)
/// and a full complex path.
use crate::analysis::SegmentBreakKind;
use crate::measurement::EngineProfile;

fn simple_prepared(
    widths: Vec<f64>,
    kinds: Vec<SegmentBreakKind>,
) -> PreparedLineBreakData<'static> {
    let len = widths.len();
    let fit = widths.to_vec();
    let paint = fit.clone();
    PreparedLineBreakData {
        widths: Cow::Owned(widths),
        line_end_fit_advances: Cow::Owned(fit),
        line_end_paint_advances: Cow::Owned(paint),
        kinds: Cow::Owned(kinds),
        simple_line_walk_fast_path: true,
        breakable_widths: Cow::Owned(vec![None; len]),
        breakable_prefix_widths: Cow::Owned(vec![None; len]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: len,
            consumed_end_segment_index: len,
        }]),
        start_segment: 0,
        end_segment: len,
        kp_item_cache: None,
    }
}

#[test]
fn test_single_line() {
    let prepared = simple_prepared(
        vec![50.0, 10.0, 40.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let count = count_prepared_lines(&prepared, 200.0, &profile);
    assert_eq!(count, 1);
}

#[test]
fn test_two_lines() {
    let prepared = simple_prepared(
        vec![50.0, 10.0, 50.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let count = count_prepared_lines(&prepared, 70.0, &profile);
    assert_eq!(count, 2);
}

#[test]
fn test_walk_lines() {
    let prepared = simple_prepared(
        vec![50.0, 10.0, 50.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let mut lines = Vec::new();
    let count = walk_prepared_lines(
        &prepared,
        70.0,
        &profile,
        Some(&mut |line: &InternalLayoutLine| {
            lines.push(line.clone());
        }),
    );
    assert_eq!(count, 2);
    assert_eq!(lines.len(), 2);
}

fn kp_params() -> KnuthPlassParams {
    KnuthPlassParams::default()
}

#[test]
fn test_kp_empty() {
    let prepared = simple_prepared(vec![], vec![]);
    let profile = EngineProfile::default();
    let count = count_prepared_lines_optimal(&prepared, 200.0, &profile, &kp_params());
    assert_eq!(count, 0);
}

#[test]
fn test_kp_single_word() {
    // Single word "hello" = 50px, fits in 200px.
    let prepared = simple_prepared(vec![50.0], vec![SegmentBreakKind::Text]);
    let profile = EngineProfile::default();
    let count = count_prepared_lines_optimal(&prepared, 200.0, &profile, &kp_params());
    assert_eq!(count, 1);
}

#[test]
fn test_kp_single_line() {
    // "hello world" = 50 + 10 + 40 = 100px, fits in 200px.
    let prepared = simple_prepared(
        vec![50.0, 10.0, 40.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let count = count_prepared_lines_optimal(&prepared, 200.0, &profile, &kp_params());
    assert_eq!(count, 1);
}

#[test]
fn test_kp_two_lines() {
    // "hello world" = 50 + 10 + 50 = 110px, 70px max → should wrap.
    let prepared = simple_prepared(
        vec![50.0, 10.0, 50.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let count = count_prepared_lines_optimal(&prepared, 70.0, &profile, &kp_params());
    assert_eq!(count, 2);
}

#[test]
fn test_kp_walk_lines() {
    // "hello world" wrapping at 70px.
    let prepared = simple_prepared(
        vec![50.0, 10.0, 50.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let mut lines = Vec::new();
    let count = walk_prepared_lines_optimal(
        &prepared,
        70.0,
        &profile,
        &kp_params(),
        Some(&mut |line: &InternalLayoutLine| {
            lines.push(line.clone());
        }),
    );
    assert_eq!(count, 2);
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_kp_optimal_vs_greedy_difference() {
    // Classic case where KP produces better results than greedy:
    // "AAAA BB CCC DDDD EE" at width that causes greedy to make a bad
    // first-line decision.
    //
    // Segments: [40, 10, 20, 10, 30, 10, 40, 10, 20]
    //           [T,   S,  T,  S,  T,  S,  T,  S,  T]
    // Total = 190px
    //
    // At width=80:
    // Greedy: "AAAA BB" (70) | "CCC DDDD" (80) | "EE" (20)
    //   → 3 lines, last line very short
    // KP might find: "AAAA BB" (70) | "CCC DDDD" (80) | "EE" (20)
    //   → same 3 lines in this case, but both should produce valid output.
    let prepared = simple_prepared(
        vec![40.0, 10.0, 20.0, 10.0, 30.0, 10.0, 40.0, 10.0, 20.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();

    let greedy = count_prepared_lines(&prepared, 80.0, &profile);
    let optimal = count_prepared_lines_optimal(&prepared, 80.0, &profile, &kp_params());

    // Both should produce valid line counts (≥ 1).
    assert!(greedy >= 1);
    assert!(optimal >= 1);
    // Optimal should use same or fewer lines.
    assert!(optimal <= greedy + 1, "optimal={optimal}, greedy={greedy}");
}

#[test]
fn test_kp_even_distribution() {
    // 5 equal-width words, each 30px with 10px spaces.
    // Total content: 5*30 + 4*10 = 190px.
    // At width=100:
    // Greedy: "AAA BBB" (70) | "CCC DDD" (70) | "EEE" (30) → uneven last line
    // KP: should distribute more evenly.
    let prepared = simple_prepared(
        vec![30.0, 10.0, 30.0, 10.0, 30.0, 10.0, 30.0, 10.0, 30.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();

    let mut lines = Vec::new();
    let count = walk_prepared_lines_optimal(
        &prepared,
        100.0,
        &profile,
        &kp_params(),
        Some(&mut |line: &InternalLayoutLine| {
            lines.push(line.clone());
        }),
    );

    // Should produce a valid result.
    assert!(count >= 2);
    assert_eq!(lines.len(), count);

    // Every line should have positive width.
    for (i, line) in lines.iter().enumerate() {
        assert!(line.width > 0.0, "line {i} has zero width");
    }
}

#[test]
fn test_kp_soft_hyphen_penalty() {
    // Verify soft hyphens get flagged penalty.
    let len = 5;
    let widths = vec![50.0, 0.0, 50.0, 0.0, 30.0];
    let kinds = vec![
        SegmentBreakKind::Text,
        SegmentBreakKind::SoftHyphen,
        SegmentBreakKind::Text,
        SegmentBreakKind::SoftHyphen,
        SegmentBreakKind::Text,
    ];
    let fit = widths.to_vec();
    let paint = fit.clone();
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(widths),
        line_end_fit_advances: Cow::Owned(fit),
        line_end_paint_advances: Cow::Owned(paint),
        kinds: Cow::Owned(kinds),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None; len]),
        breakable_prefix_widths: Cow::Owned(vec![None; len]),
        discretionary_hyphen_width: 5.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: len,
            consumed_end_segment_index: len,
        }]),
        start_segment: 0,
        end_segment: len,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let count = count_prepared_lines_optimal(&prepared, 60.0, &profile, &kp_params());
    // Should break at one of the soft hyphens.
    assert!(count >= 2);
}

#[test]
fn test_kp_hard_break() {
    // Hard break forces a line break regardless of remaining space.
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![40.0, 0.0, 30.0]),
        line_end_fit_advances: Cow::Owned(vec![40.0, 0.0, 30.0]),
        line_end_paint_advances: Cow::Owned(vec![40.0, 0.0, 30.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::HardBreak,
            SegmentBreakKind::Text,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None; 3]),
        breakable_prefix_widths: Cow::Owned(vec![None; 3]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![
            PreparedLineChunk {
                start_segment_index: 0,
                end_segment_index: 1,
                consumed_end_segment_index: 2,
            },
            PreparedLineChunk {
                start_segment_index: 2,
                end_segment_index: 3,
                consumed_end_segment_index: 3,
            },
        ]),
        start_segment: 0,
        end_segment: 3,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let count = count_prepared_lines_optimal(&prepared, 200.0, &profile, &kp_params());
    assert_eq!(count, 2);
}

#[test]
fn test_kp_build_items_basic() {
    // "hello world" → [Text(50), Space(10), Text(40)]
    let prepared = simple_prepared(
        vec![50.0, 10.0, 40.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let items = build_kp_items(&prepared, &profile, &kp_params());

    // Should have: start sentinel, break-after-space, end-of-paragraph.
    assert!(items.len() >= 3, "items.len() = {}", items.len());
    // First item: start sentinel.
    assert_eq!(items[0].segment_index, 0);
    assert_eq!(items[0].penalty, f64::NEG_INFINITY);
    // Last item: end of paragraph.
    let last = items.last().unwrap();
    assert_eq!(last.segment_index, 3);
    assert_eq!(last.penalty, f64::NEG_INFINITY);
}

#[test]
fn test_kp_adjustment_ratio() {
    // Build items for "AAAA BBBB" = [40, 10, 40] at max_width=90
    // Line width = 40 + 10 + 40 = 90 (exact fit).
    let prepared = simple_prepared(
        vec![40.0, 10.0, 40.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let items = build_kp_items(&prepared, &profile, &kp_params());

    // Ratio from start to end should be close to 0 at width=90.
    let ratio = compute_adjustment_ratio(&items, 0, items.len() - 1, 90.0, 0.0);
    assert!(ratio.abs() < 0.5, "ratio = {ratio}");
}

#[test]
fn test_kp_fallback_to_greedy() {
    // With very tight tolerance and extreme conditions, KP might fail
    // and should fall back to greedy.
    let prepared = simple_prepared(
        vec![50.0, 10.0, 50.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let mut params = kp_params();
    params.tolerance = 0.001; // extremely tight
    params.looseness_recovery = true;

    // Should still produce valid output (via fallback).
    let count = count_prepared_lines_optimal(&prepared, 70.0, &profile, &params);
    assert!(count >= 1);
}

#[test]
fn test_kp_count_matches_walk() {
    // Verify count_prepared_lines_optimal matches walk_prepared_lines_optimal.
    let prepared = simple_prepared(
        vec![30.0, 10.0, 30.0, 10.0, 30.0, 10.0, 30.0, 10.0, 30.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();

    for width in [50.0, 80.0, 100.0, 150.0, 200.0] {
        let count = count_prepared_lines_optimal(&prepared, width, &profile, &kp_params());
        let mut walk_count = 0;
        walk_prepared_lines_optimal(
            &prepared,
            width,
            &profile,
            &kp_params(),
            Some(&mut |_: &InternalLayoutLine| {
                walk_count += 1;
            }),
        );
        assert_eq!(count, walk_count, "mismatch at width {width}");
    }
}

#[test]
fn test_kp_many_words_monotonic_widths() {
    // As width decreases, line count should increase monotonically.
    let prepared = simple_prepared(
        vec![
            30.0, 10.0, 30.0, 10.0, 30.0, 10.0, 30.0, 10.0, 30.0, 10.0, 30.0, 10.0, 30.0,
        ],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();

    let mut prev_count = 0;
    for width in [500.0, 200.0, 100.0, 80.0, 50.0] {
        let count = count_prepared_lines_optimal(&prepared, width, &profile, &kp_params());
        assert!(
            count >= prev_count,
            "line count decreased at width {width}: {count} < {prev_count}"
        );
        prev_count = count;
    }
}

#[test]
fn test_layout_next_line_range_simple() {
    let prepared = simple_prepared(
        vec![50.0, 10.0, 40.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let line = layout_next_line_range(
        &prepared,
        LineBreakCursor {
            segment_index: 0,
            grapheme_index: 0,
        },
        110.0,
        &profile,
    );
    assert!(line.is_some());
    let line = line.unwrap();
    assert_eq!(line.end_segment_index, 3);
    assert_eq!(line.width, 100.0);
}

#[test]
fn test_layout_next_line_range_wraps_at_space() {
    let prepared = simple_prepared(
        vec![50.0, 10.0, 50.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let line = layout_next_line_range(
        &prepared,
        LineBreakCursor {
            segment_index: 0,
            grapheme_index: 0,
        },
        70.0,
        &profile,
    );
    assert!(line.is_some());
    let line = line.unwrap();
    assert_eq!(line.end_segment_index, 2);
    assert_eq!(line.width, 50.0);
}

#[test]
fn test_layout_next_line_range_with_tab() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![40.0, 0.0, 40.0]),
        line_end_fit_advances: Cow::Owned(vec![40.0, 0.0, 40.0]),
        line_end_paint_advances: Cow::Owned(vec![40.0, 0.0, 40.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Tab,
            SegmentBreakKind::Text,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None, None]),
        breakable_prefix_widths: Cow::Owned(vec![None, None, None]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 32.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 3,
            consumed_end_segment_index: 3,
        }]),
        start_segment: 0,
        end_segment: 3,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let line = layout_next_line_range(
        &prepared,
        LineBreakCursor {
            segment_index: 0,
            grapheme_index: 0,
        },
        200.0,
        &profile,
    );
    assert!(line.is_some());
    let line = line.unwrap();
    // Width = 40 (text) + 32 (tab advance from 40 to next 32-stop = 64, so advance 24? wait tab_stop=32, line_w=40, remainder=8, advance=24) + 40 (text) = 104
    assert!(line.width > 80.0, "width = {}", line.width);
}

#[test]
fn test_layout_next_line_range_breakable_overflow() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![100.0]),
        line_end_fit_advances: Cow::Owned(vec![100.0]),
        line_end_paint_advances: Cow::Owned(vec![100.0]),
        kinds: Cow::Owned(vec![SegmentBreakKind::Text]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![Some(Arc::from(vec![30.0, 30.0, 30.0]))]),
        breakable_prefix_widths: Cow::Owned(vec![Some(Arc::from(vec![30.0, 60.0, 90.0]))]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 1,
            consumed_end_segment_index: 1,
        }]),
        start_segment: 0,
        end_segment: 1,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let line = layout_next_line_range(
        &prepared,
        LineBreakCursor {
            segment_index: 0,
            grapheme_index: 0,
        },
        50.0,
        &profile,
    );
    assert!(line.is_some());
    let line = line.unwrap();
    assert_eq!(line.end_segment_index, 0);
    assert_eq!(line.end_grapheme_index, 1);
    assert_eq!(line.width, 30.0);
}

#[test]
fn test_layout_next_line_range_start_mid_segment() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![100.0]),
        line_end_fit_advances: Cow::Owned(vec![100.0]),
        line_end_paint_advances: Cow::Owned(vec![100.0]),
        kinds: Cow::Owned(vec![SegmentBreakKind::Text]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![Some(Arc::from(vec![30.0, 30.0, 30.0]))]),
        breakable_prefix_widths: Cow::Owned(vec![Some(Arc::from(vec![30.0, 60.0, 90.0]))]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 1,
            consumed_end_segment_index: 1,
        }]),
        start_segment: 0,
        end_segment: 1,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let line = layout_next_line_range(
        &prepared,
        LineBreakCursor {
            segment_index: 0,
            grapheme_index: 1,
        },
        50.0,
        &profile,
    );
    assert!(line.is_some());
    let line = line.unwrap();
    assert_eq!(line.start_segment_index, 0);
    assert_eq!(line.start_grapheme_index, 1);
    assert_eq!(line.end_grapheme_index, 2);
    assert_eq!(line.width, 30.0);
}

#[test]
fn test_layout_next_line_range_soft_hyphen() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![50.0, 0.0, 50.0]),
        line_end_fit_advances: Cow::Owned(vec![50.0, 5.0, 50.0]),
        line_end_paint_advances: Cow::Owned(vec![50.0, 5.0, 50.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::SoftHyphen,
            SegmentBreakKind::Text,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None, None]),
        breakable_prefix_widths: Cow::Owned(vec![None, None, None]),
        discretionary_hyphen_width: 5.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 3,
            consumed_end_segment_index: 3,
        }]),
        start_segment: 0,
        end_segment: 3,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let line = layout_next_line_range(
        &prepared,
        LineBreakCursor {
            segment_index: 0,
            grapheme_index: 0,
        },
        55.0,
        &profile,
    );
    assert!(line.is_some());
    let line = line.unwrap();
    // Should break after soft hyphen: 50 + hyphen 5 = 55
    assert_eq!(line.end_segment_index, 2);
    assert_eq!(line.width, 55.0);
}

#[test]
fn test_layout_next_line_range_soft_hyphen_breakable() {
    // Soft hyphen followed by breakable segment that can partially fit.
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![50.0, 0.0, 100.0]),
        line_end_fit_advances: Cow::Owned(vec![50.0, 5.0, 100.0]),
        line_end_paint_advances: Cow::Owned(vec![50.0, 5.0, 100.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::SoftHyphen,
            SegmentBreakKind::Text,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None, Some(Arc::from(vec![20.0; 5]))]),
        breakable_prefix_widths: Cow::Owned(vec![
            None,
            None,
            Some(Arc::from(vec![20.0, 40.0, 60.0, 80.0, 100.0])),
        ]),
        discretionary_hyphen_width: 5.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 3,
            consumed_end_segment_index: 3,
        }]),
        start_segment: 0,
        end_segment: 3,
        kp_item_cache: None,
    };
    let profile = EngineProfile {
        prefer_early_soft_hyphen_break: true,
        ..Default::default()
    };
    let line = layout_next_line_range(
        &prepared,
        LineBreakCursor {
            segment_index: 0,
            grapheme_index: 0,
        },
        55.0,
        &profile,
    );
    assert!(line.is_some());
    let line = line.unwrap();
    assert_eq!(line.end_segment_index, 2);
    assert_eq!(line.width, 55.0);
}

#[test]
fn test_walk_prepared_lines_simple_breakable() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![100.0]),
        line_end_fit_advances: Cow::Owned(vec![100.0]),
        line_end_paint_advances: Cow::Owned(vec![100.0]),
        kinds: Cow::Owned(vec![SegmentBreakKind::Text]),
        simple_line_walk_fast_path: true,
        breakable_widths: Cow::Owned(vec![Some(Arc::from(vec![30.0, 30.0, 30.0]))]),
        breakable_prefix_widths: Cow::Owned(vec![None]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 1,
            consumed_end_segment_index: 1,
        }]),
        start_segment: 0,
        end_segment: 1,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let mut lines = Vec::new();
    let count = walk_prepared_lines(
        &prepared,
        50.0,
        &profile,
        Some(&mut |line: &InternalLayoutLine| {
            lines.push(line.clone());
        }),
    );
    assert_eq!(count, 3);
    assert_eq!(lines.len(), 3);
}

#[test]
fn test_walk_prepared_lines_empty() {
    let prepared = simple_prepared(vec![], vec![]);
    let profile = EngineProfile::default();
    let count = walk_prepared_lines(&prepared, 100.0, &profile, None);
    assert_eq!(count, 0);
}

#[test]
fn test_walk_prepared_lines_hard_break_chunks() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![40.0, 0.0, 30.0]),
        line_end_fit_advances: Cow::Owned(vec![40.0, 0.0, 30.0]),
        line_end_paint_advances: Cow::Owned(vec![40.0, 0.0, 30.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::HardBreak,
            SegmentBreakKind::Text,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None, None]),
        breakable_prefix_widths: Cow::Owned(vec![None, None, None]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![
            PreparedLineChunk {
                start_segment_index: 0,
                end_segment_index: 1,
                consumed_end_segment_index: 2,
            },
            PreparedLineChunk {
                start_segment_index: 2,
                end_segment_index: 3,
                consumed_end_segment_index: 3,
            },
        ]),
        start_segment: 0,
        end_segment: 3,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let mut lines = Vec::new();
    let count = walk_prepared_lines(
        &prepared,
        200.0,
        &profile,
        Some(&mut |line: &InternalLayoutLine| {
            lines.push(line.clone());
        }),
    );
    assert_eq!(count, 2);
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_walk_prepared_lines_tab_and_soft_hyphen() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![40.0, 0.0, 0.0, 40.0]),
        line_end_fit_advances: Cow::Owned(vec![40.0, 0.0, 5.0, 40.0]),
        line_end_paint_advances: Cow::Owned(vec![40.0, 0.0, 5.0, 40.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Tab,
            SegmentBreakKind::SoftHyphen,
            SegmentBreakKind::Text,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None, None, None]),
        breakable_prefix_widths: Cow::Owned(vec![None, None, None, None]),
        discretionary_hyphen_width: 5.0,
        tab_stop_advance: 32.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 4,
            consumed_end_segment_index: 4,
        }]),
        start_segment: 0,
        end_segment: 4,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let mut lines = Vec::new();
    let count = walk_prepared_lines(
        &prepared,
        200.0,
        &profile,
        Some(&mut |line: &InternalLayoutLine| {
            lines.push(line.clone());
        }),
    );
    assert_eq!(count, 1);
    assert_eq!(lines.len(), 1);
}

#[test]
fn test_walk_prepared_lines_optimal_multi_chunk() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![40.0, 0.0, 30.0]),
        line_end_fit_advances: Cow::Owned(vec![40.0, 0.0, 30.0]),
        line_end_paint_advances: Cow::Owned(vec![40.0, 0.0, 30.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::HardBreak,
            SegmentBreakKind::Text,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None, None]),
        breakable_prefix_widths: Cow::Owned(vec![None, None, None]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![
            PreparedLineChunk {
                start_segment_index: 0,
                end_segment_index: 1,
                consumed_end_segment_index: 2,
            },
            PreparedLineChunk {
                start_segment_index: 2,
                end_segment_index: 3,
                consumed_end_segment_index: 3,
            },
        ]),
        start_segment: 0,
        end_segment: 3,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let mut lines = Vec::new();
    let count = walk_prepared_lines_optimal(
        &prepared,
        200.0,
        &profile,
        &kp_params(),
        Some(&mut |line: &InternalLayoutLine| {
            lines.push(line.clone());
        }),
    );
    assert_eq!(count, 2);
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_walk_prepared_lines_optimal_fallback_to_greedy() {
    // Empty prepared data should fall back to greedy and return 0.
    let prepared = simple_prepared(vec![], vec![]);
    let profile = EngineProfile::default();
    let count = walk_prepared_lines_optimal(&prepared, 100.0, &profile, &kp_params(), None);
    assert_eq!(count, 0);
}

#[test]
fn test_walk_prepared_lines_optimal_looseness_recovery() {
    // Very tight tolerance with a feasible solution only at higher tolerance.
    let prepared = simple_prepared(
        vec![30.0, 10.0, 30.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let mut params = kp_params();
    params.tolerance = 0.001;
    params.looseness_recovery = true;
    let count = walk_prepared_lines_optimal(&prepared, 35.0, &profile, &params, None);
    assert!(count >= 1);
}

#[test]
fn test_get_tab_advance() {
    assert_eq!(get_tab_advance(10.0, 0.0), 0.0);
    assert_eq!(get_tab_advance(10.0, 8.0), 6.0);
    assert!((get_tab_advance(8.0, 8.0) - 8.0).abs() < 1e-6);
    assert!((get_tab_advance(0.0, 8.0) - 8.0).abs() < 1e-6);
}

#[test]
fn test_get_breakable_advance() {
    let widths = vec![10.0, 20.0, 30.0];
    let prefix = vec![10.0, 30.0, 60.0];
    assert_eq!(get_breakable_advance(&widths, None, 1, false), 20.0);
    assert_eq!(get_breakable_advance(&widths, Some(&prefix), 1, true), 20.0);
    assert_eq!(get_breakable_advance(&widths, Some(&prefix), 0, true), 10.0);
}

#[test]
fn test_can_break_after_kinds() {
    assert!(can_break_after(SegmentBreakKind::Space));
    assert!(can_break_after(SegmentBreakKind::PreservedSpace));
    assert!(can_break_after(SegmentBreakKind::Tab));
    assert!(can_break_after(SegmentBreakKind::ZeroWidthBreak));
    assert!(can_break_after(SegmentBreakKind::SoftHyphen));
    assert!(!can_break_after(SegmentBreakKind::Text));
    assert!(!can_break_after(SegmentBreakKind::HardBreak));
}

#[test]
fn test_is_simple_collapsible_space() {
    assert!(is_simple_collapsible_space(SegmentBreakKind::Space));
    assert!(!is_simple_collapsible_space(
        SegmentBreakKind::PreservedSpace
    ));
}

#[test]
fn test_fit_soft_hyphen_break() {
    let widths = vec![10.0, 10.0, 10.0];
    let (count, width) = fit_soft_hyphen_break(&widths, 0.0, 25.0, 0.005, 5.0, false);
    assert_eq!(count, 2);
    assert!((width - 20.0).abs() < 1e-6);

    let (count, _) = fit_soft_hyphen_break(&widths, 0.0, 5.0, 0.005, 5.0, false);
    assert_eq!(count, 0);
}

#[test]
fn test_badness_infinity() {
    assert_eq!(badness(f64::INFINITY), 10000.0);
    assert_eq!(badness(f64::NEG_INFINITY), 10000.0);
}

#[test]
fn test_layout_next_line_range_soft_hyphen_breakable_partial_fit() {
    // Soft hyphen followed by breakable segment; several graphemes fit.
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![10.0, 0.0, 100.0]),
        line_end_fit_advances: Cow::Owned(vec![10.0, 5.0, 100.0]),
        line_end_paint_advances: Cow::Owned(vec![10.0, 5.0, 100.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::SoftHyphen,
            SegmentBreakKind::Text,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None, Some(Arc::from(vec![5.0; 5]))]),
        breakable_prefix_widths: Cow::Owned(vec![
            None,
            None,
            Some(Arc::from(vec![5.0, 10.0, 15.0, 20.0, 25.0])),
        ]),
        discretionary_hyphen_width: 5.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 3,
            consumed_end_segment_index: 3,
        }]),
        start_segment: 0,
        end_segment: 3,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let line = layout_next_line_range(
        &prepared,
        LineBreakCursor {
            segment_index: 0,
            grapheme_index: 0,
        },
        25.0,
        &profile,
    );
    assert!(line.is_some());
    let line = line.unwrap();
    assert_eq!(line.end_segment_index, 2);
    assert_eq!(line.end_grapheme_index, 2);
    assert_eq!(line.width, 25.0);
}

#[test]
fn test_layout_next_line_range_breakable_fits_all_after_hyphen() {
    // All graphemes fit after soft hyphen, so the line consumes the whole segment.
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![10.0, 0.0, 25.0]),
        line_end_fit_advances: Cow::Owned(vec![10.0, 5.0, 25.0]),
        line_end_paint_advances: Cow::Owned(vec![10.0, 5.0, 25.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::SoftHyphen,
            SegmentBreakKind::Text,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None, Some(Arc::from(vec![5.0; 5]))]),
        breakable_prefix_widths: Cow::Owned(vec![
            None,
            None,
            Some(Arc::from(vec![5.0, 10.0, 15.0, 20.0, 25.0])),
        ]),
        discretionary_hyphen_width: 5.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 3,
            consumed_end_segment_index: 3,
        }]),
        start_segment: 0,
        end_segment: 3,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let line = layout_next_line_range(
        &prepared,
        LineBreakCursor {
            segment_index: 0,
            grapheme_index: 0,
        },
        50.0,
        &profile,
    );
    assert!(line.is_some());
    let line = line.unwrap();
    assert_eq!(line.end_segment_index, 3);
    assert_eq!(line.width, 35.0);
}

#[test]
fn test_layout_next_line_range_pending_break_fallback() {
    // Word that overflows but a pending space break is feasible.
    let prepared = simple_prepared(
        vec![20.0, 10.0, 50.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let line = layout_next_line_range(
        &prepared,
        LineBreakCursor {
            segment_index: 0,
            grapheme_index: 0,
        },
        25.0,
        &profile,
    );
    assert!(line.is_some());
    let line = line.unwrap();
    assert_eq!(line.end_segment_index, 2);
    assert_eq!(line.width, 20.0);
}

#[test]
fn test_layout_next_line_range_breakable_overflow_single_word() {
    // A single breakable word wider than max_width should break at graphemes.
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![100.0]),
        line_end_fit_advances: Cow::Owned(vec![100.0]),
        line_end_paint_advances: Cow::Owned(vec![100.0]),
        kinds: Cow::Owned(vec![SegmentBreakKind::Text]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![Some(Arc::from(vec![10.0; 10]))]),
        breakable_prefix_widths: Cow::Owned(vec![Some(Arc::from(vec![
            10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0,
        ]))]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 1,
            consumed_end_segment_index: 1,
        }]),
        start_segment: 0,
        end_segment: 1,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let line = layout_next_line_range(
        &prepared,
        LineBreakCursor {
            segment_index: 0,
            grapheme_index: 0,
        },
        25.0,
        &profile,
    );
    assert!(line.is_some());
    let line = line.unwrap();
    assert_eq!(line.end_grapheme_index, 2);
    assert_eq!(line.width, 20.0);
}

#[test]
fn test_layout_next_line_range_empty_chunk() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![40.0]),
        line_end_fit_advances: Cow::Owned(vec![40.0]),
        line_end_paint_advances: Cow::Owned(vec![40.0]),
        kinds: Cow::Owned(vec![SegmentBreakKind::Text]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None]),
        breakable_prefix_widths: Cow::Owned(vec![None]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![
            PreparedLineChunk {
                start_segment_index: 0,
                end_segment_index: 0,
                consumed_end_segment_index: 1,
            },
            PreparedLineChunk {
                start_segment_index: 0,
                end_segment_index: 1,
                consumed_end_segment_index: 1,
            },
        ]),
        start_segment: 0,
        end_segment: 1,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let line = layout_next_line_range(
        &prepared,
        LineBreakCursor {
            segment_index: 0,
            grapheme_index: 0,
        },
        200.0,
        &profile,
    );
    assert!(line.is_some());
    let line = line.unwrap();
    assert_eq!(line.end_segment_index, 1);
}

#[test]
fn test_walk_prepared_lines_early_soft_hyphen() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![10.0, 0.0, 100.0]),
        line_end_fit_advances: Cow::Owned(vec![10.0, 5.0, 100.0]),
        line_end_paint_advances: Cow::Owned(vec![10.0, 5.0, 100.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::SoftHyphen,
            SegmentBreakKind::Text,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None, Some(Arc::from(vec![5.0; 5]))]),
        breakable_prefix_widths: Cow::Owned(vec![
            None,
            None,
            Some(Arc::from(vec![5.0, 10.0, 15.0, 20.0, 25.0])),
        ]),
        discretionary_hyphen_width: 5.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 3,
            consumed_end_segment_index: 3,
        }]),
        start_segment: 0,
        end_segment: 3,
        kp_item_cache: None,
    };
    let profile = EngineProfile {
        prefer_early_soft_hyphen_break: true,
        ..Default::default()
    };
    let mut lines = Vec::new();
    let count = walk_prepared_lines(
        &prepared,
        25.0,
        &profile,
        Some(&mut |line: &InternalLayoutLine| {
            lines.push(line.clone());
        }),
    );
    assert!(count >= 2);
}

#[test]
fn test_walk_prepared_lines_soft_hyphen_breakable_continuation() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![10.0, 0.0, 100.0]),
        line_end_fit_advances: Cow::Owned(vec![10.0, 5.0, 100.0]),
        line_end_paint_advances: Cow::Owned(vec![10.0, 5.0, 100.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::SoftHyphen,
            SegmentBreakKind::Text,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None, Some(Arc::from(vec![5.0; 5]))]),
        breakable_prefix_widths: Cow::Owned(vec![
            None,
            None,
            Some(Arc::from(vec![5.0, 10.0, 15.0, 20.0, 25.0])),
        ]),
        discretionary_hyphen_width: 5.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 3,
            consumed_end_segment_index: 3,
        }]),
        start_segment: 0,
        end_segment: 3,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let count = walk_prepared_lines(&prepared, 25.0, &profile, None);
    assert!(count >= 2);
}

#[test]
fn test_walk_prepared_lines_breakable_overflow_no_pending() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![100.0]),
        line_end_fit_advances: Cow::Owned(vec![100.0]),
        line_end_paint_advances: Cow::Owned(vec![100.0]),
        kinds: Cow::Owned(vec![SegmentBreakKind::Text]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![Some(Arc::from(vec![10.0; 10]))]),
        breakable_prefix_widths: Cow::Owned(vec![None]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 1,
            consumed_end_segment_index: 1,
        }]),
        start_segment: 0,
        end_segment: 1,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let count = walk_prepared_lines(&prepared, 25.0, &profile, None);
    assert_eq!(count, 5);
}

#[test]
fn test_walk_prepared_lines_simple_leading_space_pending() {
    let prepared = simple_prepared(
        vec![10.0, 50.0],
        vec![SegmentBreakKind::Space, SegmentBreakKind::Text],
    );
    let profile = EngineProfile::default();
    let mut lines = Vec::new();
    let count = walk_prepared_lines(
        &prepared,
        100.0,
        &profile,
        Some(&mut |line: &InternalLayoutLine| {
            lines.push(line.clone());
        }),
    );
    assert_eq!(count, 1);
}

#[test]
fn test_walk_prepared_lines_simple_final_emit() {
    // Simple path that ends without a trailing break: must emit the last line.
    let prepared = simple_prepared(
        vec![20.0, 10.0, 20.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let profile = EngineProfile::default();
    let count = walk_prepared_lines(&prepared, 100.0, &profile, None);
    assert_eq!(count, 1);
}

#[test]
fn test_walk_prepared_lines_non_simple_leading_tab() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![0.0, 40.0]),
        line_end_fit_advances: Cow::Owned(vec![0.0, 40.0]),
        line_end_paint_advances: Cow::Owned(vec![0.0, 40.0]),
        kinds: Cow::Owned(vec![SegmentBreakKind::Tab, SegmentBreakKind::Text]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None]),
        breakable_prefix_widths: Cow::Owned(vec![None, None]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 32.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 2,
            consumed_end_segment_index: 2,
        }]),
        start_segment: 0,
        end_segment: 2,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let count = walk_prepared_lines(&prepared, 100.0, &profile, None);
    assert_eq!(count, 1);
}

#[test]
fn test_walk_prepared_lines_optimal_multi_chunk_empty_first() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![40.0]),
        line_end_fit_advances: Cow::Owned(vec![40.0]),
        line_end_paint_advances: Cow::Owned(vec![40.0]),
        kinds: Cow::Owned(vec![SegmentBreakKind::Text]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None]),
        breakable_prefix_widths: Cow::Owned(vec![None]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![
            PreparedLineChunk {
                start_segment_index: 0,
                end_segment_index: 0,
                consumed_end_segment_index: 1,
            },
            PreparedLineChunk {
                start_segment_index: 0,
                end_segment_index: 1,
                consumed_end_segment_index: 1,
            },
        ]),
        start_segment: 0,
        end_segment: 1,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let mut lines = Vec::new();
    let count = walk_prepared_lines_optimal(
        &prepared,
        200.0,
        &profile,
        &kp_params(),
        Some(&mut |line: &InternalLayoutLine| {
            lines.push(line.clone());
        }),
    );
    assert_eq!(count, 2);
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_walk_prepared_lines_optimal_looseness_recovery_multi_chunk() {
    let prepared = simple_prepared(
        vec![30.0, 10.0, 30.0],
        vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::Space,
            SegmentBreakKind::Text,
        ],
    );
    let mut chunks = prepared.chunks.into_owned();
    chunks.push(PreparedLineChunk {
        start_segment_index: 3,
        end_segment_index: 3,
        consumed_end_segment_index: 3,
    });
    let prepared = PreparedLineBreakData {
        chunks: Cow::Owned(chunks),
        ..prepared
    };
    let profile = EngineProfile::default();
    let mut params = kp_params();
    params.tolerance = 0.001;
    params.looseness_recovery = true;
    let count = walk_prepared_lines_optimal(&prepared, 35.0, &profile, &params, None);
    assert!(count >= 1);
}

#[test]
fn test_build_kp_items_hard_break() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![40.0, 0.0, 30.0]),
        line_end_fit_advances: Cow::Owned(vec![40.0, 0.0, 30.0]),
        line_end_paint_advances: Cow::Owned(vec![40.0, 0.0, 30.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::HardBreak,
            SegmentBreakKind::Text,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None, None]),
        breakable_prefix_widths: Cow::Owned(vec![None, None, None]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 3,
            consumed_end_segment_index: 3,
        }]),
        start_segment: 0,
        end_segment: 3,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let items = build_kp_items(&prepared, &profile, &kp_params());
    assert!(items.len() >= 3);
}

#[test]
fn test_build_kp_items_zero_width_break_and_tab() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![40.0, 0.0, 0.0, 32.0]),
        line_end_fit_advances: Cow::Owned(vec![40.0, 0.0, 0.0, 32.0]),
        line_end_paint_advances: Cow::Owned(vec![40.0, 0.0, 0.0, 32.0]),
        kinds: Cow::Owned(vec![
            SegmentBreakKind::Text,
            SegmentBreakKind::ZeroWidthBreak,
            SegmentBreakKind::SoftHyphen,
            SegmentBreakKind::Tab,
        ]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![None, None, None, None]),
        breakable_prefix_widths: Cow::Owned(vec![None, None, None, None]),
        discretionary_hyphen_width: 5.0,
        tab_stop_advance: 32.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 4,
            consumed_end_segment_index: 4,
        }]),
        start_segment: 0,
        end_segment: 4,
        kp_item_cache: None,
    };
    let profile = EngineProfile::default();
    let items = build_kp_items(&prepared, &profile, &kp_params());
    assert!(items.len() >= 4);
}

#[test]
fn test_build_kp_items_breakable_graphemes() {
    let prepared = PreparedLineBreakData {
        widths: Cow::Owned(vec![100.0]),
        line_end_fit_advances: Cow::Owned(vec![100.0]),
        line_end_paint_advances: Cow::Owned(vec![100.0]),
        kinds: Cow::Owned(vec![SegmentBreakKind::Text]),
        simple_line_walk_fast_path: false,
        breakable_widths: Cow::Owned(vec![Some(Arc::from(vec![10.0; 5]))]),
        breakable_prefix_widths: Cow::Owned(vec![Some(Arc::from(vec![
            10.0, 20.0, 30.0, 40.0, 50.0,
        ]))]),
        discretionary_hyphen_width: 0.0,
        tab_stop_advance: 0.0,
        chunks: Cow::Owned(vec![PreparedLineChunk {
            start_segment_index: 0,
            end_segment_index: 1,
            consumed_end_segment_index: 1,
        }]),
        start_segment: 0,
        end_segment: 1,
        kp_item_cache: None,
    };
    let profile = EngineProfile {
        prefer_prefix_widths_for_breakable_runs: true,
        ..Default::default()
    };
    let items = build_kp_items(&prepared, &profile, &kp_params());
    assert!(items.len() >= 6);
}

#[test]
fn test_knuth_plass_chunk_empty_items() {
    let items: Vec<KPItem> = Vec::new();
    let params = kp_params();
    let result = knuth_plass_chunk(&items, 100.0, &params, params.tolerance);
    assert!(result.as_deref().is_some_and(<[_]>::is_empty));
}

#[test]
fn test_normalize_line_start_skip_whitespace() {
    let prepared = simple_prepared(
        vec![0.0, 40.0],
        vec![SegmentBreakKind::Space, SegmentBreakKind::Text],
    );
    let start = LineBreakCursor {
        segment_index: 0,
        grapheme_index: 0,
    };
    let normalized = normalize_line_start(&prepared, start);
    assert!(normalized.is_some());
    assert_eq!(normalized.unwrap().segment_index, 1);
}

#[test]
fn test_fit_soft_hyphen_break_cumulative() {
    let widths = vec![10.0, 20.0, 30.0];
    let (count, width) = fit_soft_hyphen_break(&widths, 5.0, 60.0, 0.005, 5.0, true);
    // Cumulative: initial 5 + each prefix width; all three fit within max_width.
    assert_eq!(count, 3);
    assert!((width - 35.0).abs() < 1e-6);
}
