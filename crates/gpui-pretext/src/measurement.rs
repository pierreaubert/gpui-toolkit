use hashbrown::HashMap;
use hashbrown::hash_map::RawEntryMut;
/// Text measurement abstraction and caching, ported from chenglou/pretext.
///
/// Instead of the browser's canvas `measureText()`, users provide a [`TextMeasure`]
/// implementation backed by their text rendering system (e.g., GPUI, CoreText, etc.).
use std::cell::RefCell;
use std::sync::Arc;

use unicode_segmentation::UnicodeSegmentation;

use crate::analysis::is_cjk;

thread_local! {
    /// Reusable grapheme byte-range scratch for breaking segments into
    /// user-perceived characters. Storing offsets rather than `&str` slices
    /// avoids lifetime issues with the thread-local buffer.
    static GRAPHEME_SCRATCH: RefCell<Vec<(usize, usize)>> = const { RefCell::new(Vec::new()) };
    /// Reusable f64 buffer for per-grapheme and prefix widths.
    static WIDTH_SCRATCH: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}

// ---------------------------------------------------------------------------
// TextMeasure trait
// ---------------------------------------------------------------------------

/// Trait for measuring text advance widths.
///
/// Implement this backed by your text rendering system. The implementation
/// should be configured with a specific font before being passed to `prepare()`.
pub trait TextMeasure {
    /// Measure the advance width (in pixels/points) of the given text string.
    fn measure_width(&self, text: &str) -> f64;

    /// Stable identity and revision token for retained measurement caches.
    ///
    /// Implementations whose metrics change in place should override this and
    /// change the token whenever font, scale, locale, or shaping state changes.
    /// The default distinguishes measure instances for compatibility.
    fn cache_key(&self) -> u64 {
        (self as *const Self as *const () as usize) as u64
    }

    /// Whether [`Self::cache_key`] identifies this measure across independent
    /// layout solves.
    ///
    /// The default key is the object's address, which is only safe during one
    /// solve: a later measure can be allocated at the same address with
    /// different metrics. Implementations that return a semantic identity from
    /// [`Self::cache_key`] must override this to return `true`; the identity
    /// must change whenever their font, scale, locale, or shaping state does.
    fn cache_key_is_stable(&self) -> bool {
        false
    }

    /// Shape a run, returning per-grapheme advances plus cluster mapping.
    ///
    /// Backends with a real shaper (HarfBuzz, rustybuzz, CoreText) override
    /// this to return glyph advances with ligature/kerning applied; the
    /// default returns `None`, and [`shape_run`] falls back to measuring each
    /// grapheme with [`TextMeasure::measure_width`]. Overriding this never
    /// changes [`PrepareOptions`](crate::PrepareOptions): callers keep passing
    /// the same options and `shape_run` picks the richer path automatically.
    fn shape_run(&self, _text: &str) -> Option<ShapedRun> {
        None
    }
}

// ---------------------------------------------------------------------------
// Engine profile
// ---------------------------------------------------------------------------

/// Engine-specific tuning parameters. In the original JS library these were
/// auto-detected per browser; in Rust you configure them explicitly.
#[derive(Debug, Clone, Copy)]
pub struct EngineProfile {
    /// Epsilon for float comparison when fitting text into a line width.
    /// Safari uses `1/64`, Chromium uses `0.005`.
    pub line_fit_epsilon: f64,
    /// Whether to carry CJK characters after closing quotes into the next segment.
    /// Chromium-specific behavior.
    pub carry_cjk_after_closing_quote: bool,
    /// Whether to prefer prefix-width accumulation for breakable runs.
    /// Safari-specific behavior.
    pub prefer_prefix_widths_for_breakable_runs: bool,
    /// Whether to prefer breaking at soft hyphens earlier.
    /// Safari-specific behavior.
    pub prefer_early_soft_hyphen_break: bool,
}

impl Default for EngineProfile {
    fn default() -> Self {
        Self {
            line_fit_epsilon: 0.005,
            carry_cjk_after_closing_quote: false,
            prefer_prefix_widths_for_breakable_runs: false,
            prefer_early_soft_hyphen_break: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Shaping seam
// ---------------------------------------------------------------------------

/// One shaped cluster: the advance width of a user-perceived character plus
/// the byte range it covers in the source run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph {
    /// Advance width in pixels/points.
    pub advance: f64,
    /// Byte offset of the cluster start within the shaped run.
    pub cluster_start: u32,
    /// Byte offset of the cluster end within the shaped run.
    pub cluster_end: u32,
}

/// Shaped form of a text run: per-grapheme advances with cluster mapping.
///
/// This is the seam where a real shaper (HarfBuzz/rustybuzz) plugs in without
/// changing `PrepareOptions` or the prepare/layout split: [`shape_run`]
/// prefers [`TextMeasure::shape_run`] when a backend provides it and otherwise
/// derives advances from [`TextMeasure::measure_width`].
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedRun {
    /// One entry per grapheme in the run, in order.
    pub glyphs: Arc<[ShapedGlyph]>,
    /// Total advance (sum of `glyphs` advances).
    pub width: f64,
}

impl ShapedRun {
    /// Advances only, in grapheme order.
    pub fn advances(&self) -> Vec<f64> {
        self.glyphs.iter().map(|g| g.advance).collect()
    }
}

/// Shape `text` into per-grapheme advances.
///
/// Uses [`TextMeasure::shape_run`] when the backend provides shaping;
/// otherwise measures each grapheme with [`TextMeasure::measure_width`]
/// (no ligatures/kerning). Always returns one [`ShapedGlyph`] per grapheme.
pub fn shape_run(text: &str, measure: &dyn TextMeasure) -> ShapedRun {
    if let Some(shaped) = measure.shape_run(text) {
        return shaped;
    }
    let mut glyphs = Vec::new();
    let mut width = 0.0;
    for (start, g) in text.grapheme_indices(true) {
        let advance = measure.measure_width(g);
        width += advance;
        glyphs.push(ShapedGlyph {
            advance,
            cluster_start: start as u32,
            cluster_end: (start + g.len()) as u32,
        });
    }
    ShapedRun {
        glyphs: Arc::from(glyphs.into_boxed_slice()),
        width,
    }
}

// ---------------------------------------------------------------------------
// Segment metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SegmentMetrics {
    pub width: f64,
    pub contains_cjk: bool,
    pub grapheme_widths: Option<Arc<[f64]>>,
    pub grapheme_prefix_widths: Option<Arc<[f64]>>,
}

// ---------------------------------------------------------------------------
// Measurement cache
// ---------------------------------------------------------------------------

/// Caches segment widths and grapheme-level metrics during the prepare phase.
///
/// The cache is a bounded LRU (see [`MeasureCache::with_budgets`]): once
/// `capacity` entries or `max_bytes` of retained segment text are exceeded,
/// least-recently-used entries are evicted. This bounds worst-case memory for
/// adversarial inputs (e.g. per-grapheme CJK segments) where an unbounded map
/// would grow without limit.
pub struct MeasureCache {
    cache: HashMap<Arc<str>, SegmentMetrics>,
    /// LRU order, front = least recently used.
    order: std::collections::VecDeque<Arc<str>>,
    /// Maximum entries retained.
    capacity: usize,
    /// Maximum retained segment-text bytes. `usize::MAX` means unbounded.
    max_bytes: usize,
    retained_bytes: usize,
}

/// Defaults for [`MeasureCache::new`]: generous enough that ordinary
/// paragraphs never evict, tight enough to bound adversarial growth.
pub const DEFAULT_MEASURE_CACHE_CAPACITY: usize = 4096;
/// Default byte budget for retained segment text (~4 MiB).
pub const DEFAULT_MEASURE_CACHE_MAX_BYTES: usize = 4 * 1024 * 1024;

impl MeasureCache {
    pub fn new() -> Self {
        Self::with_budgets(
            DEFAULT_MEASURE_CACHE_CAPACITY,
            DEFAULT_MEASURE_CACHE_MAX_BYTES,
        )
    }

    /// Unbounded cache (legacy behavior). Prefer [`MeasureCache::new`].
    pub fn unbounded() -> Self {
        Self::with_budgets(usize::MAX, usize::MAX)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_budgets(capacity, usize::MAX)
    }

    pub fn with_budgets(capacity: usize, max_bytes: usize) -> Self {
        Self {
            cache: HashMap::new(),
            order: std::collections::VecDeque::new(),
            capacity: capacity.max(1),
            max_bytes,
            retained_bytes: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.order.clear();
        self.retained_bytes = 0;
    }

    fn touch(&mut self, key: &Arc<str>) {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            let k = self.order.remove(pos).expect("lru position valid");
            self.order.push_back(k);
        }
    }

    fn evict_if_needed(&mut self) {
        while (self.cache.len() > self.capacity
            || (self.max_bytes != usize::MAX && self.retained_bytes > self.max_bytes))
            // Never evict the most-recently-inserted entry: callers hold a
            // reference to it across this call.
            && self.order.len() > 1
        {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if self.cache.remove(&oldest).is_some() {
                self.retained_bytes = self
                    .retained_bytes
                    .saturating_sub(oldest.len() + std::mem::size_of::<SegmentMetrics>());
            }
        }
    }

    pub fn get_segment_metrics(&mut self, seg: &str, measure: &dyn TextMeasure) -> &SegmentMetrics {
        if self.cache.contains_key(seg) {
            let stored_key = self.cache.get_key_value(seg).map(|(k, _)| Arc::clone(k));
            if let Some(k) = stored_key {
                self.touch(&k);
            }
            // Hit: the entry is not evicted on this path, so a second lookup
            // is safe.
            return self.cache.get(seg).expect("cache hit present");
        }
        let width = measure.measure_width(seg);
        let contains_cjk = is_cjk(seg);
        let key: Arc<str> = Arc::from(seg);
        self.retained_bytes += key.len() + std::mem::size_of::<SegmentMetrics>();
        self.order.push_back(Arc::clone(&key));
        let metrics_ptr = match self.cache.raw_entry_mut().from_key(seg) {
            RawEntryMut::Occupied(entry) => entry.into_mut() as *mut SegmentMetrics,
            RawEntryMut::Vacant(entry) => {
                let (_, metrics) = entry.insert(
                    key,
                    SegmentMetrics {
                        width,
                        contains_cjk,
                        grapheme_widths: None,
                        grapheme_prefix_widths: None,
                    },
                );
                metrics as *mut SegmentMetrics
            }
        };
        // Evict after inserting so the new entry counts toward the budgets.
        // The just-inserted key sits at the back of the LRU order, so it
        // survives unless every budget is zero.
        self.evict_if_needed();
        // SAFETY: `evict_if_needed` only pops from the front of the LRU order
        // while more than `capacity` entries exist; the back (this entry) is
        // removed only when capacity is zero, which `with_budgets` forbids via
        // `capacity.max(1)`.
        unsafe { &*metrics_ptr }
    }

    pub fn get_width(&mut self, seg: &str, measure: &dyn TextMeasure) -> f64 {
        self.get_segment_metrics(seg, measure).width
    }

    fn ensure_parent_entry(&mut self, seg: &str, measure: &dyn TextMeasure) {
        let _ = self.get_segment_metrics(seg, measure);
    }

    /// Get per-grapheme widths for a segment (measured individually).
    /// Returns `None` if the segment has only one grapheme.
    pub fn get_grapheme_widths(
        &mut self,
        seg: &str,
        measure: &dyn TextMeasure,
    ) -> Option<Arc<[f64]>> {
        // Check if already computed
        if let Some(metrics) = self.cache.get(seg)
            && let Some(ref gw) = metrics.grapheme_widths
        {
            if gw.is_empty() {
                return None; // sentinel: single-grapheme segment
            }
            return Some(gw.clone());
        }

        let widths = GRAPHEME_SCRATCH.with(|grapheme_scratch| {
            let mut graphemes = grapheme_scratch.borrow_mut();
            graphemes.clear();
            graphemes.extend(
                seg.grapheme_indices(true)
                    .map(|(start, g)| (start, start + g.len())),
            );

            if graphemes.len() <= 1 {
                drop(graphemes);
                self.ensure_parent_entry(seg, measure);
                if let Some(metrics) = self.cache.get_mut(seg) {
                    // sentinel: computed but single
                    metrics.grapheme_widths = Some(Arc::from(Vec::new()));
                }
                return None;
            }

            // Ensure parent entry exists before measuring graphemes
            // (grapheme measurement may insert sub-entries but not the parent)
            self.ensure_parent_entry(seg, measure);

            let widths_arc = WIDTH_SCRATCH.with(|width_scratch| {
                let mut widths = width_scratch.borrow_mut();
                widths.clear();
                widths.extend(
                    graphemes
                        .iter()
                        // These widths are single-use inputs to the parent
                        // segment. Caching each slice creates one Arc/HashMap
                        // entry per grapheme with no later hit.
                        .map(|(start, end)| measure.measure_width(&seg[*start..*end])),
                );
                let widths_arc: Arc<[f64]> = Arc::from(widths.as_slice());
                widths.clear();
                widths_arc
            });

            graphemes.clear();
            Some(widths_arc)
        })?;

        // Store in the parent segment's metrics (guaranteed to exist now)
        if let Some(metrics) = self.cache.get_mut(seg) {
            metrics.grapheme_widths = Some(widths.clone());
        }

        Some(widths)
    }

    /// Get cumulative prefix widths for a segment's graphemes.
    /// Returns `None` if the segment has only one grapheme.
    pub fn get_grapheme_prefix_widths(
        &mut self,
        seg: &str,
        measure: &dyn TextMeasure,
    ) -> Option<Arc<[f64]>> {
        // Check if already computed
        if let Some(metrics) = self.cache.get(seg)
            && let Some(ref pw) = metrics.grapheme_prefix_widths
        {
            if pw.is_empty() {
                return None; // sentinel: single-grapheme segment
            }
            return Some(pw.clone());
        }

        // Reuse per-grapheme widths and compute prefix sums in O(n) instead of
        // measuring every prefix substring (O(n²) calls into the measurer).
        let Some(widths) = self.get_grapheme_widths(seg, measure) else {
            self.ensure_parent_entry(seg, measure);
            if let Some(metrics) = self.cache.get_mut(seg) {
                metrics.grapheme_prefix_widths = Some(Arc::from(Vec::new()));
            }
            return None;
        };

        let prefix_widths: Arc<[f64]> = WIDTH_SCRATCH.with(|scratch| {
            let mut scratch = scratch.borrow_mut();
            scratch.clear();
            scratch.extend_from_slice(&widths);
            let mut running = 0.0;
            for w in scratch.iter_mut() {
                running += *w;
                *w = running;
            }
            let prefix_arc: Arc<[f64]> = Arc::from(scratch.as_slice());
            scratch.clear();
            prefix_arc
        });

        if let Some(metrics) = self.cache.get_mut(seg) {
            metrics.grapheme_prefix_widths = Some(prefix_widths.clone());
        }

        Some(prefix_widths)
    }
}

impl Default for MeasureCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedWidthMeasure {
        char_width: f64,
    }

    impl TextMeasure for FixedWidthMeasure {
        fn measure_width(&self, text: &str) -> f64 {
            text.chars().count() as f64 * self.char_width
        }
    }

    #[test]
    fn test_measure_cache() {
        let measure = FixedWidthMeasure { char_width: 10.0 };
        let mut cache = MeasureCache::new();

        assert!((cache.get_width("hello", &measure) - 50.0).abs() < 0.001);
        // Second call should use cache
        assert!((cache.get_width("hello", &measure) - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_grapheme_widths() {
        let measure = FixedWidthMeasure { char_width: 10.0 };
        let mut cache = MeasureCache::new();

        // Ensure metrics exist first
        let _ = cache.get_width("abc", &measure);
        let widths = cache.get_grapheme_widths("abc", &measure);
        assert!(widths.is_some());
        let widths = widths.unwrap();
        assert_eq!(widths.len(), 3);
    }

    #[test]
    fn test_grapheme_widths_without_pre_populate() {
        // Issue 4: get_grapheme_widths should work even if parent entry doesn't exist yet
        let measure = FixedWidthMeasure { char_width: 10.0 };
        let mut cache = MeasureCache::new();

        // Call get_grapheme_widths directly without calling get_width first
        let widths = cache.get_grapheme_widths("xyz", &measure);
        assert!(widths.is_some());
        assert_eq!(widths.as_ref().unwrap().len(), 3);

        // Second call should use cache (not re-measure) and return the same Arc allocation.
        let widths2 = cache.get_grapheme_widths("xyz", &measure);
        assert_eq!(widths, widths2);
        assert!(std::ptr::eq(
            widths.as_ref().unwrap().as_ptr(),
            widths2.as_ref().unwrap().as_ptr()
        ));
    }

    #[test]
    fn test_grapheme_widths_single_grapheme_returns_none() {
        let measure = FixedWidthMeasure { char_width: 10.0 };
        let mut cache = MeasureCache::new();
        assert!(cache.get_grapheme_widths("x", &measure).is_none());
    }

    #[test]
    fn test_grapheme_prefix_widths_without_pre_populate() {
        let measure = FixedWidthMeasure { char_width: 10.0 };
        let mut cache = MeasureCache::new();

        let widths = cache.get_grapheme_prefix_widths("abc", &measure);
        assert!(widths.is_some());
        let widths = widths.unwrap();
        assert_eq!(widths.len(), 3);
        // Prefix widths should be cumulative: 10, 20, 30
        assert!((widths[0] - 10.0).abs() < 0.001);
        assert!((widths[1] - 20.0).abs() < 0.001);
        assert!((widths[2] - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_grapheme_prefix_widths_cache_identity() {
        let measure = FixedWidthMeasure { char_width: 10.0 };
        let mut cache = MeasureCache::new();

        let first = cache.get_grapheme_prefix_widths("abc", &measure);
        let second = cache.get_grapheme_prefix_widths("abc", &measure);
        assert!(std::ptr::eq(
            first.as_ref().unwrap().as_ptr(),
            second.as_ref().unwrap().as_ptr()
        ));
    }

    #[test]
    fn test_grapheme_widths_sentinel_cache_hit() {
        let measure = FixedWidthMeasure { char_width: 10.0 };
        let mut cache = MeasureCache::new();
        assert!(cache.get_grapheme_widths("x", &measure).is_none());
        // Second call should hit the cached sentinel and return None.
        assert!(cache.get_grapheme_widths("x", &measure).is_none());
    }

    #[test]
    fn test_grapheme_prefix_widths_sentinel_cache_hit() {
        let measure = FixedWidthMeasure { char_width: 10.0 };
        let mut cache = MeasureCache::new();
        assert!(cache.get_grapheme_prefix_widths("x", &measure).is_none());
        // Second call should hit the cached sentinel and return None.
        assert!(cache.get_grapheme_prefix_widths("x", &measure).is_none());
    }

    #[test]
    fn test_measure_cache_default() {
        let cache = MeasureCache::default();
        assert!(cache.cache.is_empty());
    }

    #[test]
    fn test_bounded_cache_evicts_lru() {
        let measure = FixedWidthMeasure { char_width: 10.0 };
        let mut cache = MeasureCache::with_capacity(2);

        let _ = cache.get_width("aaa", &measure);
        let _ = cache.get_width("bbb", &measure);
        assert_eq!(cache.len(), 2);
        // Touch "aaa" so "bbb" becomes least-recently-used.
        let _ = cache.get_width("aaa", &measure);
        let _ = cache.get_width("ccc", &measure);
        assert_eq!(cache.len(), 2);
        assert!(cache.cache.contains_key("aaa"));
        assert!(cache.cache.contains_key("ccc"));
        assert!(!cache.cache.contains_key("bbb"));
    }

    #[test]
    fn test_bounded_cache_stays_correct_after_eviction() {
        struct CountingMeasure(std::cell::Cell<usize>);
        impl TextMeasure for CountingMeasure {
            fn measure_width(&self, text: &str) -> f64 {
                self.0.set(self.0.get() + 1);
                text.chars().count() as f64 * 10.0
            }
        }
        let measure = CountingMeasure(std::cell::Cell::new(0));
        let mut cache = MeasureCache::with_capacity(1);
        assert!((cache.get_width("ab", &measure) - 20.0).abs() < 0.001);
        // Evicted by the next insert; re-measure must still be correct.
        assert!((cache.get_width("cd", &measure) - 20.0).abs() < 0.001);
        assert!((cache.get_width("ab", &measure) - 20.0).abs() < 0.001);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_shape_run_fallback_measures_graphemes() {
        let measure = FixedWidthMeasure { char_width: 10.0 };
        let shaped = shape_run("abc", &measure);
        assert_eq!(shaped.glyphs.len(), 3);
        assert!((shaped.width - 30.0).abs() < 0.001);
        assert_eq!(
            shaped
                .glyphs
                .iter()
                .map(|g| (g.cluster_start, g.cluster_end))
                .collect::<Vec<_>>(),
            vec![(0, 1), (1, 2), (2, 3)]
        );
    }

    #[test]
    fn test_shape_run_prefers_backend_shaping() {
        struct KernedMeasure;
        impl TextMeasure for KernedMeasure {
            fn measure_width(&self, text: &str) -> f64 {
                text.chars().count() as f64 * 10.0
            }
            fn shape_run(&self, text: &str) -> Option<ShapedRun> {
                // Fake kerned backend: "AV" pairs cost 15 instead of 20.
                let mut glyphs = Vec::new();
                let mut width = 0.0;
                let mut chars = text.chars().peekable();
                let mut byte = 0;
                while let Some(ch) = chars.next() {
                    let len = ch.len_utf8();
                    let mut adv = 10.0;
                    if ch == 'A' && chars.peek() == Some(&'V') {
                        adv = 5.0;
                    }
                    width += adv;
                    glyphs.push(ShapedGlyph {
                        advance: adv,
                        cluster_start: byte as u32,
                        cluster_end: (byte + len) as u32,
                    });
                    byte += len;
                }
                Some(ShapedRun {
                    glyphs: Arc::from(glyphs.into_boxed_slice()),
                    width,
                })
            }
        }
        let measure = KernedMeasure;
        let shaped = shape_run("AV", &measure);
        assert!((shaped.width - 15.0).abs() < 0.001);
        assert_eq!(shaped.glyphs.len(), 2);
    }
}
