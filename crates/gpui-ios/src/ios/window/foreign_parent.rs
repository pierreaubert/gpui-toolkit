//! Foreign-parent embedding for iOS: GPUI editors inside host views.
//!
//! Implements the iOS binding of `docs/foreign-parent-editor-embedding.md`
//! (`AUv3` plugin views and other hosts that own the `UIView`). Unlike
//! [`IosWindow::new`](super::ios_window::IosWindow::new), the foreign
//! constructor creates no `UIWindow` or view controller: the Metal view and
//! the hidden text-input view become subviews of the host-provided parent.
//!
//! Input needs no per-event FFI forwarding: the foreign window reuses the
//! registered Metal/text-input view classes, so touches, hardware keys, and
//! marked-text sessions route through the shared pipeline automatically.
//! Touch mapping follows the compatibility pipeline documented in
//! [`touch`](super::touch): taps become deferred `MouseDown` + `MouseUp`,
//! drags emit `ScrollWheel` deltas (plus `MouseMove` for interactive
//! screens), and two-finger gestures emit native `Pinch` events.
//! The host only pumps frames (`CADisplayLink`), forwards resizes, and
//! reports focus/hover through the C entry points in `ffi/foreign_parent.rs`.

use super::ios_window::IosWindow;
use super::register::{register_metal_view_class, register_text_input_view_class};
use crate::momentum::{MomentumScroller, VelocityTracker};
use crate::platform_view::NativePlatformViewHost;
use gpui::{AnyWindowHandle, Bounds, DevicePixels, Size, WindowParams, px, size};
use gpui_foreign_parent::{FrameThrottle, ResizeDebounce, TransitionCell};
#[cfg(target_os = "ios")]
use objc::runtime::NO;
use objc::{
    class, msg_send,
    runtime::{BOOL, Object, YES},
    sel, sel_impl,
};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::types::{PinchState, TouchStateMap};

/// Host-provided parent view plus the editor id it was created for.
///
/// Stashed in [`PENDING_PARENT`] by the foreign create entry point so
/// [`IosPlatform::open_window`](super::super::platform::IosPlatform::open_window)
/// can route construction to [`IosWindow::new_in_parent_view`].
pub(crate) struct ParentViewInfo {
    pub(crate) parent_view: *mut Object,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) scale: f32,
    pub(crate) editor_id: u64,
}

thread_local! {
    pub(crate) static PENDING_PARENT: RefCell<Option<ParentViewInfo>> =
        const { RefCell::new(None) };
}

/// Take the pending foreign-parent handoff, if the last create stashed one.
pub(crate) fn take_pending_parent() -> Option<ParentViewInfo> {
    PENDING_PARENT.with(|pending| pending.borrow_mut().take())
}

/// Per-window foreign-embedding policy state.
///
/// Default-inert on the app path; driven by the foreign FFI entry points
/// when the window has no `UIWindow`.
#[derive(Debug)]
pub(crate) struct ForeignParentState {
    throttle: FrameThrottle,
    debounce: ResizeDebounce<Size<DevicePixels>>,
    active: TransitionCell,
    hovered: TransitionCell,
    dropped_frames: AtomicUsize,
}

impl Default for ForeignParentState {
    fn default() -> Self {
        Self {
            throttle: FrameThrottle::default(),
            debounce: ResizeDebounce::default(),
            active: TransitionCell::new(true),
            hovered: TransitionCell::new(false),
            dropped_frames: AtomicUsize::new(0),
        }
    }
}

impl ForeignParentState {
    /// True when the current host-pump tick should forward a frame.
    pub(crate) fn should_forward_frame(&self) -> bool {
        self.throttle.should_forward()
    }

    /// Display-link ticks coalesced by the frame throttle.
    pub(crate) fn coalesced_frames(&self) -> usize {
        self.throttle.coalesced()
    }

    /// Submit the newest drawable size for debounced reconfiguration.
    pub(crate) fn submit_surface_size(
        &self,
        drawable: Size<DevicePixels>,
        apply: impl FnOnce(Size<DevicePixels>) -> bool,
    ) {
        self.debounce.submit(drawable, apply);
    }

    /// Apply a stashed drawable size ahead of drawing, if due.
    pub(crate) fn poll_surface_size(&self, apply: impl FnOnce(Size<DevicePixels>) -> bool) {
        self.debounce.poll(apply);
    }

    /// Count a frame dropped on a contended renderer.
    pub(crate) fn note_dropped_frame(&self) {
        self.dropped_frames.fetch_add(1, Ordering::Relaxed);
    }

    /// Frames dropped because the renderer was busy.
    pub(crate) fn dropped_frames(&self) -> usize {
        self.dropped_frames.load(Ordering::Relaxed)
    }

    /// Current host-reported focus state.
    pub(crate) fn is_active(&self) -> bool {
        self.active.get()
    }

    /// Current host-reported pointer-in-view state.
    pub(crate) fn is_hovered(&self) -> bool {
        self.hovered.get()
    }

    /// Install the focus-transition callback.
    pub(crate) fn on_active_change(&self, callback: impl FnMut(bool) + 'static) {
        self.active.on_change(callback);
    }

    /// Install the hover-transition callback.
    pub(crate) fn on_hover_change(&self, callback: impl FnMut(bool) + 'static) {
        self.hovered.on_change(callback);
    }

    /// Update host focus state, notifying only on transition.
    pub(crate) fn update_active(&self, is_active: bool) {
        self.active.update(is_active);
    }

    /// Update pointer-in-view state, notifying only on transition.
    pub(crate) fn update_hovered(&self, is_hovered: bool) {
        self.hovered.update(is_hovered);
    }
}

impl IosWindow {
    /// Create a window rendering into a host-provided `UIView`.
    ///
    /// Consumes the one-shot handoff taken by the platform when the
    /// foreign create entry point ran. Creates no `UIWindow` or view
    /// controller; the Metal view and hidden text-input view are added as
    /// subviews of the foreign parent. Renderer construction is
    /// best-effort per contract §5: a GPU failure is logged and retried
    /// from later frames instead of failing the attach.
    pub(crate) fn new_in_parent_view(
        handle: AnyWindowHandle,
        _params: WindowParams,
        info: ParentViewInfo,
    ) -> anyhow::Result<Self> {
        // SAFETY: `NSThread` and its `isMainThread` class method always
        // exist; the message takes no arguments and returns a `BOOL`.
        #[cfg(debug_assertions)]
        let is_main: BOOL = unsafe { msg_send![class!(NSThread), isMainThread] };
        #[cfg(debug_assertions)]
        assert!(
            is_main == YES,
            "IosWindow must be created on the main thread"
        );
        if info.parent_view.is_null() {
            anyhow::bail!("foreign-parent view is null");
        }

        // SAFETY: `parent_view` is null-checked above and documented as a
        // live `UIView` outliving the window; every created view is added
        // to the UIKit hierarchy below, which retains it.
        unsafe {
            let parent = info.parent_view;
            let frame = core_graphics::geometry::CGRect {
                origin: core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
                size: core_graphics::geometry::CGSize {
                    width: f64::from(info.width),
                    height: f64::from(info.height),
                },
            };

            // Our custom Metal view, as a subview of the foreign parent.
            let metal_view_class = register_metal_view_class();
            let view: *mut Object = msg_send![metal_view_class, alloc];
            let view: *mut Object = msg_send![view, initWithFrame: frame];

            let layer: *mut Object = msg_send![view, layer];
            let _: () = msg_send![
                layer,
                setContentsScale: info.scale as core_graphics::base::CGFloat
            ];

            // Follow the parent on host resizes; same mask as the app path.
            let _: () = msg_send![view, setAutoresizingMask: 18_usize]; // 0x02 | 0x10
            let _: () = msg_send![view, setUserInteractionEnabled: YES];
            let _: () = msg_send![view, setMultipleTouchEnabled: YES];

            #[cfg(target_os = "ios")]
            {
                // Indirect (trackpad/mouse-wheel) scrolls arrive through a
                // pan recognizer, mirroring the app-path setup.
                let recognizer: *mut Object = msg_send![class!(UIPanGestureRecognizer), alloc];
                let recognizer: *mut Object =
                    msg_send![recognizer, initWithTarget: view action: sel!(handleIndirectScroll:)];
                let _: () = msg_send![recognizer, setAllowedScrollTypesMask: 3_isize];
                let _: () = msg_send![recognizer, setDelegate: view];
                let _: () = msg_send![recognizer, setRequiresExclusiveTouchType: NO];
                let _: () = msg_send![recognizer, setCancelsTouchesInView: NO];
                let _: () = msg_send![recognizer, setDelaysTouchesBegan: NO];
                let _: () = msg_send![recognizer, setDelaysTouchesEnded: NO];
                let _: () = msg_send![view, addGestureRecognizer: recognizer];
            }

            let _: () = msg_send![parent, addSubview: view];

            // Hidden text-input view, mirroring the app path.
            let text_input_class = register_text_input_view_class();
            let text_input_view: *mut Object = msg_send![text_input_class, alloc];
            let text_input_frame = core_graphics::geometry::CGRect {
                origin: core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
                size: core_graphics::geometry::CGSize {
                    width: 1.0,
                    height: 1.0,
                },
            };
            let text_input_view: *mut Object =
                msg_send![text_input_view, initWithFrame: text_input_frame];
            let _: () = msg_send![text_input_view, setAlpha: 0.01_f64];
            let _: () = msg_send![text_input_view, setUserInteractionEnabled: YES];
            let _: () = msg_send![view, addSubview: text_input_view];

            let window = Self {
                window: std::ptr::null_mut(),
                view_controller: std::ptr::null_mut(),
                view,
                text_input_view,
                bounds: Cell::new(Bounds {
                    origin: Default::default(),
                    size: size(px(info.width), px(info.height)),
                }),
                scale_factor: Cell::new(info.scale),
                transparent: Cell::new(false),
                input_handler: RefCell::new(None),
                request_frame_callback: RefCell::new(None),
                forced_frame_pending: Cell::new(false),
                input_callback: RefCell::new(None),
                input_dispatching: Cell::new(false),
                pending_input_events: RefCell::new(VecDeque::new()),
                active_status_callback: RefCell::new(None),
                hover_status_callback: RefCell::new(None),
                resize_callback: RefCell::new(None),
                moved_callback: RefCell::new(None),
                should_close_callback: RefCell::new(None),
                hit_test_callback: RefCell::new(None),
                close_callback: RefCell::new(None),
                appearance_changed_callback: RefCell::new(None),
                mouse_position: Cell::new(gpui::Point::default()),
                modifiers: Cell::new(gpui::Modifiers::default()),
                touch_pressed: Cell::new(false),
                touch_states: RefCell::new(TouchStateMap::new()),
                direct_claimed_touches: RefCell::new(HashSet::new()),
                touch_dispatching: Cell::new(false),
                pending_touches: RefCell::new(VecDeque::new()),
                pinch_state: RefCell::new(PinchState::default()),
                velocity_tracker: RefCell::new(VelocityTracker::new()),
                momentum_scroller: RefCell::new(MomentumScroller::new()),
                momentum_pumping: Cell::new(false),
                keyboard_observers: RefCell::new(Vec::new()),
                renderer: parking_lot::Mutex::new(None),
                sprite_atlas: parking_lot::Mutex::new(None),
                accessibility_elements: RefCell::new(HashMap::new()),
                prev_accessibility_snapshot: RefCell::new(None),
                accessibility_diff_scratch: RefCell::new(Default::default()),
                platform_view_host: NativePlatformViewHost::new(view.cast::<c_void>()),
                foreign: ForeignParentState::default(),
            };

            window.focus_hardware_keyboard_view();

            let pixel_w = (info.width * info.scale).round().max(1.0) as i32;
            let pixel_h = (info.height * info.scale).round().max(1.0) as i32;
            if let Err(e) = super::renderer::init_metal_renderer(
                &window,
                handle,
                size(DevicePixels(pixel_w), DevicePixels(pixel_h)),
            ) {
                // Best-effort: keep the placeholder layer and let later
                // resizes/frames converge instead of failing the attach.
                log::error!("GPUI iOS: foreign renderer init deferred: {e:#}");
            }

            Ok(window)
        }
    }

    /// Register a foreign window after it reaches a stable address.
    ///
    /// Sets the view ivars and keyboard observers like the app path, but
    /// skips the app window list so host lifecycle broadcasts never drive
    /// embedded editors.
    pub(crate) fn register_foreign_with_ffi(&self) {
        self.set_view_ivars();
        self.register_keyboard_observers();
    }

    /// Request a frame from the host display link (foreign path).
    pub(crate) fn request_frame_foreign(&self) {
        if !self.foreign.should_forward_frame() {
            return;
        }
        super::super::ffi::request_frame_for_window(self);
    }

    /// Apply a debounced foreign resize ahead of drawing, if one is due.
    /// No-op on the app path (which never stashes a pending size).
    pub(super) fn poll_foreign_surface_size(&self) {
        self.foreign.poll_surface_size(|size| {
            if let Some(mut renderer) = self.renderer.try_lock()
                && let Some(renderer) = renderer.as_mut()
            {
                renderer.update_drawable_size(size);
                true
            } else {
                false
            }
        });
    }

    /// Host-driven resize (foreign path).
    ///
    /// Bounds, scale, and the Metal layer track the newest host geometry
    /// immediately; only the surface reconfigure is debounced.
    pub(crate) fn resize_foreign(&self, width: f32, height: f32, scale: f32) {
        let new_size = size(px(width), px(height));
        self.bounds.set(Bounds {
            origin: Default::default(),
            size: new_size,
        });
        self.scale_factor.set(scale);
        crate::accessibility::update_accesskit_scale_factor(scale);

        // SAFETY: `view` is the live Metal subview created by the foreign
        // constructor; `layer` messaging uses valid selectors.
        unsafe {
            let layer: *mut Object = msg_send![self.view, layer];
            let _: () = msg_send![layer, setContentsScale: scale as core_graphics::base::CGFloat];
            let frame = core_graphics::geometry::CGRect {
                origin: core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
                size: core_graphics::geometry::CGSize {
                    width: f64::from(width),
                    height: f64::from(height),
                },
            };
            let _: () = msg_send![self.view, setFrame: frame];
        }

        let pixel_w = (width * scale).round().max(1.0) as i32;
        let pixel_h = (height * scale).round().max(1.0) as i32;
        self.foreign.submit_surface_size(
            size(DevicePixels(pixel_w), DevicePixels(pixel_h)),
            |size| {
                if let Some(mut renderer) = self.renderer.try_lock()
                    && let Some(renderer) = renderer.as_mut()
                {
                    renderer.update_drawable_size(size);
                    true
                } else {
                    false
                }
            },
        );

        let cb = self.resize_callback.borrow_mut().take();
        if let Some(mut cb) = cb {
            cb(new_size, scale);
            let mut slot = self.resize_callback.borrow_mut();
            if slot.is_none() {
                *slot = Some(cb);
            }
        }
    }

    /// Host-driven focus report (foreign path), transition-only.
    pub(crate) fn set_active_foreign(&self, is_active: bool) {
        let was_active = self.foreign.is_active();
        self.foreign.update_active(is_active);
        if is_active && !was_active {
            self.focus_hardware_keyboard_view();
        }
    }

    /// Host-driven hover report (foreign path), transition-only.
    pub(crate) fn set_hovered_foreign(&self, is_hovered: bool) {
        self.foreign.update_hovered(is_hovered);
    }

    /// Realtime health counters: dropped and coalesced frames.
    pub(crate) fn foreign_frame_stats(&self) -> (usize, usize) {
        (
            self.foreign.dropped_frames(),
            self.foreign.coalesced_frames(),
        )
    }
}
