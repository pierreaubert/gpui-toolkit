//! Alert component tests

use gpui::rgb;
use gpui_ui_kit::alert::{Alert, AlertVariant, InlineAlert};
use gpui_ui_kit::theme::Theme;

#[test]
fn test_alert_creation() {
    let alert = Alert::new("test-alert", "This is an alert message");
    drop(alert);
}

#[test]
fn test_alert_variants() {
    let variants = [
        AlertVariant::Info,
        AlertVariant::Success,
        AlertVariant::Warning,
        AlertVariant::Error,
    ];

    for variant in &variants {
        let alert = Alert::new("id", "msg").variant(*variant);
        drop(alert);
    }
}

#[test]
fn test_alert_configuration() {
    let alert = Alert::new("id", "msg")
        .title("Title")
        .closeable(true)
        .icon("bell")
        .on_close(|_window, _cx| {});

    drop(alert);
}

#[test]
fn test_inline_alert() {
    let alert = InlineAlert::new("Inline message").variant(AlertVariant::Warning);
    drop(alert);
}

// -- New tests --

#[test]
fn test_inline_alert_all_variants() {
    let variants = [
        AlertVariant::Info,
        AlertVariant::Success,
        AlertVariant::Warning,
        AlertVariant::Error,
    ];

    for variant in &variants {
        let alert = InlineAlert::new("msg").variant(*variant);
        drop(alert);
    }
}

#[test]
fn test_alert_not_closeable() {
    let alert = Alert::new("id", "msg").closeable(false);
    drop(alert);
}

#[test]
fn test_alert_uses_theme_colors_not_hardcoded() {
    let mut theme = Theme::dark();
    theme.alert_info_bg = rgb(0xabcdef);
    theme.alert_success_bg = rgb(0xfedcba);
    theme.alert_warning_bg = rgb(0x123456);
    theme.alert_error_bg = rgb(0x654321);

    let (bg, _, _) = AlertVariant::Info.colors(&theme);
    assert_eq!(bg, theme.alert_info_bg, "Info bg should use theme.alert_info_bg");

    let (bg, _, _) = AlertVariant::Success.colors(&theme);
    assert_eq!(bg, theme.alert_success_bg, "Success bg should use theme.alert_success_bg");

    let (bg, _, _) = AlertVariant::Warning.colors(&theme);
    assert_eq!(bg, theme.alert_warning_bg, "Warning bg should use theme.alert_warning_bg");

    let (bg, _, _) = AlertVariant::Error.colors(&theme);
    assert_eq!(bg, theme.alert_error_bg, "Error bg should use theme.alert_error_bg");
}
