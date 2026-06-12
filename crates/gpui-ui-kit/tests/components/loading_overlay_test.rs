//! LoadingOverlay component tests

use gpui_ui_kit::loading_overlay::LoadingOverlay;
use gpui_ui_kit::spinner::SpinnerSize;

#[test]
fn test_loading_overlay_creation() {
    let overlay = LoadingOverlay::new("test");
    drop(overlay);
}

#[test]
fn test_loading_overlay_message() {
    let overlay = LoadingOverlay::new("test").message("Loading...");
    drop(overlay);
}

#[test]
fn test_loading_overlay_subtitle() {
    let overlay = LoadingOverlay::new("test")
        .message("Loading library")
        .subtitle("This may take a moment");
    drop(overlay);
}

#[test]
fn test_loading_overlay_spinner_size() {
    for size in [SpinnerSize::Sm, SpinnerSize::Md, SpinnerSize::Lg] {
        let overlay = LoadingOverlay::new("test").spinner_size(size);
        drop(overlay);
    }
}

#[test]
fn test_loading_overlay_spinner_color() {
    let overlay = LoadingOverlay::new("test").spinner_color(gpui::rgba(0x007accff));
    drop(overlay);
}

#[test]
fn test_loading_overlay_dismissible() {
    let overlay = LoadingOverlay::new("test").dismissible(true);
    drop(overlay);

    let overlay = LoadingOverlay::new("test").dismissible(false);
    drop(overlay);
}

#[test]
fn test_loading_overlay_on_dismiss() {
    let overlay = LoadingOverlay::new("test")
        .dismissible(true)
        .on_dismiss(|_window, _cx| {});
    drop(overlay);
}

#[test]
fn test_loading_overlay_full_configuration() {
    let overlay = LoadingOverlay::new("loading-screen")
        .message("Loading library...")
        .subtitle("Scanning audio files")
        .spinner_size(SpinnerSize::Lg)
        .spinner_color(gpui::rgba(0x22c55eff))
        .dismissible(true)
        .on_dismiss(|_window, _cx| {});
    drop(overlay);
}
