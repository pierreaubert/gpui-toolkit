//! Embed `AppKit` `NSView`s next to GPUI content on macOS.
//!
//! Hosting a native view (`WebKit`, `AVPlayer`, a vendor SDK view) inside a
//! GPUI window is a positioning problem, not a rendering one: the native
//! view is a sibling in the same window, so this crate reports where GPUI
//! laid out each [`MacPlatformView`] placeholder and the host moves its
//! `NSView` to match — the "follows-the-view" loop:
//!
//! 1. Place [`MacPlatformView`] elements in the GPUI tree and
//!    [`register_view`] each native view under the same id.
//! 2. Insert each native view once: [`insert_above_gpui`] for interactive
//!    views (they receive clicks directly), or [`insert_below_gpui`] for
//!    display content (video, 3D) that a transparent GPUI region reveals
//!    (`WindowBackgroundAppearance::Transparent` plus no GPUI background
//!    painted over the placeholder).
//! 3. Each frame (e.g. from your display link), call [`apply_snapshot`]
//!    to move every registered view to its placeholder's frame, and
//!    [`unregister_view`] when a placeholder leaves the tree.
//!
//! [`ns_window`] / [`window_number`] expose the hosting `NSWindow` for
//! hosts that need it directly, and [`make_first_responder`] hands the
//! keyboard to a native view. Pointer events still land on the topmost
//! sibling: GPUI receives them over below-GPUI views, so below-GPUI
//! hosting is for display plus programmatic focus, while interactive
//! native views belong above. [`VerticalStack`] plus one
//! `MacPlatformView` per section covers the common settings/editor UI.

// Rust guideline compliant 2026-10-09

mod host;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::{CString, c_char, c_void};
use std::time::{Duration, Instant};

pub use host::{
    apply_snapshot, gpui_view, insert_above_gpui, insert_below_gpui, make_first_responder,
    ns_window, remove_from_superview, set_view_frame_appkit, set_view_hidden, window_number,
};

use gpui::{
    AnyElement, App, Bounds, IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Styled,
    Window, canvas, div, px,
};

/// Opaque wrapper around a host-owned `NSView` pointer.
///
/// The view stays owned (and retained) by the host/AppKit view hierarchy;
/// this handle never retains or releases it. It exists so app code can
/// pass views around without juggling raw pointers at every call site.
///
/// # Safety
///
/// The pointer must be a live `NSView` for as long as the handle is used,
/// and — like all `AppKit` objects — it may only be used on the main thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewHandle {
    view: *mut c_void,
}

// SAFETY: the handle is only a borrowed pointer; the main-thread-only
// contract above is documented and matches how GPUI itself is used.
unsafe impl Send for ViewHandle {}
// SAFETY: see the Send impl above.
unsafe impl Sync for ViewHandle {}

impl ViewHandle {
    /// Wrap a raw `NSView` pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `view` must be null or a live `NSView` that outlives the handle.
    pub unsafe fn from_ptr(view: *mut c_void) -> Option<Self> {
        if view.is_null() {
            None
        } else {
            Some(Self { view })
        }
    }

    /// The raw `NSView` pointer, for handing to Objective-C/Swift code.
    pub fn as_ptr(self) -> *mut c_void {
        self.view
    }
}

/// A rectangle in GPUI points (top-left origin), as reported to the host.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl MacRect {
    /// Convert to `AppKit` coordinates (bottom-left origin) for a host view
    /// of the given height.
    pub fn to_appkit(self, host_height: f32) -> MacRect {
        MacRect {
            x: self.x,
            y: host_height - (self.y + self.height),
            width: self.width,
            height: self.height,
        }
    }
}

impl From<Bounds<Pixels>> for MacRect {
    fn from(bounds: Bounds<Pixels>) -> Self {
        Self {
            x: bounds.origin.x.into(),
            y: bounds.origin.y.into(),
            width: bounds.size.width.into(),
            height: bounds.size.height.into(),
        }
    }
}

/// One laid-out platform view, as of the last completed frame.
#[derive(Debug, Clone, PartialEq)]
pub struct MacPlatformViewFrame {
    pub id: String,
    pub frame: MacRect,
    pub z_index: i32,
    pub visible: bool,
    /// Window backing scale factor when the frame was painted, for
    /// hosts sizing layer-backed content (`CAMetalLayer.drawableSize`).
    pub scale_factor: f32,
}

struct FrameRecord {
    frame: MacPlatformViewFrame,
    painted_at: Instant,
}

thread_local! {
    static FRAMES: RefCell<BTreeMap<String, FrameRecord>> =
        const { RefCell::new(BTreeMap::new()) };
    static REGISTRY: RefCell<BTreeMap<String, ViewHandle>> =
        const { RefCell::new(BTreeMap::new()) };
}

fn record_frame(frame: MacPlatformViewFrame) {
    record_frame_at(frame, Instant::now());
}

fn record_frame_at(frame: MacPlatformViewFrame, painted_at: Instant) {
    FRAMES.with(|frames| {
        frames
            .borrow_mut()
            .insert(frame.id.clone(), FrameRecord { frame, painted_at });
    });
}

/// Register the native `view` for the [`MacPlatformView`] placeholder
/// with this `id`, replacing any previous registration. Insertion into
/// the window's view hierarchy is separate: see [`insert_below_gpui`]
/// and [`insert_above_gpui`].
pub fn register_view(id: impl Into<String>, view: ViewHandle) {
    REGISTRY.with(|registry| {
        registry.borrow_mut().insert(id.into(), view);
    });
}

/// Unregister `id` and forget its recorded frame, e.g. when the
/// placeholder leaves the tree. The native view stays in the window
/// hierarchy until the host removes it ([`remove_from_superview`]).
/// Returns the previously registered view, if any.
pub fn unregister_view(id: &str) -> Option<ViewHandle> {
    forget_frame(id);
    REGISTRY.with(|registry| registry.borrow_mut().remove(id))
}

/// The native view registered for `id`, if any.
pub fn registered_view(id: &str) -> Option<ViewHandle> {
    REGISTRY.with(|registry| registry.borrow().get(id).copied())
}

/// Forget the recorded frame for `id` (placeholder unmounted).
/// Returns whether a frame was recorded.
pub fn forget_frame(id: &str) -> bool {
    FRAMES.with(|frames| frames.borrow_mut().remove(id).is_some())
}

/// Drop frames not repainted within `max_age` and return how many
/// were dropped. Hosts driving views from [`snapshot_frames`] should
/// call this once per tick so unmounted placeholders do not linger.
pub fn prune_stale_frames(max_age: Duration) -> usize {
    FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        let before = frames.len();
        let now = Instant::now();
        frames.retain(|_, record| now.duration_since(record.painted_at) <= max_age);
        before - frames.len()
    })
}

/// Frames of every live [`MacPlatformView`], sorted back-tofront
/// (`z_index`, then id). Call once per host frame tick.
pub fn snapshot_frames() -> Vec<MacPlatformViewFrame> {
    FRAMES.with(|frames| {
        let mut snapshot: Vec<MacPlatformViewFrame> = frames
            .borrow()
            .values()
            .map(|record| record.frame.clone())
            .collect();
        snapshot.sort_by(|left, right| {
            left.z_index
                .cmp(&right.z_index)
                .then_with(|| left.id.cmp(&right.id))
        });
        snapshot
    })
}

/// The latest frame recorded for `id`, if that view has painted.
pub fn frame_for(id: &str) -> Option<MacPlatformViewFrame> {
    FRAMES.with(|frames| frames.borrow().get(id).map(|record| record.frame.clone()))
}

/// Drop all recorded frames and view registrations (host teardown;
/// also used by tests).
pub fn clear_frames() {
    FRAMES.with(|frames| frames.borrow_mut().clear());
    REGISTRY.with(|registry| registry.borrow_mut().clear());
}

/// A GPUI placeholder for a host-owned `NSView`.
///
/// Paints nothing itself; on every frame it records its window-space
/// bounds for [`snapshot_frames`]. Size it with normal GPUI layout (it
/// fills the space its parent gives it by default).
#[derive(IntoElement)]
pub struct MacPlatformView {
    id: SharedString,
    #[allow(dead_code, reason = "held for the host to retrieve via view()")]
    view: ViewHandle,
    z_index: i32,
    visible: bool,
}

impl MacPlatformView {
    pub fn new(id: impl Into<SharedString>, view: ViewHandle) -> Self {
        Self {
            id: id.into(),
            view,
            z_index: 0,
            visible: true,
        }
    }

    /// Stacking order among platform views (back-to-front in snapshots).
    pub fn z_index(mut self, z_index: i32) -> Self {
        self.z_index = z_index;
        self
    }

    /// Report the view hidden without removing it from the tree.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// The host view this placeholder stands for.
    pub fn view(&self) -> ViewHandle {
        self.view
    }
}

impl RenderOnce for MacPlatformView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl gpui::IntoElement {
        let id = self.id.to_string();
        let z_index = self.z_index;
        let visible = self.visible;
        canvas(
            move |bounds, window, _cx| {
                record_frame(MacPlatformViewFrame {
                    id,
                    frame: bounds.into(),
                    z_index,
                    visible,
                    scale_factor: window.scale_factor(),
                });
            },
            |_bounds, _state, _window, _cx| {},
        )
        .size_full()
    }
}

/// Y offsets for a simple top-to-bottom stack of fixed-height sections.
///
/// Pure layout math for hosts stacking platform views manually; the first
/// section sits at `0` and each later one below the previous plus
/// `spacing` points of gap.
pub fn vertical_stack_offsets(heights: &[f32], spacing: f32) -> Vec<f32> {
    let mut offsets = Vec::with_capacity(heights.len());
    let mut y = 0.0;
    for height in heights {
        offsets.push(y);
        y += height + spacing;
    }
    offsets
}

/// A fixed-height vertical stack for settings/editor-style UIs.
///
/// Pairs well with one [`MacPlatformView`] per section: GPUI content and
/// native sections share one predictable top-to-bottom flow.
#[derive(IntoElement)]
pub struct VerticalStack {
    children: Vec<AnyElement>,
    spacing: Pixels,
}

impl Default for VerticalStack {
    fn default() -> Self {
        Self::new()
    }
}

impl VerticalStack {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            spacing: px(0.0),
        }
    }

    pub fn spacing(mut self, spacing: Pixels) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn child(mut self, child: impl gpui::IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl RenderOnce for VerticalStack {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl gpui::IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(self.spacing)
            .children(self.children)
    }
}

/// JSON snapshot of [`snapshot_frames`] for Swift/ObjC hosts.
///
/// The caller owns the returned string and must release it with
/// `gpui_macos_platform_views_free_string`. Returns null only on
/// allocation/serialization failure.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_macos_platform_views_snapshot_json() -> *mut c_char {
    let frames: Vec<serde_json::Value> = snapshot_frames()
        .iter()
        .map(|frame| {
            serde_json::json!({
                "id": frame.id,
                "x": frame.frame.x,
                "y": frame.frame.y,
                "width": frame.frame.width,
                "height": frame.frame.height,
                "z_index": frame.z_index,
                "visible": frame.visible,
                "scale_factor": frame.scale_factor,
            })
        })
        .collect();
    let Ok(json) = serde_json::to_string(&frames) else {
        return std::ptr::null_mut();
    };
    CString::new(json).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// Free a string returned by `gpui_macos_platform_views_snapshot_json`.
///
/// # Safety
///
/// `ptr` must be null or a live pointer from that function, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_macos_platform_views_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: per this function's contract the pointer came from
    // `CString::into_raw` in `gpui_macos_platform_views_snapshot_json`.
    drop(unsafe { CString::from_raw(ptr) });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appkit_conversion_flips_y() {
        let rect = MacRect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let flipped = rect.to_appkit(600.0);
        assert_eq!(flipped.y, 530.0);
        assert_eq!(flipped.x, rect.x);
    }

    #[test]
    fn vertical_stack_offsets_accumulate_heights_and_spacing() {
        assert_eq!(
            vertical_stack_offsets(&[100.0, 50.0, 25.0], 10.0),
            vec![0.0, 110.0, 170.0]
        );
        assert!(vertical_stack_offsets(&[], 10.0).is_empty());
    }

    #[test]
    fn snapshot_sorts_back_to_front() {
        clear_frames();
        record_frame(MacPlatformViewFrame {
            id: "front".into(),
            frame: MacRect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            z_index: 2,
            visible: true,
            scale_factor: 2.0,
        });
        record_frame(MacPlatformViewFrame {
            id: "back".into(),
            frame: MacRect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            z_index: -1,
            visible: false,
            scale_factor: 2.0,
        });

        let snapshot = snapshot_frames();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].id, "back");
        assert_eq!(snapshot[1].id, "front");
        assert_eq!(frame_for("front").map(|frame| frame.z_index), Some(2));
        clear_frames();
        assert!(snapshot_frames().is_empty());
    }

    #[test]
    fn snapshot_json_round_trips_and_frees() {
        clear_frames();
        record_frame(MacPlatformViewFrame {
            id: "web".into(),
            frame: MacRect {
                x: 1.5,
                y: 2.5,
                width: 30.0,
                height: 40.0,
            },
            z_index: 1,
            visible: true,
            scale_factor: 2.0,
        });

        let raw = gpui_macos_platform_views_snapshot_json();
        assert!(!raw.is_null());
        // SAFETY: raw came from the snapshot function above.
        let json = unsafe { std::ffi::CStr::from_ptr(raw) }
            .to_str()
            .expect("snapshot JSON is UTF-8")
            .to_owned();
        // SAFETY: freeing exactly the pointer returned above, once.
        unsafe { gpui_macos_platform_views_free_string(raw) };

        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed[0]["id"], "web");
        assert_eq!(parsed[0]["z_index"], 1);
        clear_frames();
    }

    #[test]
    fn view_handle_rejects_null() {
        // SAFETY: null is the documented rejection case.
        assert!(unsafe { ViewHandle::from_ptr(std::ptr::null_mut()) }.is_none());
    }

    fn test_frame(id: &str) -> MacPlatformViewFrame {
        MacPlatformViewFrame {
            id: id.into(),
            frame: MacRect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            z_index: 0,
            visible: true,
            scale_factor: 1.0,
        }
    }

    #[test]
    fn registry_round_trips_and_unregister_forgets_frame() {
        clear_frames();
        // SAFETY: the registry never dereferences the handle; the
        // dangling pointer is only compared for identity here.
        let view = unsafe { ViewHandle::from_ptr(std::ptr::dangling_mut::<c_void>()) }.unwrap();
        register_view("web", view);
        record_frame(test_frame("web"));

        assert_eq!(registered_view("web"), Some(view));
        assert_eq!(unregister_view("web"), Some(view));
        assert_eq!(registered_view("web"), None);
        assert_eq!(frame_for("web"), None);
        assert_eq!(unregister_view("web"), None);
    }

    #[test]
    fn prune_stale_frames_drops_only_unpainted() {
        clear_frames();
        let past = Instant::now().checked_sub(Duration::from_secs(10)).unwrap();
        record_frame_at(test_frame("stale"), past);
        record_frame(test_frame("fresh"));

        assert_eq!(prune_stale_frames(Duration::from_secs(1)), 1);
        assert_eq!(frame_for("stale"), None);
        assert!(frame_for("fresh").is_some());
        assert!(forget_frame("fresh"));
        assert!(!forget_frame("fresh"));
        clear_frames();
    }

    #[test]
    fn snapshot_json_includes_scale_factor() {
        clear_frames();
        record_frame(test_frame("web"));

        let raw = gpui_macos_platform_views_snapshot_json();
        assert!(!raw.is_null());
        // SAFETY: raw came from the snapshot function above.
        let json = unsafe { std::ffi::CStr::from_ptr(raw) }
            .to_str()
            .expect("snapshot JSON is UTF-8")
            .to_owned();
        // SAFETY: freeing exactly the pointer returned above, once.
        unsafe { gpui_macos_platform_views_free_string(raw) };

        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed[0]["scale_factor"], 1.0);
        clear_frames();
    }
}
