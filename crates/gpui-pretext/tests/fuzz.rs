use gpui_pretext::*;

struct FixedMeasure;
impl TextMeasure for FixedMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        text.chars().count() as f64 * 10.0
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
