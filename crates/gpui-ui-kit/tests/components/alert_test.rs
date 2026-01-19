//! Alert component tests

use gpui_ui_kit::alert::{Alert, AlertVariant, InlineAlert};

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
        .icon("🔔")
        .on_close(|_window, _cx| {});

    drop(alert);
}

#[test]
fn test_inline_alert() {
    let alert = InlineAlert::new("Inline message").variant(AlertVariant::Warning);
    drop(alert);
}
