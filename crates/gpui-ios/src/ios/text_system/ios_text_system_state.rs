use super::font;
use super::misc::apply_features_and_fallbacks;
use super::misc::kCGImageAlphaOnly;
use super::misc::recti_to_bounds_device_pixels;
use super::string_index_converter::StringIndexConverter;
use super::types::FontKey;
use anyhow::anyhow;
use core_foundation::{
    attributed_string::CFMutableAttributedString,
    base::{CFRange, TCFType},
    number::CFNumber,
    string::CFString,
};
use core_graphics::{
    base::{CGFloat, CGGlyph, kCGImageAlphaPremultipliedLast},
    color_space::CGColorSpace,
    context::{CGContext, CGTextDrawingMode},
    geometry::CGPoint,
};
use core_text::{
    font::CTFont,
    font_descriptor::{
        kCTFontSlantTrait, kCTFontSymbolicTrait, kCTFontWeightTrait, kCTFontWidthTrait,
    },
    line::CTLine,
    string_attributes::kCTFontAttributeName,
};
use font_kit::{
    font::Font as FontKitFont, handle::Handle, hinting::HintingOptions, source::SystemSource,
    sources::mem::MemSource,
};
use gpui::{
    Bounds, DevicePixels, Font, FontFallbacks, FontFeatures, FontId, FontRun, GlyphId, LineLayout,
    Pixels, RenderGlyphParams, Result, SUBPIXEL_VARIANTS_X, ShapedGlyph, ShapedRun, Size, point,
    px, size,
};
use pathfinder_geometry::transform2d::Transform2F;
use smallvec::SmallVec;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::{borrow::Borrow, borrow::Cow, char, sync::Arc};

thread_local! {
    /// Reusable bitmap scratch buffer for glyph rasterization.
    static GLYPH_BITMAP_SCRATCH: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    /// Reusable Core Graphics context for text glyphs (grayscale).
    static GLYPH_TEXT_CONTEXT_CACHE: RefCell<Option<CachedContext>> = const { RefCell::new(None) };
    /// Reusable Core Graphics context for emoji glyphs (RGBA).
    static GLYPH_EMOJI_CONTEXT_CACHE: RefCell<Option<CachedContext>> = const { RefCell::new(None) };
    /// Test-only counter for how many bitmap contexts have been created.
    static GLYPH_CONTEXT_CREATE_COUNT: Cell<usize> = const { Cell::new(0) };
}

struct CachedContext {
    context: CGContext,
    width: usize,
    height: usize,
    bytes_per_row: usize,
}

#[derive(Clone, Debug)]
pub(super) struct LayoutCacheKey {
    text: Arc<str>,
    font_size: Pixels,
    runs: SmallVec<[FontRun; 2]>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(super) struct LayoutCacheKeyRef<'a> {
    text: &'a str,
    font_size: Pixels,
    runs: &'a [FontRun],
}

pub(super) trait AsLayoutCacheKeyRef {
    fn as_layout_cache_key_ref(&self) -> LayoutCacheKeyRef<'_>;
}

impl AsLayoutCacheKeyRef for LayoutCacheKey {
    fn as_layout_cache_key_ref(&self) -> LayoutCacheKeyRef<'_> {
        LayoutCacheKeyRef {
            text: &self.text,
            font_size: self.font_size,
            runs: &self.runs,
        }
    }
}

impl AsLayoutCacheKeyRef for LayoutCacheKeyRef<'_> {
    fn as_layout_cache_key_ref(&self) -> LayoutCacheKeyRef<'_> {
        *self
    }
}

impl PartialEq for dyn AsLayoutCacheKeyRef + '_ {
    fn eq(&self, other: &dyn AsLayoutCacheKeyRef) -> bool {
        self.as_layout_cache_key_ref() == other.as_layout_cache_key_ref()
    }
}

impl Eq for dyn AsLayoutCacheKeyRef + '_ {}

impl Hash for dyn AsLayoutCacheKeyRef + '_ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_layout_cache_key_ref().hash(state);
    }
}

impl PartialEq for LayoutCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.as_layout_cache_key_ref() == other.as_layout_cache_key_ref()
    }
}

impl Eq for LayoutCacheKey {}

impl Hash for LayoutCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_layout_cache_key_ref().hash(state);
    }
}

impl<'a> Borrow<dyn AsLayoutCacheKeyRef + 'a> for LayoutCacheKey {
    fn borrow(&self) -> &(dyn AsLayoutCacheKeyRef + 'a) {
        self as &dyn AsLayoutCacheKeyRef
    }
}

pub(super) struct IosTextSystemState {
    pub(super) memory_source: MemSource,
    pub(super) system_source: SystemSource,
    pub(super) fonts: Vec<FontKitFont>,
    pub(super) font_selections: HashMap<Font, FontId>,
    pub(super) font_ids_by_postscript_name: HashMap<String, FontId>,
    pub(super) font_ids_by_font_key: HashMap<FontKey, Arc<[FontId]>>,
    pub(super) postscript_names_by_font_id: HashMap<FontId, String>,
    pub(super) layout_cache: HashMap<LayoutCacheKey, Arc<LineLayout>>,
}

impl IosTextSystemState {
    pub(super) fn add_fonts(&mut self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        let fonts = fonts
            .into_iter()
            .map(|bytes| match bytes {
                Cow::Borrowed(embedded_font) => {
                    let data_provider = unsafe {
                        core_graphics::data_provider::CGDataProvider::from_slice(embedded_font)
                    };
                    let font = core_graphics::font::CGFont::from_data_provider(data_provider)
                        .map_err(|()| anyhow!("Could not load an embedded font."))?;
                    let font = font_kit::loaders::core_text::Font::from_core_graphics_font(font);
                    Ok(Handle::from_native(&font))
                }
                Cow::Owned(bytes) => Ok(Handle::from_memory(Arc::new(bytes), 0)),
            })
            .collect::<Result<Vec<_>>>()?;
        self.memory_source.add_fonts(fonts.into_iter())?;
        Ok(())
    }

    pub(super) fn load_family(
        &mut self,
        name: &str,
        features: &FontFeatures,
        fallbacks: Option<&FontFallbacks>,
    ) -> Result<Arc<[FontId]>> {
        let mut font_ids: SmallVec<[FontId; 4]> = SmallVec::new();
        // Map virtual font names to iOS equivalents.
        // gpui uses ".SystemUIFont", gpui-ui-kit theme uses ".SystemUI" — both
        // must resolve to the iOS system font (.AppleSystemUIFont = San Francisco).
        let name = gpui::font_name_with_fallbacks(name, ".AppleSystemUIFont");
        let name = match name {
            ".SystemUI" => ".AppleSystemUIFont",
            _ => name,
        };
        let family = self
            .memory_source
            .select_family_by_name(name)
            .or_else(|_| self.system_source.select_family_by_name(name))?;
        for font in family.fonts() {
            let mut font = font.load()?;
            apply_features_and_fallbacks(&mut font, features, fallbacks)?;
            {
                let has_m_glyph = font.glyph_for_char('m').is_some();
                let is_segoe_fluent_icons = font.full_name() == "Segoe Fluent Icons";
                if !has_m_glyph && !is_segoe_fluent_icons {
                    log::warn!(
                        "font '{}' has no 'm' character and was not loaded",
                        font.full_name()
                    );
                    continue;
                }
            }
            let traits = font.native_font().all_traits();
            if unsafe {
                !(traits
                    .get(kCTFontSymbolicTrait)
                    .downcast::<CFNumber>()
                    .is_some()
                    && traits
                        .get(kCTFontWidthTrait)
                        .downcast::<CFNumber>()
                        .is_some()
                    && traits
                        .get(kCTFontWeightTrait)
                        .downcast::<CFNumber>()
                        .is_some()
                    && traits
                        .get(kCTFontSlantTrait)
                        .downcast::<CFNumber>()
                        .is_some())
            } {
                log::error!(
                    "Failed to read traits for font {}",
                    font.postscript_name().as_deref().unwrap_or("<unknown>")
                );
                continue;
            }
            let font_id = FontId(self.fonts.len());
            font_ids.push(font_id);
            if let Some(postscript_name) = font.postscript_name() {
                self.font_ids_by_postscript_name
                    .insert(postscript_name.clone(), font_id);
                self.postscript_names_by_font_id
                    .insert(font_id, postscript_name);
            } else {
                log::warn!(
                    "Font '{}' has no PostScript name; skipping name-based lookups",
                    font.full_name()
                );
            }
            self.fonts.push(font);
        }
        Ok(Arc::from(font_ids.as_slice()))
    }

    pub(super) fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        self.fonts[font_id.0].glyph_for_char(ch).map(GlyphId)
    }

    pub(super) fn id_for_native_font(&mut self, requested_font: CTFont) -> FontId {
        let postscript_name = requested_font.postscript_name();
        if let Some(font_id) = self
            .font_ids_by_postscript_name
            .get(postscript_name.as_str())
        {
            *font_id
        } else {
            let font_id = FontId(self.fonts.len());
            self.font_ids_by_postscript_name
                .insert(postscript_name.clone(), font_id);
            self.postscript_names_by_font_id
                .insert(font_id, postscript_name);
            self.fonts
                .push(font_kit::font::Font::from_core_graphics_font(
                    requested_font.copy_to_CGFont(),
                ));
            font_id
        }
    }

    pub(super) fn is_emoji(&self, font_id: FontId) -> bool {
        self.postscript_names_by_font_id
            .get(&font_id)
            .is_some_and(|postscript_name| {
                postscript_name == "AppleColorEmoji" || postscript_name == ".AppleColorEmojiUI"
            })
    }

    pub(super) fn raster_bounds(&self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        let font = &self.fonts[params.font_id.0];
        let scale = Transform2F::from_scale(params.scale_factor);
        Ok(recti_to_bounds_device_pixels(font.raster_bounds(
            params.glyph_id.0,
            params.font_size.into(),
            scale,
            HintingOptions::None,
            font_kit::canvas::RasterizationOptions::GrayscaleAa,
        )?))
    }

    pub(super) fn rasterize_glyph(
        &self,
        params: &RenderGlyphParams,
        glyph_bounds: Bounds<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        if glyph_bounds.size.width.0 == 0 || glyph_bounds.size.height.0 == 0 {
            anyhow::bail!("glyph bounds are empty");
        }
        let mut bitmap_size = glyph_bounds.size;
        if params.subpixel_variant.x > 0 {
            bitmap_size.width += DevicePixels(1);
        }
        if params.subpixel_variant.y > 0 {
            bitmap_size.height += DevicePixels(1);
        }
        let bitmap_size = bitmap_size;
        let needed = if params.is_emoji {
            bitmap_size.width.0 as usize * 4 * bitmap_size.height.0 as usize
        } else {
            bitmap_size.width.0 as usize * bitmap_size.height.0 as usize
        };
        let mut bitmap = Vec::with_capacity(needed);
        // SAFETY: every byte in [0, needed) is initialized by the
        // copy_from_slice calls below before the Vec is returned.
        unsafe { bitmap.set_len(needed) };

        let req_width = bitmap_size.width.0 as usize;
        let req_height = bitmap_size.height.0 as usize;
        let is_emoji = params.is_emoji;

        GLYPH_BITMAP_SCRATCH.with(|scratch| {
            let mut scratch = scratch.borrow_mut();
            scratch.resize(needed, 0);

            let (color_space, alpha_info, out_bytes_per_row) = if is_emoji {
                (
                    CGColorSpace::create_device_rgb(),
                    kCGImageAlphaPremultipliedLast,
                    req_width * 4,
                )
            } else {
                (
                    CGColorSpace::create_device_gray(),
                    kCGImageAlphaOnly,
                    req_width,
                )
            };

            // Reuse an existing context if it is at least as large as the
            // requested bitmap; otherwise create a new one sized exactly to the
            // current glyph. Text and emoji use separate caches so interleaved
            // glyph types do not evict each other.
            let cache = if is_emoji {
                &GLYPH_EMOJI_CONTEXT_CACHE
            } else {
                &GLYPH_TEXT_CONTEXT_CACHE
            };
            cache.with(|c| {
                let mut c = c.borrow_mut();
                let fits = c
                    .as_ref()
                    .is_some_and(|c| c.width >= req_width && c.height >= req_height);
                if !fits {
                    let context = CGContext::create_bitmap_context(
                        Some(scratch.as_mut_ptr() as *mut _),
                        req_width,
                        req_height,
                        8,
                        out_bytes_per_row,
                        &color_space,
                        alpha_info,
                    );
                    *c = Some(CachedContext {
                        context,
                        width: req_width,
                        height: req_height,
                        bytes_per_row: out_bytes_per_row,
                    });
                    GLYPH_CONTEXT_CREATE_COUNT.with(|c| c.set(c.get() + 1));
                }
            });

            cache.with(|c| {
                let cache = c.borrow();
                let cached = cache.as_ref().expect("context cache populated above");

                // The cached context may be larger than the current glyph, so
                // size the scratch buffer to match the cached context and clear
                // it before drawing.
                let cached_bytes = cached.height * cached.bytes_per_row;
                scratch.resize(cached_bytes, 0);
                scratch[..cached_bytes].fill(0);

                let cx = &cached.context;
                cx.translate(
                    -glyph_bounds.origin.x.0 as CGFloat,
                    (glyph_bounds.origin.y.0 + glyph_bounds.size.height.0) as CGFloat,
                );
                cx.scale(
                    params.scale_factor as CGFloat,
                    params.scale_factor as CGFloat,
                );

                let subpixel_shift = params
                    .subpixel_variant
                    .map(|v| v as f32 / SUBPIXEL_VARIANTS_X as f32);
                cx.set_text_drawing_mode(CGTextDrawingMode::CGTextFill);
                cx.set_gray_fill_color(0.0, 1.0);
                cx.set_allows_antialiasing(true);
                cx.set_should_antialias(true);
                cx.set_allows_font_subpixel_positioning(true);
                cx.set_should_subpixel_position_fonts(true);
                cx.set_allows_font_subpixel_quantization(false);
                cx.set_should_subpixel_quantize_fonts(false);
                self.fonts[params.font_id.0]
                    .native_font()
                    .clone_with_font_size(f32::from(params.font_size) as CGFloat)
                    .draw_glyphs(
                        &[params.glyph_id.0 as CGGlyph],
                        &[CGPoint::new(
                            (subpixel_shift.x / params.scale_factor) as CGFloat,
                            (subpixel_shift.y / params.scale_factor) as CGFloat,
                        )],
                        cx.clone(),
                    );

                if is_emoji {
                    for pixel in scratch.chunks_exact_mut(4) {
                        gpui::swap_rgba_pa_to_bgra(pixel);
                    }
                }

                // Copy only the requested sub-rectangle, in case the cached
                // context is larger than the current glyph.
                if cached.width == req_width && cached.bytes_per_row == out_bytes_per_row {
                    bitmap.copy_from_slice(&scratch[..needed]);
                } else {
                    for y in 0..req_height {
                        let src_start = y * cached.bytes_per_row;
                        let dst_start = y * out_bytes_per_row;
                        bitmap[dst_start..dst_start + out_bytes_per_row]
                            .copy_from_slice(&scratch[src_start..src_start + out_bytes_per_row]);
                    }
                }
            });
        });

        Ok((bitmap_size, bitmap))
    }

    pub(super) fn layout_line(
        &mut self,
        text: &str,
        font_size: Pixels,
        font_runs: &[FontRun],
    ) -> LineLayout {
        let key_ref = LayoutCacheKeyRef {
            text,
            font_size,
            runs: font_runs,
        };
        if let Some(cached) = self.layout_cache.get(&key_ref as &dyn AsLayoutCacheKeyRef) {
            return Self::clone_layout(cached);
        }

        let key = LayoutCacheKey {
            text: text.into(),
            font_size,
            runs: font_runs.iter().copied().collect(),
        };
        let result = self.layout_line_uncached(text, font_size, font_runs);
        self.layout_cache
            .insert(key, Arc::new(Self::clone_layout(&result)));
        result
    }

    fn clone_layout(layout: &LineLayout) -> LineLayout {
        LineLayout {
            font_size: layout.font_size,
            width: layout.width,
            ascent: layout.ascent,
            descent: layout.descent,
            runs: layout.runs.clone(),
            len: layout.len,
        }
    }

    fn layout_line_uncached(
        &mut self,
        text: &str,
        font_size: Pixels,
        font_runs: &[FontRun],
    ) -> LineLayout {
        let mut string = CFMutableAttributedString::new();
        let mut max_ascent = 0.0f32;
        let mut max_descent = 0.0f32;
        {
            let mut text = text;
            let mut break_ligature = true;
            for run in font_runs {
                let text_run;
                (text_run, text) = text.split_at(run.len);
                let utf16_start = string.char_len();
                string.replace_str(&CFString::new(text_run), CFRange::init(utf16_start, 0));
                let utf16_end = string.char_len();
                let length = utf16_end - utf16_start;
                let cf_range = CFRange::init(utf16_start, length);
                let font = &self.fonts[run.font_id.0];
                let font_metrics = font.metrics();
                let font_scale = font_size.as_f32() / font_metrics.units_per_em as f32;
                max_ascent = max_ascent.max(font_metrics.ascent * font_scale);
                max_descent = max_descent.max(-font_metrics.descent * font_scale);
                let font_size = if break_ligature {
                    px(font_size.as_f32().next_up())
                } else {
                    font_size
                };
                unsafe {
                    string.set_attribute(
                        cf_range,
                        kCTFontAttributeName,
                        &font.native_font().clone_with_font_size(font_size.into()),
                    );
                }
                break_ligature = !break_ligature;
            }
        }
        let line = CTLine::new_with_attributed_string(string.as_concrete_TypeRef());
        let glyph_runs = line.glyph_runs();
        let mut runs = <Vec<ShapedRun>>::with_capacity(glyph_runs.len() as usize);
        let mut ix_converter = StringIndexConverter::new(text);
        for run in glyph_runs.into_iter() {
            let attributes = run.attributes().unwrap();
            let font = unsafe {
                attributes
                    .get(kCTFontAttributeName)
                    .downcast::<CTFont>()
                    .unwrap()
            };
            let font_id = self.id_for_native_font(font);
            let glyphs = match runs.last_mut() {
                Some(run) if run.font_id == font_id => &mut run.glyphs,
                _ => {
                    runs.push(ShapedRun {
                        font_id,
                        glyphs: Vec::with_capacity(run.glyph_count().try_into().unwrap_or(0)),
                    });
                    &mut runs.last_mut().unwrap().glyphs
                }
            };
            for ((&glyph_id, position), &glyph_utf16_ix) in run
                .glyphs()
                .iter()
                .zip(run.positions().iter())
                .zip(run.string_indices().iter())
            {
                let glyph_utf16_ix = usize::try_from(glyph_utf16_ix).unwrap();
                if ix_converter.utf16_ix > glyph_utf16_ix {
                    ix_converter = StringIndexConverter::new(text);
                }
                ix_converter.advance_to_utf16_ix(glyph_utf16_ix);
                glyphs.push(ShapedGlyph {
                    id: GlyphId(glyph_id as u32),
                    position: point(position.x as f32, position.y as f32).map(px),
                    index: ix_converter.utf8_ix,
                    is_emoji: self.is_emoji(font_id),
                });
            }
        }
        let typographic_bounds = line.get_typographic_bounds();
        LineLayout {
            runs,
            font_size,
            width: typographic_bounds.width.into(),
            ascent: max_ascent.into(),
            descent: max_descent.into(),
            len: text.len(),
        }
    }
}

#[cfg(test)]
fn context_create_count() -> usize {
    GLYPH_CONTEXT_CREATE_COUNT.with(|c| c.get())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
