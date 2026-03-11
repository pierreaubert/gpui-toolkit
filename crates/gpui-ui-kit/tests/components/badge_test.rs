//! Badge component tests

use gpui_ui_kit::badge::{Badge, BadgeDot, BadgeSize, BadgeVariant};

#[test]
fn test_badge_variants() {
    let variants = [
        BadgeVariant::Default,
        BadgeVariant::Primary,
        BadgeVariant::Success,
        BadgeVariant::Warning,
        BadgeVariant::Error,
    ];

    for variant in &variants {
        let badge = Badge::new("test").variant(*variant);
        let _ = badge;
    }
}

#[test]
fn test_badge_creation() {
    let badge = Badge::new("Badge Text");
    let _ = badge;
}

#[test]
fn test_badge_all_sizes() {
    let sizes = [BadgeSize::Sm, BadgeSize::Md, BadgeSize::Lg];

    for size in &sizes {
        let badge = Badge::new("test").size(*size);
        let _ = badge;
    }
}

#[test]
fn test_badge_rounded() {
    let badge = Badge::new("Rounded").rounded(true);
    let _ = badge;

    let badge = Badge::new("Not Rounded").rounded(false);
    let _ = badge;
}

#[test]
fn test_badge_with_icon() {
    let badge = Badge::new("Status").icon("x");
    let _ = badge;
}

#[test]
fn test_badge_dot_creation() {
    let dot = BadgeDot::new();
    let _ = dot;
}

#[test]
fn test_badge_dot_variants() {
    let variants = [
        BadgeVariant::Default,
        BadgeVariant::Primary,
        BadgeVariant::Success,
        BadgeVariant::Warning,
        BadgeVariant::Error,
    ];

    for variant in &variants {
        let dot = BadgeDot::new().variant(*variant);
        let _ = dot;
    }
}

#[test]
fn test_badge_dot_custom_size() {
    let dot = BadgeDot::new().size(gpui::px(12.0));
    let _ = dot;
}
