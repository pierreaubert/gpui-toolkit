#![no_main]

use gpui_pretext::{
    EngineProfile, PrepareOptions, TextMeasure, layout, layout_optimal, layout_with_lines,
    layout_with_lines_optimal, prepare, prepare_with_segments,
};
use libfuzzer_sys::fuzz_target;

struct FixedMeasure;

impl TextMeasure for FixedMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        text.chars().count() as f64 * 10.0
    }
}

fuzz_target!(|data: &[u8]| {
    // Keep a fuzz case bounded so pathological input cannot turn this target
    // into an unbounded allocation or line-break benchmark.
    let bounded = &data[..data.len().min(1024)];
    let text = String::from_utf8_lossy(bounded);
    let measure = FixedMeasure;
    let profile = EngineProfile::default();
    let options = PrepareOptions::default();
    let prepared = prepare(&text, &measure, &profile, &options);
    let _ = layout(&prepared, 10.0, 20.0, &profile);
    let _ = layout_optimal(&prepared, 10.0, 20.0, &profile, &Default::default());

    let segmented = prepare_with_segments(&text, &measure, &profile, &options);
    let _ = layout_with_lines(&segmented, 10.0, 20.0, &profile);
    let _ = layout_with_lines_optimal(&segmented, 10.0, 20.0, &profile, &Default::default());
});
