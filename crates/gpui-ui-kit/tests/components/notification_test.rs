//! Notification component tests

use gpui_ui_kit::notification::{Notification, NotificationVariant};

#[test]
fn test_notification_creation() {
    let notif = Notification::new("notif-1", "File saved");
    drop(notif);
}

#[test]
fn test_notification_all_variants() {
    for variant in [
        NotificationVariant::Info,
        NotificationVariant::Success,
        NotificationVariant::Warning,
        NotificationVariant::Error,
    ] {
        let notif = Notification::new("notif-v", "Message").variant(variant);
        drop(notif);
    }
}

#[test]
fn test_notification_description() {
    let notif =
        Notification::new("notif-desc", "Connected").description("Your wallet is ready to use");
    drop(notif);
}

#[test]
fn test_notification_icon() {
    let notif = Notification::new("notif-icon", "Update").icon("🔄");
    drop(notif);
}

#[test]
fn test_notification_dismissible() {
    let notif = Notification::new("notif-dismiss", "Banner").dismissible(false);
    drop(notif);
}

#[test]
fn test_notification_on_dismiss() {
    let notif = Notification::new("notif-on-dismiss", "Alert")
        .dismissible(true)
        .on_dismiss(|_window, _cx| {});
    drop(notif);
}

#[test]
fn test_notification_action() {
    let notif = Notification::new("notif-action", "Wallet connected")
        .action("Disconnect", |_window, _cx| {});
    drop(notif);
}

#[test]
fn test_notification_full_configuration() {
    let notif = Notification::new("notif-full", "Wallet connected")
        .variant(NotificationVariant::Success)
        .description("Your wallet is ready to use")
        .icon("✓")
        .dismissible(true)
        .on_dismiss(|_window, _cx| {})
        .action("Disconnect", |_window, _cx| {});
    drop(notif);
}
