//! IconButton component tests

use gpui::prelude::ParentElement;
use gpui_ui_kit::ComponentSize;
use gpui_ui_kit::icon_button::{IconButton, IconButtonSize, IconButtonTheme, IconButtonVariant};

#[test]
fn test_icon_button_creation() {
    let icon_button = IconButton::new("test", "x");
    drop(icon_button);
}

#[test]
fn test_icon_button_supports_mouse_click() {
    let icon_button = IconButton::new("test", "x").on_click(|_window, _cx| {});
    drop(icon_button);
}

#[test]
fn test_icon_button_all_sizes() {
    let sizes = [
        IconButtonSize::Xs,
        IconButtonSize::Sm,
        IconButtonSize::Md,
        IconButtonSize::Lg,
        IconButtonSize::Xl,
        IconButtonSize::Custom(36),
    ];

    for size in sizes {
        let btn = IconButton::new("test", "x").size(size);
        drop(btn);
    }
}

#[test]
fn test_icon_button_size_values() {
    // Sizes are in rems so the click target scales with window.set_rem_size
    // (font zoom). Sm/Md/Lg/Xl all meet WCAG 2.5.8 24×24 at 1× zoom; Xs is
    // intentionally below that floor for dense / chart-internal use.
    // If you change the rem mapping, audit every IconButton call site in
    // app-gpui — Sm and Md are visually identical at 1× zoom on purpose,
    // so changes propagate to footer-transport, dialog-close, and other
    // IconButton::Sm sites.
    assert_eq!(IconButtonSize::Xs.size(), gpui::rems(1.0));
    assert_eq!(IconButtonSize::Sm.size(), gpui::rems(1.5));
    assert_eq!(IconButtonSize::Md.size(), gpui::rems(1.5));
    assert_eq!(IconButtonSize::Lg.size(), gpui::rems(2.0));
    assert_eq!(IconButtonSize::Xl.size(), gpui::rems(3.0));
    assert_eq!(IconButtonSize::Custom(40).size(), gpui::rems(2.5));
}

#[test]
fn test_icon_button_all_variants() {
    let variants = [
        IconButtonVariant::Ghost,
        IconButtonVariant::Filled,
        IconButtonVariant::Outline,
    ];

    for variant in &variants {
        let btn = IconButton::new("test", "x").variant(*variant);
        drop(btn);
    }
}

#[test]
fn test_icon_button_with_child() {
    let btn = IconButton::with_child("test", gpui::div().child("SVG"));
    drop(btn);
}

#[test]
fn test_icon_button_disabled() {
    let btn = IconButton::new("test", "x").disabled(true);
    drop(btn);

    let btn = IconButton::new("test", "x")
        .disabled(true)
        .on_click(|_window, _cx| {});
    drop(btn);
}

#[test]
fn test_icon_button_selected() {
    let btn = IconButton::new("test", "x").selected(true);
    drop(btn);

    let btn = IconButton::new("test", "x")
        .selected(true)
        .variant(IconButtonVariant::Filled);
    drop(btn);
}

#[test]
fn test_icon_button_rounded_full() {
    let btn = IconButton::new("test", "x").rounded_full();
    drop(btn);
}

#[test]
fn test_icon_button_custom_padding() {
    let btn = IconButton::new("test", "x").padding(gpui::px(8.0));
    drop(btn);
}

#[test]
fn test_icon_button_theme_construction() {
    let theme = IconButtonTheme {
        ghost_bg: gpui::rgba(0x00000000),
        ghost_hover_bg: gpui::rgba(0x3a3a3aff),
        selected_bg: gpui::rgba(0x3a3a3aff),
        selected_hover_bg: gpui::rgba(0x4a4a4aff),
        filled_bg: gpui::rgba(0x3a3a3aff),
        filled_hover_bg: gpui::rgba(0x4a4a4aff),
        accent: gpui::rgba(0x007accff),
        accent_hover: gpui::rgba(0x0098ffff),
        text: gpui::rgba(0xccccccff),
        text_on_accent: gpui::rgba(0xffffffff),
        border: gpui::rgba(0x555555ff),
    };
    let btn = IconButton::new("themed", "x").theme(theme);
    drop(btn);
}

#[test]
fn test_icon_button_size_from_component_size() {
    let conversions: Vec<(ComponentSize, IconButtonSize)> = vec![
        (ComponentSize::Xs, IconButtonSize::Xs),
        (ComponentSize::Sm, IconButtonSize::Sm),
        (ComponentSize::Md, IconButtonSize::Md),
        (ComponentSize::Lg, IconButtonSize::Lg),
        (ComponentSize::Xl, IconButtonSize::Xl),
    ];

    for (component_size, expected) in conversions {
        let icon_size: IconButtonSize = component_size.into();
        assert_eq!(icon_size, expected);
    }
}

#[test]
fn test_icon_button_full_config() {
    let btn = IconButton::new("full", "x")
        .size(IconButtonSize::Lg)
        .variant(IconButtonVariant::Outline)
        .disabled(false)
        .selected(true)
        .rounded_full()
        .padding(gpui::px(4.0))
        .on_click(|_window, _cx| {});
    drop(btn);
}
