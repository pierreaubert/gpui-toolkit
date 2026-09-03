use objc::{msg_send, sel, sel_impl};

pub(super) const GPUI_WINDOW_IVAR: &str = "gpui_window_ptr";

pub(super) static METAL_VIEW_CLASS_REGISTERED: std::sync::Once = std::sync::Once::new();

pub(super) static VC_CLASS_REGISTERED: std::sync::Once = std::sync::Once::new();

#[cfg(target_os = "ios")]
pub(super) static WINDOW_CLASS_REGISTERED: std::sync::Once = std::sync::Once::new();

pub(super) static TEXT_INPUT_VIEW_CLASS_REGISTERED: std::sync::Once = std::sync::Once::new();

pub(super) static ACCESSIBILITY_ELEMENT_CLASS_REGISTERED: std::sync::Once = std::sync::Once::new();

/// Global storage for the current status bar style.
/// 0 = default (dark content), 1 = light content.
/// Accessed from the main thread only.
pub(super) static STATUS_BAR_STYLE: std::sync::atomic::AtomicI32 =
    std::sync::atomic::AtomicI32::new(0);

/// Set the iOS status bar content style (light or dark text/icons).
///
/// This updates the stored style and asks the root view controller
/// to re-query `preferredStatusBarStyle`.
#[allow(dead_code)]
pub fn set_status_bar_style(style: crate::StatusBarContentStyle) {
    use crate::StatusBarContentStyle;

    let value = match style {
        StatusBarContentStyle::Light => 1,
        StatusBarContentStyle::Dark => 0,
    };
    STATUS_BAR_STYLE.store(value, std::sync::atomic::Ordering::Relaxed);

    // Ask UIKit to re-query the status bar style
    unsafe {
        if let Some(wrapper) = super::super::ffi::IOS_WINDOW_LIST.get() {
            let windows = &*wrapper.0.get();
            if let Some(&window_ptr) = windows.last()
                && !window_ptr.is_null()
            {
                let window = &*window_ptr;
                let vc = window.view_controller;
                if !vc.is_null() {
                    let _: () = msg_send![vc, setNeedsStatusBarAppearanceUpdate];
                }
            }
        }
    }
}

/// iOS Window backed by UIWindow + UIViewController.
/// Distance (logical px) the finger must travel before a touch
/// is promoted from a potential tap to a scroll gesture.
pub(super) const SCROLL_SLOP: f32 = 8.0;
