//! Toast component tests

use gpui::rgb;
use gpui_ui_kit::theme::Theme;
use gpui_ui_kit::toast::{Toast, ToastContainer, ToastPosition, ToastVariant};

#[test]
fn test_toast_configuration() {
    let toast = Toast::new("toast-1", "Operation successful")
        .title("Success")
        .variant(ToastVariant::Success)
        .closeable(true)
        .duration_secs(Some(10.0))
        .on_close(|_window, _cx| {});

    drop(toast);
}

#[test]
fn test_persistent_toast() {
    let toast = Toast::new("persistent", "I stay here").persistent();
    assert!(toast.get_duration_secs().is_none());
    assert!(toast.get_duration_ms().is_none());
    drop(toast);
}

#[test]
fn test_toast_container() {
    let t1 = Toast::new("1", "One");
    let t2 = Toast::new("2", "Two");

    let container = ToastContainer::new(ToastPosition::TopRight)
        .toast(t1)
        .toasts(vec![t2]);

    drop(container);
}

#[test]
fn test_toast_variants() {
    let variants = [
        ToastVariant::Info,
        ToastVariant::Success,
        ToastVariant::Warning,
        ToastVariant::Error,
    ];

    for variant in &variants {
        let toast = Toast::new("id", "msg").variant(*variant);
        drop(toast);
    }
}

#[test]
fn test_toast_positions() {
    let positions = [
        ToastPosition::TopLeft,
        ToastPosition::TopCenter,
        ToastPosition::TopRight,
        ToastPosition::BottomLeft,
        ToastPosition::BottomCenter,
        ToastPosition::BottomRight,
    ];

    for position in &positions {
        let container = ToastContainer::new(*position);
        drop(container);
    }
}

// -- New tests --

#[test]
fn test_toast_custom_duration() {
    let toast = Toast::new("d", "msg").duration_secs(Some(15.0));
    assert_eq!(toast.get_duration_secs(), Some(15.0));
    assert_eq!(toast.get_duration_ms(), Some(15000));
    drop(toast);
}

#[test]
fn test_toast_default_duration() {
    let toast = Toast::new("d", "msg");
    assert_eq!(
        toast.get_duration_secs(),
        Some(Toast::DEFAULT_DURATION_SECS)
    );
}

#[test]
fn test_toast_closeable_flag() {
    let toast_closeable = Toast::new("c", "msg").closeable(true);
    drop(toast_closeable);

    let toast_not_closeable = Toast::new("nc", "msg").closeable(false);
    drop(toast_not_closeable);
}

#[test]
fn test_toast_uses_theme_colors_not_hardcoded() {
    let mut theme = Theme::dark();
    theme.alert_success_bg = rgb(0xfedcba);
    theme.alert_warning_bg = rgb(0x123456);
    theme.alert_error_bg = rgb(0x654321);

    let (bg, _, _) = ToastVariant::Success.colors(&theme);
    assert_eq!(
        bg, theme.alert_success_bg,
        "Success bg should use theme.alert_success_bg"
    );

    let (bg, _, _) = ToastVariant::Warning.colors(&theme);
    assert_eq!(
        bg, theme.alert_warning_bg,
        "Warning bg should use theme.alert_warning_bg"
    );

    let (bg, _, _) = ToastVariant::Error.colors(&theme);
    assert_eq!(
        bg, theme.alert_error_bg,
        "Error bg should use theme.alert_error_bg"
    );
}
