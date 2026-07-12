use gpui_pretext::{
    EngineProfile, PrepareOptions, TextMeasure, layout_with_lines, prepare_with_segments,
};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
struct Corpus {
    schema_version: u32,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    text: String,
    direction: String,
    ime_commit: String,
    dynamic_type_scale: f64,
}

struct GraphemeMeasure;

impl TextMeasure for GraphemeMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        unicode_segmentation::UnicodeSegmentation::graphemes(text, true).count() as f64 * 8.0
    }
}

#[test]
fn shared_unicode_ime_rtl_dynamic_type_corpus_lays_out_safely() {
    let corpus: Corpus =
        serde_json::from_str(include_str!("../../../qa/corpora/text-input-v1.json")).unwrap();
    assert_eq!(corpus.schema_version, 1);
    assert!(corpus.cases.len() >= 8);

    let mut ids = BTreeSet::new();
    let profile = EngineProfile::default();
    let options = PrepareOptions::default();
    for case in corpus.cases {
        assert!(
            ids.insert(case.id.clone()),
            "duplicate corpus id {}",
            case.id
        );
        assert!(["ltr", "rtl", "mixed"].contains(&case.direction.as_str()));
        assert!(!case.ime_commit.is_empty());
        assert!((1.0..=2.0).contains(&case.dynamic_type_scale));

        let prepared = prepare_with_segments(&case.text, &GraphemeMeasure, &profile, &options);
        for width in [80.0, 240.0, 640.0] {
            let line_height = 20.0 * case.dynamic_type_scale;
            let result = layout_with_lines(&prepared, width, line_height, &profile);
            assert!(
                result.height.is_finite(),
                "{} produced invalid height",
                case.id
            );
            assert!(result.line_count > 0, "{} produced no lines", case.id);
            assert!(result.lines.iter().all(|line| line.width.is_finite()));
        }
    }
}
