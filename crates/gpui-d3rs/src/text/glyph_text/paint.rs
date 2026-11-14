use super::chart::chart_text_layout;
use super::glyph_text_config::GlyphTextConfig;
use super::rasterize::rasterize_rotated_text;
use super::types::ChartTextLayout;
use super::types::HorizontalTextAnchor;
use super::types::VerticalTextAnchor;
use gpui::{Corners, Hsla, RenderImage, Rgba, TransformationMatrix, px};
use image::{Frame, RgbaImage};
use std::{
    cell::RefCell,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::util::bucket_cache::BucketCache;

const GLYPH_RASTER_CACHE_CAPACITY: usize = 512;
/// Maximum retained raster bytes (RGBA payloads + key/overhead charge).
const GLYPH_RASTER_CACHE_MAX_BYTES: usize = 32 * 1024 * 1024;
/// Maximum retained chart-text layout bytes (SVG payloads + key charge).
const CHART_TEXT_LAYOUT_CACHE_MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GlyphRasterKey {
    text: String,
    font_size: u32,
    color: [u32; 4],
    rotation: u32,
    letter_spacing: u32,
}

impl GlyphRasterKey {
    fn from_lookup(lookup: GlyphRasterLookup<'_>) -> Self {
        Self {
            text: lookup.text.into(),
            font_size: lookup.font_size,
            color: lookup.color,
            rotation: lookup.rotation,
            letter_spacing: lookup.letter_spacing,
        }
    }

    fn matches(&self, lookup: &GlyphRasterLookup<'_>) -> bool {
        self.text == lookup.text
            && self.font_size == lookup.font_size
            && self.color == lookup.color
            && self.rotation == lookup.rotation
            && self.letter_spacing == lookup.letter_spacing
    }
}

/// Borrowed cache lookup. The cache stores owned text only on a miss, avoiding
/// a per-paint `String` allocation after glyphs have warmed up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphRasterLookup<'a> {
    text: &'a str,
    font_size: u32,
    color: [u32; 4],
    rotation: u32,
    letter_spacing: u32,
}

impl<'a> GlyphRasterLookup<'a> {
    fn new(text: &'a str, config: &GlyphTextConfig) -> Self {
        Self {
            text,
            font_size: config.font_size.to_bits(),
            color: [
                config.color.r.to_bits(),
                config.color.g.to_bits(),
                config.color.b.to_bits(),
                config.color.a.to_bits(),
            ],
            rotation: config.rotation.to_bits(),
            letter_spacing: config.letter_spacing.to_bits(),
        }
    }
}

#[derive(Clone)]
struct CachedRaster {
    image: Arc<RenderImage>,
    width: u32,
    height: u32,
    paint_offset: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ChartTextKey {
    raster: GlyphRasterKey,
    horizontal_anchor: u8,
    vertical_anchor: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ChartTextLookup<'a> {
    raster: GlyphRasterLookup<'a>,
    horizontal_anchor: u8,
    vertical_anchor: u8,
}

impl ChartTextKey {
    fn from_lookup(lookup: ChartTextLookup<'_>) -> Self {
        Self {
            raster: GlyphRasterKey::from_lookup(lookup.raster),
            horizontal_anchor: lookup.horizontal_anchor,
            vertical_anchor: lookup.vertical_anchor,
        }
    }

    fn matches(&self, lookup: &ChartTextLookup<'_>) -> bool {
        self.raster.matches(&lookup.raster)
            && self.horizontal_anchor == lookup.horizontal_anchor
            && self.vertical_anchor == lookup.vertical_anchor
    }
}

impl<'a> ChartTextLookup<'a> {
    fn new(
        text: &'a str,
        config: &GlyphTextConfig,
        horizontal_anchor: HorizontalTextAnchor,
        vertical_anchor: VerticalTextAnchor,
    ) -> Self {
        Self {
            raster: GlyphRasterLookup::new(text, config),
            horizontal_anchor: horizontal_anchor as u8,
            vertical_anchor: vertical_anchor as u8,
        }
    }
}

fn cache_hash(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

type ChartTextLayoutCache = BucketCache<ChartTextKey, Arc<ChartTextLayout>>;
type GlyphRasterCache = BucketCache<GlyphRasterKey, CachedRaster>;

fn raster_charge(key: &GlyphRasterKey, raster: &CachedRaster) -> usize {
    // RGBA payload + owned key text + entry overhead.
    (raster.width as usize)
        .saturating_mul(raster.height as usize)
        .saturating_mul(4)
        .saturating_add(key.text.len())
        .saturating_add(128)
}

fn layout_charge(key: &ChartTextKey, layout: &Arc<ChartTextLayout>) -> usize {
    layout
        .svg
        .len()
        .saturating_add(layout.cache_key.len())
        .saturating_add(key.raster.text.len())
        .saturating_add(128)
}

thread_local! {
    /// GPUI paints on one UI thread. Keeping the cache thread-local avoids a
    /// lock in tick-heavy axis and legend painting while retaining the image
    /// allocation across ordinary rerenders.
    static GLYPH_RASTER_CACHE: RefCell<GlyphRasterCache> = RefCell::new(
        BucketCache::with_budgets(GLYPH_RASTER_CACHE_CAPACITY, GLYPH_RASTER_CACHE_MAX_BYTES),
    );
    static CHART_TEXT_LAYOUT_CACHE: RefCell<ChartTextLayoutCache> = RefCell::new(
        BucketCache::with_budgets(GLYPH_RASTER_CACHE_CAPACITY, CHART_TEXT_LAYOUT_CACHE_MAX_BYTES),
    );
}

#[allow(
    clippy::too_many_arguments,
    reason = "text painter takes window context, text, position, style, and anchors"
)]
pub fn paint_chart_text_at(
    window: &mut gpui::Window,
    cx: &gpui::App,
    text: &str,
    x: f32,
    y: f32,
    config: &GlyphTextConfig,
    horizontal_anchor: HorizontalTextAnchor,
    vertical_anchor: VerticalTextAnchor,
) {
    let layout = cached_chart_text_layout(text, config, horizontal_anchor, vertical_anchor);
    let bounds = gpui::Bounds {
        origin: gpui::point(px(x - layout.anchor[0]), px(y - layout.anchor[1])),
        size: gpui::size(px(layout.width.max(1.0)), px(layout.height.max(1.0))),
    };
    let _ = window.paint_svg(
        bounds,
        layout.cache_key.clone(),
        Some(layout.svg.as_bytes()),
        TransformationMatrix::unit(),
        Hsla::from(config.color),
        cx,
    );
}

fn cached_chart_text_layout(
    text: &str,
    config: &GlyphTextConfig,
    horizontal_anchor: HorizontalTextAnchor,
    vertical_anchor: VerticalTextAnchor,
) -> Arc<ChartTextLayout> {
    let lookup = ChartTextLookup::new(text, config, horizontal_anchor, vertical_anchor);
    let hash = cache_hash(&lookup);
    CHART_TEXT_LAYOUT_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(layout) = cache
            .buckets
            .get(&hash)
            .and_then(|entries| entries.iter().find(|(key, _)| key.matches(&lookup)))
            .map(|(_, layout)| Arc::clone(layout))
        {
            cache.hits = cache.hits.saturating_add(1);
            return layout;
        }
        cache.misses = cache.misses.saturating_add(1);
        let layout = Arc::new(chart_text_layout(
            text,
            config,
            horizontal_anchor,
            vertical_anchor,
        ));
        let key = ChartTextKey::from_lookup(lookup);
        let charge = layout_charge(&key, &layout);
        cache.insert(hash, key, Arc::clone(&layout), charge, layout_charge);
        layout
    })
}

fn cached_raster(text: &str, config: &GlyphTextConfig) -> CachedRaster {
    let lookup = GlyphRasterLookup::new(text, config);
    let hash = cache_hash(&lookup);
    GLYPH_RASTER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(raster) = cache
            .buckets
            .get(&hash)
            .and_then(|entries| entries.iter().find(|(key, _)| key.matches(&lookup)))
            .map(|(_, raster)| raster.clone())
        {
            cache.hits = cache.hits.saturating_add(1);
            return raster;
        }
        cache.misses = cache.misses.saturating_add(1);

        let raster = rasterize_rotated_text(
            text,
            config,
            HorizontalTextAnchor::Start,
            VerticalTextAnchor::Top,
        );
        let entry = CachedRaster {
            width: raster.width,
            height: raster.height,
            paint_offset: raster.paint_offset,
            image: Arc::new(RenderImage::new(vec![Frame::new(
                RgbaImage::from_raw(raster.width, raster.height, raster.pixels)
                    .expect("glyph raster dimensions match its pixel buffer"),
            )])),
        };
        let key = GlyphRasterKey::from_lookup(lookup);
        let charge = raster_charge(&key, &entry);
        cache.insert(hash, key, entry.clone(), charge, raster_charge);
        entry
    })
}

pub fn paint_glyph_text_at(
    window: &mut gpui::Window,
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
    color: impl Into<Rgba>,
    rotation: f32,
) {
    let config = GlyphTextConfig::rotated(font_size, color, rotation);
    let raster = cached_raster(text, &config);
    let bounds = gpui::Bounds {
        origin: gpui::point(
            px(x + raster.paint_offset[0]),
            px(y + raster.paint_offset[1]),
        ),
        size: gpui::size(
            px(raster.width.max(1) as f32),
            px(raster.height.max(1) as f32),
        ),
    };
    let _ = window.paint_image(bounds, bounds, Corners::default(), raster.image, 0, false);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "profiler")]
    use gpui_profiler::{AllocProbe, AllocationBudget};
    #[cfg(feature = "profiler")]
    use std::hint::black_box;

    #[test]
    fn repeated_glyphs_reuse_the_same_raster_image() {
        GLYPH_RASTER_CACHE.with(|cache| cache.borrow_mut().clear());
        let config = GlyphTextConfig::default();
        let first = cached_raster("cached tick", &config);
        let second = cached_raster("cached tick", &config);

        assert!(Arc::ptr_eq(&first.image, &second.image));
        GLYPH_RASTER_CACHE.with(|cache| assert_eq!(cache.borrow().len(), 1));
    }

    #[test]
    fn repeated_chart_labels_reuse_their_svg_layout() {
        CHART_TEXT_LAYOUT_CACHE.with(|cache| cache.borrow_mut().clear());
        let config = GlyphTextConfig::default();
        let first = cached_chart_text_layout(
            "1 kHz",
            &config,
            HorizontalTextAnchor::Middle,
            VerticalTextAnchor::Top,
        );
        let second = cached_chart_text_layout(
            "1 kHz",
            &config,
            HorizontalTextAnchor::Middle,
            VerticalTextAnchor::Top,
        );

        assert_eq!(first.svg, second.svg);
        CHART_TEXT_LAYOUT_CACHE.with(|cache| assert_eq!(cache.borrow().len(), 1));
    }

    #[cfg(feature = "profiler")]
    #[test]
    fn warmed_glyph_and_chart_text_caches_are_allocation_free() {
        GLYPH_RASTER_CACHE.with(|cache| cache.borrow_mut().clear());
        CHART_TEXT_LAYOUT_CACHE.with(|cache| cache.borrow_mut().clear());
        let config = GlyphTextConfig::default();

        black_box(cached_raster("cached tick", &config));
        black_box(cached_chart_text_layout(
            "1 kHz",
            &config,
            HorizontalTextAnchor::Middle,
            VerticalTextAnchor::Top,
        ));

        let mut probe = AllocProbe::new();
        probe.reset();
        black_box(cached_raster("cached tick", &config));
        black_box(cached_chart_text_layout(
            "1 kHz",
            &config,
            HorizontalTextAnchor::Middle,
            VerticalTextAnchor::Top,
        ));
        AllocationBudget::zero("d3rs-warmed-glyph-and-chart-text-cache")
            .assert_contains(probe.sample("d3rs-warmed-glyph-and-chart-text-cache"));
    }
}
