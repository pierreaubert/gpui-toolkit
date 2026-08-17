use super::{ComponentLab, LabAppConfig};
use crate::ComponentLabVisualCase;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const COMPONENT_LAB_CAPTURE_REPORT_SCHEMA_VERSION: u32 = 1;
pub const COMPONENT_LAB_CAPTURE_REPORT_TYPE: &str = "gpui-component-lab-render-capture";
pub const COMPONENT_LAB_HEADLESS_PIXEL_SCALE: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentLabCaptureStatus {
    Captured,
    RenderFailed,
    UnexpectedDimensions,
    Blank,
    WriteFailed,
}

impl ComponentLabCaptureStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Captured => "captured",
            Self::RenderFailed => "render-failed",
            Self::UnexpectedDimensions => "unexpected-dimensions",
            Self::Blank => "blank",
            Self::WriteFailed => "write-failed",
        }
    }

    pub const fn is_captured(self) -> bool {
        matches!(self, Self::Captured)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLabCaptureCaseReport {
    pub capture_id: String,
    pub story_id: String,
    pub renderer_id: String,
    pub actual_path: String,
    pub status: ComponentLabCaptureStatus,
    pub width: u32,
    pub height: u32,
    pub rgba_checksum: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLabCaptureReport {
    pub schema_version: u32,
    pub report_type: String,
    pub renderer_id: String,
    pub passed: bool,
    pub requested_count: usize,
    pub captured_count: usize,
    pub failed_count: usize,
    pub cases: Vec<ComponentLabCaptureCaseReport>,
}

impl ComponentLabCaptureReport {
    pub fn to_markdown_table(&self) -> String {
        let mut out = format!(
            "# GPUI Component Lab Renderer Capture\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - renderer: `{}`\n\
             - passed: {}\n\
             - captured: {}/{}\n\n\
             | capture | status | dimensions | checksum | actual |\n\
             | --- | --- | ---: | --- | --- |\n",
            self.schema_version,
            self.report_type,
            self.renderer_id,
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

pub fn capture_component_lab_cases(
    renderer_id: &str,
    cases: &[ComponentLabVisualCase],
    stories_dir: &Path,
    token_paths: &[PathBuf],
) -> Result<ComponentLabCaptureReport> {
    #[cfg(target_os = "macos")]
    {
        capture_component_lab_cases_macos(renderer_id, cases, stories_dir, token_paths)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (renderer_id, cases, stories_dir, token_paths);
        anyhow::bail!(
            "renderer-backed component capture is not implemented for this platform; use the native Linux/Windows capture lane"
        )
    }
}

#[cfg(target_os = "macos")]
fn capture_component_lab_cases_macos(
    renderer_id: &str,
    cases: &[ComponentLabVisualCase],
    stories_dir: &Path,
    token_paths: &[PathBuf],
) -> Result<ComponentLabCaptureReport> {
    use gpui::{AnyWindowHandle, AppContext as _, HeadlessAppContext, Platform as _, px, size};
    use gpui_design::DesignSystemState;
    use gpui_macos::{MacPlatform, metal_renderer::MetalHeadlessRenderer};
    use gpui_ui_kit::accessibility::AccessibilityTree;
    use gpui_ui_kit::spinner::freeze_visual_animations;
    use gpui_ui_kit::theme::{ThemeState, ThemeVariant};
    use std::sync::Arc;

    if cases.iter().any(|case| case.renderer_id != renderer_id) {
        anyhow::bail!("capture cases contain a renderer namespace mismatch");
    }
    if cases
        .iter()
        .any(|case| case.pixel_scale != COMPONENT_LAB_HEADLESS_PIXEL_SCALE)
    {
        anyhow::bail!(
            "Metal headless capture requires pixel_scale={}",
            COMPONENT_LAB_HEADLESS_PIXEL_SCALE
        );
    }

    let _animation_guard = freeze_visual_animations(0.0);
    let platform = MacPlatform::new(true);
    let text_system = platform.text_system();
    drop(platform);
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
        MetalHeadlessRenderer::try_new().map(|renderer| {
            let renderer: Box<dyn gpui::PlatformHeadlessRenderer> = Box::new(renderer);
            renderer
        })
    });
    cx.update(|app| {
        app.set_global(ThemeState::with_variant(ThemeVariant::Light));
        app.set_global(DesignSystemState::new());
        app.set_global(AccessibilityTree::new());
    });

    let mut reports = Vec::with_capacity(cases.len());
    for case in cases {
        let config = LabAppConfig::new(stories_dir.to_path_buf(), token_paths.to_vec())
            .for_visual_capture(
                case.story_id.clone(),
                case.viewport_id.clone(),
                case.theme_id.clone(),
                case.reduced_motion,
            );
        let capture = (|| -> Result<_> {
            let handle = cx.open_window(
                size(
                    px(case.viewport_width as f32),
                    px(case.viewport_height as f32),
                ),
                |_window, app| app.new(|entity_cx| ComponentLab::new(config, entity_cx)),
            )?;
            let any_handle: AnyWindowHandle = handle.into();
            let capture_result = (|| -> Result<_> {
                cx.update_window(any_handle, |_view, window, app| {
                    let _ = window.draw(app);
                })?;
                cx.capture_screenshot(any_handle)
            })();
            let release_result = handle.update(&mut cx, |lab, _window, entity_cx| {
                lab.release_visual_capture_resources(entity_cx);
                entity_cx.notify();
            });
            let cleanup_draw_result = cx.update_window(any_handle, |_view, window, app| {
                let _ = window.draw(app);
            });
            let remove_result = cx.update_window(any_handle, |_view, window, _app| {
                window.remove_window();
            });
            release_result.context("release visual capture resources")?;
            cleanup_draw_result.context("draw released visual capture frame")?;
            remove_result.context("remove visual capture window")?;
            capture_result
        })();

        let report = match capture {
            Ok(image) => persist_capture(case, image),
            Err(error) => capture_error(
                case,
                ComponentLabCaptureStatus::RenderFailed,
                format!("renderer capture failed: {error:#}"),
            ),
        };
        reports.push(report);
    }

    let captured_count = reports
        .iter()
        .filter(|case| case.status.is_captured())
        .count();
    let failed_count = reports.len() - captured_count;
    Ok(ComponentLabCaptureReport {
        schema_version: COMPONENT_LAB_CAPTURE_REPORT_SCHEMA_VERSION,
        report_type: COMPONENT_LAB_CAPTURE_REPORT_TYPE.to_string(),
        renderer_id: renderer_id.to_string(),
        passed: failed_count == 0,
        requested_count: reports.len(),
        captured_count,
        failed_count,
        cases: reports,
    })
}

#[cfg(target_os = "macos")]
fn persist_capture(
    case: &ComponentLabVisualCase,
    image: image::RgbaImage,
) -> ComponentLabCaptureCaseReport {
    let (width, height) = image.dimensions();
    let checksum = rgba_checksum(&image);
    let expected = (
        case.viewport_width.saturating_mul(case.pixel_scale),
        case.viewport_height.saturating_mul(case.pixel_scale),
    );
    let path = Path::new(&case.actual_path);
    if let Some(parent) = path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        return capture_error(
            case,
            ComponentLabCaptureStatus::WriteFailed,
            format!("create actual directory: {error}"),
        );
    }
    if let Err(error) = image.save(path) {
        return capture_error(
            case,
            ComponentLabCaptureStatus::WriteFailed,
            format!("write actual PNG: {error}"),
        );
    }
    if (width, height) != expected {
        return ComponentLabCaptureCaseReport {
            capture_id: case.capture_id.clone(),
            story_id: case.story_id.clone(),
            renderer_id: case.renderer_id.clone(),
            actual_path: case.actual_path.clone(),
            status: ComponentLabCaptureStatus::UnexpectedDimensions,
            width,
            height,
            rgba_checksum: checksum,
            message: format!(
                "captured {}x{}, expected {}x{}",
                width, height, expected.0, expected.1
            ),
        };
    }
    if image_is_blank(&image) {
        return ComponentLabCaptureCaseReport {
            capture_id: case.capture_id.clone(),
            story_id: case.story_id.clone(),
            renderer_id: case.renderer_id.clone(),
            actual_path: case.actual_path.clone(),
            status: ComponentLabCaptureStatus::Blank,
            width,
            height,
            rgba_checksum: checksum,
            message: "capture contains only one RGBA value".to_string(),
        };
    }
    ComponentLabCaptureCaseReport {
        capture_id: case.capture_id.clone(),
        story_id: case.story_id.clone(),
        renderer_id: case.renderer_id.clone(),
        actual_path: case.actual_path.clone(),
        status: ComponentLabCaptureStatus::Captured,
        width,
        height,
        rgba_checksum: checksum,
        message: "renderer pixels captured".to_string(),
    }
}

fn capture_error(
    case: &ComponentLabVisualCase,
    status: ComponentLabCaptureStatus,
    message: impl Into<String>,
) -> ComponentLabCaptureCaseReport {
    ComponentLabCaptureCaseReport {
        capture_id: case.capture_id.clone(),
        story_id: case.story_id.clone(),
        renderer_id: case.renderer_id.clone(),
        actual_path: case.actual_path.clone(),
        status,
        width: 0,
        height: 0,
        rgba_checksum: String::new(),
        message: message.into(),
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
