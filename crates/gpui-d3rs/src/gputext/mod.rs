//! Deterministic GPU-text core: shaped runs over bundled fonts.
//!
//! Implements Phase 1 of `reviews/20260906-gpu-text.md`. [`FontEngine`] owns
//! a parley [`LayoutContext`] and a fontique `Collection` seeded **only**
//! (code span, not a link: fontique is an optional dependency and this
//! module documents the no-default-features build too).
//! from the embedded DejaVu bytes below — system font enumeration is never
//! enabled, so shaping is identical on macOS, Linux, Windows, CI, and wasm.
//! [`FontEngine::shape`] positions one line of text into owned [`ShapedRun`]s;
//! Phase 2 encodes those runs into the vello (GPU) and vello_cpu backends,
//! Phase 4 bakes them into 3D SDF billboards.
//!
//! Font assets (`DejaVuSans.ttf`, `DejaVuSans-Bold.ttf`, `DejaVu-LICENSE`)
//! are the official 2.37 release; the pre-existing `DejaVuSansMono.ttf` is
//! reused unchanged.

#[cfg(feature = "vello")]
use std::sync::Arc;

#[cfg(feature = "vello")]
use fontique::Blob;
#[cfg(feature = "vello")]
use parley::layout::PositionedLayoutItem;
#[cfg(feature = "vello")]
use parley::style::{FontFamily, FontWeight};
#[cfg(feature = "vello")]
use parley::{FontContext, FontData, Layout, LayoutContext, StyleProperty};

/// Re-exported so chart callers name weights without depending on parley.
#[cfg(feature = "vello")]
pub use parley::style::FontWeight as TextWeight;

/// 3D billboard atlas over the same bundled bytes (jump-flooded SDF).
#[cfg(feature = "gpu-3d")]
pub mod sdf;

/// World-anchored SDF billboard pass shared by the wgpu 3D renderers.
///
/// `not(test)` like [`crate::gpu3d`]: the pass reuses the screen-text anchor
/// types from [`crate::text`], which only exists in non-test builds. CPU-side
/// layout is covered by `tests/billboard_tests.rs` instead.
#[cfg(all(feature = "gpu-3d", not(test)))]
pub mod billboard;

/// Bundled DejaVu Sans Regular (proportional chart text, SDF source).
#[cfg(any(feature = "vello", feature = "gpu-3d"))]
pub(crate) static SANS_TTF: &[u8] = include_bytes!("../../assets/DejaVuSans.ttf");
/// Bundled DejaVu Sans Bold (titles, emphasized ticks).
#[cfg(feature = "vello")]
static SANS_BOLD_TTF: &[u8] = include_bytes!("../../assets/DejaVuSans-Bold.ttf");
/// Bundled DejaVu Sans Mono (numeric columns, pre-existing asset).
#[cfg(feature = "vello")]
static MONO_TTF: &[u8] = include_bytes!("../../assets/DejaVuSansMono.ttf");

/// Family name the bundled Sans registers under.
#[cfg(feature = "vello")]
pub const FAMILY_SANS: &str = "DejaVu Sans";
/// Family name the bundled Mono registers under.
#[cfg(feature = "vello")]
pub const FAMILY_MONO: &str = "DejaVu Sans Mono";

#[cfg(feature = "vello")]
/// One fully positioned glyph: font-space id plus baseline-relative offset.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedGlyph {
    /// Glyph id within [`ShapedRun::font`] (0 is `.notdef` — never emitted).
    pub id: u32,
    /// Horizontal offset in px from the run origin.
    pub x: f32,
    /// Vertical offset in px from the baseline (positive down).
    pub y: f32,
    /// Advance in px consumed along the baseline.
    pub advance: f32,
}

#[cfg(feature = "vello")]
/// One style-uniform run of positioned glyphs, ready for backend encoding.
///
/// All fields are `Clone + Send + Sync` so runs can cross the scene-build /
/// replay boundary and thread boundaries unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedRun {
    /// Resolved font file backing every glyph in [`ShapedRun::glyphs`].
    pub font: FontData,
    /// Requested size in px.
    pub size: f32,
    /// Total advance in px along the baseline.
    pub advance: f32,
    /// Glyphs in visual order with baseline-relative positions.
    pub glyphs: Vec<ShapedGlyph>,
}

#[cfg(feature = "vello")]
/// Shaping engine: parley layout over a bundled-only fontique collection.
///
/// The engine is deliberately `&mut`-driven (parley's contexts reuse internal
/// caches across calls); shaping itself allocates only the returned runs.
pub struct FontEngine {
    font: FontContext,
    layout: LayoutContext<()>,
}

#[cfg(feature = "vello")]
impl FontEngine {
    /// Build an engine with DejaVu Sans Regular + Bold + Mono registered.
    ///
    /// System fonts are never loaded: every run resolves to a bundled file.
    #[must_use]
    pub fn new() -> Self {
        let mut font = FontContext::new();
        for bytes in [SANS_TTF, SANS_BOLD_TTF, MONO_TTF] {
            font.collection
                .register_fonts(Blob::new(Arc::new(bytes)), None);
        }
        Self {
            font,
            layout: LayoutContext::new(),
        }
    }

    /// Whether `family` resolves in the bundled collection.
    pub fn has_family(&mut self, family: &str) -> bool {
        self.font.collection.family_by_name(family).is_some()
    }

    /// Shape one line of `text` at `size` px into visual-order runs.
    ///
    /// `family` names a bundled family ([`FAMILY_SANS`], [`FAMILY_MONO`]);
    /// `weight` selects the registered face ([`TextWeight::NORMAL`]/
    /// [`TextWeight::BOLD`]). No line breaking is applied — chart labels are
    /// single-line; rotation and placement compose later via affine maps.
    pub fn shape(
        &mut self,
        text: &str,
        size: f32,
        family: &str,
        weight: FontWeight,
    ) -> Vec<ShapedRun> {
        let mut builder = self.layout.ranged_builder(&mut self.font, text, 1.0, false);
        builder.push_default(StyleProperty::FontFamily(FontFamily::named(family)));
        builder.push_default(StyleProperty::FontSize(size));
        builder.push_default(StyleProperty::FontWeight(weight));
        let mut layout = Layout::new();
        builder.build_into(&mut layout, text);
        layout.break_all_lines(None);
        let mut runs = Vec::new();
        for line in layout.lines() {
            for item in line.items() {
                if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                    let run = glyph_run.run();
                    runs.push(ShapedRun {
                        font: run.font().clone(),
                        size: run.font_size(),
                        advance: glyph_run.advance(),
                        glyphs: glyph_run
                            .positioned_glyphs()
                            .map(|glyph| ShapedGlyph {
                                id: glyph.id,
                                x: glyph.x,
                                y: glyph.y,
                                advance: glyph.advance,
                            })
                            .collect(),
                    });
                }
            }
        }
        runs
    }

    /// Total advance in px of already-shaped runs (single-line width).
    #[must_use]
    pub fn line_width(runs: &[ShapedRun]) -> f32 {
        runs.iter().map(|run| run.advance).sum()
    }
}

#[cfg(feature = "vello")]
impl Default for FontEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "vello"))]
mod tests {
    use super::*;

    /// Printable ASCII plus the tick-format charset audited for Phase 4
    /// (`°`, `$`, `-`, `.`, `,` — month/flare names arrive with billboards).
    const TICK_CHARSET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~°";

    fn engine() -> FontEngine {
        FontEngine::new()
    }

    fn assert_no_notdef(runs: &[ShapedRun]) {
        for run in runs {
            for glyph in &run.glyphs {
                assert_ne!(glyph.id, 0, ".notdef emitted for a covered char");
            }
        }
    }

    #[test]
    fn bundled_families_are_registered() {
        let mut engine = engine();
        assert!(engine.has_family(FAMILY_SANS));
        assert!(engine.has_family(FAMILY_MONO));
    }

    #[test]
    fn empty_text_shapes_to_no_runs() {
        let mut engine = engine();
        assert!(
            engine
                .shape("", 12.0, FAMILY_SANS, FontWeight::NORMAL)
                .is_empty()
        );
    }

    #[test]
    fn single_style_text_is_a_single_run() {
        let mut engine = engine();
        let runs = engine.shape("100°F", 12.0, FAMILY_SANS, FontWeight::NORMAL);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].glyphs.len(), 5);
        assert_eq!(runs[0].size, 12.0);
        assert_no_notdef(&runs);
    }

    #[test]
    fn shaping_is_deterministic() {
        let mut engine = engine();
        let first = engine.shape("100°F", 12.0, FAMILY_SANS, FontWeight::NORMAL);
        let second = engine.shape("100°F", 12.0, FAMILY_SANS, FontWeight::NORMAL);
        assert_eq!(first, second);
    }

    #[test]
    fn advance_scales_with_size() {
        let mut engine = engine();
        let small =
            FontEngine::line_width(&engine.shape("100°F", 10.0, FAMILY_SANS, FontWeight::NORMAL));
        let large =
            FontEngine::line_width(&engine.shape("100°F", 20.0, FAMILY_SANS, FontWeight::NORMAL));
        assert!(small > 0.0 && large > small);
        assert!((large / small - 2.0).abs() < 1e-4);
    }

    #[test]
    fn width_is_sane_for_tick_labels() {
        let mut engine = engine();
        // Five glyphs averaging well under one em each at 12px.
        let width =
            FontEngine::line_width(&engine.shape("100°F", 12.0, FAMILY_SANS, FontWeight::NORMAL));
        assert!(width > 0.2 * 12.0 * 5.0 && width < 1.2 * 12.0 * 5.0);
    }

    #[test]
    fn mono_advance_is_uniform_per_glyph() {
        let mut engine = engine();
        let narrow =
            FontEngine::line_width(&engine.shape("iiii", 12.0, FAMILY_MONO, FontWeight::NORMAL));
        let wide =
            FontEngine::line_width(&engine.shape("MMMM", 12.0, FAMILY_MONO, FontWeight::NORMAL));
        assert_eq!(narrow, wide);
        let digits = FontEngine::line_width(&engine.shape(
            "0123456789",
            12.0,
            FAMILY_MONO,
            FontWeight::NORMAL,
        ));
        let zero =
            FontEngine::line_width(&engine.shape("0", 12.0, FAMILY_MONO, FontWeight::NORMAL));
        assert_eq!(digits, 10.0 * zero);
    }

    #[test]
    fn weight_selects_a_different_face() {
        let mut engine = engine();
        let regular = engine.shape("100°F", 12.0, FAMILY_SANS, FontWeight::NORMAL);
        let bold = engine.shape("100°F", 12.0, FAMILY_SANS, FontWeight::BOLD);
        assert_eq!(regular.len(), 1);
        assert_eq!(bold.len(), 1);
        assert_ne!(regular[0].font, bold[0].font);
    }

    #[test]
    fn family_selects_a_different_face() {
        let mut engine = engine();
        let sans = engine.shape("100°F", 12.0, FAMILY_SANS, FontWeight::NORMAL);
        let mono = engine.shape("100°F", 12.0, FAMILY_MONO, FontWeight::NORMAL);
        assert_eq!(sans.len(), 1);
        assert_eq!(mono.len(), 1);
        assert_ne!(sans[0].font, mono[0].font);
    }

    #[test]
    fn tick_charset_has_full_coverage() {
        let mut engine = engine();
        for family in [FAMILY_SANS, FAMILY_MONO] {
            let runs = engine.shape(TICK_CHARSET, 12.0, family, FontWeight::NORMAL);
            let glyphs: usize = runs.iter().map(|run| run.glyphs.len()).sum();
            assert_eq!(
                glyphs,
                TICK_CHARSET.chars().count(),
                "{family} dropped chars"
            );
            assert_no_notdef(&runs);
        }
        let bold = engine.shape(TICK_CHARSET, 12.0, FAMILY_SANS, FontWeight::BOLD);
        assert_no_notdef(&bold);
    }

    #[test]
    fn shaped_runs_are_thread_safe() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ShapedRun>();
        assert_send_sync::<Vec<ShapedRun>>();
    }
}
