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
/// Retained-byte and activity counters for [`TextMeasureCache`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextMeasureCacheStats {
    /// Charged retained bytes (estimated; see `prepared_charge`).
    pub retained_bytes: usize,
    /// Peak charged retained bytes since creation.
    pub high_water_bytes: usize,
    /// Entries dropped by capacity/byte eviction.
    pub evictions: u64,
}

#[derive(Debug)]
pub struct TextMeasureCache {
    /// `measure_key` → `text` → prepared text without segment strings.
    prepared_vertical: HashMap<MeasureCacheKey, HashMap<Arc<str>, PreparedText>>,
    /// `measure_key` → `text` → prepared text with segment strings.
    prepared_horizontal: HashMap<MeasureCacheKey, HashMap<Arc<str>, PreparedTextWithSegments>>,
    /// `(measure_key, cross_size, line_height, axis)` → `text` → computed size.
    sizes: HashMap<TextSizeKey, TextSizeMap>,
    solve_epoch: u64,
    retained_bytes: usize,
    high_water_bytes: usize,
    evictions: u64,
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
/// Byte budget across all retained prepared texts and sizes. The count
/// caps alone left one huge `PreparedText` (or 64 measures x 128 of
/// them) unbounded in bytes.
const TEXT_MEASURE_CACHE_MAX_BYTES: usize = 4 * 1024 * 1024;

/// Charged bytes for one prepared entry: key text plus pretext's
/// per-segment storage, estimated at 8 bytes per source byte
/// (segment strings + f64 widths scale with grapheme count) plus a
/// fixed entry overhead. Conservative by design.
fn prepared_charge(text: &str) -> usize {
    text.len().saturating_mul(8).saturating_add(256)
}

/// Charged bytes for one computed-size entry (the `Arc<str>` key is
/// shared with the prepared maps, so only entry overhead is charged).
const SIZE_ENTRY_CHARGE: usize = 64;

impl TextMeasureCache {
    /// Create a new, empty text-measurement cache.
    pub fn new() -> Self {
        Self {
            prepared_vertical: HashMap::new(),
            prepared_horizontal: HashMap::new(),
            sizes: HashMap::new(),
            solve_epoch: 0,
            retained_bytes: 0,
            high_water_bytes: 0,
            evictions: 0,
        }
    }

    /// Drop all cached prepared text and measured sizes.
    pub fn clear(&mut self) {
        self.prepared_vertical.clear();
        self.prepared_horizontal.clear();
        self.sizes.clear();
        self.retained_bytes = 0;
    }

    /// Drop all cached data and release map capacity (memory-pressure hook).
    pub fn trim(&mut self) {
        self.clear();
        self.prepared_vertical.shrink_to_fit();
        self.prepared_horizontal.shrink_to_fit();
        self.sizes.shrink_to_fit();
    }

    /// Current retained-byte/activity counters.
    #[must_use]
    pub fn stats(&self) -> TextMeasureCacheStats {
        TextMeasureCacheStats {
            retained_bytes: self.retained_bytes,
            high_water_bytes: self.high_water_bytes,
            evictions: self.evictions,
        }
    }

    fn charge(&mut self, bytes: usize) {
        self.retained_bytes = self.retained_bytes.saturating_add(bytes);
        self.high_water_bytes = self.high_water_bytes.max(self.retained_bytes);
    }

    fn uncharge(&mut self, bytes: usize) {
        self.uncharge_many(bytes, 1);
    }

    fn uncharge_many(&mut self, bytes: usize, entries: u64) {
        self.retained_bytes = self.retained_bytes.saturating_sub(bytes);
        self.evictions = self.evictions.saturating_add(entries);
    }

    /// Evict prepared entries (largest charges first by construction)
    /// and then size entries until the byte budget holds.
    fn enforce_byte_budget(&mut self) {
        while self.retained_bytes > TEXT_MEASURE_CACHE_MAX_BYTES {
            if let Some((&key, map)) = self.prepared_vertical.iter().next()
                && let Some(text) = map.keys().next().cloned()
            {
                let charge = prepared_charge(&text);
                if let Some(map) = self.prepared_vertical.get_mut(&key) {
                    map.remove(&text);
                    if map.is_empty() {
                        self.prepared_vertical.remove(&key);
                    }
                }
                self.uncharge(charge);
                continue;
            }
            if let Some((&key, map)) = self.prepared_horizontal.iter().next()
                && let Some(text) = map.keys().next().cloned()
            {
                let charge = prepared_charge(&text);
                if let Some(map) = self.prepared_horizontal.get_mut(&key) {
                    map.remove(&text);
                    if map.is_empty() {
                        self.prepared_horizontal.remove(&key);
                    }
                }
                self.uncharge(charge);
                continue;
            }
            if let Some((&key, map)) = self.sizes.iter().next()
                && let Some(text) = map.keys().next().cloned()
            {
                if let Some(map) = self.sizes.get_mut(&key) {
                    map.remove(&text);
                    if map.is_empty() {
                        self.sizes.remove(&key);
                    }
                }
                self.uncharge(SIZE_ENTRY_CHARGE);
                continue;
            }
            break;
        }
    }

    pub(super) fn begin_solve(&mut self) {
        self.solve_epoch = self.solve_epoch.wrapping_add(1);
    }

    fn measure_key(&self, measure: &dyn TextMeasure) -> MeasureCacheKey {
        MeasureCacheKey {
            identity: measure.cache_key(),
            epoch: if measure.cache_key_is_stable() {
                0
            } else {
                self.solve_epoch
            },
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
pub(super) fn compute_text_size(
    input: TextSizeInput<'_>,
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
    // Charges for entries inserted below; applied after the prepared
    // borrows end (the counter lives on the same cache struct).
    let mut inserted_charge = 0usize;
    // Evictions that happen while a map entry borrow is live are
    // accumulated here and applied with `inserted_charge` below.
    let mut evicted_charge = 0usize;
    let mut evicted_entries = 0u64;
    let size = match input.axis {
        Axis::Vertical => {
            if !cache.prepared_vertical.contains_key(&measure_key)
                && cache.prepared_vertical.len() >= TEXT_MEASURE_CACHE_MEASURE_CAPACITY
                && let Some(evicted_key) = cache.prepared_vertical.keys().next().copied()
            {
                if let Some(evicted) = cache.prepared_vertical.remove(&evicted_key) {
                    for text in evicted.keys() {
                        cache.uncharge(prepared_charge(text));
                    }
                }
            }
            let prepared = cache.prepared_vertical.entry(measure_key).or_default();
            let is_new = !prepared.contains_key(input.text);
            if is_new
                && prepared.len() >= TEXT_MEASURE_CACHE_TEXT_CAPACITY
                && let Some(evicted_text) = prepared.keys().next().cloned()
            {
                prepared.remove(&evicted_text);
                evicted_charge = evicted_charge.saturating_add(prepared_charge(&evicted_text));
                evicted_entries = evicted_entries.saturating_add(1);
            }
            let prepared = prepared.entry(Arc::clone(&text_arc)).or_insert_with(|| {
                prepare(input.text, input.measure, input.profile, input.options)
            });
            if is_new {
                inserted_charge = inserted_charge.saturating_add(prepared_charge(input.text));
            }
            layout(
                prepared,
                f64::from(input.cross_size),
                f64::from(input.line_height),
                input.profile,
            )
            .height as f32
        }
        Axis::Horizontal => {
            if !cache.prepared_horizontal.contains_key(&measure_key)
                && cache.prepared_horizontal.len() >= TEXT_MEASURE_CACHE_MEASURE_CAPACITY
                && let Some(evicted_key) = cache.prepared_horizontal.keys().next().copied()
            {
                if let Some(evicted) = cache.prepared_horizontal.remove(&evicted_key) {
                    for text in evicted.keys() {
                        cache.uncharge(prepared_charge(text));
                    }
                }
            }
            let prepared = cache.prepared_horizontal.entry(measure_key).or_default();
            let is_new = !prepared.contains_key(input.text);
            if is_new
                && prepared.len() >= TEXT_MEASURE_CACHE_TEXT_CAPACITY
                && let Some(evicted_text) = prepared.keys().next().cloned()
            {
                prepared.remove(&evicted_text);
                evicted_charge = evicted_charge.saturating_add(prepared_charge(&evicted_text));
                evicted_entries = evicted_entries.saturating_add(1);
            }
            let prepared = prepared.entry(Arc::clone(&text_arc)).or_insert_with(|| {
                prepare_with_segments(input.text, input.measure, input.profile, input.options)
            });
            if is_new {
                inserted_charge = inserted_charge.saturating_add(prepared_charge(input.text));
            }
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
        if let Some(evicted) = cache.sizes.remove(&evicted_key) {
            for _ in evicted.values() {
                cache.uncharge(SIZE_ENTRY_CHARGE);
            }
        }
    }
    let is_new_size = !cache
        .sizes
        .get(&params_key)
        .is_some_and(|sizes| sizes.contains_key(input.text));
    let sizes = cache.sizes.entry(params_key).or_default();
    if is_new_size
        && sizes.len() >= TEXT_MEASURE_CACHE_TEXT_CAPACITY
        && let Some(evicted_text) = sizes.keys().next().cloned()
    {
        sizes.remove(&evicted_text);
        evicted_charge = evicted_charge.saturating_add(SIZE_ENTRY_CHARGE);
        evicted_entries = evicted_entries.saturating_add(1);
    }
    sizes.insert(text_arc, size);
    if is_new_size {
        inserted_charge = inserted_charge.saturating_add(SIZE_ENTRY_CHARGE);
    }
    cache.uncharge_many(evicted_charge, evicted_entries);
    cache.charge(inserted_charge);
    cache.enforce_byte_budget();
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
    fn retained_text_cache_respects_byte_budget() {
        let cache = RefCell::new(TextMeasureCache::new());
        let profile = EngineProfile::default();
        let options = PrepareOptions::default();
        let measure = FixedMeasure(4242);
        // One huge text: charged above the whole byte budget, so its
        // prepared entry must be evicted right after use.
        let huge = "x".repeat(1_000_000);
        compute_text_size(
            TextSizeInput {
                text: &huge,
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
        let stats = cache.borrow().stats();
        assert!(
            stats.retained_bytes <= TEXT_MEASURE_CACHE_MAX_BYTES,
            "{stats:?}"
        );
        assert!(stats.high_water_bytes >= stats.retained_bytes, "{stats:?}");
        assert!(stats.evictions > 0, "{stats:?}");
        // Many moderate texts plateau under the budget as well.
        for index in 0..400 {
            let text = "y".repeat(20_000) + &index.to_string();
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
        let stats = cache.borrow().stats();
        assert!(
            stats.retained_bytes <= TEXT_MEASURE_CACHE_MAX_BYTES,
            "{stats:?}"
        );
        cache.borrow_mut().trim();
        assert_eq!(cache.borrow().stats().retained_bytes, 0);
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
