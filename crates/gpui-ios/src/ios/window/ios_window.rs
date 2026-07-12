use super::super::IosDisplay;
use super::super::events::*;
use super::accessibility::accessibility_traits_for_node;
use super::accessibility::accessibility_value_for_node;
use super::consts::GPUI_WINDOW_IVAR;
use super::consts::SCROLL_SLOP;
use super::fallback_atlas::FallbackAtlas;
use super::ios_raw_handles::IosRawHandles;
use super::misc::ns_string_to_string;
use super::misc::query_scene_metrics;
use super::misc::view_safe_area_insets;
use super::misc::{
    UIAccessibilityAnnouncementNotification, UIAccessibilityLayoutChangedNotification,
    UIAccessibilityPostNotification,
};
#[cfg(target_os = "ios")]
use super::register::input_diag_log;
use super::register::register_accessibility_element_class;
use super::register::register_metal_view_class;
use super::register::register_text_input_view_class;
use super::register::register_view_controller_class;
#[cfg(target_os = "ios")]
use super::register::register_window_class;
use super::types::{PinchState, TouchState, TouchStateMap, pinch_geometry};
use crate::momentum::{MomentumScroller, VelocityTracker};
use crate::native::{DynamicTypeCategory, IosSceneMetrics, SizeClass};
use crate::platform_view::NativePlatformViewHost;
use gpui::{
    AnyWindowHandle, Bounds, Capslock, DevicePixels, DispatchEventResult, GpuSpecs, Modifiers,
    Pixels, PlatformAtlas, PlatformDisplay, PlatformInput, PlatformInputHandler, PlatformWindow,
    Point, PromptButton, PromptLevel, RequestFrameOptions, Scene, Size, WindowAppearance,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowParams, px, size,
};
use gpui_wgpu::{WgpuContext, WgpuRenderer, WgpuSurfaceConfig, wgpu};
use objc::{
    Message, class, msg_send,
    runtime::{BOOL, NO, Object, Sel, YES},
    sel, sel_impl,
};
use parking_lot::Mutex;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, UiKitDisplayHandle, UiKitWindowHandle};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ffi::c_void,
    ptr::{self, NonNull},
    rc::Rc,
    sync::Arc,
};

fn keyboard_height_from_notification(notification: *mut Object) -> Option<f32> {
    if notification.is_null() {
        return None;
    }

    unsafe {
        let user_info: *mut Object = msg_send![notification, userInfo];
        if user_info.is_null() {
            return None;
        }
        let frame_key = super::super::ns_string_from_str("UIKeyboardFrameEndUserInfoKey");
        let frame_value: *mut Object = msg_send![user_info, objectForKey: frame_key];
        if frame_value.is_null() {
            return None;
        }
        let frame: core_graphics::geometry::CGRect = msg_send![frame_value, CGRectValue];
        Some(frame.size.height as f32)
    }
}

pub(crate) struct IosWindow {
    /// The UIWindow object
    pub(super) window: *mut Object,
    /// The UIViewController
    pub(super) view_controller: *mut Object,
    /// The Metal-backed UIView
    pub(super) view: *mut Object,
    /// The hidden text input view for keyboard input
    pub(super) text_input_view: *mut Object,
    /// Current bounds in pixels
    pub(super) bounds: Cell<Bounds<Pixels>>,
    /// Scale factor
    pub(super) scale_factor: Cell<f32>,
    /// Input handler for text input
    pub(super) input_handler: RefCell<Option<PlatformInputHandler>>,
    /// Callback for frame requests
    /// Note: pub(super) to allow ffi.rs to access this for the display link callback
    pub(in super::super) request_frame_callback:
        RefCell<Option<Box<dyn FnMut(RequestFrameOptions)>>>,
    /// Callback for input events
    pub(super) input_callback:
        RefCell<Option<Box<dyn FnMut(PlatformInput) -> DispatchEventResult>>>,
    /// Callback for active status changes
    pub(super) active_status_callback: RefCell<Option<Box<dyn FnMut(bool)>>>,
    /// Callback for hover status changes (not really applicable on iOS)
    pub(super) hover_status_callback: RefCell<Option<Box<dyn FnMut(bool)>>>,
    /// Callback for resize events
    pub(super) resize_callback: RefCell<Option<Box<dyn FnMut(Size<Pixels>, f32)>>>,
    /// Callback for move events (not applicable on iOS)
    pub(super) moved_callback: RefCell<Option<Box<dyn FnMut()>>>,
    /// Callback for should close
    pub(super) should_close_callback: RefCell<Option<Box<dyn FnMut() -> bool>>>,
    /// Callback for hit test
    pub(super) hit_test_callback: RefCell<Option<Box<dyn FnMut() -> Option<WindowControlArea>>>>,
    /// Callback for close
    pub(super) close_callback: RefCell<Option<Box<dyn FnOnce()>>>,
    /// Callback for appearance changes
    pub(super) appearance_changed_callback: RefCell<Option<Box<dyn FnMut()>>>,
    /// Current mouse position (from touch)
    pub(super) mouse_position: Cell<Point<Pixels>>,
    /// Current modifiers
    pub(super) modifiers: Cell<Modifiers>,
    /// Track if a touch is currently pressed
    pub(super) touch_pressed: Cell<bool>,
    /// Per-touch gesture state machine — distinguishes taps from scroll drags.
    /// Keyed by the UITouch pointer address.
    pub(super) touch_states: RefCell<TouchStateMap>,
    /// Active two-finger pinch recognizer state.
    pub(super) pinch_state: RefCell<PinchState>,
    /// Velocity tracker — records recent touch samples during drag gestures
    /// so we can compute the release velocity when the finger lifts.
    pub(super) velocity_tracker: RefCell<VelocityTracker>,
    /// Momentum scroller — produces decelerating scroll deltas after a fling
    /// gesture, driven by the CADisplayLink frame callback.
    pub(super) momentum_scroller: RefCell<MomentumScroller>,
    /// NotificationCenter observer tokens for keyboard show/hide callbacks.
    pub(super) keyboard_observers: RefCell<Vec<*mut Object>>,
    /// The wgpu renderer (Metal backend on iOS).
    /// Wrapped in a `Mutex<Option<…>>` so that `draw()` (called from the
    /// `request_frame` callback) can acquire a mutable reference without
    /// conflicting with the outer `&self` borrow.
    pub(super) renderer: Mutex<Option<WgpuRenderer>>,
    /// Cached atlas handle so `sprite_atlas()` does not need to lock the
    /// renderer mutex on every call once the renderer exists.
    pub(super) sprite_atlas: Mutex<Option<Arc<dyn PlatformAtlas>>>,
    /// Reusable UIKit accessibility elements keyed by accessibility node id.
    pub(super) accessibility_elements: RefCell<HashMap<String, *mut Object>>,
    /// Previous accessibility snapshot used to diff against the current one.
    /// Only mutated on the main thread.
    pub(super) prev_accessibility_snapshot:
        RefCell<Option<Arc<crate::accessibility::IosAccessibilitySnapshot>>>,
    /// Reusable index buffers for snapshot diffs; avoids per-refresh vectors.
    pub(super) accessibility_diff_scratch: RefCell<crate::accessibility::AccessibilityDiffScratch>,
    /// Native UIKit/SwiftUI views overlaid with GPUI-managed bounds.
    pub(super) platform_view_host: NativePlatformViewHost,
}

unsafe impl Send for IosWindow {}

unsafe impl Sync for IosWindow {}

impl Drop for IosWindow {
    fn drop(&mut self) {
        self.unregister_keyboard_observers();

        // Release any accessibility elements we retained for reuse.
        unsafe {
            for element in self.accessibility_elements.borrow_mut().drain() {
                if !element.1.is_null() {
                    let _: () = msg_send![element.1, release];
                }
            }
        }

        // Unregister from the global window list so lifecycle callbacks
        // don't dereference freed memory.
        super::super::ffi::unregister_window(self as *const Self);

        // Clear Objective-C ivars so post-destruction callbacks find null
        // instead of a dangling pointer.
        unsafe {
            if !self.view.is_null() {
                (*self.view).set_ivar(GPUI_WINDOW_IVAR, std::ptr::null::<c_void>());
            }
            if !self.text_input_view.is_null() {
                (*self.text_input_view).set_ivar(GPUI_WINDOW_IVAR, std::ptr::null::<c_void>());
            }
        }
    }
}

impl IosWindow {
    pub fn new(handle: AnyWindowHandle, _params: WindowParams) -> anyhow::Result<Self> {
        #[cfg(debug_assertions)]
        unsafe {
            let is_main: BOOL = msg_send![class!(NSThread), isMainThread];
            assert!(
                is_main == YES,
                "IosWindow must be created on the main thread"
            );
        }

        // Create the window on the main screen
        let screen = IosDisplay::main();
        let scale_factor = screen.scale();

        unsafe {
            // Create UIWindow
            let screen_obj: *mut Object = msg_send![class!(UIScreen), mainScreen];
            let screen_bounds_cg: core_graphics::geometry::CGRect = msg_send![screen_obj, bounds];
            #[cfg(target_os = "ios")]
            let window_class = register_window_class();
            #[cfg(not(target_os = "ios"))]
            let window_class = class!(UIWindow);
            let window: *mut Object = msg_send![window_class, alloc];
            let window: *mut Object = msg_send![window, initWithFrame: screen_bounds_cg];
            #[cfg(target_os = "ios")]
            input_diag_log("window using legacy initWithFrame");
            #[cfg(target_os = "ios")]
            input_diag_log(&format!(
                "window created temp_dir={}",
                std::env::temp_dir().display()
            ));

            // Create our custom UIViewController subclass that supports
            // dynamic `preferredStatusBarStyle` overrides.
            let vc_class = register_view_controller_class();
            let view_controller: *mut Object = msg_send![vc_class, alloc];
            let view_controller: *mut Object = msg_send![view_controller, init];

            // Create our custom Metal view using the registered class
            let metal_view_class = register_metal_view_class();
            let view: *mut Object = msg_send![metal_view_class, alloc];
            let view: *mut Object = msg_send![view, initWithFrame: screen_bounds_cg];

            // Configure the Metal layer — wgpu will use it for rendering but
            // we still need to set contentsScale so the drawable size is correct.
            let layer: *mut Object = msg_send![view, layer];
            let scale: core_graphics::base::CGFloat = msg_send![screen_obj, scale];
            let _: () = msg_send![layer, setContentsScale: scale];

            // Auto-resize the Metal view when the parent view changes size
            // (e.g. rotation). UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight
            let _: () = msg_send![view, setAutoresizingMask: 18_usize]; // 0x02 | 0x10

            // Enable user interaction on the Metal view for touch handling
            let _: () = msg_send![view, setUserInteractionEnabled: YES];
            let _: () = msg_send![view, setMultipleTouchEnabled: YES];

            #[cfg(target_os = "ios")]
            {
                // iPad pointer devices and the iOS Simulator deliver trackpad
                // and mouse-wheel scrolls through a pan recognizer configured
                // for indirect scroll types, not through touchesMoved.
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
                input_diag_log("installed indirect scroll pan recognizer");
            }

            // Set the view as the view controller's view
            let _: () = msg_send![view_controller, setView: view];

            // Set the root view controller
            let _: () = msg_send![window, setRootViewController: view_controller];

            // Make the window visible
            let _: () = msg_send![window, makeKeyAndVisible];

            // Create a hidden text input view for keyboard handling.
            // Uses our custom GPUITextInputView which implements UIKeyInput
            // so iOS actually routes keyboard text to us.
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

            // UIKit may resolve the actual scene size only after the window is
            // visible, especially on iPad in Split View or Stage Manager. Force
            // a layout pass now so the first GPUI render uses the current view
            // bounds instead of the full screen bounds.
            let _: () = msg_send![window, layoutIfNeeded];
            let _: () = msg_send![view, layoutIfNeeded];

            let initial_metrics =
                query_scene_metrics(view, scale_factor).unwrap_or_else(|| IosSceneMetrics {
                    width: screen_bounds_cg.size.width as f32,
                    height: screen_bounds_cg.size.height as f32,
                    scale_factor,
                    horizontal_size_class: SizeClass::Unspecified,
                    vertical_size_class: SizeClass::Unspecified,
                    dynamic_type: DynamicTypeCategory::Medium,
                    safe_area: view_safe_area_insets(view),
                    keyboard_height: crate::keyboard_height(),
                });
            let initial_bounds = Bounds {
                origin: Default::default(),
                size: size(px(initial_metrics.width), px(initial_metrics.height)),
            };
            let initial_scale = initial_metrics.scale_factor as core_graphics::base::CGFloat;
            let _: () = msg_send![layer, setContentsScale: initial_scale];

            // --- Initialise the wgpu renderer (Metal backend) ---------------
            let pixel_w = (initial_metrics.width * initial_metrics.scale_factor)
                .round()
                .max(1.0) as i32;
            let pixel_h = (initial_metrics.height * initial_metrics.scale_factor)
                .round()
                .max(1.0) as i32;

            let _handle = handle; // consumed but not stored
            let ios_window = Self {
                window,
                view_controller,
                view,
                text_input_view,
                bounds: Cell::new(initial_bounds),
                scale_factor: Cell::new(initial_metrics.scale_factor),
                input_handler: RefCell::new(None),
                request_frame_callback: RefCell::new(None),
                input_callback: RefCell::new(None),
                active_status_callback: RefCell::new(None),
                hover_status_callback: RefCell::new(None),
                resize_callback: RefCell::new(None),
                moved_callback: RefCell::new(None),
                should_close_callback: RefCell::new(None),
                hit_test_callback: RefCell::new(None),
                close_callback: RefCell::new(None),
                appearance_changed_callback: RefCell::new(None),
                mouse_position: Cell::new(Point::default()),
                modifiers: Cell::new(Modifiers::default()),
                touch_pressed: Cell::new(false),
                touch_states: RefCell::new(TouchStateMap::new()),
                pinch_state: RefCell::new(PinchState::default()),
                velocity_tracker: RefCell::new(VelocityTracker::new()),
                momentum_scroller: RefCell::new(MomentumScroller::new()),
                keyboard_observers: RefCell::new(Vec::new()),
                renderer: Mutex::new(None),
                sprite_atlas: Mutex::new(None),
                accessibility_elements: RefCell::new(HashMap::new()),
                prev_accessibility_snapshot: RefCell::new(None),
                accessibility_diff_scratch: RefCell::new(Default::default()),
                platform_view_host: NativePlatformViewHost::new(view as *mut c_void),
            };

            // Keep app-level hardware keyboard shortcuts alive when no text
            // input is active.
            ios_window.focus_hardware_keyboard_view();

            // Create the wgpu renderer using the Metal backend.
            //
            // `gpui_wgpu::WgpuContext::instance()` only enables Vulkan+GL,
            // so we create our own wgpu instance with Metal enabled, build
            // a surface from the UIView's raw window handle, construct the
            // WgpuContext with that instance, and pre-populate the
            // shared GpuContext so WgpuRenderer::new() reuses it.
            let config = WgpuSurfaceConfig {
                size: size(DevicePixels(pixel_w), DevicePixels(pixel_h)),
                transparent: false,
                preferred_present_mode: None,
            };

            // Build raw-window-handle wrapper for the renderer. We can't
            // pass `&IosWindow` directly because WgpuRenderer::new requires
            // `Debug + Clone + Send + Sync + 'static`.
            let window_handle = ios_window
                .window_handle()
                .expect("iOS window handle unavailable");
            let display_handle = ios_window
                .display_handle()
                .expect("iOS display handle unavailable");
            let raw_handles = IosRawHandles {
                window: window_handle.as_raw(),
                display: display_handle.as_raw(),
            };

            let metal_instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::METAL,
                flags: wgpu::InstanceFlags::default(),
                backend_options: wgpu::BackendOptions::default(),
                memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
                display: Some(Box::new(raw_handles.clone())),
            });

            let target = wgpu::SurfaceTargetUnsafe::RawHandle {
                // The display is attached to the instance. WgpuRenderer::new
                // uses the same convention when it recreates this surface.
                raw_display_handle: None,
                raw_window_handle: raw_handles.window,
            };

            // Build a Metal-backed WgpuContext, pre-populate the shared
            // GpuContext (Rc<RefCell<Option<WgpuContext>>>), then call
            // WgpuRenderer::new which will reuse our context instead of
            // falling back to the Vulkan+GL default.
            let surface_result = metal_instance.create_surface_unsafe(target);
            match surface_result {
                Ok(surface) => match WgpuContext::new(metal_instance, &surface, None) {
                    Ok(context) => {
                        let gpu_context: Rc<RefCell<Option<WgpuContext>>> =
                            Rc::new(RefCell::new(Some(context)));
                        drop(surface); // no longer needed — new() creates its own

                        match WgpuRenderer::new(gpu_context, &raw_handles, config, None) {
                            Ok(renderer) => {
                                log::info!("iOS wgpu renderer created (Metal)");
                                *ios_window.renderer.lock() = Some(renderer);
                            }
                            Err(e) => {
                                log::error!("Failed to create iOS wgpu renderer: {e:#}");
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to create iOS WgpuContext: {e:#}");
                    }
                },
                Err(e) => {
                    log::error!("Failed to create iOS wgpu Metal surface: {e:#}");
                }
            }

            Ok(ios_window)
        }
    }

    /// Get the raw pointer to the UIViewController.
    #[allow(
        dead_code,
        reason = "kept for Swift/Objective-C integration points that are not used by the Rust demo"
    )]
    pub fn view_controller_ptr(&self) -> *mut Object {
        self.view_controller
    }

    /// Get the raw pointer to the GPUIMetalView.
    #[allow(
        dead_code,
        reason = "kept for Swift/Objective-C integration points that are not used by the Rust demo"
    )]
    pub fn metal_view_ptr(&self) -> *mut Object {
        self.view
    }

    /// Register this window with the FFI layer after it's been stored.
    /// This must be called after the window is placed at a stable address
    /// (e.g., in a Box or Arc).
    pub(crate) fn register_with_ffi(&self) {
        super::super::ffi::register_window(self as *const Self);

        // Set the window pointer on the view so touch events can find us,
        // and on the text input view so keyboard input can find us.
        unsafe {
            let window_ptr = self as *const Self as *mut std::ffi::c_void;
            (*self.view).set_ivar(GPUI_WINDOW_IVAR, window_ptr);
            (*self.text_input_view).set_ivar(GPUI_WINDOW_IVAR, window_ptr);
            log::info!(
                "GPUI iOS: Set window pointer {:p} on view {:p} and text input {:p}",
                window_ptr,
                self.view,
                self.text_input_view
            );
        }

        // Listen for keyboard show/hide so we can expose the keyboard height.
        self.register_keyboard_observers();
    }

    /// Register for keyboard show/hide notifications so we can track the
    /// keyboard height and allow the UI to shift content above the keyboard.
    pub(crate) fn register_keyboard_observers(&self) {
        if !self.keyboard_observers.borrow().is_empty() {
            return;
        }

        unsafe {
            let notification_center: *mut Object =
                msg_send![class!(NSNotificationCenter), defaultCenter];

            let show_name = super::super::ns_string_from_str("UIKeyboardWillShowNotification");
            let hide_name = super::super::ns_string_from_str("UIKeyboardWillHideNotification");
            let frame_name =
                super::super::ns_string_from_str("UIKeyboardWillChangeFrameNotification");
            let input_mode_name = super::super::ns_string_from_str(
                "UITextInputCurrentInputModeDidChangeNotification",
            );

            // Block that fires when the keyboard appears. It extracts the
            // end-frame height and stores it in the global atomic.
            // The closure takes `*mut c_void` because block2 only encodes
            // C-ABI types; we cast back to `*mut Object` inside.
            let show_block = block2::RcBlock::new(move |notification: *mut c_void| {
                if let Some(height) = keyboard_height_from_notification(notification as *mut Object)
                {
                    log::info!("GPUI iOS: Keyboard will show, height={}", height);
                    crate::set_keyboard_height(height);
                }
            });

            let hide_block = block2::RcBlock::new(move |_notification: *mut c_void| {
                log::info!("GPUI iOS: Keyboard will hide");
                crate::set_keyboard_height(0.0);
            });

            let frame_block = block2::RcBlock::new(move |notification: *mut c_void| {
                if let Some(height) = keyboard_height_from_notification(notification as *mut Object)
                {
                    log::info!("GPUI iOS: Keyboard frame changed, height={}", height);
                    crate::set_keyboard_height(height);
                }
            });

            let input_mode_block = block2::RcBlock::new(move |_notification: *mut c_void| {
                log::info!("GPUI iOS: Keyboard input mode changed");
                crate::dispatch_keyboard_layout_change();
            });

            let show_observer: *mut Object = msg_send![notification_center,
                addObserverForName: show_name
                object: std::ptr::null::<Object>()
                queue: std::ptr::null::<Object>()
                usingBlock: &*show_block
            ];
            let hide_observer: *mut Object = msg_send![notification_center,
                addObserverForName: hide_name
                object: std::ptr::null::<Object>()
                queue: std::ptr::null::<Object>()
                usingBlock: &*hide_block
            ];
            let frame_observer: *mut Object = msg_send![notification_center,
                addObserverForName: frame_name
                object: std::ptr::null::<Object>()
                queue: std::ptr::null::<Object>()
                usingBlock: &*frame_block
            ];
            let input_mode_observer: *mut Object = msg_send![notification_center,
                addObserverForName: input_mode_name
                object: std::ptr::null::<Object>()
                queue: std::ptr::null::<Object>()
                usingBlock: &*input_mode_block
            ];

            let mut observers = self.keyboard_observers.borrow_mut();
            if !show_observer.is_null() {
                observers.push(show_observer);
            }
            if !hide_observer.is_null() {
                observers.push(hide_observer);
            }
            if !frame_observer.is_null() {
                observers.push(frame_observer);
            }
            if !input_mode_observer.is_null() {
                observers.push(input_mode_observer);
            }
        }
    }

    pub(super) fn unregister_keyboard_observers(&self) {
        let mut observers = self.keyboard_observers.borrow_mut();
        if observers.is_empty() {
            return;
        }

        unsafe {
            let notification_center: *mut Object =
                msg_send![class!(NSNotificationCenter), defaultCenter];
            for observer in observers.drain(..) {
                if !observer.is_null() {
                    let _: () = msg_send![notification_center, removeObserver: observer];
                }
            }
        }
    }

    fn request_forced_frame(&self) {
        if let Some(callback) = self.request_frame_callback.borrow_mut().as_mut() {
            callback(RequestFrameOptions {
                force_render: true,
                ..Default::default()
            });
        }
    }

    fn emit_pinch_for_active_touches(
        &self,
        states: &mut TouchStateMap,
        emit: &mut impl FnMut(PlatformInput) -> DispatchEventResult,
        modifiers: Modifiers,
    ) -> bool {
        let Some((first, second)) = states.two_active_points() else {
            return false;
        };
        let Some((x, y, distance)) = pinch_geometry(first, second) else {
            return false;
        };

        self.momentum_scroller.borrow_mut().cancel();
        self.velocity_tracker.borrow_mut().reset();
        states.clear_states();

        let mut pinch = self.pinch_state.borrow_mut();
        let (delta, phase) = if pinch.is_active() {
            (
                pinch.update(distance).unwrap_or(0.0),
                gpui::TouchPhase::Moved,
            )
        } else {
            pinch.start(distance);
            (0.0, gpui::TouchPhase::Started)
        };

        emit(PlatformInput::Pinch(gpui::PinchEvent {
            position: gpui::point(gpui::px(x), gpui::px(y)),
            delta,
            modifiers,
            phase,
        }));
        self.request_forced_frame();
        true
    }

    fn end_active_pinch(
        &self,
        position: Point<Pixels>,
        modifiers: Modifiers,
        emit: &mut impl FnMut(PlatformInput) -> DispatchEventResult,
    ) -> bool {
        let mut pinch = self.pinch_state.borrow_mut();
        if !pinch.is_active() {
            return false;
        }
        pinch.reset();
        self.velocity_tracker.borrow_mut().reset();
        self.momentum_scroller.borrow_mut().cancel();
        emit(PlatformInput::Pinch(gpui::PinchEvent {
            position,
            delta: 0.0,
            modifiers,
            phase: gpui::TouchPhase::Ended,
        }));
        self.request_forced_frame();
        true
    }

    /// Handle a touch event from UIKit.
    ///
    /// Uses a state machine to distinguish **taps** from **drag gestures**:
    ///
    ///   DOWN  → record start position, enter "pending" (NO MouseDown yet)
    ///   MOVE  → if finger moved > threshold → switch to "scrolling",
    ///           emit `ScrollWheel` deltas (for scrollable containers) AND
    ///           `MouseMove` (for interactive canvas screens like Animations)
    ///   UP    → if still "pending" → emit `MouseDown` + `MouseUp` (tap)
    ///           if "scrolling"   → emit final `ScrollWheel` (Ended) +
    ///           `MouseUp` (so drag-to-throw works)
    ///
    /// MouseDown is **deferred** until finger-up so that starting a scroll
    /// near a button or tab doesn't accidentally trigger navigation.
    /// Interactive screens use `MouseMove` to track the finger during drags
    /// and `MouseUp` to detect the end of a throw/drag gesture.
    pub fn handle_touch(&self, touch: *mut Object, _event: *mut Object) {
        let position = touch_location_in_view(touch, self.view);
        let phase = touch_phase(touch);
        let tap_count = touch_tap_count(touch);
        let modifiers = self.modifiers.get();

        let logical_x: f32 = position.x.into();
        let logical_y: f32 = position.y.into();

        self.mouse_position.set(position);
        self.dispatch_pointer_sample(touch, logical_x, logical_y);

        let touch_id: usize = unsafe { msg_send![touch, hash] };
        let mut callback = self.input_callback.borrow_mut();
        let mut states = self.touch_states.borrow_mut();
        let mut ts = states.get(touch_id).unwrap_or(TouchState::Idle);

        let mut emit = |input: PlatformInput| -> DispatchEventResult {
            if let Some(callback) = callback.as_mut() {
                callback(input)
            } else {
                DispatchEventResult {
                    propagate: true,
                    default_prevented: false,
                }
            }
        };

        match phase {
            UITouchPhase::Began => {
                self.touch_pressed.set(true);
                // Cancel any active momentum fling — the user touched the
                // screen again, so inertia scrolling must stop immediately.
                self.momentum_scroller.borrow_mut().cancel();
                self.velocity_tracker.borrow_mut().reset();

                ts = TouchState::Pending {
                    start_x: logical_x,
                    start_y: logical_y,
                };
                states.insert(touch_id, ts, logical_x, logical_y);
                if self.emit_pinch_for_active_touches(&mut states, &mut emit, modifiers) {
                    return;
                }
                emit(PlatformInput::MouseMove(gpui::MouseMoveEvent {
                    position,
                    modifiers,
                    pressed_button: None,
                }));
                self.request_forced_frame();
                // Do NOT emit MouseDown here — wait until we know whether
                // this is a tap or a scroll.  Emitting MouseDown immediately
                // causes accidental navigation when the user starts scrolling
                // near a button/tab.
                //
                // - Tap (finger lifts within slop) → emit MouseDown + MouseUp
                //   together in Ended phase.
                // - Scroll (finger exceeds slop) → emit only MouseMove +
                //   ScrollWheel, no MouseDown.
            }

            UITouchPhase::Moved => {
                states.insert(touch_id, ts, logical_x, logical_y);
                if self.emit_pinch_for_active_touches(&mut states, &mut emit, modifiers) {
                    return;
                }
                // Record every move for velocity estimation.
                self.velocity_tracker
                    .borrow_mut()
                    .record(logical_x, logical_y);

                match ts {
                    TouchState::Pending { start_x, start_y } => {
                        let dx = logical_x - start_x;
                        let dy = logical_y - start_y;
                        let distance = (dx * dx + dy * dy).sqrt();

                        if distance > SCROLL_SLOP {
                            let vertical_scroll = dy.abs() >= dx.abs();
                            emit(PlatformInput::MouseMove(gpui::MouseMoveEvent {
                                position,
                                modifiers,
                                pressed_button: Some(gpui::MouseButton::Left),
                            }));
                            if vertical_scroll {
                                // GPUI stores scroll offsets as negative values
                                // once content moves upward, and its scroll
                                // handler adds deltas directly. A finger moving
                                // up therefore needs a negative y delta. Do not
                                // probe with MouseDown first; menu rows and
                                // buttons would treat the beginning of a scroll
                                // as an activation.
                                #[cfg(target_os = "ios")]
                                input_diag_log(&format!(
                                    "direct_touch scroll started dx={dx:.2} dy={dy:.2} pos=({logical_x:.2},{logical_y:.2})"
                                ));
                                emit(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                                    position,
                                    delta: gpui::ScrollDelta::Pixels(gpui::point(
                                        gpui::px(dx),
                                        gpui::px(dy),
                                    )),
                                    modifiers,
                                    touch_phase: gpui::TouchPhase::Started,
                                }));
                                self.request_forced_frame();
                                ts = TouchState::Scrolling {
                                    prev_x: logical_x,
                                    prev_y: logical_y,
                                };
                            } else {
                                // Horizontal gestures are more likely to be
                                // sliders, canvas tools, etc. Probe with
                                // MouseDown so those elements can claim the
                                // touch as a drag.
                                let start_pos = gpui::point(gpui::px(start_x), gpui::px(start_y));
                                let result = emit(PlatformInput::MouseDown(gpui::MouseDownEvent {
                                    button: gpui::MouseButton::Left,
                                    position: start_pos,
                                    modifiers,
                                    click_count: 1,
                                    first_mouse: false,
                                }));

                                if !result.propagate {
                                    ts = TouchState::Dragging;
                                } else {
                                    emit(PlatformInput::MouseUp(gpui::MouseUpEvent {
                                        button: gpui::MouseButton::Left,
                                        position: start_pos,
                                        modifiers,
                                        click_count: 1,
                                    }));
                                    ts = TouchState::Scrolling {
                                        prev_x: logical_x,
                                        prev_y: logical_y,
                                    };
                                }
                            }
                        }
                        if matches!(ts, TouchState::Pending { .. }) {
                            // Keep GPUI's mouse hit-test under the finger while
                            // the gesture is still inside the scroll slop.
                            emit(PlatformInput::MouseMove(gpui::MouseMoveEvent {
                                position,
                                modifiers,
                                pressed_button: Some(gpui::MouseButton::Left),
                            }));
                        }
                    }
                    TouchState::Dragging => {
                        // Element is driving its own drag — only emit
                        // MouseMove (no ScrollWheel).
                        emit(PlatformInput::MouseMove(gpui::MouseMoveEvent {
                            position,
                            modifiers,
                            pressed_button: Some(gpui::MouseButton::Left),
                        }));
                    }
                    TouchState::Scrolling { prev_x, prev_y } => {
                        let dx = logical_x - prev_x;
                        let dy = logical_y - prev_y;
                        ts = TouchState::Scrolling {
                            prev_x: logical_x,
                            prev_y: logical_y,
                        };
                        // Update GPUI's scroll target before dispatching the
                        // wheel event; scroll hit-testing follows the current
                        // mouse position.
                        emit(PlatformInput::MouseMove(gpui::MouseMoveEvent {
                            position,
                            modifiers,
                            pressed_button: Some(gpui::MouseButton::Left),
                        }));
                        // Scroll event for scrollable containers.
                        #[cfg(target_os = "ios")]
                        input_diag_log(&format!(
                            "direct_touch scroll moved dx={dx:.2} dy={dy:.2} pos=({logical_x:.2},{logical_y:.2})"
                        ));
                        emit(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                            position,
                            delta: gpui::ScrollDelta::Pixels(gpui::point(
                                gpui::px(dx),
                                gpui::px(dy),
                            )),
                            modifiers,
                            touch_phase: gpui::TouchPhase::Moved,
                        }));
                        self.request_forced_frame();
                    }
                    TouchState::Idle => {
                        // Spurious move without a preceding down — ignore.
                    }
                }
            }

            UITouchPhase::Ended | UITouchPhase::Cancelled => {
                self.touch_pressed.set(false);
                if self.end_active_pinch(position, modifiers, &mut emit) {
                    states.remove(touch_id);
                    return;
                }
                match ts {
                    TouchState::Pending { start_x, start_y } => {
                        // Finger lifted without exceeding slop → tap.
                        // Emit MouseDown + MouseUp together at the original
                        // down position so hit-testing matches the initial
                        // touch point.
                        self.velocity_tracker.borrow_mut().reset();
                        let tap_pos = gpui::point(gpui::px(start_x), gpui::px(start_y));
                        emit(PlatformInput::MouseDown(gpui::MouseDownEvent {
                            button: gpui::MouseButton::Left,
                            position: tap_pos,
                            modifiers,
                            click_count: tap_count as usize,
                            first_mouse: false,
                        }));
                        emit(PlatformInput::MouseUp(gpui::MouseUpEvent {
                            button: gpui::MouseButton::Left,
                            position: tap_pos,
                            modifiers,
                            click_count: tap_count as usize,
                        }));
                    }
                    TouchState::Dragging => {
                        // Element was driving a drag — just emit MouseUp
                        // to let it finalize (no scroll, no momentum).
                        self.velocity_tracker.borrow_mut().reset();
                        emit(PlatformInput::MouseUp(gpui::MouseUpEvent {
                            button: gpui::MouseButton::Left,
                            position,
                            modifiers,
                            click_count: 1,
                        }));
                    }
                    TouchState::Scrolling { prev_x, prev_y } => {
                        // End the active touch-scroll gesture.
                        let dx = logical_x - prev_x;
                        let dy = logical_y - prev_y;
                        #[cfg(target_os = "ios")]
                        input_diag_log(&format!(
                            "direct_touch scroll ended dx={dx:.2} dy={dy:.2} pos=({logical_x:.2},{logical_y:.2})"
                        ));
                        emit(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                            position,
                            delta: gpui::ScrollDelta::Pixels(gpui::point(
                                gpui::px(dx),
                                gpui::px(dy),
                            )),
                            modifiers,
                            touch_phase: gpui::TouchPhase::Ended,
                        }));
                        self.request_forced_frame();
                        // Also emit MouseUp so interactive screens can
                        // detect the end of a drag (e.g. fling a ball).
                        emit(PlatformInput::MouseUp(gpui::MouseUpEvent {
                            button: gpui::MouseButton::Left,
                            position,
                            modifiers,
                            click_count: 1,
                        }));

                        // ── Start momentum / inertia scrolling ───────────
                        // Compute release velocity from recent touch samples
                        // and kick off the momentum scroller.  Subsequent
                        // frames will pump synthetic ScrollWheel events via
                        // `pump_momentum()` until velocity decays below the
                        // threshold.
                        let (vx, vy) = self.velocity_tracker.borrow().velocity();
                        self.velocity_tracker.borrow_mut().reset();
                        self.momentum_scroller
                            .borrow_mut()
                            .fling(vx, vy, logical_x, logical_y);
                    }
                    TouchState::Idle => {}
                }
                states.remove(touch_id);
                return;
            }

            UITouchPhase::Stationary => {
                // No change — ignore.
                return;
            }
        }

        states.insert(touch_id, ts, logical_x, logical_y);
    }

    #[cfg(target_os = "ios")]
    pub fn handle_indirect_scroll(&self, recognizer: *mut Object) {
        if recognizer.is_null() {
            return;
        }

        const GESTURE_BEGAN: i64 = 1;
        const GESTURE_CHANGED: i64 = 2;
        const GESTURE_ENDED: i64 = 3;
        const GESTURE_CANCELLED: i64 = 4;

        unsafe {
            let state: i64 = msg_send![recognizer, state];
            let touch_phase = match state {
                GESTURE_BEGAN => gpui::TouchPhase::Started,
                GESTURE_CHANGED => gpui::TouchPhase::Moved,
                GESTURE_ENDED | GESTURE_CANCELLED => gpui::TouchPhase::Ended,
                _ => return,
            };

            let translation: core_graphics::geometry::CGPoint =
                msg_send![recognizer, translationInView: self.view];
            let location: core_graphics::geometry::CGPoint =
                msg_send![recognizer, locationInView: self.view];
            let position = gpui::point(gpui::px(location.x as f32), gpui::px(location.y as f32));
            let delta = gpui::point(
                gpui::px(translation.x as f32),
                gpui::px(translation.y as f32),
            );
            input_diag_log(&format!(
                "indirect_scroll translation=({:.2},{:.2}) location=({:.2},{:.2}) state={state}",
                translation.x, translation.y, location.x, location.y
            ));

            if translation.x != 0.0 || translation.y != 0.0 || state != GESTURE_CHANGED {
                if let Some(callback) = self.input_callback.borrow_mut().as_mut() {
                    callback(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                        position,
                        delta: gpui::ScrollDelta::Pixels(delta),
                        modifiers: self.modifiers.get(),
                        touch_phase,
                    }));
                }
                self.request_forced_frame();
            }

            let zero = core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 };
            let _: () = msg_send![recognizer, setTranslation: zero inView: self.view];

            if matches!(state, GESTURE_ENDED | GESTURE_CANCELLED) {
                let velocity: core_graphics::geometry::CGPoint =
                    msg_send![recognizer, velocityInView: self.view];
                self.momentum_scroller.borrow_mut().fling(
                    velocity.x as f32,
                    velocity.y as f32,
                    location.x as f32,
                    location.y as f32,
                );
            }
        }
    }

    pub(super) fn dispatch_pointer_sample(
        &self,
        touch: *mut Object,
        logical_x: f32,
        logical_y: f32,
    ) {
        if touch.is_null() {
            return;
        }
        unsafe {
            // SAFETY: UIKit supplies a live UITouch pointer while processing the
            // touch callback on the main thread. Selectors used here are stable
            // UITouch APIs on the supported iOS deployment target.
            let touch_type: i64 = (&*touch).send_message(Sel::register("type"), ()).unwrap();
            let force: core_graphics::base::CGFloat = msg_send![touch, force];
            let max_force: core_graphics::base::CGFloat = msg_send![touch, maximumPossibleForce];
            let altitude_angle: core_graphics::base::CGFloat = msg_send![touch, altitudeAngle];
            let azimuth_angle: core_graphics::base::CGFloat =
                msg_send![touch, azimuthAngleInView: self.view];
            let timestamp: f64 = msg_send![touch, timestamp];
            let device = match touch_type {
                0 => crate::pencil::IosPointerDevice::Touch,
                1 => crate::pencil::IosPointerDevice::IndirectPointer,
                2 => crate::pencil::IosPointerDevice::Pencil,
                3 => crate::pencil::IosPointerDevice::IndirectPointer,
                _ => crate::pencil::IosPointerDevice::Unknown,
            };
            let pressure = if max_force > 0.0 {
                (force / max_force) as f32
            } else {
                0.0
            };
            crate::pencil::dispatch_pencil_sample(crate::pencil::IosPencilSample {
                x: logical_x,
                y: logical_y,
                pressure,
                altitude_angle: altitude_angle as f32,
                azimuth_angle: azimuth_angle as f32,
                timestamp_seconds: timestamp,
                device,
            });
        }
    }

    /// Query the safe area insets from the UIView.
    ///
    /// Returns `(top, left, bottom, right)` in logical points (matching
    /// `UIEdgeInsets` field order — see implementation below). These
    /// represent the areas occupied by system UI (status bar, home
    /// indicator, camera notch) that content should avoid.
    pub fn safe_area_insets(&self) -> (f32, f32, f32, f32) {
        let insets = unsafe { view_safe_area_insets(self.view) };
        (insets.top, insets.left, insets.bottom, insets.right)
    }

    /// Query current scene metrics from the actual UIKit view.
    pub fn scene_metrics(&self) -> Option<IosSceneMetrics> {
        unsafe { query_scene_metrics(self.view, self.scale_factor.get()) }
    }

    pub fn attach_to_parent_view(&self, parent: *mut c_void) {
        if parent.is_null() || self.view.is_null() {
            return;
        }
        self.platform_view_host.set_parent_view(parent);
        unsafe {
            // SAFETY: `parent` is supplied by Swift as a live UIView pointer for
            // the duration of this call, and `self.view` is the retained GPUI
            // Metal UIView owned by this window on the main thread.
            let parent = parent as *mut Object;
            let _: () = msg_send![parent, addSubview: self.view];
            let bounds: core_graphics::geometry::CGRect = msg_send![parent, bounds];
            let _: () = msg_send![self.view, setFrame: bounds];
            let _: () = msg_send![self.view, setAutoresizingMask: 18_usize];
        }
        crate::instrumentation::emit_signpost(
            crate::instrumentation::IosSignpostCategory::PlatformView,
            "attach_gpui_host_view",
        );
        self.handle_layout_change();
        self.focus_hardware_keyboard_view();
    }

    pub fn detach_from_parent_view(&self) {
        if self.view.is_null() {
            return;
        }
        unsafe {
            // SAFETY: `self.view` is a live UIView owned by this window. UIKit
            // permits `removeFromSuperview` even when there is no superview.
            let _: () = msg_send![self.view, removeFromSuperview];
        }
        crate::instrumentation::emit_signpost(
            crate::instrumentation::IosSignpostCategory::PlatformView,
            "detach_gpui_host_view",
        );
    }

    pub fn refresh_accessibility(&self) {
        use crate::accessibility::compute_accessibility_diff_into;

        let snapshot = crate::accessibility::accessibility_snapshot();
        let element_count = snapshot
            .as_ref()
            .map(|snapshot| snapshot.flattened_node_slice().len())
            .unwrap_or_default();
        crate::instrumentation::emit_signpost(
            crate::instrumentation::IosSignpostCategory::Accessibility,
            format!("accessibility_nodes={element_count}"),
        );

        let prev_snapshot = self.prev_accessibility_snapshot.borrow().clone();
        let mut diff = self.accessibility_diff_scratch.borrow_mut();
        let has_diff = if let Some(next) = snapshot.as_ref() {
            compute_accessibility_diff_into(prev_snapshot.as_deref(), next, &mut diff);
            true
        } else {
            diff.clear();
            false
        };

        // Store the current snapshot for the next diff, even on non-Apple hosts
        // where the UIKit mutations below are a no-op.
        *self.prev_accessibility_snapshot.borrow_mut() = snapshot.clone();

        #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
        let _ = has_diff;

        #[cfg(any(target_os = "ios", target_os = "tvos"))]
        unsafe {
            // SAFETY: All UIKit accessibility objects are created and assigned
            // on the main thread while `self.view` is a live UIView owned by
            // this IosWindow. UIKit retains the array assigned through the
            // `accessibilityElements` property.
            if self.view.is_null() {
                return;
            }

            let _: () = msg_send![self.view, setIsAccessibilityElement: false];

            let Some(snapshot) = snapshot.as_ref() else {
                self.clear_accessibility_elements();
                let _: () = msg_send![
                    self.view,
                    setAccessibilityElements: std::ptr::null_mut::<Object>()
                ];
                return;
            };

            if !has_diff {
                return;
            }
            let nodes = snapshot.flattened_node_slice();

            let element_class = register_accessibility_element_class();
            let mut elements_map = self.accessibility_elements.borrow_mut();

            // Create or update elements for the current snapshot.
            for &idx in diff.added_indices() {
                let node = &nodes[idx];
                let element: *mut Object = msg_send![element_class, alloc];
                let element: *mut Object = msg_send![
                    element,
                    initWithAccessibilityContainer: self.view
                ];
                if element.is_null() {
                    continue;
                }
                elements_map.insert(node.id.clone(), element);

                let id = super::super::ns_string_from_str(&node.id);
                let _: () = msg_send![element, setAccessibilityIdentifier: id];

                if let Some(label) = node.label.as_deref() {
                    let label = super::super::ns_string_from_str(label);
                    let _: () = msg_send![element, setAccessibilityLabel: label];
                }
                if let Some(hint) = node.hint.as_deref() {
                    let hint = super::super::ns_string_from_str(hint);
                    let _: () = msg_send![element, setAccessibilityHint: hint];
                }
                if let Some(value) = accessibility_value_for_node(node) {
                    let value = super::super::ns_string_from_str(&value);
                    let _: () = msg_send![element, setAccessibilityValue: value];
                }

                let frame = core_graphics::geometry::CGRect {
                    origin: core_graphics::geometry::CGPoint {
                        x: node.frame.x as core_graphics::base::CGFloat,
                        y: node.frame.y as core_graphics::base::CGFloat,
                    },
                    size: core_graphics::geometry::CGSize {
                        width: node.frame.width as core_graphics::base::CGFloat,
                        height: node.frame.height as core_graphics::base::CGFloat,
                    },
                };
                let _: () = msg_send![element, setAccessibilityFrameInContainerSpace: frame];

                let traits = accessibility_traits_for_node(node);
                let _: () = msg_send![element, setAccessibilityTraits: traits];
            }

            for &(idx, changes) in diff.changed_indices() {
                let node = &nodes[idx];
                let Some(&element) = elements_map.get(&node.id) else {
                    continue;
                };

                if changes.label_changed {
                    if let Some(label) = node.label.as_deref() {
                        let label = super::super::ns_string_from_str(label);
                        let _: () = msg_send![element, setAccessibilityLabel: label];
                    }
                }
                if changes.hint_changed {
                    if let Some(hint) = node.hint.as_deref() {
                        let hint = super::super::ns_string_from_str(hint);
                        let _: () = msg_send![element, setAccessibilityHint: hint];
                    }
                }
                if changes.value_changed {
                    if let Some(value) = accessibility_value_for_node(node) {
                        let value = super::super::ns_string_from_str(&value);
                        let _: () = msg_send![element, setAccessibilityValue: value];
                    }
                }
                if changes.frame_changed {
                    let frame = core_graphics::geometry::CGRect {
                        origin: core_graphics::geometry::CGPoint {
                            x: node.frame.x as core_graphics::base::CGFloat,
                            y: node.frame.y as core_graphics::base::CGFloat,
                        },
                        size: core_graphics::geometry::CGSize {
                            width: node.frame.width as core_graphics::base::CGFloat,
                            height: node.frame.height as core_graphics::base::CGFloat,
                        },
                    };
                    let _: () = msg_send![element, setAccessibilityFrameInContainerSpace: frame];
                }
                if changes.traits_changed {
                    let traits = accessibility_traits_for_node(node);
                    let _: () = msg_send![element, setAccessibilityTraits: traits];
                }
            }

            // Removed indices resolve against the previous cached snapshot,
            // avoiding temporary HashSet/String collections.
            if let Some(prev) = prev_snapshot.as_ref() {
                let prev_nodes = prev.flattened_node_slice();
                for &idx in diff.removed_indices() {
                    if let Some(element) = elements_map.remove(&prev_nodes[idx].id) {
                        let _: () = msg_send![element, release];
                    }
                }
            }

            // Only rebuild the `accessibilityElements` array and post the layout
            // notification when the node set or ordering changed.
            if diff.order_changed {
                let ordered_elements: Vec<*mut Object> = snapshot
                    .flattened_node_slice()
                    .iter()
                    .filter_map(|node| elements_map.get(&node.id).copied())
                    .collect();
                drop(elements_map);

                let elements: *mut Object =
                    msg_send![class!(NSMutableArray), arrayWithCapacity: ordered_elements.len()];
                for element in ordered_elements.iter().copied() {
                    let _: () = msg_send![elements, addObject: element];
                }
                let _: () = msg_send![self.view, setAccessibilityElements: elements];

                if element_count > 0 {
                    let first_element: *mut Object = msg_send![elements, firstObject];
                    UIAccessibilityPostNotification(
                        UIAccessibilityLayoutChangedNotification,
                        first_element,
                    );
                }
            } else {
                drop(elements_map);
            }

            for announcement in &snapshot.announcements {
                let announcement = super::super::ns_string_from_str(announcement);
                UIAccessibilityPostNotification(
                    UIAccessibilityAnnouncementNotification,
                    announcement,
                );
            }
        }
    }

    fn clear_accessibility_elements(&self) {
        unsafe {
            for element in self.accessibility_elements.borrow_mut().drain() {
                if !element.1.is_null() {
                    let _: () = msg_send![element.1, release];
                }
            }
        }
    }

    // ── tvOS: Siri Remote button handling ─────────────────────────────────
    //
    // Maps hardware button presses to GPUI events:
    //   Select (4)    → MouseDown/MouseUp at last known position (click)
    //   Menu (5)      → Escape keystroke
    //   Play/Pause (6)→ Space keystroke
    //   Arrows (0-3)  → ScrollWheel impulse for list navigation
    #[cfg(target_os = "tvos")]
    pub fn handle_press(&self, press_type: i64, is_down: bool) {
        let modifiers = self.modifiers.get();
        let position = self.mouse_position.get();

        let emit = |input: PlatformInput| {
            if let Some(callback) = self.input_callback.borrow_mut().as_mut() {
                callback(input);
            }
        };

        // UIPressType constants
        const UP_ARROW: i64 = 0;
        const DOWN_ARROW: i64 = 1;
        const LEFT_ARROW: i64 = 2;
        const RIGHT_ARROW: i64 = 3;
        const SELECT: i64 = 4;
        const MENU: i64 = 5;
        const PLAY_PAUSE: i64 = 6;

        match press_type {
            SELECT => {
                if is_down {
                    emit(PlatformInput::MouseDown(gpui::MouseDownEvent {
                        button: gpui::MouseButton::Left,
                        position,
                        modifiers,
                        click_count: 1,
                        first_mouse: false,
                    }));
                } else {
                    emit(PlatformInput::MouseUp(gpui::MouseUpEvent {
                        button: gpui::MouseButton::Left,
                        position,
                        modifiers,
                        click_count: 1,
                    }));
                }
            }
            MENU => {
                if is_down {
                    emit(PlatformInput::KeyDown(gpui::KeyDownEvent {
                        keystroke: gpui::Keystroke::parse("escape").unwrap(),
                        is_held: false,
                        prefer_character_input: false,
                    }));
                } else {
                    emit(PlatformInput::KeyUp(gpui::KeyUpEvent {
                        keystroke: gpui::Keystroke::parse("escape").unwrap(),
                    }));
                }
            }
            PLAY_PAUSE => {
                if is_down {
                    emit(PlatformInput::KeyDown(gpui::KeyDownEvent {
                        keystroke: gpui::Keystroke::parse("space").unwrap(),
                        is_held: false,
                        prefer_character_input: false,
                    }));
                } else {
                    emit(PlatformInput::KeyUp(gpui::KeyUpEvent {
                        keystroke: gpui::Keystroke::parse("space").unwrap(),
                    }));
                }
            }
            UP_ARROW | DOWN_ARROW | LEFT_ARROW | RIGHT_ARROW => {
                // Emit a scroll impulse on press-down (repeat-friendly).
                // 60 px per press gives comfortable list scrolling.
                if is_down {
                    let (dx, dy) = match press_type {
                        UP_ARROW => (0.0, 60.0),
                        DOWN_ARROW => (0.0, -60.0),
                        LEFT_ARROW => (60.0, 0.0),
                        RIGHT_ARROW => (-60.0, 0.0),
                        _ => (0.0, 0.0),
                    };
                    emit(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                        position,
                        delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(dx), gpui::px(dy))),
                        modifiers,
                        touch_phase: gpui::TouchPhase::Moved,
                    }));
                    self.request_forced_frame();
                }
            }
            _ => {}
        }
    }

    /// Advance the momentum scroller by one frame and emit a synthetic
    /// `ScrollWheel` event if the fling is still active.
    ///
    /// Called from `gpui_ios_request_frame` on every CADisplayLink tick,
    /// **before** the GPUI render callback runs, so that the scroll delta
    /// is picked up during the current frame's layout/paint cycle.
    pub(crate) fn pump_momentum(&self) {
        let mut scroller = self.momentum_scroller.borrow_mut();
        if !scroller.is_active() {
            return;
        }

        if let Some(delta) = scroller.step() {
            let modifiers = self.modifiers.get();
            let position = gpui::point(gpui::px(delta.position_x), gpui::px(delta.position_y));
            let fling_ended = !scroller.is_active();

            if let Some(callback) = self.input_callback.borrow_mut().as_mut() {
                callback(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position,
                    delta: gpui::ScrollDelta::Pixels(gpui::point(
                        gpui::px(delta.dx),
                        gpui::px(delta.dy),
                    )),
                    modifiers,
                    touch_phase: gpui::TouchPhase::Moved,
                }));
                self.request_forced_frame();

                // If this was the last momentum frame, send Ended now.
                if fling_ended {
                    callback(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                        position,
                        delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.0), gpui::px(0.0))),
                        modifiers,
                        touch_phase: gpui::TouchPhase::Ended,
                    }));
                    self.request_forced_frame();
                }
            }
        } else if scroller.is_finished() {
            // Fling truly finished — emit one final Ended event so GPUI knows
            // the scroll gesture is complete.  We only do this when
            // `is_finished()` is true, which distinguishes a natural stop
            // from a sub-microsecond `dt` where `step()` returns `None`
            // but the scroller is still active.
            let position = gpui::point(
                gpui::px(scroller.position_x()),
                gpui::px(scroller.position_y()),
            );
            let modifiers = self.modifiers.get();
            if let Some(callback) = self.input_callback.borrow_mut().as_mut() {
                callback(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position,
                    delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.0), gpui::px(0.0))),
                    modifiers,
                    touch_phase: gpui::TouchPhase::Ended,
                }));
                self.request_forced_frame();
            }
        }
    }

    /// Show the software keyboard with the specified keyboard type.
    ///
    /// The actual `becomeFirstResponder` call is deferred to the next run-loop
    /// iteration via `performSelector:withObject:afterDelay:` to avoid re-entering
    /// GPUI's event dispatch while an entity lease is active (UIKit's keyboard
    /// presentation can synchronously trigger layout callbacks).
    pub fn show_keyboard_with_type(&self, keyboard_type: crate::KeyboardType) {
        log::info!("GPUI iOS: Showing keyboard (type={:?})", keyboard_type);
        unsafe {
            use crate::KeyboardType;
            let kb_type: isize = match keyboard_type {
                KeyboardType::Default => 0,      // UIKeyboardTypeDefault
                KeyboardType::EmailAddress => 7, // UIKeyboardTypeEmailAddress
                KeyboardType::Phone => 5,        // UIKeyboardTypePhonePad
                KeyboardType::NumberPad => 4,    // UIKeyboardTypeNumberPad
                KeyboardType::URL => 3,          // UIKeyboardTypeURL
                KeyboardType::Decimal => 8,      // UIKeyboardTypeDecimalPad
            };
            log::info!(
                "GPUI iOS: text_input_view={:p}, setKeyboardType: {}",
                self.text_input_view,
                kb_type
            );
            if self.text_input_view.is_null() {
                log::error!("GPUI iOS: text_input_view is NULL!");
                return;
            }
            let _: () = msg_send![self.text_input_view, setKeyboardType: kb_type];
            log::info!("GPUI iOS: setAutocorrectionType");
            let _: () = msg_send![self.text_input_view, setAutocorrectionType: 1_isize];
            log::info!("GPUI iOS: setAutocapitalizationType");
            let _: () = msg_send![self.text_input_view, setAutocapitalizationType: 0_isize];
            log::info!("GPUI iOS: scheduling becomeFirstResponder");

            // Defer becomeFirstResponder to the next run-loop iteration.
            let _: () = msg_send![self.text_input_view,
                performSelector: sel!(becomeFirstResponder)
                withObject: ptr::null::<Object>()
                afterDelay: 0.0_f64
            ];
            log::info!("GPUI iOS: show_keyboard_with_type done");
        }
    }

    /// Hide the software keyboard.
    ///
    /// Deferred to the next run-loop iteration (like `show_keyboard_with_type`)
    /// to avoid re-entering GPUI event dispatch.
    pub fn hide_keyboard(&self) {
        log::info!("GPUI iOS: Hiding keyboard");
        unsafe {
            let _: () = msg_send![self.text_input_view,
                performSelector: sel!(resignFirstResponder)
                withObject: ptr::null::<Object>()
                afterDelay: 0.0_f64
            ];
        }
        self.focus_hardware_keyboard_view();
    }

    /// Make the Metal view the first responder so attached iPad keyboards can
    /// send app-level key events when no text field is active.
    pub(super) fn focus_hardware_keyboard_view(&self) {
        if self.view.is_null() {
            return;
        }

        unsafe {
            if !self.text_input_view.is_null() {
                let text_input_is_first_responder: BOOL =
                    msg_send![self.text_input_view, isFirstResponder];
                if text_input_is_first_responder == YES {
                    return;
                }
            }

            let _: () = msg_send![self.view,
                performSelector: sel!(becomeFirstResponder)
                withObject: ptr::null::<Object>()
                afterDelay: 0.0_f64
            ];
        }
    }

    /// Handle text input from the software keyboard
    pub fn handle_text_input(&self, text: *mut Object) {
        if text.is_null() {
            return;
        }

        if let Some(text_str) = ns_string_to_string(text) {
            log::info!("GPUI iOS: Text input: {:?}", text_str);

            // Try the global text input callback (for our TextInput components).
            // The text is captured in PENDING_TEXT regardless of whether we also
            // send key events below.
            let dispatched = crate::dispatch_text_input(&text_str);

            // Try the input handler (for GPUI's built-in text fields)
            if !dispatched {
                if let Some(handler) = self.input_handler.borrow_mut().as_mut() {
                    handler.replace_text_in_range(None, &text_str);
                    return;
                }
            }

            // Send key events through GPUI's input callback.
            // Even if dispatch_text_input captured the text, we still send key
            // events so GPUI triggers a re-render cycle (which runs
            // drain_pending_text and updates the UI).
            //
            // We send the *entire* composed string as a single KeyDown event
            // rather than one event per codepoint, so grapheme clusters
            // (e.g. emoji with ZWJ, combining characters) stay intact.
            let keystroke = gpui::Keystroke {
                modifiers: Modifiers::default(),
                key: text_str.clone(),
                key_char: Some(text_str),
            };

            let event = PlatformInput::KeyDown(gpui::KeyDownEvent {
                keystroke,
                is_held: false,
                prefer_character_input: true,
            });

            if let Some(callback) = self.input_callback.borrow_mut().as_mut() {
                callback(event);
            }
        }
    }

    /// Handle the delete-backward action from the software keyboard.
    ///
    /// This is called by the `GPUITextInputView` when the user taps the
    /// backspace key.  We dispatch a special sentinel ("\x08") through the
    /// global text input callback so the active TextInput component can
    /// remove the last character.
    pub fn handle_delete_backward(&self) {
        log::info!("GPUI iOS: deleteBackward");

        // Try the global callback first (backspace = "\x08")
        crate::dispatch_text_input("\x08");

        // Always send a Backspace KeyDown event through GPUI to trigger
        // a re-render cycle (which runs drain_pending_text).
        let keystroke = gpui::Keystroke {
            modifiers: Modifiers::default(),
            key: "backspace".to_string(),
            key_char: None,
        };
        let event = PlatformInput::KeyDown(gpui::KeyDownEvent {
            keystroke,
            is_held: false,
            prefer_character_input: false,
        });
        if let Some(callback) = self.input_callback.borrow_mut().as_mut() {
            callback(event);
        }
    }

    /// Handle a key event from an external keyboard
    pub fn handle_key_event(&self, key_code: u32, modifier_flags: u32, is_key_down: bool) {
        self.handle_key_event_with_characters(key_code, modifier_flags, None, is_key_down);
    }

    /// Handle a key event from an external keyboard, optionally using UIKit's
    /// layout-aware character value for printable input.
    pub(super) fn handle_key_event_with_characters(
        &self,
        key_code: u32,
        modifier_flags: u32,
        characters: Option<String>,
        is_key_down: bool,
    ) {
        use super::super::text_input::{
            key_code_to_key_down, key_code_to_key_down_with_characters, key_code_to_key_up,
            key_code_to_string, modifier_flags_to_modifiers,
        };

        let key = key_code_to_string(key_code);
        let modifiers = modifier_flags_to_modifiers(modifier_flags);
        self.modifiers.set(modifiers);

        if matches!(key_code, 0xE0..=0xE7) {
            return;
        }

        log::info!(
            "GPUI iOS: Key event - key: {:?}, modifiers: {:?}, down: {}",
            key,
            modifiers,
            is_key_down
        );

        // On key-down, dispatch cursor-movement control codes through the
        // global text input callback so TextField-based components receive them.
        if is_key_down {
            match key_code {
                0x50 => {
                    crate::dispatch_text_input("\x1b[D");
                } // Left arrow
                0x4F => {
                    crate::dispatch_text_input("\x1b[C");
                } // Right arrow
                0x4A => {
                    crate::dispatch_text_input("\x1b[H");
                } // Home
                0x4D => {
                    crate::dispatch_text_input("\x1b[F");
                } // End
                _ => {}
            }
        }

        let event = if is_key_down {
            if characters.is_some() {
                key_code_to_key_down_with_characters(key_code, modifier_flags, characters)
            } else {
                key_code_to_key_down(key_code, modifier_flags)
            }
        } else {
            key_code_to_key_up(key_code, modifier_flags)
        };

        if let Some(callback) = self.input_callback.borrow_mut().as_mut() {
            callback(event);
        }
    }

    /// Notify the window of active status changes (foreground/background).
    ///
    /// This is called by the FFI layer when the app transitions between
    /// foreground and background states.
    pub fn notify_active_status_change(&self, is_active: bool) {
        log::info!("GPUI iOS: Window active status changed to: {}", is_active);

        if let Some(callback) = self.active_status_callback.borrow_mut().as_mut() {
            callback(is_active);
        }
        if is_active {
            self.focus_hardware_keyboard_view();
        }
    }

    /// Handle a layout change (e.g. rotation, split-screen resize).
    ///
    /// Called from `viewDidLayoutSubviews` on the GPUIViewController.
    /// Queries the current UIView bounds, updates the stored bounds/scale,
    /// reconfigures the Metal layer + wgpu surface, and fires the resize callback.
    pub fn handle_layout_change(&self) {
        let Some(metrics) = self.scene_metrics() else {
            return;
        };

        let new_w = metrics.width;
        let new_h = metrics.height;
        let new_scale = metrics.scale_factor;

        let old_bounds = self.bounds.get();
        let old_scale = self.scale_factor.get();

        let new_size = size(px(new_w), px(new_h));

        // Only process if something actually changed.
        if old_bounds.size == new_size && (old_scale - new_scale).abs() < 0.01 {
            return;
        }

        log::info!(
            "GPUI iOS: Layout changed — {:?} @{:.1}x → {:?} @{:.1}x ({:?})",
            old_bounds.size,
            old_scale,
            new_size,
            new_scale,
            metrics.layout_mode(),
        );

        // Update stored bounds (in logical pixels, matching GPUI convention).
        let new_bounds = Bounds {
            origin: Default::default(),
            size: new_size,
        };
        self.bounds.set(new_bounds);
        self.scale_factor.set(new_scale);

        unsafe {
            // Update the Metal layer's contentsScale so the drawable has the correct pixel dimensions.
            let layer: *mut Object = msg_send![self.view, layer];
            let scale = new_scale as core_graphics::base::CGFloat;
            let _: () = msg_send![layer, setContentsScale: scale];
        }

        // Update the wgpu renderer's surface configuration.
        let pixel_w = (new_w * new_scale).round().max(1.0) as i32;
        let pixel_h = (new_h * new_scale).round().max(1.0) as i32;
        {
            let mut guard = self.renderer.lock();
            if let Some(renderer) = guard.as_mut() {
                renderer.update_drawable_size(size(DevicePixels(pixel_w), DevicePixels(pixel_h)));
            }
        }

        // Fire the resize callback so GPUI re-layouts at the new size.
        let cb = self.resize_callback.borrow_mut().take();
        if let Some(mut cb) = cb {
            cb(new_size, new_scale);
            // Restore the callback for future resize events.
            let mut slot = self.resize_callback.borrow_mut();
            if slot.is_none() {
                *slot = Some(cb);
            }
        }
    }
}

impl HasWindowHandle for IosWindow {
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        let view = NonNull::new(self.view as *mut c_void)
            .ok_or(raw_window_handle::HandleError::Unavailable)?;
        let handle = UiKitWindowHandle::new(view);
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(handle.into()) })
    }
}

impl HasDisplayHandle for IosWindow {
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        let handle = UiKitDisplayHandle::new();
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(handle.into()) })
    }
}

impl PlatformWindow for IosWindow {
    fn bounds(&self) -> Bounds<Pixels> {
        self.bounds.get()
    }

    fn is_maximized(&self) -> bool {
        true // iOS windows are always "maximized"
    }

    fn window_bounds(&self) -> WindowBounds {
        WindowBounds::Fullscreen(self.bounds.get())
    }

    fn content_size(&self) -> Size<Pixels> {
        self.bounds.get().size
    }

    fn resize(&mut self, _size: Size<Pixels>) {
        // iOS windows cannot be resized programmatically
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor.get()
    }

    fn appearance(&self) -> WindowAppearance {
        unsafe {
            let trait_collection: *mut Object = msg_send![self.view, traitCollection];
            let style: i64 = msg_send![trait_collection, userInterfaceStyle];
            match style {
                2 => WindowAppearance::Dark,
                _ => WindowAppearance::Light,
            }
        }
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(Rc::new(IosDisplay::main()))
    }

    fn mouse_position(&self) -> Point<Pixels> {
        self.mouse_position.get()
    }

    fn modifiers(&self) -> Modifiers {
        self.modifiers.get()
    }

    fn capslock(&self) -> Capslock {
        // Would need to check UIKeyModifierFlags
        Capslock { on: false }
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        *self.input_handler.borrow_mut() = Some(input_handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.input_handler.borrow_mut().take()
    }

    fn prompt(
        &self,
        _level: PromptLevel,
        msg: &str,
        detail: Option<&str>,
        answers: &[PromptButton],
    ) -> Option<futures::channel::oneshot::Receiver<usize>> {
        let (tx, rx) = futures::channel::oneshot::channel();
        let tx = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));

        unsafe {
            // Create UIAlertController
            let alert_style: i64 = 1; // UIAlertControllerStyleAlert

            let title_str = super::super::ns_string_from_str(msg);
            let message_str = super::super::ns_string_from_str(detail.unwrap_or(""));

            let alert: *mut Object = msg_send![
                class!(UIAlertController),
                alertControllerWithTitle: title_str
                message: message_str
                preferredStyle: alert_style
            ];

            // Add buttons
            for (index, button) in answers.iter().enumerate() {
                let button_title = super::super::ns_string_from_str(button.label());

                let action_style: i64 = if button.is_cancel() { 1 } else { 0 }; // UIAlertActionStyleCancel or Default

                let tx_clone = tx.clone();
                let block = block2::RcBlock::new(move |_action: *mut c_void| {
                    if let Ok(mut guard) = tx_clone.lock() {
                        if let Some(sender) = guard.take() {
                            let _ = sender.send(index);
                        }
                    }
                });

                let action: *mut Object = msg_send![
                    class!(UIAlertAction),
                    actionWithTitle: button_title
                    style: action_style
                    handler: &*block
                ];

                let _: () = msg_send![alert, addAction: action];
            }

            // Present the alert
            let _: () = msg_send![
                self.view_controller,
                presentViewController: alert
                animated: YES
                completion: ptr::null::<Object>()
            ];
        }

        Some(rx)
    }

    fn activate(&self) {
        unsafe {
            let _: () = msg_send![self.window, makeKeyAndVisible];
        }
    }

    fn is_active(&self) -> bool {
        unsafe {
            let app: *mut Object = msg_send![class!(UIApplication), sharedApplication];
            let key_window: *mut Object = msg_send![app, keyWindow];
            self.window == key_window
        }
    }

    fn is_hovered(&self) -> bool {
        // Hover isn't really applicable on iOS
        false
    }

    fn set_title(&mut self, _title: &str) {
        // iOS apps don't have window titles
    }

    fn background_appearance(&self) -> WindowBackgroundAppearance {
        WindowBackgroundAppearance::Opaque
    }

    fn set_background_appearance(&self, _background_appearance: WindowBackgroundAppearance) {
        // Could adjust view background color
    }

    fn minimize(&self) {
        // iOS apps cannot be minimized
    }

    fn zoom(&self) {
        // iOS apps cannot be zoomed
    }

    fn toggle_fullscreen(&self) {
        // iOS apps are always fullscreen
    }

    fn is_fullscreen(&self) -> bool {
        true
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        *self.request_frame_callback.borrow_mut() = Some(callback);
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> DispatchEventResult>) {
        *self.input_callback.borrow_mut() = Some(callback);
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        *self.active_status_callback.borrow_mut() = Some(callback);
    }

    fn on_hover_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        *self.hover_status_callback.borrow_mut() = Some(callback);
    }

    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        *self.resize_callback.borrow_mut() = Some(callback);
    }

    fn on_moved(&self, callback: Box<dyn FnMut()>) {
        *self.moved_callback.borrow_mut() = Some(callback);
    }

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        *self.should_close_callback.borrow_mut() = Some(callback);
    }

    fn on_hit_test_window_control(&self, callback: Box<dyn FnMut() -> Option<WindowControlArea>>) {
        *self.hit_test_callback.borrow_mut() = Some(callback);
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        *self.close_callback.borrow_mut() = Some(callback);
    }

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        *self.appearance_changed_callback.borrow_mut() = Some(callback);
    }

    fn draw(&self, scene: &Scene) {
        let mut guard = self.renderer.lock();
        if let Some(renderer) = guard.as_mut() {
            renderer.draw(scene);
        } else {
            log::trace!("GPUI iOS: draw called but no renderer available");
        }
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        if let Some(atlas) = self.sprite_atlas.lock().as_ref() {
            return atlas.clone();
        }

        let atlas: Arc<dyn PlatformAtlas> = {
            let guard = self.renderer.lock();
            if let Some(renderer) = guard.as_ref() {
                renderer.sprite_atlas().clone()
            } else {
                // Fallback: return a dummy atlas so GPUI doesn't panic before
                // the renderer is initialised.
                Arc::new(FallbackAtlas::new())
            }
        };

        *self.sprite_atlas.lock() = Some(atlas.clone());
        atlas
    }

    fn is_subpixel_rendering_supported(&self) -> bool {
        // iOS uses grayscale antialiasing, not subpixel rendering
        false
    }

    fn gpu_specs(&self) -> Option<GpuSpecs> {
        let guard = self.renderer.lock();
        guard.as_ref().map(|r| r.gpu_specs())
    }

    fn update_ime_position(&self, _bounds: Bounds<Pixels>) {
        // iOS handles IME positioning automatically
    }
}
