use super::au_text_system_state::AuTextSystemState;
use super::font::font_style_to_fontkit;
use super::font::font_weight_to_fontkit;
use super::misc::lenient_font_attributes;
use super::misc::metrics_to_font_metrics;
use super::misc::rectf_to_bounds_f32;
use super::misc::vec2f_to_size_f32;
use super::types::FontKey;
use font_kit::{source::SystemSource, sources::mem::MemSource};
use gpui::{
    Bounds, DevicePixels, Font, FontId, FontMetrics, FontRun, GlyphId, LineLayout, Pixels,
    PlatformTextSystem, RenderGlyphParams, Result, Size, TextRenderingMode,
};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::{borrow::Cow, char, sync::Arc};

pub struct AuTextSystem(pub(super) RwLock<AuTextSystemState>);

impl AuTextSystem {
    pub fn new() -> Self {
        Self(RwLock::new(AuTextSystemState {
            memory_source: MemSource::empty(),
            system_source: SystemSource::new(),
            fonts: Vec::new(),
            font_selections: HashMap::default(),
            font_ids_by_postscript_name: HashMap::default(),
            font_ids_by_font_key: HashMap::default(),
            postscript_names_by_font_id: HashMap::default(),
            is_emoji: Vec::new(),
            layout_cache: HashMap::default(),
        }))
    }
}

impl Default for AuTextSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformTextSystem for AuTextSystem {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        self.0.write().add_fonts(fonts)
    }

    fn all_font_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        let collection = core_text::font_collection::create_for_all_families();
        let Some(descriptors) = collection.get_descriptors() else {
            return names;
        };
        for descriptor in &descriptors {
            names.extend(lenient_font_attributes::family_name(&descriptor));
        }
        if let Ok(fonts_in_memory) = self.0.read().memory_source.all_families() {
            names.extend(fonts_in_memory);
        }
        names
    }

    fn font_id(&self, descriptor: &Font) -> Result<FontId> {
        let lock = self.0.upgradable_read();
        if let Some(font_id) = lock.font_selections.get(descriptor) {
            Ok(*font_id)
        } else {
            let mut lock = RwLockUpgradableReadGuard::upgrade(lock);
            let font_key = Arc::new(FontKey {
                family: descriptor.family.clone(),
                features: descriptor.features.clone(),
                fallbacks: descriptor.fallbacks.clone(),
            });
            let candidates: &SmallVec<[FontId; 4]> =
                if let Some(font_ids) = lock.font_ids_by_font_key.get(&font_key) {
                    font_ids
                } else {
                    let font_ids = lock.load_family(
                        &descriptor.family,
                        &descriptor.features,
                        descriptor.fallbacks.as_ref(),
                    )?;
                    lock.font_ids_by_font_key
                        .insert(Arc::clone(&font_key), font_ids);
                    lock.font_ids_by_font_key.get(&font_key).unwrap()
                };
            let candidate_properties: SmallVec<[font_kit::properties::Properties; 4]> = candidates
                .iter()
                .map(|font_id| lock.fonts[font_id.0].properties())
                .collect();
            let ix = font_kit::matching::find_best_match(
                &candidate_properties,
                &font_kit::properties::Properties {
                    style: font_style_to_fontkit(descriptor.style),
                    weight: font_weight_to_fontkit(descriptor.weight),
                    stretch: Default::default(),
                },
            )?;
            let font_id = candidates[ix];
            lock.font_selections.insert(descriptor.clone(), font_id);
            Ok(font_id)
        }
    }

    fn font_metrics(&self, font_id: FontId) -> FontMetrics {
        metrics_to_font_metrics(self.0.read().fonts[font_id.0].metrics())
    }

    fn typographic_bounds(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Bounds<f32>> {
        Ok(rectf_to_bounds_f32(
            self.0.read().fonts[font_id.0].typographic_bounds(glyph_id.0)?,
        ))
    }

    fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Size<f32>> {
        let lock = self.0.read();
        let advance = lock.fonts[font_id.0].advance(glyph_id.0)?;
        Ok(vec2f_to_size_f32(advance))
    }

    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        self.0.read().glyph_for_char(font_id, ch)
    }

    fn glyph_raster_bounds(&self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        self.0.read().raster_bounds(params)
    }

    fn rasterize_glyph(
        &self,
        params: &RenderGlyphParams,
        raster_bounds: Bounds<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        self.0.read().rasterize_glyph(params, raster_bounds)
    }

    fn rasterize_glyph_into(
        &self,
        params: &RenderGlyphParams,
        raster_bounds: Bounds<DevicePixels>,
        output: &mut Vec<u8>,
    ) -> Result<Size<DevicePixels>> {
        self.0
            .read()
            .rasterize_glyph_into(params, raster_bounds, output)
    }

    fn layout_line(&self, text: &str, font_size: Pixels, runs: &[FontRun]) -> Arc<LineLayout> {
        // Keep an upgradable read lock across the cache lookup. A miss can then
        // promote the same lock instead of dropping a read lock and queuing for
        // an exclusive lock while another caller fills the entry.
        let lock = self.0.upgradable_read();
        if let Some(layout) = lock.cached_layout_line(text, font_size, runs) {
            return layout;
        }
        RwLockUpgradableReadGuard::upgrade(lock).layout_line(text, font_size, runs)
    }

    fn recommended_rendering_mode(
        &self,
        _font_id: FontId,
        _font_size: Pixels,
    ) -> TextRenderingMode {
        TextRenderingMode::Grayscale
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Font, FontFeatures, FontStyle, FontWeight};

    fn test_font(family: &str) -> Font {
        Font {
            family: family.to_string().into(),
            features: FontFeatures::default(),
            fallbacks: None,
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
        }
    }

    #[test]
    fn test_font_id_caches_selection() {
        let system = AuTextSystem::new();
        let font = test_font(".AppleSystemUIFont");
        let id1 = system.font_id(&font).unwrap();
        let id2 = system.font_id(&font).unwrap();
        assert_eq!(id1, id2);
        assert!(system.0.read().font_selections.contains_key(&font));
    }
}
