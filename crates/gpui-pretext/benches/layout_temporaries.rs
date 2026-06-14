use criterion::{Criterion, black_box, criterion_group, criterion_main};
use gpui_pretext::{
    EngineProfile, KnuthPlassParams, PrepareOptions, TextMeasure, layout_optimal, prepare,
};

struct FixedWidthMeasure {
    char_width: f64,
}

impl TextMeasure for FixedWidthMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        text.chars().count() as f64 * self.char_width
    }
}

fn bench_grapheme_prefix_widths(c: &mut Criterion) {
    let measure = FixedWidthMeasure { char_width: 10.0 };
    let mut cache = gpui_pretext::measurement::MeasureCache::new();
    let text = "The quick brown fox jumps over the lazy dog. The five boxing wizards jump quickly.";

    c.bench_function("measurement/get_grapheme_prefix_widths", |b| {
        b.iter(|| {
            let result = cache.get_grapheme_prefix_widths(black_box(text), &measure);
            black_box(&result);
        });
    });
}

fn bench_layout_optimal(c: &mut Criterion) {
    let measure = FixedWidthMeasure { char_width: 10.0 };
    let profile = EngineProfile::default();
    let options = PrepareOptions::default();
    let text = "The quick brown fox jumps over the lazy dog. The five boxing wizards jump quickly.";
    let prepared = prepare(text, &measure, &profile, &options);
    let params = KnuthPlassParams::default();

    c.bench_function("layout/layout_optimal", |b| {
        b.iter(|| {
            let result = layout_optimal(black_box(&prepared), 200.0, 20.0, &profile, &params);
            black_box(&result);
        });
    });
}

criterion_group!(benches, bench_grapheme_prefix_widths, bench_layout_optimal);
criterion_main!(benches);
