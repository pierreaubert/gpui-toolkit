use crate::types::Axis;
use gpui_pretext::{
    EngineProfile, PrepareOptions, PreparedText, PreparedTextWithSegments, TextMeasure, layout,
    prepare, prepare_with_segments, walk_line_ranges,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Persistent cache for `Sizing::Text` measurements and their underlying
/// [`PreparedText`] data.
///
/// The cache survives across `solve` calls, so repeated layouts of the same
/// text (same content, measure, profile and options) avoid re-running text
/// analysis and measurement.
///
/// The cache is meant to be shared via [`Rc`]`<`[`RefCell`]`<`[`TextMeasureCache`]`>>`
/// and passed into [`solve_with_cache`](crate::solver::solve_with_cache) /
/// [`solve_tree_with_cache`](crate::solver::solve_tree_with_cache). For a cache
/// retained across calls, [`TextMeasure::cache_key`](gpui_pretext::TextMeasure::cache_key)
/// must be a semantic identity that changes when metrics do; the pointer-based
/// trait default is suitable only for a single solve call.
#[derive(Debug)]
pub struct TextMeasureCache {
    /// `measure_key` → `text` → prepared text without segment strings.
    prepared_vertical: HashMap<MeasureCacheKey, HashMap<Arc<str>, PreparedText>>,
    /// `measure_key` → `text` → prepared text with segment strings.
    prepared_horizontal: HashMap<MeasureCacheKey, HashMap<Arc<str>, PreparedTextWithSegments>>,
    /// `(measure_key, cross_size, line_height, axis)` → `text` → computed size.
    sizes: HashMap<TextSizeKey, TextSizeMap>,
    solve_epoch: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct MeasureCacheKey {
    identity: u64,
    epoch: u64,
}

pub(crate) type TextSizeKey = (MeasureCacheKey, u32, u32, Axis);
pub(crate) type TextSizeMap = HashMap<Arc<str>, f32>;

const TEXT_MEASURE_CACHE_MEASURE_CAPACITY: usize = 64;
const TEXT_MEASURE_CACHE_TEXT_CAPACITY: usize = 128;
const TEXT_MEASURE_CACHE_SIZE_KEY_CAPACITY: usize = 128;

impl TextMeasureCache {
    /// Create a new, empty text-measurement cache.
    pub fn new() -> Self {
        Self {
            prepared_vertical: HashMap::new(),
            prepared_horizontal: HashMap::new(),
            sizes: HashMap::new(),
            solve_epoch: 0,
        }
    }

    /// Drop all cached prepared text and measured sizes.
    pub fn clear(&mut self) {
        self.prepared_vertical.clear();
        self.prepared_horizontal.clear();
        self.sizes.clear();
    }

    pub(super) fn begin_solve(&mut self) {
        self.solve_epoch = self.solve_epoch.wrapping_add(1);
    }

    fn measure_key(&self, measure: &dyn TextMeasure) -> MeasureCacheKey {
        MeasureCacheKey {
            identity: measure.cache_key(),
            epoch: measure
                .cache_key_is_stable()
                .then_some(0)
                .unwrap_or(self.solve_epoch),
        }
    }
}

impl Default for TextMeasureCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Create an ephemeral cache for the non-`_with_cache` entry points.
///
/// A `TextMeasure`'s backwards-compatible default cache key is its address,
/// which can be recycled after the measure is dropped. Keeping that cache
/// across independent calls would allow stale text widths to be reused. The
/// public explicit-cache APIs retain their cache only when the caller has a
/// stable semantic key for its measure.
pub(super) fn default_text_cache() -> Rc<RefCell<TextMeasureCache>> {
    thread_local! {
        static DEFAULT_TEXT_CACHE: Rc<RefCell<TextMeasureCache>> =
            Rc::new(RefCell::new(TextMeasureCache::new()));
    }

    DEFAULT_TEXT_CACHE.with(Rc::clone)
}

/// Construct a cache owned by a retained solver or explicit caller.
pub(super) fn fresh_text_cache() -> Rc<RefCell<TextMeasureCache>> {
    Rc::new(RefCell::new(TextMeasureCache::new()))
}

pub(super) struct TextSizeInput<'a> {
    pub(super) text: &'a str,
    pub(super) measure: &'a dyn gpui_pretext::TextMeasure,
    pub(super) line_height: f32,
    pub(super) min: f32,
    pub(super) axis: Axis,
    pub(super) cross_size: f32,
    pub(super) profile: &'a EngineProfile,
    pub(super) options: &'a PrepareOptions,
}

/// Compute the size for a `Sizing::Text` node using gpui-pretext.
///
/// - In a **vertical** container (main axis = height): returns text height
///   with `cross_size` (the container's width) as `max_width`.
/// - In a **horizontal** container (main axis = width): returns the maximum
///   line width with no wrapping constraint.
///
/// Results and the intermediate [`PreparedText`] values are cached in the
/// provided [`TextMeasureCache`], which survives across `solve` calls when
/// shared by the caller.
pub(super) fn compute_text_size<'a>(
    input: TextSizeInput<'a>,
    cache: &RefCell<TextMeasureCache>,
) -> f32 {
    let mut cache = cache.borrow_mut();
    let measure_key = cache.measure_key(input.measure);
    let cross_bits = input.cross_size.to_bits();
    let line_bits = input.line_height.to_bits();
    let params_key = (measure_key, cross_bits, line_bits, input.axis);

    // Fast-path: probe the size cache using the borrowed `&str` key so cache
    // hits do not allocate an `Arc<str>`.
    if let Some(by_text) = cache.sizes.get(&params_key)
        && let Some(&size) = by_text.get(input.text)
    {
        return size.max(input.min);
    }

    // Size is not cached; prepare the text (reusing any cached `PreparedText`).
    let text_arc: Arc<str> = Arc::from(input.text);
    let size = match input.axis {
        Axis::Vertical => {
            if !cache.prepared_vertical.contains_key(&measure_key)
                && cache.prepared_vertical.len() >= TEXT_MEASURE_CACHE_MEASURE_CAPACITY
                && let Some(evicted_key) = cache.prepared_vertical.keys().next().copied()
            {
                cache.prepared_vertical.remove(&evicted_key);
            }
            let prepared = cache.prepared_vertical.entry(measure_key).or_default();
            if !prepared.contains_key(input.text)
                && prepared.len() >= TEXT_MEASURE_CACHE_TEXT_CAPACITY
                && let Some(evicted_text) = prepared.keys().next().cloned()
            {
                prepared.remove(&evicted_text);
            }
            let prepared = prepared.entry(Arc::clone(&text_arc)).or_insert_with(|| {
                prepare(input.text, input.measure, input.profile, input.options)
            });
            layout(
                prepared,
                input.cross_size as f64,
                input.line_height as f64,
                input.profile,
            )
            .height as f32
        }
        Axis::Horizontal => {
            if !cache.prepared_horizontal.contains_key(&measure_key)
                && cache.prepared_horizontal.len() >= TEXT_MEASURE_CACHE_MEASURE_CAPACITY
                && let Some(evicted_key) = cache.prepared_horizontal.keys().next().copied()
            {
                cache.prepared_horizontal.remove(&evicted_key);
            }
            let prepared = cache.prepared_horizontal.entry(measure_key).or_default();
            if !prepared.contains_key(input.text)
                && prepared.len() >= TEXT_MEASURE_CACHE_TEXT_CAPACITY
                && let Some(evicted_text) = prepared.keys().next().cloned()
            {
                prepared.remove(&evicted_text);
            }
            let prepared = prepared.entry(Arc::clone(&text_arc)).or_insert_with(|| {
                prepare_with_segments(input.text, input.measure, input.profile, input.options)
            });
            let mut max_width = 0.0_f64;
            walk_line_ranges(prepared, f64::MAX, input.profile, |line| {
                max_width = max_width.max(line.width);
            });
            max_width as f32
        }
    };

    if !cache.sizes.contains_key(&params_key)
        && cache.sizes.len() >= TEXT_MEASURE_CACHE_SIZE_KEY_CAPACITY
        && let Some(evicted_key) = cache.sizes.keys().next().copied()
    {
        cache.sizes.remove(&evicted_key);
    }
    let sizes = cache.sizes.entry(params_key).or_default();
    if !sizes.contains_key(input.text)
        && sizes.len() >= TEXT_MEASURE_CACHE_TEXT_CAPACITY
        && let Some(evicted_text) = sizes.keys().next().cloned()
    {
        sizes.remove(&evicted_text);
    }
    sizes.insert(text_arc, size);
    size.max(input.min)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedMeasure(u64);

    impl gpui_pretext::TextMeasure for FixedMeasure {
        fn measure_width(&self, text: &str) -> f64 {
            text.chars().count() as f64
        }

        fn cache_key(&self) -> u64 {
            self.0
        }

        fn cache_key_is_stable(&self) -> bool {
            true
        }
    }

    #[test]
    fn retained_text_cache_is_bounded() {
        let cache = RefCell::new(TextMeasureCache::new());
        let profile = EngineProfile::default();
        let options = PrepareOptions::default();

        for key in 0..=(TEXT_MEASURE_CACHE_MEASURE_CAPACITY as u64 + 1) {
            let measure = FixedMeasure(key);
            let text = format!("measure-{key}");
            compute_text_size(
                TextSizeInput {
                    text: &text,
                    measure: &measure,
                    line_height: 16.0,
                    min: 0.0,
                    axis: Axis::Vertical,
                    cross_size: 300.0,
                    profile: &profile,
                    options: &options,
                },
                &cache,
            );
        }

        for index in 0..=(TEXT_MEASURE_CACHE_TEXT_CAPACITY + 1) {
            let measure = FixedMeasure(999);
            let text = format!("text-{index}");
            compute_text_size(
                TextSizeInput {
                    text: &text,
                    measure: &measure,
                    line_height: 16.0,
                    min: 0.0,
                    axis: Axis::Vertical,
                    cross_size: 300.0,
                    profile: &profile,
                    options: &options,
                },
                &cache,
            );
        }

        for index in 0..=(TEXT_MEASURE_CACHE_SIZE_KEY_CAPACITY + 1) {
            let measure = FixedMeasure(1_000);
            compute_text_size(
                TextSizeInput {
                    text: "cross-size",
                    measure: &measure,
                    line_height: 16.0,
                    min: 0.0,
                    axis: Axis::Vertical,
                    cross_size: index as f32 + 1.0,
                    profile: &profile,
                    options: &options,
                },
                &cache,
            );
        }

        let cache = cache.borrow();
        assert!(cache.prepared_vertical.len() <= TEXT_MEASURE_CACHE_MEASURE_CAPACITY);
        assert!(
            cache
                .prepared_vertical
                .values()
                .all(|texts| { texts.len() <= TEXT_MEASURE_CACHE_TEXT_CAPACITY })
        );
        assert!(cache.sizes.len() <= TEXT_MEASURE_CACHE_SIZE_KEY_CAPACITY);
        assert!(
            cache
                .sizes
                .values()
                .all(|texts| texts.len() <= TEXT_MEASURE_CACHE_TEXT_CAPACITY)
        );
    }
}
