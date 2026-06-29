use super::default::default_options;
use super::default::default_profile;
use super::fake_measure::FakeMeasure;
use super::misc::kp_params;
use super::misc::pre_wrap_options;
#[doc = " Integration tests ported from the TypeScript pretext test suite."]
#[doc = ""]
#[doc = " Uses a deterministic \"fake canvas\" measure that mimics the original test's"]
#[doc = " character-width model:"]
#[doc = " - Space: 0.33× font_size (compressible)"]
#[doc = " - Tab: 1.32× font_size"]
#[doc = " - Punctuation (.,!?:;()[]{}\"'): 0.4× font_size"]
#[doc = " - Regular characters: 0.6× font_size"]
use gpui_pretext::{
    LayoutCursor, LineBreakStrategy, TextMeasure, layout, layout_next_line, layout_optimal,
    layout_with_lines, layout_with_lines_optimal, layout_with_strategy, prepare,
    prepare_with_segments, walk_line_ranges, walk_line_ranges_optimal,
};

#[test]
fn test_empty_string() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare("", &m, &p, &o);
    let result = layout(&prepared, 300.0, 20.0, &p);
    assert_eq!(result.line_count, 0);
    assert_eq!(result.height, 0.0);
}

#[test]
fn test_single_word_fits() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    // "hello" = 5 * 0.6 * 16 = 48px
    let prepared = prepare("hello", &m, &p, &o);
    let result = layout(&prepared, 300.0, 20.0, &p);
    assert_eq!(result.line_count, 1);
    assert_eq!(result.height, 20.0);
}

#[test]
fn test_two_words_one_line() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    // "hello world" = 48 + 5.28 + 48 = 101.28px
    let prepared = prepare("hello world", &m, &p, &o);
    let result = layout(&prepared, 200.0, 20.0, &p);
    assert_eq!(result.line_count, 1);
}

#[test]
fn test_two_words_wrap() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    // "hello world" = 48 + 5.28 + 48 = 101.28px
    let prepared = prepare("hello world", &m, &p, &o);
    let result = layout(&prepared, 80.0, 20.0, &p);
    assert_eq!(result.line_count, 2);
}

#[test]
fn test_three_words_wrap() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    // "the quick fox" — each word ~30-48px, space ~5.28px
    let prepared = prepare("the quick fox", &m, &p, &o);
    let result = layout(&prepared, 65.0, 20.0, &p);
    // "the quick" = ~82px > 65, so wraps
    assert!(result.line_count >= 2);
}

#[test]
fn test_whitespace_collapsing_normal() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    // Multiple spaces should collapse to one
    let p1 = prepare("hello  world", &m, &p, &o);
    let p2 = prepare("hello world", &m, &p, &o);
    let r1 = layout(&p1, 300.0, 20.0, &p);
    let r2 = layout(&p2, 300.0, 20.0, &p);
    assert_eq!(r1.line_count, r2.line_count);
}

#[test]
fn test_leading_trailing_whitespace_stripped() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let p1 = prepare("  hello  ", &m, &p, &o);
    let p2 = prepare("hello", &m, &p, &o);
    let r1 = layout(&p1, 300.0, 20.0, &p);
    let r2 = layout(&p2, 300.0, 20.0, &p);
    assert_eq!(r1.line_count, r2.line_count);
}

#[test]
fn test_pre_wrap_preserves_spaces() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = pre_wrap_options();
    let prepared = prepare_with_segments("a  b", &m, &p, &o);
    let result = layout_with_lines(&prepared, 300.0, 20.0, &p);
    assert_eq!(result.line_count, 1);
    // Text should preserve the double space
    assert!(result.lines[0].text.contains("  "));
}

#[test]
fn test_pre_wrap_hard_breaks() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = pre_wrap_options();
    let prepared = prepare("a\nb\nc", &m, &p, &o);
    let result = layout(&prepared, 300.0, 20.0, &p);
    assert_eq!(result.line_count, 3);
}

#[test]
fn test_layout_with_lines_content() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare_with_segments("hello world", &m, &p, &o);
    let result = layout_with_lines(&prepared, 60.0, 20.0, &p);
    assert_eq!(result.line_count, 2);
    assert_eq!(result.lines[0].text, "hello ");
    assert_eq!(result.lines[1].text, "world");
}

#[test]
fn test_layout_line_widths_positive() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare_with_segments("the quick brown fox", &m, &p, &o);
    let result = layout_with_lines(&prepared, 100.0, 20.0, &p);
    for line in &result.lines {
        assert!(line.width > 0.0, "line should have positive width");
    }
}

#[test]
fn test_layout_next_line_iterates_all() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare_with_segments("one two three four five", &m, &p, &o);

    let mut cursor = LayoutCursor {
        segment_index: 0,
        grapheme_index: 0,
    };
    let mut lines = Vec::new();
    while let Some(line) = layout_next_line(&prepared, cursor, 80.0, &p) {
        cursor = line.end;
        lines.push(line);
    }
    assert!(lines.len() >= 2);
    // All text should be covered
    let all_text: String = lines.iter().map(|l| l.text.as_ref()).collect();
    assert!(all_text.contains("one"));
    assert!(all_text.contains("five"));
}

#[test]
fn test_walk_line_ranges_count() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare_with_segments("hello world foo bar", &m, &p, &o);
    let mut count = 0;
    let total = walk_line_ranges(&prepared, 80.0, &p, |_range| {
        count += 1;
    });
    assert_eq!(total, count);
    assert!(count >= 2);
}

#[test]
fn test_cjk_per_character_breaking() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    // Each CJK character is ~0.6 * 16 = 9.6px
    // With width 20px, should fit ~2 chars per line
    let prepared = prepare("你好世界测试", &m, &p, &o);
    let result = layout(&prepared, 20.0, 20.0, &p);
    assert!(result.line_count >= 3, "CJK should break per-character");
}

#[test]
fn test_punctuation_attaches_to_word() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare_with_segments("hello, world.", &m, &p, &o);
    let result = layout_with_lines(&prepared, 300.0, 20.0, &p);
    assert_eq!(result.line_count, 1);
    // Comma should be attached to "hello"
    let text = &result.lines[0].text;
    assert!(text.contains("hello,"));
    assert!(text.contains("world."));
}

#[test]
fn test_multiple_layouts_same_prepared() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare("the quick brown fox jumps over the lazy dog", &m, &p, &o);

    let r1 = layout(&prepared, 500.0, 20.0, &p);
    let r2 = layout(&prepared, 200.0, 20.0, &p);
    let r3 = layout(&prepared, 100.0, 20.0, &p);
    let r4 = layout(&prepared, 50.0, 20.0, &p);

    assert!(r1.line_count <= r2.line_count);
    assert!(r2.line_count <= r3.line_count);
    assert!(r3.line_count <= r4.line_count);
}

#[test]
fn test_layout_count_matches_walk() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare_with_segments("the quick brown fox jumps over", &m, &p, &o);

    for width in [50.0, 100.0, 150.0, 200.0, 300.0] {
        let fast = layout(
            &prepare("the quick brown fox jumps over", &m, &p, &o),
            width,
            20.0,
            &p,
        );
        let rich = layout_with_lines(&prepared, width, 20.0, &p);
        assert_eq!(
            fast.line_count, rich.line_count,
            "mismatch at width {width}"
        );
    }
}

#[test]
fn test_single_character() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare("a", &m, &p, &o);
    let result = layout(&prepared, 300.0, 20.0, &p);
    assert_eq!(result.line_count, 1);
}

#[test]
fn test_only_spaces_normal() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    // In normal mode, spaces-only text normalizes to empty
    let prepared = prepare("   ", &m, &p, &o);
    let result = layout(&prepared, 300.0, 20.0, &p);
    assert_eq!(result.line_count, 0);
}

#[test]
fn test_very_narrow_width() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    // Width smaller than any word — every word gets its own line
    let prepared = prepare("a b c d", &m, &p, &o);
    let result = layout(&prepared, 1.0, 20.0, &p);
    assert_eq!(result.line_count, 4);
}

#[test]
fn test_exact_fit() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    // "ab" = 2 * 0.6 * 16 = 19.2px
    let prepared = prepare("ab", &m, &p, &o);
    let result = layout(&prepared, 19.2, 20.0, &p);
    assert_eq!(result.line_count, 1);
}

#[test]
fn test_pre_wrap_count_matches_lines() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = pre_wrap_options();
    let text = "first line\nsecond line\nthird line";
    let fast = layout(&prepare(text, &m, &p, &o), 300.0, 20.0, &p);
    let prepared_rich = prepare_with_segments(text, &m, &p, &o);
    let rich = layout_with_lines(&prepared_rich, 300.0, 20.0, &p);
    assert_eq!(fast.line_count, rich.line_count);
    assert_eq!(rich.line_count, 3);
}

#[test]
fn test_pre_wrap_wrapping_plus_hard_breaks() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = pre_wrap_options();
    // Line 1 wraps due to width, line 2 is a hard break
    let text = "hello world\nfoo";
    let prepared_rich = prepare_with_segments(text, &m, &p, &o);
    let result = layout_with_lines(&prepared_rich, 60.0, 20.0, &p);
    // "hello " wraps, then "world\n" is a hard break, then "foo"
    assert!(result.line_count >= 3);
}

#[test]
fn test_grapheme_widths_cache_correctness() {
    // Verify that get_grapheme_widths works even without pre-populating cache
    use gpui_pretext::measurement::MeasureCache;

    struct CharMeasure;
    impl TextMeasure for CharMeasure {
        fn measure_width(&self, text: &str) -> f64 {
            text.chars().count() as f64 * 8.0
        }
    }

    let measure = CharMeasure;
    let mut cache = MeasureCache::new();

    // Directly call get_grapheme_widths without get_width first
    let widths = cache.get_grapheme_widths("hello", &measure);
    assert!(widths.is_some());
    assert_eq!(widths.as_ref().unwrap().len(), 5);

    // Verify cached on second call
    let widths2 = cache.get_grapheme_widths("hello", &measure);
    assert_eq!(widths, widths2);
}

#[test]
fn test_optimal_empty_string() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare("", &m, &p, &o);
    let result = layout_optimal(&prepared, 300.0, 20.0, &p, &kp_params());
    assert_eq!(result.line_count, 0);
    assert_eq!(result.height, 0.0);
}

#[test]
fn test_optimal_single_word_fits() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare("hello", &m, &p, &o);
    let result = layout_optimal(&prepared, 300.0, 20.0, &p, &kp_params());
    assert_eq!(result.line_count, 1);
    assert_eq!(result.height, 20.0);
}

#[test]
fn test_optimal_two_words_one_line() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare("hello world", &m, &p, &o);
    let result = layout_optimal(&prepared, 200.0, 20.0, &p, &kp_params());
    assert_eq!(result.line_count, 1);
}

#[test]
fn test_optimal_two_words_wrap() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare("hello world", &m, &p, &o);
    let result = layout_optimal(&prepared, 80.0, 20.0, &p, &kp_params());
    assert_eq!(result.line_count, 2);
}

#[test]
fn test_optimal_layout_with_lines_content() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare_with_segments("hello world", &m, &p, &o);
    let result = layout_with_lines_optimal(&prepared, 60.0, 20.0, &p, &kp_params());
    assert_eq!(result.line_count, 2);
    assert_eq!(result.lines.len(), 2);
    // First line should contain "hello" (with or without trailing space).
    assert!(result.lines[0].text.contains("hello"));
    // Second line should contain "world".
    assert!(result.lines[1].text.contains("world"));
}

#[test]
fn test_optimal_walk_line_ranges() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare_with_segments("hello world foo bar", &m, &p, &o);
    let mut count = 0;
    let total = walk_line_ranges_optimal(&prepared, 80.0, &p, &kp_params(), |_range| {
        count += 1;
    });
    assert_eq!(total, count);
    assert!(count >= 2);
}

#[test]
fn test_optimal_count_matches_lines() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();

    let text = "the quick brown fox jumps over the lazy dog";
    for width in [50.0, 100.0, 150.0, 200.0, 300.0] {
        let fast = layout_optimal(&prepare(text, &m, &p, &o), width, 20.0, &p, &kp_params());
        let prepared_rich = prepare_with_segments(text, &m, &p, &o);
        let rich = layout_with_lines_optimal(&prepared_rich, width, 20.0, &p, &kp_params());
        assert_eq!(
            fast.line_count, rich.line_count,
            "optimal count mismatch at width {width}"
        );
    }
}

#[test]
fn test_optimal_monotonic_line_counts() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();

    let text = "the quick brown fox jumps over the lazy dog and more words follow after";
    let prepared = prepare(text, &m, &p, &o);

    let widths = [500.0, 300.0, 200.0, 150.0, 100.0, 80.0, 50.0];
    let mut prev_count = 0;
    for &width in &widths {
        let result = layout_optimal(&prepared, width, 20.0, &p, &kp_params());
        assert!(
            result.line_count >= prev_count,
            "line count decreased at width {width}: {} < {prev_count}",
            result.line_count
        );
        prev_count = result.line_count;
    }
}

#[test]
fn test_optimal_line_widths_positive() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare_with_segments("the quick brown fox", &m, &p, &o);
    let result = layout_with_lines_optimal(&prepared, 100.0, 20.0, &p, &kp_params());
    for line in &result.lines {
        assert!(line.width > 0.0, "line should have positive width");
    }
}

#[test]
fn test_strategy_dispatch_greedy() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare("hello world", &m, &p, &o);

    let greedy = layout(&prepared, 80.0, 20.0, &p);
    let via_strategy = layout_with_strategy(
        &prepared,
        80.0,
        20.0,
        &p,
        LineBreakStrategy::Greedy,
        &kp_params(),
    );

    assert_eq!(greedy.line_count, via_strategy.line_count);
    assert_eq!(greedy.height, via_strategy.height);
}

#[test]
fn test_strategy_dispatch_optimal() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare("hello world", &m, &p, &o);

    let optimal = layout_optimal(&prepared, 80.0, 20.0, &p, &kp_params());
    let via_strategy = layout_with_strategy(
        &prepared,
        80.0,
        20.0,
        &p,
        LineBreakStrategy::Optimal,
        &kp_params(),
    );

    assert_eq!(optimal.line_count, via_strategy.line_count);
    assert_eq!(optimal.height, via_strategy.height);
}

#[test]
fn test_optimal_paragraph_quality() {
    // Test the classic Knuth-Plass advantage: a paragraph where greedy
    // produces a very uneven last line but optimal distributes better.
    //
    // 7 words of varying widths.
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();

    let text = "aaa bbb ccc ddd eee fff ggg";
    let prepared_rich = prepare_with_segments(text, &m, &p, &o);

    let greedy = layout_with_lines(&prepared_rich, 70.0, 20.0, &p);
    let optimal = layout_with_lines_optimal(&prepared_rich, 70.0, 20.0, &p, &kp_params());

    // Both should produce valid results.
    assert!(greedy.line_count >= 2);
    assert!(optimal.line_count >= 2);

    // Optimal should have lines — verify text coverage.
    let greedy_text: String = greedy.lines.iter().map(|l| l.text.as_ref()).collect();
    let optimal_text: String = optimal.lines.iter().map(|l| l.text.as_ref()).collect();
    assert!(greedy_text.contains("aaa"));
    assert!(optimal_text.contains("aaa"));
    assert!(greedy_text.contains("ggg"));
    assert!(optimal_text.contains("ggg"));
}

#[test]
fn test_optimal_very_narrow_width() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();
    let prepared = prepare("a b c d", &m, &p, &o);
    let result = layout_optimal(&prepared, 1.0, 20.0, &p, &kp_params());
    // Each letter on its own line.
    assert_eq!(result.line_count, 4);
}

#[test]
fn test_optimal_pre_wrap_hard_breaks() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = pre_wrap_options();
    let prepared = prepare("a\nb\nc", &m, &p, &o);
    let result = layout_optimal(&prepared, 300.0, 20.0, &p, &kp_params());
    assert_eq!(result.line_count, 3);
}

#[test]
fn test_optimal_custom_params() {
    let m = FakeMeasure::new(16.0);
    let p = default_profile();
    let o = default_options();

    let text = "the quick brown fox jumps over the lazy dog";
    let prepared = prepare(text, &m, &p, &o);

    // With very high hyphen penalty, should avoid hyphenation.
    let mut params = kp_params();
    params.hyphen_penalty = 10000.0;
    let result = layout_optimal(&prepared, 100.0, 20.0, &p, &params);
    assert!(result.line_count >= 2);

    // With very loose tolerance, might produce different results.
    params.tolerance = 10.0;
    let loose = layout_optimal(&prepared, 100.0, 20.0, &p, &params);
    assert!(loose.line_count >= 1);
}
