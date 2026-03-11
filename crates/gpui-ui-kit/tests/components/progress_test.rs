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

    let _ = progress;
}

#[test]
fn test_circular_progress() {
    let progress = CircularProgress::new(75.0)
        .max(100.0)
        .size(gpui::px(64.0))
        .thickness(gpui::px(8.0))
        .variant(ProgressVariant::Error)
        .show_label(true);

    let _ = progress;
}

// -- New tests --

#[test]
fn test_progress_all_variants() {
    let variants = [
        ProgressVariant::Default,
        ProgressVariant::Success,
        ProgressVariant::Warning,
        ProgressVariant::Error,
    ];

    for variant in &variants {
        let progress = Progress::new(0.5).variant(*variant);
        let _ = progress;
    }
}

#[test]
fn test_progress_all_sizes() {
    let sizes = [
        ProgressSize::Xs,
        ProgressSize::Sm,
        ProgressSize::Md,
        ProgressSize::Lg,
    ];

    for size in &sizes {
        let progress = Progress::new(0.5).size(*size);
        let _ = progress;
    }
}

#[test]
fn test_progress_zero_value() {
    let progress = Progress::new(0.0);
    let _ = progress;
}

#[test]
fn test_progress_full_value() {
    let progress = Progress::new(1.0).max(1.0);
    let _ = progress;
}

#[test]
fn test_progress_custom_max() {
    let progress = Progress::new(50.0).max(200.0);
    let _ = progress;
}
