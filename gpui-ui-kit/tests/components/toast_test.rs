//! Toast component tests

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
