use gpui_pretext::*;

struct FixedMeasure;
impl TextMeasure for FixedMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        text.chars().count() as f64 * 10.0
    }
}

/// Adversarial corpus targeting the untested analysis core from review:
/// `measure_analysis` (via prepare), breakpoint lists (via optimal layout),
/// `build_merged_segmentation` (via `analyze_text`), and bidi levels.
fn adversarial_inputs() -> Vec<String> {
    let mut inputs = vec![
        // Bidi: Hebrew + Arabic + Latin + numbers, explicit embeddings/isolates.
        "\u{5d0}\u{5d1}\u{5d2} hello \u{627}\u{628}\u{629} 123".to_string(),
        "\u{202a}hello\u{202c} \u{202b}\u{5d0}\u{202c}".to_string(),
        "\u{2066}user \u{2069} \u{2067}\u{627}\u{2069}".to_string(),
        "\u{200e}ltr\u{200f}rtl".to_string(),
        // Emoji: ZWJ sequences, flags, skin tones, variation selectors.
        "\u{1f469}\u{200d}\u{1f4bb}\u{1f1fa}\u{1f1f8}a\u{1f3fd}\u{fe0f}".to_string(),
        "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467} family".to_string(),
        // Combining marks and Myanmar medial glue.
        "e\u{301}a\u{302}\u{309} \u{1000}\u{103c}\u{1000}".to_string(),
        // CJK kinsoku boundaries, closing-quote carry, fullwidth spaces.
        "\u{300c}\u{6f22}\u{300d}\u{6f22}\u{3001}\u{6f22}\u{3002}\u{6f22}".to_string(),
        "\u{3000}\u{6f22}\u{5b57} \u{3000}".to_string(),
        // Soft hyphens, zero-width spaces/breaks, word joiners.
        "su\u{ad}per\u{ad}cali \u{200b}fragi\u{2060}listic".to_string(),
        // Line/paragraph separators and stray carriage returns.
        "a\u{2028}b\u{2029}c\r\u{2028}\u{2029}".to_string(),
        // Tabs mixed with CJK and soft hyphens.
        "\u{6f22}\t\u{5b57}\u{ad}\u{6e2c}".to_string(),
        // URL/numeric/punctuation merge paths with CJK + RTL nearby.
        "https://example.com/\u{6f22}?a=1&b=2 \u{627} 12:34 +1-800-555-0199".to_string(),
        "(he) [ar] {cjk}".to_string(),
    ];
    // Long unbroken word forces the breakable (overflow-wrap) path.
    inputs.push("a".repeat(500));
    // Long CJK run forces per-grapheme segment breaking.
    inputs.push("\u{6f22}".repeat(300));
    // Alternating spaces/tabs/newlines stress chunk compilation.
    inputs.push("  \t \n \r\n \u{3000} ".repeat(50));
    // Repeated bidi direction flips stress level runs.
    inputs.push("\u{5d0}a\u{627}b".repeat(100));
    inputs
}

#[test]
fn fuzz_analysis_bidi_breakpoints() {
    use gpui_pretext::bidi::compute_segment_levels;

    let profile = EngineProfile::default();
    for text in adversarial_inputs() {
        // Segmentation must not panic; starts must be ascending valid
        // boundaries inside the normalized text.
        let analysis_profile = gpui_pretext::analysis::AnalysisProfile {
            carry_cjk_after_closing_quote: profile.carry_cjk_after_closing_quote,
        };
        let analysis =
            gpui_pretext::analysis::analyze_text(&text, &analysis_profile, WhiteSpaceMode::Normal);
        let mut prev = 0usize;
        for &start in &analysis.starts {
            assert!(start >= prev, "segment starts not ascending in {text:?}");
            assert!(
                analysis.normalized.is_char_boundary(start),
                "segment start not a char boundary in {text:?}"
            );
            prev = start;
        }
        assert_eq!(analysis.starts.len(), analysis.len());
        assert_eq!(analysis.texts.len(), analysis.len());
        assert_eq!(analysis.kinds.len(), analysis.len());

        // Bidi levels: None (uniform LTR) or exactly one level per segment.
        let levels = compute_segment_levels(&analysis.normalized, &analysis.starts);
        if let Some(levels) = levels {
            assert_eq!(
                levels.len(),
                analysis.starts.len(),
                "bidi level count mismatch in {text:?}"
            );
        }

        // Pre-wrap must accept the same input without panicking.
        let _ = gpui_pretext::analysis::analyze_text(&text, &analysis_profile, WhiteSpaceMode::PreWrap);
    }
}

#[test]
fn fuzz_adversarial_layout() {
    let measure = FixedMeasure;
    let profile = EngineProfile::default();
    let options = PrepareOptions::default();
    let kp = KnuthPlassParams::default();

    for text in adversarial_inputs() {
        for width in [1.0, 10.0, 100.0] {
            let prepared = prepare(&text, &measure, &profile, &options);
            let greedy = layout(&prepared, width, 20.0, &profile);
            let optimal = layout_optimal(&prepared, width, 20.0, &profile, &kp);
            if !text.is_empty() {
                assert!(greedy.line_count > 0, "no lines for {text:?}");
                assert!(optimal.line_count > 0, "no optimal lines for {text:?}");
            }
            // Determinism: preparing twice yields identical counts.
            let again = prepare(&text, &measure, &profile, &options);
            assert_eq!(
                layout(&again, width, 20.0, &profile).line_count,
                greedy.line_count,
                "nondeterministic layout for {text:?}"
            );

            // Cursor walk must terminate and cover every line once.
            let with_segments = prepare_with_segments(&text, &measure, &profile, &options);
            let mut cursor = LayoutCursor {
                segment_index: 0,
                grapheme_index: 0,
            };
            let mut walked = 0;
            let mut guard = 0;
            while let Some(line) = layout_next_line(&with_segments, cursor, width, &profile) {
                cursor = line.end;
                walked += 1;
                guard += 1;
                assert!(guard < 100_000, "cursor walk diverged for {text:?}");
            }
            assert_eq!(walked, greedy.line_count, "walk missed lines for {text:?}");
        }
    }
}

#[test]
fn fuzz_all_inputs() {
    let measure = FixedMeasure;
    let profile = EngineProfile::default();
    let options = PrepareOptions::default();
    let kp = KnuthPlassParams::default();
    let prewrap = PrepareOptions {
        white_space: WhiteSpaceMode::PreWrap,
    };

    let inputs = vec![
        "",
        "a",
        "ab",
        "abc",
        "hello world",
        "a b c d e",
        "你好世界",
        "a\tb",
        "a\n\nb",
        "a\r\nb",
        "a\r\r\nb",
        "a \r\n b",
        "   ",
        "a-123-456-b",
        "https://example.com/path?query=1",
        "12:34:56",
        " (",
        "a.",
        "...",
        "\"hello\"",
    ];

    for text in &inputs {
        let prepared = prepare(text, &measure, &profile, &options);
        let _ = layout(&prepared, 1.0, 20.0, &profile);
        let _ = layout(&prepared, 10.0, 20.0, &profile);
        let _ = layout(&prepared, 100.0, 20.0, &profile);
        let _ = layout_optimal(&prepared, 10.0, 20.0, &profile, &kp);

        let prepared = prepare_with_segments(text, &measure, &profile, &options);
        let _ = layout_with_lines(&prepared, 10.0, 20.0, &profile);
        let _ = layout_with_lines_optimal(&prepared, 10.0, 20.0, &profile, &kp);

        let mut cursor = LayoutCursor {
            segment_index: 0,
            grapheme_index: 0,
        };
        while let Some(line) = layout_next_line(&prepared, cursor, 10.0, &profile) {
            cursor = line.end;
        }

        let prepared = prepare_with_segments(text, &measure, &profile, &prewrap);
        let _ = layout_with_lines(&prepared, 10.0, 20.0, &profile);
    }
}
