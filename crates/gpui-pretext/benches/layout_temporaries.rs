use criterion::{Criterion, criterion_group, criterion_main};
use gpui_pretext::measurement::MeasureCache;
use gpui_pretext::{
    EngineProfile, PrepareOptions, TextMeasure, layout_optimal, layout_with_lines, prepare,
    prepare_with_segments,
};
use std::hint::black_box;

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
    let mut cache = MeasureCache::new();
    let text = "The quick brown fox jumps over the lazy dog. The five boxing wizards jump quickly.";

    c.bench_function("measurement/get_grapheme_prefix_widths", |b| {
        b.iter(|| {
            let result = cache.get_grapheme_prefix_widths(black_box(text), &measure);
            black_box(&result);
        });
    });
}

fn bench_measure_cache_hit(c: &mut Criterion) {
    let measure = FixedWidthMeasure { char_width: 10.0 };
    let mut cache = MeasureCache::new();
    let text = "some moderately long text";

    // Warm the cache so the benchmarked loop only hits cache hits.
    let _ = cache.get_width(text, &measure);

    c.bench_function("measurement/get_width_cache_hit", |b| {
        b.iter(|| {
            let result = cache.get_width(black_box(text), &measure);
            black_box(result);
        });
    });
}

fn bench_grapheme_widths_cache_hit(c: &mut Criterion) {
    let measure = FixedWidthMeasure { char_width: 10.0 };
    let mut cache = MeasureCache::new();
    let text = "alphabet";

    // Warm the cache so the benchmarked loop only hits cache hits.
    let _ = cache.get_grapheme_widths(text, &measure);

    c.bench_function("measurement/get_grapheme_widths_cache_hit", |b| {
        b.iter(|| {
            let result = cache.get_grapheme_widths(black_box(text), &measure);
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
    let params = gpui_pretext::KnuthPlassParams::default();

    c.bench_function("layout/layout_optimal", |b| {
        b.iter(|| {
            let result = layout_optimal(black_box(&prepared), 200.0, 20.0, &profile, &params);
            black_box(&result);
        });
    });
}

fn bench_layout_with_lines(c: &mut Criterion) {
    let measure = FixedWidthMeasure { char_width: 10.0 };
    let profile = EngineProfile::default();
    let options = PrepareOptions::default();
    let text = "The quick brown fox jumps over the lazy dog. The five boxing wizards jump quickly.";
    let prepared = prepare_with_segments(text, &measure, &profile, &options);
    let line_height = 20.0;

    c.bench_function("layout/layout_with_lines", |b| {
        b.iter(|| {
            let result = layout_with_lines(black_box(&prepared), f64::MAX, line_height, &profile);
            black_box(&result);
        });
    });
}

/// Miss-path benchmarks: every iteration measures uncached text, covering
/// the CJK/emoji paths the hit-only benches above miss.
fn bench_measure_cache_miss_cjk(c: &mut Criterion) {
    let measure = FixedWidthMeasure { char_width: 10.0 };
    // Distinct CJK segments defeat the cache: each iteration is a miss.
    let texts: Vec<String> = (0..256)
        .map(|i| format!("\u{6f22}\u{5b57}\u{6e2c}\u{8a66}{i}"))
        .collect();

    c.bench_function("measurement/get_width_cache_miss_cjk", |b| {
        let mut cache = MeasureCache::new();
        let mut i = 0;
        b.iter(|| {
            i += 1;
            let result = cache.get_width(black_box(&texts[i % texts.len()]), &measure);
            black_box(result);
        });
    });
}

fn bench_measure_cache_miss_emoji(c: &mut Criterion) {
    let measure = FixedWidthMeasure { char_width: 10.0 };
    // ZWJ emoji sequences: multi-codepoint graphemes on the miss path.
    let texts: Vec<String> = (0..256)
        .map(|i| format!("\u{1f469}\u{200d}\u{1f4bb}{i}\u{fe0f}"))
        .collect();

    c.bench_function("measurement/get_grapheme_widths_miss_emoji", |b| {
        let mut cache = MeasureCache::new();
        let mut i = 0;
        b.iter(|| {
            i += 1;
            let result = cache.get_grapheme_widths(black_box(&texts[i % texts.len()]), &measure);
            black_box(&result);
        });
    });
}

fn bench_measure_cache_bounded_evict(c: &mut Criterion) {
    let measure = FixedWidthMeasure { char_width: 10.0 };
    let texts: Vec<String> = (0..64).map(|i| format!("segment {i}")).collect();

    c.bench_function("measurement/bounded_cache_evict", |b| {
        let mut cache = MeasureCache::with_capacity(8);
        let mut i = 0;
        b.iter(|| {
            i += 1;
            let result = cache.get_width(black_box(&texts[i % texts.len()]), &measure);
            black_box(result);
        });
    });
}

criterion_group!(
    benches,
    bench_grapheme_prefix_widths,
    bench_measure_cache_hit,
    bench_grapheme_widths_cache_hit,
    bench_measure_cache_miss_cjk,
    bench_measure_cache_miss_emoji,
    bench_measure_cache_bounded_evict,
    bench_layout_optimal,
    bench_layout_with_lines
);
criterion_main!(benches);
