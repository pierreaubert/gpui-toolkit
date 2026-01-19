//! Spinner component tests

use gpui_ui_kit::spinner::{LoadingDots, Spinner, SpinnerSize};

#[test]
fn test_spinner_configuration() {
    let spinner = Spinner::new()
        .size(SpinnerSize::Lg)
        .color(gpui::rgb(0xFFFFFF))
        .label("Loading...");

    drop(spinner);
}

#[test]
fn test_loading_dots() {
    let dots = LoadingDots::new()
        .size(SpinnerSize::Sm)
        .color(gpui::rgb(0xCCCCCC));

    drop(dots);
}
