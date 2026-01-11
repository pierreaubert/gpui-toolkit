//! Badge component tests

use gpui_ui_kit::badge::{Badge, BadgeVariant};

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
        drop(badge);
    }
}

#[test]
fn test_badge_creation() {
    let badge = Badge::new("Badge Text");
    drop(badge);
}
