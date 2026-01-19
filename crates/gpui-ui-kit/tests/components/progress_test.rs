//! Progress component tests

use gpui_ui_kit::progress::{CircularProgress, Progress, ProgressSize, ProgressVariant};

#[test]
fn test_progress_bar() {
    let progress = Progress::new(0.5)
        .max(1.0)
        .variant(ProgressVariant::Success)
        .size(ProgressSize::Lg)
        .show_label(true)
        .striped(true)
        .animated(true);

    drop(progress);
}

#[test]
fn test_circular_progress() {
    let progress = CircularProgress::new(75.0)
        .max(100.0)
        .size(gpui::px(64.0))
        .thickness(gpui::px(8.0))
        .variant(ProgressVariant::Error)
        .show_label(true);

    drop(progress);
}
