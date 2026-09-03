use gpui::*;
pub(super) use gpui_python_runtime::showcase::{color_scale, parse_spec};
use std::env;
use std::path::{Path, PathBuf};

pub(super) fn default_showcase_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/showcase.py")
}

pub(super) fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(super) fn apply_size(mut element: Div, width: Option<f32>, height: Option<f32>) -> Div {
    if let Some(width) = width {
        element = element.w(px(width));
    }
    if let Some(height) = height {
        element = element.h(px(height));
    }
    element
}

pub(super) fn tone_color(tone: &str, theme: &gpui_ui_kit::theme::Theme) -> Rgba {
    match tone {
        "secondary" => theme.text_secondary,
        "muted" => theme.text_muted,
        "accent" => theme.accent,
        "success" => theme.success,
        "warning" => theme.warning,
        "error" => theme.error,
        "info" => theme.info,
        _ => theme.text_primary,
    }
}

pub(super) fn badge_colors(tone: &str, theme: &gpui_ui_kit::theme::Theme) -> (Rgba, Rgba) {
    match tone {
        "accent" | "primary" => (theme.badge_primary_bg, theme.badge_primary_text),
        "success" => (theme.badge_success_bg, theme.badge_success_text),
        "warning" => (theme.badge_warning_bg, theme.badge_warning_text),
        "error" => (theme.badge_error_bg, theme.badge_error_text),
        "info" => (theme.badge_info_bg, theme.badge_info_text),
        _ => (theme.muted, theme.text_secondary),
    }
}
