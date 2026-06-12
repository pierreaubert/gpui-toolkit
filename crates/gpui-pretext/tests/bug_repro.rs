use gpui_pretext::*;

struct FixedMeasure;
impl TextMeasure for FixedMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        text.chars().count() as f64 * 10.0
    }
}

#[test]
fn test_cjk_optimal_falls_back_to_greedy() {
    let measure = FixedMeasure;
    let profile = EngineProfile::default();
    let options = PrepareOptions::default();
    let kp = KnuthPlassParams {
        looseness_recovery: false,
        ..Default::default()
    };

    // CJK text should be breakable in optimal mode, not fallback to greedy
    let prepared = prepare_with_segments("你好世界", &measure, &profile, &options);
    let greedy = layout_with_lines(&prepared, 15.0, 20.0, &profile);
    let optimal = layout_with_lines_optimal(&prepared, 15.0, 20.0, &profile, &kp);

    println!(
        "greedy lines: {:?}",
        greedy
            .lines
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
    );
    println!(
        "optimal lines: {:?}",
        optimal
            .lines
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
    );

    // With proper CJK breakpoints, optimal should also produce 4 lines
    // (or at least not fall back to greedy in a way that changes results)
    assert_eq!(
        greedy.line_count, optimal.line_count,
        "optimal should handle CJK without fallback"
    );
}

#[test]
fn test_prewrap_rrn() {
    let measure = FixedMeasure;
    let profile = EngineProfile::default();
    let options = PrepareOptions {
        white_space: WhiteSpaceMode::PreWrap,
    };

    let prepared = prepare_with_segments("a\r\r\nb", &measure, &profile, &options);
    let result = layout_with_lines(&prepared, 100.0, 20.0, &profile);

    println!(
        "\\r\\r\\n lines: {:?}",
        result
            .lines
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
    );

    // \r\r\n should produce exactly 2 lines, not 3
    assert_eq!(result.line_count, 2, "\\r\\r\\n should produce 2 lines");
}
