//! Headless Metal capture for showcase sections (macOS only).
//!
//! Mirrors the component-lab capture harness: each manifest case opens the
//! real [`Showcase`](crate::showcase::Showcase) app at the case viewport,
//! selects the case section, draws one frame through `MetalHeadlessRenderer`,
//! and persists a PNG plus a JSON-serializable report. Theme and design are
//! pinned per run so captures compare directly against wasm gallery
//! snapshots (wasm defaults: dark theme, neutral design).

use crate::release_artifacts::{ShowcaseVisualCapture, slug};
use crate::showcase::{Showcase, ShowcaseSection};
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SHOWCASE_CAPTURE_REPORT_SCHEMA_VERSION: u32 = 1;
pub const SHOWCASE_CAPTURE_REPORT_TYPE: &str = "gpui-showcase-render-capture";
/// Headless windows render at 2x; captures are `viewport * 2` pixels.
pub const SHOWCASE_HEADLESS_PIXEL_SCALE: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShowcaseCaptureStatus {
    Captured,
    RenderFailed,
    UnexpectedDimensions,
    Blank,
    WriteFailed,
    UnknownSection,
}

impl ShowcaseCaptureStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Captured => "captured",
            Self::RenderFailed => "render-failed",
            Self::UnexpectedDimensions => "unexpected-dimensions",
            Self::Blank => "blank",
            Self::WriteFailed => "write-failed",
            Self::UnknownSection => "unknown-section",
        }
    }

    pub const fn is_captured(self) -> bool {
        matches!(self, Self::Captured)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowcaseCaptureCaseReport {
    pub capture_id: String,
    pub section: String,
    pub renderer_id: String,
    pub actual_path: String,
    pub status: ShowcaseCaptureStatus,
    pub width: u32,
    pub height: u32,
    pub rgba_checksum: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowcaseCaptureReport {
    pub schema_version: u32,
    pub report_type: String,
    pub renderer_id: String,
    pub theme: String,
    pub design: String,
    pub passed: bool,
    pub requested_count: usize,
    pub captured_count: usize,
    pub failed_count: usize,
    pub cases: Vec<ShowcaseCaptureCaseReport>,
}

impl ShowcaseCaptureReport {
    pub fn to_markdown_table(&self) -> String {
        let mut out = format!(
            "# GPUI Showcase Renderer Capture\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - renderer: `{}`\n\
             - theme: `{}`\n\
             - design: `{}`\n\
             - passed: {}\n\
             - captured: {}/{}\n\n\
             | capture | status | dimensions | checksum | actual |\n\
             | --- | --- | ---: | --- | --- |\n",
            self.schema_version,
            self.report_type,
            self.renderer_id,
            self.theme,
            self.design,
            self.passed,
            self.captured_count,
            self.requested_count
        );
        for case in &self.cases {
            out.push_str(&format!(
                "| `{}` | {} | {}x{} | `{}` | `{}` |\n",
                case.capture_id,
                case.status.as_str(),
                case.width,
                case.height,
                case.rgba_checksum,
                case.actual_path
            ));
        }
        out
    }
}

/// Resolve a manifest section slug back to its [`ShowcaseSection`].
pub fn section_from_slug(section_slug: &str) -> Option<ShowcaseSection> {
    ShowcaseSection::all()
        .iter()
        .copied()
        .find(|section| slug(section.label()) == section_slug)
}

/// Parse a theme id with the same vocabulary as the wasm `?theme=` query
/// (`web_initial_theme`); unknown ids fall back to dark.
pub fn theme_variant_from_id(id: &str) -> gpui_ui_kit::theme::ThemeVariant {
    use gpui_ui_kit::theme::ThemeVariant;
    match id.to_ascii_lowercase().as_str() {
        "light" => ThemeVariant::Light,
        "midnight" => ThemeVariant::Midnight,
        "forest" => ThemeVariant::Forest,
        "black-and-white" | "black_and_white" => ThemeVariant::BlackAndWhite,
        "onyx" => ThemeVariant::Onyx,
        "carbon-white" | "carbon_white" => ThemeVariant::CarbonWhite,
        "carbon-gray-10" | "carbon_gray_10" => ThemeVariant::CarbonGray10,
        "carbon-gray-90" | "carbon_gray_90" => ThemeVariant::CarbonGray90,
        "carbon-gray-100" | "carbon_gray_100" => ThemeVariant::CarbonGray100,
        _ => ThemeVariant::Dark,
    }
}

/// Parse a design id with the same vocabulary as the wasm `?style=` query;
/// unknown ids fall back to neutral.
pub fn design_system_from_id(id: &str) -> gpui_design::DesignSystem {
    gpui_design::DesignSystem::from_language_id(id)
        .unwrap_or_else(gpui_design::DesignSystem::neutral)
}

pub struct ShowcaseCaptureRequest<'a> {
    pub captures: &'a [ShowcaseVisualCapture],
    pub output_root: &'a Path,
    pub theme_id: &'a str,
    pub design_id: &'a str,
    /// Optional `WIDTHxHEIGHT` override applied to every case (parity runs
    /// against wasm viewports); defaults to each manifest case viewport.
    pub viewport_override: Option<(u32, u32)>,
}

pub fn capture_showcase_cases(
    renderer_id: &str,
    request: &ShowcaseCaptureRequest,
) -> Result<ShowcaseCaptureReport> {
    #[cfg(target_os = "macos")]
    {
        capture_showcase_cases_macos(renderer_id, request)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (renderer_id, request);
        anyhow::bail!(
            "renderer-backed showcase capture is not implemented for this platform; use the native Linux/Windows capture lane"
        )
    }
}

#[cfg(target_os = "macos")]
fn capture_showcase_cases_macos(
    renderer_id: &str,
    request: &ShowcaseCaptureRequest,
) -> Result<ShowcaseCaptureReport> {
    use gpui::{
        AnyWindowHandle, AppContext as _, HeadlessAppContext, PlatformTextSystem as _, px, size,
    };
    use gpui_design::DesignSystemState;
    use gpui_macos::metal_renderer::MetalHeadlessRenderer;
    use gpui_ui_kit::accessibility::AccessibilityTree;
    use gpui_ui_kit::i18n::I18nState;
    use gpui_ui_kit::spinner::freeze_visual_animations;
    use gpui_ui_kit::theme::ThemeState;
    use std::sync::Arc;

    let theme = theme_variant_from_id(request.theme_id);
    let design = design_system_from_id(request.design_id);
    let _animation_guard = freeze_visual_animations(0.0);
    // Use the exact text stack the wasm platform uses (cosmic-text plus the
    // bundled Plex/Lilex fonts, no system fonts). The CoreText system stack
    // resolves "IBM Plex Sans" to a fallback face on machines without Plex
    // installed, whose wider glyphs wrap paragraphs onto extra lines and
    // shift every element below (dialog/popover/tooltip parity failures).
    let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new_without_system_fonts(
        "IBM Plex Sans",
    ));
    text_system.add_fonts(gpui_miniapp::bundled_ui_fonts())?;
    let text_system: Arc<dyn gpui::PlatformTextSystem> = text_system;
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
        MetalHeadlessRenderer::try_new().map(|renderer| {
            let renderer: Box<dyn gpui::PlatformHeadlessRenderer> = Box::new(renderer);
            renderer
        })
    });
    cx.update(|app| {
        app.set_global(ThemeState::with_variant(theme));
        app.set_global(DesignSystemState::with_system(design));
        app.set_global(AccessibilityTree::new());
        // Mirror `MiniApp::run` (`with_i18n(true)`): without this global
        // `cx.t()` falls back to `"???"` and every translated title renders
        // as question marks. Default English matches the wasm gallery.
        app.set_global(I18nState::new());
    });

    let actual_dir: PathBuf = request.output_root.join("metal").join("actual");
    let mut reports = Vec::with_capacity(request.captures.len());
    for capture in request.captures {
        let Some(section) = section_from_slug(&capture.section) else {
            reports.push(ShowcaseCaptureCaseReport {
                capture_id: capture.id.clone(),
                section: capture.section.clone(),
                renderer_id: renderer_id.to_string(),
                actual_path: String::new(),
                status: ShowcaseCaptureStatus::UnknownSection,
                width: 0,
                height: 0,
                rgba_checksum: String::new(),
                message: format!("no ShowcaseSection matches slug {:?}", capture.section),
            });
            continue;
        };
        let (width, height) = request
            .viewport_override
            .unwrap_or((capture.width, capture.height));
        let actual_path = actual_dir.join(format!("{}.png", capture.id));
        let report = (|| -> Result<_> {
            let handle = cx
                .open_window(size(px(width as f32), px(height as f32)), |_window, app| {
                    app.new(Showcase::new)
                })?;
            handle.update(&mut cx, |showcase, _window, entity_cx| {
                showcase.select_section(section, entity_cx);
            })?;
            let any_handle: AnyWindowHandle = handle.into();
            let capture_result = (|| -> Result<_> {
                cx.update_window(any_handle, |_view, window, app| {
                    let _ = window.draw(app);
                })?;
                cx.capture_screenshot(any_handle)
            })();
            let remove_result = cx.update_window(any_handle, |_view, window, _app| {
                window.remove_window();
            });
            remove_result.context("remove visual capture window")?;
            capture_result
        })();

        let report = match report {
            Ok(image) => persist_capture(
                renderer_id,
                &capture.id,
                &capture.section,
                &actual_path,
                width,
                height,
                image,
            ),
            Err(error) => ShowcaseCaptureCaseReport {
                capture_id: capture.id.clone(),
                section: capture.section.clone(),
                renderer_id: renderer_id.to_string(),
                actual_path: actual_path.display().to_string(),
                status: ShowcaseCaptureStatus::RenderFailed,
                width: 0,
                height: 0,
                rgba_checksum: String::new(),
                message: format!("renderer capture failed: {error:#}"),
            },
        };
        reports.push(report);
    }

    let captured_count = reports
        .iter()
        .filter(|case| case.status.is_captured())
        .count();
    let failed_count = reports.len() - captured_count;
    Ok(ShowcaseCaptureReport {
        schema_version: SHOWCASE_CAPTURE_REPORT_SCHEMA_VERSION,
        report_type: SHOWCASE_CAPTURE_REPORT_TYPE.to_string(),
        renderer_id: renderer_id.to_string(),
        theme: request.theme_id.to_string(),
        design: request.design_id.to_string(),
        passed: failed_count == 0,
        requested_count: reports.len(),
        captured_count,
        failed_count,
        cases: reports,
    })
}

#[cfg(target_os = "macos")]
#[allow(clippy::too_many_arguments)]
fn persist_capture(
    renderer_id: &str,
    capture_id: &str,
    section: &str,
    actual_path: &Path,
    width: u32,
    height: u32,
    image: image::RgbaImage,
) -> ShowcaseCaptureCaseReport {
    let (actual_width, actual_height) = image.dimensions();
    let checksum = rgba_checksum(&image);
    let expected = (
        width.saturating_mul(SHOWCASE_HEADLESS_PIXEL_SCALE),
        height.saturating_mul(SHOWCASE_HEADLESS_PIXEL_SCALE),
    );
    if let Some(parent) = actual_path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        return ShowcaseCaptureCaseReport {
            capture_id: capture_id.to_string(),
            section: section.to_string(),
            renderer_id: renderer_id.to_string(),
            actual_path: actual_path.display().to_string(),
            status: ShowcaseCaptureStatus::WriteFailed,
            width: 0,
            height: 0,
            rgba_checksum: String::new(),
            message: format!("create actual directory: {error}"),
        };
    }
    if let Err(error) = image.save(actual_path) {
        return ShowcaseCaptureCaseReport {
            capture_id: capture_id.to_string(),
            section: section.to_string(),
            renderer_id: renderer_id.to_string(),
            actual_path: actual_path.display().to_string(),
            status: ShowcaseCaptureStatus::WriteFailed,
            width: 0,
            height: 0,
            rgba_checksum: String::new(),
            message: format!("write actual PNG: {error}"),
        };
    }
    if (actual_width, actual_height) != expected {
        return ShowcaseCaptureCaseReport {
            capture_id: capture_id.to_string(),
            section: section.to_string(),
            renderer_id: renderer_id.to_string(),
            actual_path: actual_path.display().to_string(),
            status: ShowcaseCaptureStatus::UnexpectedDimensions,
            width: actual_width,
            height: actual_height,
            rgba_checksum: checksum,
            message: format!(
                "captured {actual_width}x{actual_height}, expected {}x{}",
                expected.0, expected.1
            ),
        };
    }
    if image_is_blank(&image) {
        return ShowcaseCaptureCaseReport {
            capture_id: capture_id.to_string(),
            section: section.to_string(),
            renderer_id: renderer_id.to_string(),
            actual_path: actual_path.display().to_string(),
            status: ShowcaseCaptureStatus::Blank,
            width: actual_width,
            height: actual_height,
            rgba_checksum: checksum,
            message: "capture contains only one RGBA value".to_string(),
        };
    }
    ShowcaseCaptureCaseReport {
        capture_id: capture_id.to_string(),
        section: section.to_string(),
        renderer_id: renderer_id.to_string(),
        actual_path: actual_path.display().to_string(),
        status: ShowcaseCaptureStatus::Captured,
        width: actual_width,
        height: actual_height,
        rgba_checksum: checksum,
        message: "renderer pixels captured".to_string(),
    }
}

#[cfg(target_os = "macos")]
fn image_is_blank(image: &image::RgbaImage) -> bool {
    let mut pixels = image.pixels();
    let Some(first) = pixels.next() else {
        return true;
    };
    pixels.all(|pixel| pixel == first)
}

#[cfg(target_os = "macos")]
fn rgba_checksum(image: &image::RgbaImage) -> String {
    // Stable FNV-1a over the raw RGBA bytes. This is evidence metadata, not a
    // security primitive; the PNG itself remains the authoritative artifact.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in image.as_raw() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_slugs_round_trip() {
        for section in ShowcaseSection::all() {
            let resolved = section_from_slug(&slug(section.label()));
            assert_eq!(resolved, Some(*section), "slug for {:?}", section.label());
        }
    }

    #[test]
    fn unknown_slug_resolves_to_none() {
        assert_eq!(section_from_slug("no-such-section"), None);
    }

    #[test]
    fn theme_ids_match_wasm_vocabulary() {
        use gpui_ui_kit::theme::ThemeVariant;
        assert_eq!(theme_variant_from_id("light"), ThemeVariant::Light);
        assert_eq!(
            theme_variant_from_id("carbon-gray-90"),
            ThemeVariant::CarbonGray90
        );
        assert_eq!(theme_variant_from_id("bogus"), ThemeVariant::Dark);
    }

    #[test]
    fn unknown_design_falls_back_to_neutral() {
        let design = design_system_from_id("bogus");
        assert_eq!(design, gpui_design::DesignSystem::neutral());
    }
}
