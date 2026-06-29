use crate::types::Axis;
use gpui_pretext::{
    EngineProfile, PrepareOptions, PreparedText, PreparedTextWithSegments, layout,
    layout_with_lines, prepare, prepare_with_segments,
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
/// [`solve_tree_with_cache`](crate::solver::solve_tree_with_cache). A thread-local
/// default cache is used by the non-`_with_cache` entry points so existing callers
/// still benefit from cross-frame caching without any API change.
#[derive(Debug)]
pub struct TextMeasureCache {
    /// `measure_ptr` → `text` → prepared text without segment strings.
    prepared_vertical: HashMap<usize, HashMap<Arc<str>, PreparedText>>,
    /// `measure_ptr` → `text` → prepared text with segment strings.
    prepared_horizontal: HashMap<usize, HashMap<Arc<str>, PreparedTextWithSegments>>,
    /// `(measure_ptr, cross_size, line_height, axis)` → `text` → computed size.
    sizes: HashMap<TextSizeKey, TextSizeMap>,
}

pub(crate) type TextSizeKey = (usize, u32, u32, Axis);
pub(crate) type TextSizeMap = HashMap<Arc<str>, f32>;

impl TextMeasureCache {
    /// Create a new, empty text-measurement cache.
    pub fn new() -> Self {
        Self {
            prepared_vertical: HashMap::new(),
            prepared_horizontal: HashMap::new(),
            sizes: HashMap::new(),
        }
    }

    /// Drop all cached prepared text and measured sizes.
    pub fn clear(&mut self) {
        self.prepared_vertical.clear();
        self.prepared_horizontal.clear();
        self.sizes.clear();
    }
}

impl Default for TextMeasureCache {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    /// Thread-local default cache used by the non-`_with_cache` solver entry
    /// points so callers are not required to manage a cache handle.
    static DEFAULT_TEXT_CACHE: RefCell<Rc<RefCell<TextMeasureCache>>> = RefCell::new(Rc::new(RefCell::new(TextMeasureCache::new())));
}

/// Access the thread-local default text-measurement cache handle.
pub(super) fn default_text_cache() -> Rc<RefCell<TextMeasureCache>> {
    DEFAULT_TEXT_CACHE.with(|cache| Rc::clone(&cache.borrow()))
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
    let measure_ptr = (input.measure as *const dyn gpui_pretext::TextMeasure) as *const () as usize;
    let cross_bits = input.cross_size.to_bits();
    let line_bits = input.line_height.to_bits();
    let params_key = (measure_ptr, cross_bits, line_bits, input.axis);

    let mut cache = cache.borrow_mut();

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
            let prepared = cache
                .prepared_vertical
                .entry(measure_ptr)
                .or_default()
                .entry(Arc::clone(&text_arc))
                .or_insert_with(|| {
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
            let prepared = cache
                .prepared_horizontal
                .entry(measure_ptr)
                .or_default()
                .entry(Arc::clone(&text_arc))
                .or_insert_with(|| {
                    prepare_with_segments(input.text, input.measure, input.profile, input.options)
                });
            let result =
                layout_with_lines(prepared, f64::MAX, input.line_height as f64, input.profile);
            result.lines.iter().map(|l| l.width).fold(0.0_f64, f64::max) as f32
        }
    };

    cache
        .sizes
        .entry(params_key)
        .or_default()
        .insert(text_arc, size);
    size.max(input.min)
}

/// Clear the persistent text measurement cache.
///
/// Mostly useful in tests to get deterministic baseline behaviour.
#[cfg(test)]
pub(super) fn clear_text_cache() {
    DEFAULT_TEXT_CACHE.with(|cache| {
        cache.borrow().borrow_mut().clear();
    })
}
