use anyhow::Result;
use collections::HashMap;
use cosmic_text::{FontSystem, ShapeBuffer};
use gpui::{
    Bounds, DevicePixels, Font, FontId, FontMetrics, FontRun, GlyphId, LineLayout, Pixels,
    PlatformTextSystem, RenderGlyphParams, Size, TextRenderingMode, point, size,
};
use itertools::Itertools;
use parking_lot::RwLock;
use std::borrow::Cow;
use std::sync::Arc;
use swash::scale::ScaleContext;
mod cosmic_text_system_state;
mod find;
mod font_key;
mod misc;
mod types;

use cosmic_text_system_state::CosmicTextSystemState;
#[cfg(feature = "font-kit")]
use find::find_best_match;
#[cfg(not(feature = "font-kit"))]
use find::find_best_match;
use font_key::FontKey;

pub struct CosmicTextSystem(RwLock<CosmicTextSystemState>);

impl CosmicTextSystem {
    /// Load system font files through fontdb's file-backed path.
    pub fn add_font_paths<'a>(
        &self,
        paths: impl IntoIterator<Item = &'a std::path::Path>,
    ) -> Result<()> {
        self.0.write().add_font_paths(paths)
    }

    pub fn new(system_font_fallback: &str) -> Self {
        let font_system = FontSystem::new();

        Self(RwLock::new(CosmicTextSystemState {
            font_system,
            scratch: ShapeBuffer::default(),
            swash_scale_context: ScaleContext::new(),
            loaded_fonts: Vec::new(),
            font_ids_by_family_cache: HashMap::default(),
            system_font_fallback: system_font_fallback.to_string(),
        }))
    }

    pub fn new_without_system_fonts(system_font_fallback: &str) -> Self {
        let font_system = FontSystem::new_with_locale_and_db(
            "en-US".to_string(),
            cosmic_text::fontdb::Database::new(),
        );

        Self(RwLock::new(CosmicTextSystemState {
            font_system,
            scratch: ShapeBuffer::default(),
            swash_scale_context: ScaleContext::new(),
            loaded_fonts: Vec::new(),
            font_ids_by_family_cache: HashMap::default(),
            system_font_fallback: system_font_fallback.to_string(),
        }))
    }
}

impl PlatformTextSystem for CosmicTextSystem {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        self.0.write().add_fonts(fonts)
    }

    fn all_font_names(&self) -> Vec<String> {
        let mut result = self
            .0
            .read()
            .font_system
            .db()
            .faces()
            .filter_map(|face| face.families.first().map(|family| family.0.clone()))
            .collect_vec();
        result.sort();
        result.dedup();
        result
    }

    fn font_id(&self, font: &Font) -> Result<FontId> {
        let mut state = self.0.write();
        let key = FontKey::new(font.family.clone(), font.features.clone());
        let candidates = if let Some(font_ids) = state.font_ids_by_family_cache.get(&key) {
            font_ids.as_slice()
        } else {
            let font_ids = state.load_family(&font.family, &font.features)?;
            state.font_ids_by_family_cache.insert(key.clone(), font_ids);
            state.font_ids_by_family_cache[&key].as_ref()
        };

        let ix = find_best_match(font, candidates, &state)?;

        Ok(candidates[ix])
    }

    fn font_metrics(&self, font_id: FontId) -> FontMetrics {
        let metrics = self
            .0
            .read()
            .loaded_font(font_id)
            .font
            .as_swash()
            .metrics(&[]);

        FontMetrics {
            units_per_em: metrics.units_per_em as u32,
            ascent: metrics.ascent,
            descent: -metrics.descent,
            line_gap: metrics.leading,
            underline_position: metrics.underline_offset,
            underline_thickness: metrics.stroke_size,
            cap_height: metrics.cap_height,
            x_height: metrics.x_height,
            bounding_box: Bounds {
                origin: point(0.0, 0.0),
                size: size(metrics.max_width, metrics.ascent + metrics.descent),
            },
        }
    }

    fn typographic_bounds(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Bounds<f32>> {
        let lock = self.0.read();
        let glyph_metrics = lock.loaded_font(font_id).font.as_swash().glyph_metrics(&[]);
        let glyph_id = glyph_id.0 as u16;
        Ok(Bounds {
            origin: point(0.0, 0.0),
            size: size(
                glyph_metrics.advance_width(glyph_id),
                glyph_metrics.advance_height(glyph_id),
            ),
        })
    }

    fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Size<f32>> {
        self.0.read().advance(font_id, glyph_id)
    }

    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        self.0.read().glyph_for_char(font_id, ch)
    }

    fn glyph_raster_bounds(&self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        self.0.write().raster_bounds(params)
    }

    fn rasterize_glyph(
        &self,
        params: &RenderGlyphParams,
        raster_bounds: Bounds<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        self.0.write().rasterize_glyph(params, raster_bounds)
    }

    fn layout_line(&self, text: &str, font_size: Pixels, runs: &[FontRun]) -> Arc<LineLayout> {
        Arc::new(self.0.write().layout_line(text, font_size, runs))
    }

    fn recommended_rendering_mode(
        &self,
        _font_id: FontId,
        _font_size: Pixels,
    ) -> TextRenderingMode {
        TextRenderingMode::Subpixel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{font, px};

    const IBM_PLEX_REGULAR: &[u8] =
        include_bytes!("../../../assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf");
    const IBM_PLEX_SEMIBOLD: &[u8] =
        include_bytes!("../../../assets/fonts/ibm-plex-sans/IBMPlexSans-SemiBold.ttf");
    const LILEX_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/lilex/Lilex-Regular.ttf");

    #[test]
    fn bundled_font_weights_and_mixed_combining_runs_shape() {
        let text_system = CosmicTextSystem::new_without_system_fonts("IBM Plex Sans");
        text_system
            .add_fonts(vec![
                Cow::Borrowed(IBM_PLEX_REGULAR),
                Cow::Borrowed(IBM_PLEX_SEMIBOLD),
                Cow::Borrowed(LILEX_REGULAR),
            ])
            .unwrap();

        let regular = text_system.font_id(&font("IBM Plex Sans")).unwrap();
        let bold = text_system.font_id(&font("IBM Plex Sans").bold()).unwrap();
        let lilex = text_system.font_id(&font("Lilex")).unwrap();
        assert_ne!(
            regular, bold,
            "weight selection must use a distinct font face"
        );
        assert_ne!(
            regular, lilex,
            "family selection must use a distinct font face"
        );

        let first = "Cafe\u{301}";
        let second = " | Lilex";
        let text = format!("{first}{second}");
        let layout = text_system.layout_line(
            &text,
            px(16.0),
            &[
                FontRun {
                    len: first.len(),
                    font_id: regular,
                },
                FontRun {
                    len: second.len(),
                    font_id: lilex,
                },
            ],
        );
        assert_eq!(layout.len, text.len());
        assert!(layout.width > Pixels::ZERO);
        assert!(f32::from(layout.width).is_finite());
        for selected in [regular, lilex] {
            assert!(
                layout
                    .runs
                    .iter()
                    .any(|run| run.font_id == selected && !run.glyphs.is_empty()),
                "selected font {selected:?} must shape visible glyphs"
            );
        }
        assert!(layout.runs.iter().flat_map(|run| &run.glyphs).all(|glyph| {
            text.is_char_boundary(glyph.index)
                && f32::from(glyph.position.x).is_finite()
                && f32::from(glyph.position.y).is_finite()
        }));
    }
}

#[cfg(all(test, feature = "headless-qa"))]
mod paint_tests {
    use super::CosmicTextSystem;
    use crate::WgpuHeadlessRenderer;
    use gpui::{
        AnyWindowHandle, AppContext, Context, FontRun, HeadlessAppContext, IntoElement,
        ParentElement, PlatformTextSystem, Render, Styled, Window, div, font, px, rgb, size,
    };
    use std::{borrow::Cow, env, fs, sync::Arc};

    const WIDTH: u32 = 640;
    const ROW_HEIGHT: u32 = 72;
    const LABELS: [&str; 5] = ["Hello", "e\u{301}cole", "مرحبا", "漢字かな", "😀"];
    const IBM_PLEX_REGULAR: &[u8] =
        include_bytes!("../../../assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf");

    struct FontLines;

    impl Render for FontLines {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut root = div()
                .w(px(WIDTH as f32))
                .h(px((ROW_HEIGHT * LABELS.len() as u32) as f32))
                .bg(rgb(0xffffff))
                .flex_col();
            for label in LABELS {
                root = root.child(
                    div()
                        .h(px(ROW_HEIGHT as f32))
                        .font_family("IBM Plex Sans")
                        .text_size(px(32.0))
                        .text_color(rgb(0x000000))
                        .child(label),
                );
            }
            root
        }
    }

    #[test]
    fn wgpu_paints_latin_combining_rtl_cjk_and_emoji() {
        let text_system = Arc::new(CosmicTextSystem::new("IBM Plex Sans"));
        text_system
            .add_fonts(vec![Cow::Borrowed(IBM_PLEX_REGULAR)])
            .expect("load bundled Latin font");
        let initial_font = text_system
            .font_id(&font("IBM Plex Sans"))
            .expect("select bundled font");
        for label in LABELS {
            let layout = text_system.layout_line(
                label,
                px(32.0),
                &[FontRun {
                    len: label.len(),
                    font_id: initial_font,
                }],
            );
            let glyphs: Vec<_> = layout.runs.iter().flat_map(|run| &run.glyphs).collect();
            assert!(!glyphs.is_empty(), "{label:?} produced no shaped glyphs");
            assert!(
                glyphs.iter().all(|glyph| glyph.id.0 != 0),
                "{label:?} used the missing-glyph ID"
            );
            for ch in label.chars().filter(|ch| !ch.is_whitespace()) {
                assert!(
                    layout
                        .runs
                        .iter()
                        .any(|run| { text_system.glyph_for_char(run.font_id, ch).is_some() }),
                    "{label:?} has no selected font covering {ch:?}"
                );
            }
            let faces: Vec<_> = {
                let state = text_system.0.read();
                layout
                    .runs
                    .iter()
                    .map(|run| {
                        let loaded = state.loaded_font(run.font_id);
                        let face = state
                            .font_system
                            .db()
                            .face(loaded.font.id())
                            .expect("selected font face remains registered");
                        face.families
                            .first()
                            .expect("selected face has a family")
                            .0
                            .clone()
                    })
                    .collect()
            };
            println!(
                "FONT_SHAPE label={label:?} faces={faces:?} glyphs={:?} indices={:?}",
                glyphs.iter().map(|glyph| glyph.id.0).collect::<Vec<_>>(),
                glyphs.iter().map(|glyph| glyph.index).collect::<Vec<_>>()
            );
            if label == "مرحبا" {
                let mut positions: Vec<_> = glyphs
                    .iter()
                    .map(|glyph| (f32::from(glyph.position.x), glyph.index))
                    .collect();
                positions.sort_by(|left, right| left.0.total_cmp(&right.0));
                assert!(positions.len() >= 2);
                assert!(
                    positions.first().unwrap().1 > positions.last().unwrap().1,
                    "Arabic visual order must reverse logical byte order"
                );
            }
        }
        let height = ROW_HEIGHT * LABELS.len() as u32;
        let device_size = gpui::Size {
            width: gpui::DevicePixels(WIDTH as i32),
            height: gpui::DevicePixels(height as i32),
        };
        let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), move || {
            Some(Box::new(
                WgpuHeadlessRenderer::new(device_size, false)
                    .expect("WGPU adapter is required for this paint gate"),
            ))
        });
        let window = cx
            .open_window(size(px(WIDTH as f32), px(height as f32)), |_window, app| {
                app.new(|_| FontLines)
            })
            .expect("open font paint window");
        let handle: AnyWindowHandle = window.into();
        let scale = cx
            .update_window(handle, |_, window, _| window.scale_factor())
            .expect("read headless window scale");
        assert!(scale.is_finite() && scale > 0.0);
        let physical_width = (WIDTH as f32 * scale).round() as u32;
        let physical_row_height = (ROW_HEIGHT as f32 * scale).round() as u32;
        for _ in 0..2 {
            cx.update_window(handle, |_, window, app| {
                let _ = window.draw(app);
            })
            .expect("draw font paint window");
            cx.run_until_parked();
        }
        let image = cx
            .capture_screenshot(handle)
            .expect("capture WGPU text pixels");
        assert_eq!(
            image.dimensions(),
            (physical_width, physical_row_height * LABELS.len() as u32)
        );
        for (row, label) in LABELS.iter().enumerate() {
            let row_pixels: Vec<_> = image
                .enumerate_pixels()
                .filter(|(_, y, _)| (*y / physical_row_height) == row as u32)
                .map(|(_, _, pixel)| pixel)
                .collect();
            assert_eq!(
                row_pixels.len(),
                (physical_width * physical_row_height) as usize
            );
            let painted = row_pixels
                .iter()
                .filter(|pixel| pixel.0[..3].iter().any(|channel| *channel < 245))
                .count();
            let background = row_pixels
                .iter()
                .filter(|pixel| pixel.0[..3].iter().all(|channel| *channel >= 245))
                .count();
            assert!(
                background > row_pixels.len() / 2,
                "{label:?} has no painted white panel"
            );
            assert!(
                painted > 20,
                "{label:?} produced no visible WGPU text pixels"
            );
        }
        if let Ok(dir) = env::var("GPUI_FONT_CAPTURE_DIR") {
            fs::create_dir_all(&dir).expect("create font capture directory");
            image
                .save(format!("{dir}/wgpu-font-scripts.png"))
                .expect("save font paint evidence");
        }
        cx.update_window(handle, |_, window, _| window.remove_window())
            .expect("close font paint window");
        cx.run_until_parked();
    }
}
