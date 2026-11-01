//! Shared headless Metal context for static Spinorama image rendering.
//!
//! Used by the `--capture-bench` and `--render-images` modes (macOS only).

// Rust guideline compliant 2026-02-21

use crate::app::SpinoramaApp;
use gpui::{
    AnyWindowHandle, AppContext as _, HeadlessAppContext, Platform as _, WindowHandle, px, size,
};
use gpui_design::DesignSystemState;
use gpui_macos::{MacPlatform, metal_renderer::MetalHeadlessRenderer};
use gpui_ui_kit::accessibility::AccessibilityTree;
use gpui_ui_kit::i18n::I18nState;
use gpui_ui_kit::theme::{ThemeState, ThemeVariant};
use std::sync::Arc;

/// Creates a headless context with the UI globals `SpinoramaApp` expects.
///
/// # Errors
///
/// Returns an error when no Metal adapter is available.
pub fn make_context() -> Result<HeadlessAppContext, String> {
    if MetalHeadlessRenderer::try_new().is_none() {
        return Err("Metal adapter unavailable".into());
    }
    let platform = MacPlatform::new(true);
    let text_system = platform.text_system();
    drop(platform);
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
        Ok(MetalHeadlessRenderer::try_new().map(|renderer| {
            let renderer: Box<dyn gpui::PlatformHeadlessRenderer> = Box::new(renderer);
            renderer
        }))
    });
    cx.update(|app| {
        app.set_global(ThemeState::with_variant(ThemeVariant::Light));
        app.set_global(DesignSystemState::with_system(
            gpui_design::DesignSystem::neutral(),
        ));
        app.set_global(AccessibilityTree::new());
        app.set_global(I18nState::new());
    });
    Ok(cx)
}

/// Opens a headless window hosting a fresh `SpinoramaApp`.
///
/// # Errors
///
/// Returns an error when the window cannot be created.
pub fn open_app_window(
    cx: &mut HeadlessAppContext,
    width: f32,
    height: f32,
) -> Result<WindowHandle<SpinoramaApp>, String> {
    cx.open_window(size(px(width), px(height)), |_window, app| {
        app.new(SpinoramaApp::new_offline)
    })
    .map_err(|e| format!("open window: {e}"))
}

/// Draws one frame and reads back the pixels.
///
/// # Errors
///
/// Returns an error when drawing or the framebuffer read-back fails.
pub fn draw_capture(
    cx: &mut HeadlessAppContext,
    window: AnyWindowHandle,
) -> Result<image::RgbaImage, String> {
    cx.update_window(window, |_view, window, app| {
        let _ = window.draw(app);
    })
    .map_err(|e| format!("draw: {e}"))?;
    cx.capture_screenshot(window)
        .map_err(|e| format!("capture: {e}"))
}
