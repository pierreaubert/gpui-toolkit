/// Text measurement abstraction and caching, ported from chenglou/pretext.
///
/// Instead of the browser's canvas `measureText()`, users provide a [`TextMeasure`]
/// implementation backed by their text rendering system (e.g., GPUI, CoreText, etc.).
use std::cell::RefCell;
use std::collections::HashMap;
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
pub struct MeasureCache {
    cache: HashMap<Arc<str>, SegmentMetrics>,
}

impl MeasureCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get_segment_metrics(&mut self, seg: &str, measure: &dyn TextMeasure) -> &SegmentMetrics {
        self.cache.entry(seg.into()).or_insert_with(|| {
            let width = measure.measure_width(seg);
            let contains_cjk = is_cjk(seg);
            SegmentMetrics {
                width,
                contains_cjk,
                grapheme_widths: None,
                grapheme_prefix_widths: None,
            }
        });
        self.cache.get(seg).unwrap()
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
                        .map(|(start, end)| self.get_width(&seg[*start..*end], measure)),
                );
                let widths_arc: Arc<[f64]> = Arc::from(widths.clone());
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
            let prefix_arc: Arc<[f64]> = Arc::from(scratch.clone());
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
}
