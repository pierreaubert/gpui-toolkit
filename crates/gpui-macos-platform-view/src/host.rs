//! `AppKit` host integration: reach the `NSWindow`, insert native views
//! below or above the GPUI view, and drive their frames from
//! [`snapshot_frames`](crate::snapshot_frames).
//!
//! All functions must be called on the main thread (like GPUI itself).
//! Off macOS they are inert stubs returning `None`/`false`/`0` so
//! cross-platform hosts can call them unconditionally.

// Rust guideline compliant 2026-10-09

use std::ffi::c_void;

use gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::{MacRect, ViewHandle, registered_view, snapshot_frames};

#[cfg(target_os = "macos")]
use objc2_app_kit::{NSView, NSWindow, NSWindowOrderingMode};

/// The GPUI view itself (`GPUIView`), as reported by the window's
/// `raw_window_handle`. Its superview is the window's content view.
pub fn gpui_view(window: &Window) -> Option<ViewHandle> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return None;
    };
    // SAFETY: the AppKit handle of a live GPUI window is its NSView,
    // and this runs on the main thread.
    unsafe { ViewHandle::from_ptr(appkit.ns_view.as_ptr()) }
}

#[cfg(target_os = "macos")]
fn ns_view(handle: ViewHandle) -> &'static NSView {
    // SAFETY: the handle contract requires a live NSView used on the
    // main thread; the reference is only used for immediate AppKit
    // calls within the calling function.
    unsafe { &*handle.as_ptr().cast::<NSView>() }
}

#[cfg(target_os = "macos")]
fn ns_window_retained(window: &Window) -> Option<objc2::rc::Retained<NSWindow>> {
    ns_view(gpui_view(window)?).window()
}

/// The raw `NSWindow*` hosting this GPUI window, if any.
///
/// Non-owning: the window is owned by GPUI/AppKit and must not be
/// retained or released by the caller.
pub fn ns_window(window: &Window) -> Option<*mut c_void> {
    #[cfg(target_os = "macos")]
    {
        let ns_window = ns_window_retained(window)?;
        Some(objc2::rc::Retained::as_ptr(&ns_window) as *mut c_void)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        None
    }
}

/// The `NSWindow.windowNumber` of the hosting window, if any.
pub fn window_number(window: &Window) -> Option<isize> {
    #[cfg(target_os = "macos")]
    {
        Some(ns_window_retained(window)?.windowNumber())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        None
    }
}

/// Height of the window's content view in points (for
/// [`MacRect::to_appkit`] conversion), if the window is AppKit-backed.
#[cfg(target_os = "macos")]
fn content_height(window: &Window) -> Option<f32> {
    let content = ns_window_retained(window)?.contentView()?;
    Some(content.bounds().size.height as f32)
}

#[cfg(target_os = "macos")]
fn insert(window: &Window, view: ViewHandle, mode: NSWindowOrderingMode) -> bool {
    let Some(gpui_handle) = gpui_view(window) else {
        return false;
    };
    let gpui_ns_view = ns_view(gpui_handle);
    // SAFETY: GPUIView was added to the content view at window
    // creation; `superview` is only unsafe to *store* unretained,
    // and the retained value is used immediately.
    let Some(content_view) = (unsafe { gpui_ns_view.superview() }) else {
        return false;
    };
    content_view.addSubview_positioned_relativeTo(ns_view(view), mode, Some(gpui_ns_view));
    true
}

/// Insert `view` into the window's content view directly **below**
/// the GPUI view, so transparent GPUI regions reveal it. Pair with a
/// window created with a transparent background appearance and no
/// GPUI background painted over the placeholder. Returns `false` if
/// the window is not AppKit-backed (or has no content view yet).
///
/// For several below-GPUI views, insert them back-to-front
/// ([`snapshot_frames`] order): each insertion lands directly below
/// the GPUI view, in front of previously inserted ones.
pub fn insert_below_gpui(window: &Window, view: ViewHandle) -> bool {
    #[cfg(target_os = "macos")]
    {
        insert(window, view, NSWindowOrderingMode::Below)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, view);
        false
    }
}

/// Insert `view` into the window's content view directly **above**
/// the GPUI view (the M1 sibling-overlay arrangement: the native view
/// receives pointer events itself). Returns `false` under the same
/// conditions as [`insert_below_gpui`].
pub fn insert_above_gpui(window: &Window, view: ViewHandle) -> bool {
    #[cfg(target_os = "macos")]
    {
        insert(window, view, NSWindowOrderingMode::Above)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, view);
        false
    }
}

/// Remove `view` from its superview, if it has one.
pub fn remove_from_superview(view: ViewHandle) {
    #[cfg(target_os = "macos")]
    ns_view(view).removeFromSuperview();
    #[cfg(not(target_os = "macos"))]
    let _ = view;
}

/// Set `view`'s frame, in `AppKit` coordinates (bottom-left origin, as
/// produced by [`MacRect::to_appkit`]).
pub fn set_view_frame_appkit(view: ViewHandle, frame: MacRect) {
    #[cfg(target_os = "macos")]
    ns_view(view).setFrame(objc2_foundation::NSRect {
        origin: objc2_foundation::NSPoint {
            x: f64::from(frame.x),
            y: f64::from(frame.y),
        },
        size: objc2_foundation::NSSize {
            width: f64::from(frame.width),
            height: f64::from(frame.height),
        },
    });
    #[cfg(not(target_os = "macos"))]
    let _ = (view, frame);
}

/// Show or hide `view` without removing it from the hierarchy.
pub fn set_view_hidden(view: ViewHandle, hidden: bool) {
    #[cfg(target_os = "macos")]
    ns_view(view).setHidden(hidden);
    #[cfg(not(target_os = "macos"))]
    let _ = (view, hidden);
}

/// Make `view` the window's first responder, returning whether `AppKit`
/// accepted it.
pub fn make_first_responder(window: &Window, view: ViewHandle) -> bool {
    #[cfg(target_os = "macos")]
    {
        let Some(ns_window) = ns_window_retained(window) else {
            return false;
        };
        ns_window.makeFirstResponder(Some(ns_view(view)))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, view);
        false
    }
}

/// Drive every registered view to its placeholder's latest frame:
/// converts to `AppKit` coordinates with the content view's height and
/// applies visibility. Call once per host frame tick (display link,
/// GPUI task, ...). Returns how many views were updated.
pub fn apply_snapshot(window: &Window) -> usize {
    #[cfg(target_os = "macos")]
    {
        let Some(height) = content_height(window) else {
            return 0;
        };
        let mut applied = 0;
        for frame in snapshot_frames() {
            let Some(view) = registered_view(&frame.id) else {
                continue;
            };
            set_view_frame_appkit(view, frame.frame.to_appkit(height));
            set_view_hidden(view, !frame.visible);
            applied += 1;
        }
        applied
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        0
    }
}
