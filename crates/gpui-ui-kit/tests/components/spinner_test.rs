//! Spinner component tests

use gpui_ui_kit::spinner::{LoadingDots, Spinner, SpinnerSize};

#[test]
fn test_spinner_configuration() {
    let spinner = Spinner::new()
        .size(SpinnerSize::Lg)
        .color(gpui::rgb(0xFFFFFF))
        .label("Loading...");

    let _ = spinner;
}

#[test]
fn test_loading_dots() {
    let dots = LoadingDots::new()
        .size(SpinnerSize::Sm)
        .color(gpui::rgb(0xCCCCCC));

    let _ = dots;
}

// -- New tests --

#[test]
fn test_spinner_all_sizes() {
    let sizes = [SpinnerSize::Sm, SpinnerSize::Md, SpinnerSize::Lg];

    for size in sizes {
        let spinner = Spinner::new().size(size);
        let _ = spinner;
    }
}

#[test]
fn test_spinner_default() {
    let spinner = Spinner::new();
    let _ = spinner;
}

#[test]
fn test_spinner_no_label() {
    let spinner = Spinner::new().size(SpinnerSize::Md);
    let _ = spinner;
}

#[test]
fn test_loading_dots_all_sizes() {
    let sizes = [SpinnerSize::Sm, SpinnerSize::Md, SpinnerSize::Lg];

    for size in sizes {
        let dots = LoadingDots::new().size(size);
        let _ = dots;
    }
}
