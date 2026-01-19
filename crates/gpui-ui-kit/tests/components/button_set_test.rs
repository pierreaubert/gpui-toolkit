//! ButtonSet component tests

use gpui_ui_kit::button_set::{ButtonSet, ButtonSetOption, ButtonSetSize};

#[test]
fn test_button_set_creation() {
    let options = vec![
        ButtonSetOption::new("list", "List").icon("list"),
        ButtonSetOption::new("grid", "Grid").disabled(true),
    ];

    let button_set = ButtonSet::new("view-toggle")
        .options(options)
        .selected("list")
        .size(ButtonSetSize::Sm)
        .disabled(false)
        .on_change(|val, _window, _cx| {
            println!("Selected: {}", val);
        });

    drop(button_set);
}

#[test]
fn test_button_set_sizes() {
    let sizes = [
        ButtonSetSize::Xs,
        ButtonSetSize::Sm,
        ButtonSetSize::Md,
        ButtonSetSize::Lg,
    ];

    for size in &sizes {
        let bs = ButtonSet::new("test").size(*size);
        drop(bs);
    }
}
