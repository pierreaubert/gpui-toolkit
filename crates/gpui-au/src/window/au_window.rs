use super::super::AuDisplay;
use super::PENDING_VIEW;
use super::au_raw_window::AuRawWindow;
use super::fallback_atlas::FallbackAtlas;
use crate::helpers::{nslog, nslog_verbose};
use crate::params::AuParameterTree;
use gpui::{
    AnyWindowHandle, Bounds, Capslock, DevicePixels, DispatchEventResult, GpuSpecs, Modifiers,
    Pixels, PlatformAtlas, PlatformDisplay, PlatformInput, PlatformInputHandler, PlatformWindow,
    Point, PromptButton, PromptLevel, RequestFrameOptions, Scene, Size, WindowAppearance,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowParams, WindowVisibility,
    px, size,
};
use gpui_foreign_parent::{CallbackSlot, FrameThrottle, ResizeDebounce, TransitionCell};
use gpui_wgpu::{GpuContext, WgpuRenderer, WgpuSurfaceConfig};
use objc::{class, msg_send, runtime::Object, sel, sel_impl};
use parking_lot::Mutex;
use raw_window_handle::{
    AppKitDisplayHandle, AppKitWindowHandle, HasDisplayHandle, HasWindowHandle,
};
use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    ptr::NonNull,
    rc::Rc,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

/// Single shared fallback atlas used when no wgpu renderer is available.
/// Avoids allocating a fresh `Mutex`+`HashMap` on every `sprite_atlas()` call.
static FALLBACK_ATLAS: OnceLock<Arc<FallbackAtlas>> = OnceLock::new();

fn previous_utf16_code_point_start(caret: usize, suffix: &str) -> usize {
    suffix.chars().last().map_or(caret, |character| {
        caret.saturating_sub(character.len_utf16())
    })
}

pub(crate) struct AuWindow {
    /// The `NSView` we render into (owned by the Swift `AUViewController`)
    pub(super) view: *mut Object,
    pub(super) bounds: Cell<Bounds<Pixels>>,
    pub(super) scale_factor: Cell<f32>,
    pub(super) input_handler: RefCell<Option<PlatformInputHandler>>,
    pub(crate) request_frame_callback: CallbackSlot<RequestFrameOptions>,
    pub(super) input_callback: CallbackSlot<PlatformInput, DispatchEventResult>,
    active: TransitionCell,
    hovered: TransitionCell,
    pub(super) resize_callback: CallbackSlot<(Size<Pixels>, f32)>,
    pub(super) moved_callback: CallbackSlot<()>,
    pub(super) should_close_callback: CallbackSlot<(), bool>,
    pub(super) hit_test_callback: CallbackSlot<(), Option<WindowControlArea>>,
    pub(super) close_callback: RefCell<Option<Box<dyn FnOnce()>>>,
    pub(super) appearance_changed_callback: CallbackSlot<()>,
    pub(super) mouse_position: Cell<Point<Pixels>>,
    pub(super) modifiers: Cell<Modifiers>,
    pub(super) renderer: Mutex<Option<Rc<Mutex<WgpuRenderer>>>>,
    /// Shared GPU context, created up front (cheap) and consumed by the
    /// deferred renderer construction.
    gpu_context: GpuContext,
    /// Deferred wgpu construction args. `AuWindow::new` runs on the plugin
    /// main thread during AU instantiation, so it only attaches the cheap
    /// `CAMetalLayer` and stashes these; the blocking `WgpuRenderer::new`
    /// runs lazily from `ensure_renderer_initialized()` on the first frame.
    /// (`AppKit` surface creation is main-thread-affine, so a background
    /// thread cannot safely take this work instead.)
    pending_gpu_init: Mutex<Option<PendingGpuInit>>,
    /// Debounced wgpu surface reconfiguration (contract §§5–6).
    surface_debounce: ResizeDebounce<Size<DevicePixels>>,
    /// Host-pump frame-request throttle (contract §6).
    frame_throttle: FrameThrottle,
    /// Frames dropped because the renderer was busy (`try_lock` failed).
    dropped_frames: AtomicUsize,
    /// Minimal host parameter bridge (`AUParameterTree` analogue).
    parameters: AuParameterTree,
}

/// Deferred arguments for lazy wgpu renderer construction.
struct PendingGpuInit {
    ns_view: *mut c_void,
    size: Size<DevicePixels>,
}

impl AuWindow {
    /// Create a new AU window that renders into the `NSView` from `PENDING_VIEW`.
    ///
    /// The `NSView` must have been set in the `PENDING_VIEW` thread-local by
    /// `gpui_au_create` before calling `app.run()` / `open_window()`.
    pub fn new(_handle: AnyWindowHandle, _params: WindowParams) -> Self {
        nslog_verbose(b"SOTF AuWindow::new: entry");

        let view_info = PENDING_VIEW.with(|pv| pv.borrow_mut().take());
        let (ns_view, width, height, scale) = if let Some(info) = view_info {
            nslog_verbose(b"SOTF AuWindow::new: PENDING_VIEW found");
            (info.ns_view, info.width, info.height, info.scale)
        } else {
            nslog_verbose(b"SOTF AuWindow::new: No PENDING_VIEW -- creating without renderer");
            return Self::without_view();
        };

        // Configure the NSView with a CAMetalLayer for wgpu rendering.
        // This is cheap and stays on the instantiation path; the blocking
        // wgpu device/surface creation is deferred to the first frame.
        nslog_verbose(b"SOTF AuWindow::new: setting up CAMetalLayer");
        // SAFETY: `ns_view` is the non-null view stashed by
        // `gpui_au_create`; `CAMetalLayer` setup follows AppKit conventions.
        unsafe {
            let _: () = msg_send![ns_view, setWantsLayer: true];

            // Create a CAMetalLayer and set it as the view's layer
            let metal_layer: *mut Object = msg_send![class!(CAMetalLayer), layer];
            let _: () =
                msg_send![metal_layer, setContentsScale: scale as core_graphics::base::CGFloat];
            // Match the view's bounds
            let view_bounds: core_graphics::geometry::CGRect = msg_send![ns_view, bounds];
            let _: () = msg_send![metal_layer, setFrame: view_bounds];
            let _: () = msg_send![ns_view, setLayer: metal_layer];
        }

        // Defer the blocking wgpu device/surface creation to the first frame
        // (see `ensure_renderer_initialized`): AU instantiation must return
        // to the host quickly with only the placeholder layer in place.
        let pixel_w = ((width * scale) as i32).max(1);
        let pixel_h = ((height * scale) as i32).max(1);

        Self {
            view: ns_view,
            bounds: Cell::new(Bounds {
                origin: Default::default(),
                size: size(px(width), px(height)),
            }),
            scale_factor: Cell::new(scale),
            input_handler: RefCell::new(None),
            request_frame_callback: CallbackSlot::new(),
            input_callback: CallbackSlot::new(),
            active: TransitionCell::new(true),
            hovered: TransitionCell::new(false),
            resize_callback: CallbackSlot::new(),
            moved_callback: CallbackSlot::new(),
            should_close_callback: CallbackSlot::new(),
            hit_test_callback: CallbackSlot::new(),
            close_callback: RefCell::new(None),
            appearance_changed_callback: CallbackSlot::new(),
            mouse_position: Cell::new(Point::default()),
            modifiers: Cell::new(Modifiers::default()),
            renderer: Mutex::new(None),
            gpu_context: Rc::new(RefCell::new(None)),
            pending_gpu_init: Mutex::new(Some(PendingGpuInit {
                ns_view: ns_view.cast::<c_void>(),
                size: size(DevicePixels(pixel_w), DevicePixels(pixel_h)),
            })),
            surface_debounce: ResizeDebounce::default(),
            frame_throttle: FrameThrottle::default(),
            dropped_frames: AtomicUsize::new(0),
            parameters: AuParameterTree::with_default_plugin_params(),
        }
    }

    /// Window without a host view (no `PENDING_VIEW)`: placeholder bounds and
    /// no renderer. Drawing falls back to the shared CPU atlas stub and all
    /// surface work is skipped until a view arrives via `handle_resize`.
    fn without_view() -> Self {
        Self {
            view: std::ptr::null_mut(),
            bounds: Cell::new(Bounds {
                origin: Default::default(),
                size: size(px(600.0), px(400.0)),
            }),
            scale_factor: Cell::new(2.0),
            input_handler: RefCell::new(None),
            request_frame_callback: CallbackSlot::new(),
            input_callback: CallbackSlot::new(),
            active: TransitionCell::new(true),
            hovered: TransitionCell::new(false),
            resize_callback: CallbackSlot::new(),
            moved_callback: CallbackSlot::new(),
            should_close_callback: CallbackSlot::new(),
            hit_test_callback: CallbackSlot::new(),
            close_callback: RefCell::new(None),
            appearance_changed_callback: CallbackSlot::new(),
            mouse_position: Cell::new(Point::default()),
            modifiers: Cell::new(Modifiers::default()),
            renderer: Mutex::new(None),
            gpu_context: Rc::new(RefCell::new(None)),
            pending_gpu_init: Mutex::new(None),
            surface_debounce: ResizeDebounce::default(),
            frame_throttle: FrameThrottle::default(),
            dropped_frames: AtomicUsize::new(0),
            parameters: AuParameterTree::with_default_plugin_params(),
        }
    }

    /// Construct the wgpu renderer on first use, not on the AU instantiation
    /// path. No-op once initialized, when there is no host view, or when a
    /// previous attempt already consumed the deferred args (a later
    /// `handle_resize` re-arms the attempt with the newest size).
    fn ensure_renderer_initialized(&self) {
        if self.renderer.lock().is_some() || self.view.is_null() {
            return;
        }
        let pending = self.pending_gpu_init.lock().take();
        let Some(pending) = pending else { return };
        nslog_verbose(b"SOTF AuWindow: lazy wgpu renderer init");
        // Create a lightweight raw window handle for WgpuRenderer::new().
        // WgpuRenderer::new handles instance + surface creation internally.
        let raw_window = AuRawWindow {
            ns_view: pending.ns_view,
        };
        // `preferred_present_mode: None` selects `PresentMode::Fifo`
        // (VSync) in the renderer's surface setup, so frames stay paced by
        // the display link instead of free-running.
        let config = WgpuSurfaceConfig {
            size: pending.size,
            transparent: false,
            preferred_present_mode: None,
        };
        match WgpuRenderer::new(Rc::clone(&self.gpu_context), &raw_window, config, None) {
            Ok(renderer) => {
                // `Rc` (not `Arc`): every use is gated on the host main
                // thread by `with_au_window`, and `WgpuRenderer` is `!Send`.
                nslog_verbose(b"SOTF AuWindow: lazy wgpu renderer created OK");
                *self.renderer.lock() = Some(Rc::new(Mutex::new(renderer)));
            }
            Err(e) => {
                let msg = format!("SOTF AuWindow: lazy wgpu renderer FAILED: {e:#}");
                nslog(msg.as_bytes());
            }
        }
    }

    /// Attempt a surface reconfigure without blocking: returns false when
    /// there is no renderer yet or its locks are contended. The realtime
    /// path must never wait on the GPU.
    fn try_reconfigure_surface(&self, drawable: Size<DevicePixels>) -> bool {
        let Some(outer) = self.renderer.try_lock() else {
            return false;
        };
        let Some(renderer) = outer.as_ref() else {
            return false;
        };
        let Some(mut inner) = renderer.try_lock() else {
            return false;
        };
        inner.update_drawable_size(drawable);
        true
    }

    /// Apply a new drawable size to the wgpu surface.
    ///
    /// The expensive `surface.configure` is debounced through the shared
    /// policy; a size that arrives inside the window is stashed and picked
    /// up by `apply_pending_surface_size()` on the next drawn frame, so the
    /// surface always converges to the latest host size.
    fn reconfigure_surface(&self, drawable: Size<DevicePixels>) {
        self.surface_debounce
            .submit(drawable, |size| self.try_reconfigure_surface(size));
    }

    /// Apply a debounced resize ahead of drawing, if one is pending and the
    /// debounce window has elapsed.
    fn apply_pending_surface_size(&self) {
        self.surface_debounce
            .poll(|size| self.try_reconfigure_surface(size));
    }

    /// Request a frame render (called from Swift via FFI)
    pub fn request_frame(&self) {
        // Coalesce CVDisplayLink overdrive: ticks arriving faster than the
        // throttle interval are counted and dropped instead of queueing
        // redundant GPUI frame work.
        if !self.frame_throttle.should_forward() {
            return;
        }
        self.request_frame_callback
            .emit(RequestFrameOptions::default());
    }

    /// Update host focus state and notify GPUI only on a transition.
    pub fn update_active_status(&self, is_active: bool) {
        self.active.update(is_active);
    }

    /// Update pointer-in-view state and notify GPUI only on a transition.
    pub fn update_hover_status(&self, is_hovered: bool) {
        self.hovered.update(is_hovered);
    }

    /// Handle resize from the host (called from Swift via FFI)
    pub fn handle_resize(&self, width: f32, height: f32, scale: f32) {
        let new_size = size(px(width), px(height));
        self.bounds.set(Bounds {
            origin: Default::default(),
            size: new_size,
        });
        self.scale_factor.set(scale);

        // Update Metal layer scale and frame
        if !self.view.is_null() {
            // SAFETY: `view` is null-checked and live; `layer` messaging
            // uses valid selectors with matching argument types.
            unsafe {
                let layer: *mut Object = msg_send![self.view, layer];
                let _: () =
                    msg_send![layer, setContentsScale: scale as core_graphics::base::CGFloat];
                let new_frame = core_graphics::geometry::CGRect {
                    origin: core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
                    size: core_graphics::geometry::CGSize {
                        width: f64::from(width),
                        height: f64::from(height),
                    },
                };
                let _: () = msg_send![layer, setFrame: new_frame];
            }
        }

        // Update wgpu surface (debounced; bounds/layer/callback above already
        // track the newest host size immediately).
        let pixel_w = ((width * scale) as i32).max(1);
        let pixel_h = ((height * scale) as i32).max(1);
        let drawable = size(DevicePixels(pixel_w), DevicePixels(pixel_h));
        if !self.view.is_null() && self.renderer.lock().is_none() {
            // No renderer yet (lazy init pending or a previous attempt
            // failed): refresh the deferred args so the next frame builds
            // the surface at the newest size.
            *self.pending_gpu_init.lock() = Some(PendingGpuInit {
                ns_view: self.view.cast::<c_void>(),
                size: drawable,
            });
        }
        self.reconfigure_surface(drawable);

        // Fire resize callback
        self.resize_callback.emit((new_size, scale));
    }

    /// Dispatch a mouse event (called from Swift via FFI)
    pub fn dispatch_input(&self, event: PlatformInput) {
        // Update tracked mouse position for MouseMove/Down/Up
        match &event {
            PlatformInput::MouseDown(e) => {
                self.mouse_position.set(e.position);
                self.modifiers.set(e.modifiers);
            }
            PlatformInput::MouseUp(e) => {
                self.mouse_position.set(e.position);
                self.modifiers.set(e.modifiers);
            }
            PlatformInput::MouseMove(e) => {
                self.mouse_position.set(e.position);
                self.modifiers.set(e.modifiers);
            }
            PlatformInput::ScrollWheel(e) => self.modifiers.set(e.modifiers),
            PlatformInput::KeyDown(e) => self.modifiers.set(e.keystroke.modifiers),
            PlatformInput::KeyUp(e) => self.modifiers.set(e.keystroke.modifiers),
            _ => {}
        }

        // The dispatch result is informational; embedded input has no
        // further propagation target.
        let _ = self.input_callback.emit_with(event);
    }

    pub fn insert_text(&self, text: &str) {
        if let Some(handler) = self.input_handler.borrow_mut().as_mut() {
            handler.replace_text_in_range(None, text);
        }
    }

    pub fn set_marked_text(&self, text: &str, selected_location: usize, selected_length: usize) {
        if let Some(handler) = self.input_handler.borrow_mut().as_mut() {
            let text_len = text.encode_utf16().count();
            let start = selected_location.min(text_len);
            let end = start.saturating_add(selected_length).min(text_len);
            handler.replace_and_mark_text_in_range(None, text, Some(start..end));
        }
    }

    pub fn unmark_text(&self) {
        if let Some(handler) = self.input_handler.borrow_mut().as_mut() {
            handler.unmark_text();
        }
    }

    pub fn delete_backward(&self) {
        if let Some(handler) = self.input_handler.borrow_mut().as_mut()
            && let Some(selection) = handler.selected_text_range(true)
        {
            let start = if selection.range.is_empty() {
                let caret = selection.range.start;
                let mut adjusted = None;
                handler
                    .text_for_range(caret.saturating_sub(2)..caret, &mut adjusted)
                    .map_or_else(
                        || caret.saturating_sub(1),
                        |suffix| previous_utf16_code_point_start(caret, &suffix),
                    )
            } else {
                selection.range.start
            };
            handler.replace_text_in_range(Some(start..selection.range.end), "");
        }
    }

    /// Frames dropped because the renderer was busy when `draw` ran.
    pub fn dropped_frames(&self) -> usize {
        self.dropped_frames.load(Ordering::Relaxed)
    }

    /// `CVDisplayLink` ticks coalesced by the frame-request throttle.
    pub fn coalesced_frames(&self) -> usize {
        self.frame_throttle.coalesced()
    }

    /// Number of parameters in the plugin's parameter tree.
    pub fn parameter_count(&self) -> usize {
        self.parameters.parameter_count()
    }

    /// Current value of a parameter, or `None` for an unknown id.
    pub fn parameter_value(&self, id: u32) -> Option<f32> {
        self.parameters.get_value(id)
    }

    /// Store a clamped parameter value. Returns false for an unknown id.
    pub fn set_parameter_value(&self, id: u32, value: f32) -> bool {
        self.parameters.set_value(id, value).is_ok()
    }

    /// Register a host parameter. Returns false on duplicate ids or
    /// invalid ranges.
    pub fn register_parameter(
        &self,
        id: u32,
        name: &str,
        min_value: f32,
        max_value: f32,
        default_value: f32,
    ) -> bool {
        self.parameters
            .add_parameter(id, name, min_value, max_value, default_value)
            .is_ok()
    }

    /// Serialize the plugin state (`fullState` analogue).
    pub fn capture_plugin_state(&self) -> Vec<u8> {
        self.parameters.capture_state()
    }

    /// Restore plugin state. Returns false on a corrupt payload.
    pub fn restore_plugin_state(&self, bytes: &[u8]) -> bool {
        self.parameters.restore_state(bytes).is_ok()
    }
}

impl HasWindowHandle for AuWindow {
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        let view = NonNull::new(self.view.cast::<c_void>())
            .ok_or(raw_window_handle::HandleError::Unavailable)?;
        let handle = AppKitWindowHandle::new(view);
        // SAFETY: `view` is a non-null live `NSView` for the borrowed
        // lifetime; the handle only names it for surface creation.
        let raw = unsafe { raw_window_handle::WindowHandle::borrow_raw(handle.into()) };
        Ok(raw)
    }
}

impl HasDisplayHandle for AuWindow {
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        let handle = AppKitDisplayHandle::new();
        // SAFETY: the AppKit display handle is a process-global unit value
        // with no lifetime obligations beyond the borrow.
        let raw = unsafe { raw_window_handle::DisplayHandle::borrow_raw(handle.into()) };
        Ok(raw)
    }
}

impl PlatformWindow for AuWindow {
    fn visibility(&self) -> WindowVisibility {
        // The host drives presentation; the embedded view is treated as
        // presented whenever it requests frames.
        WindowVisibility::Visible
    }

    fn on_visibility_change(&self, _callback: Box<dyn FnMut(WindowVisibility)>) {}

    fn on_display_changed(&self, _callback: Box<dyn FnMut()>) {}

    fn bounds(&self) -> Bounds<Pixels> {
        self.bounds.get()
    }

    fn is_maximized(&self) -> bool {
        false
    }

    fn window_bounds(&self) -> WindowBounds {
        WindowBounds::Windowed(self.bounds.get())
    }

    fn content_size(&self) -> Size<Pixels> {
        self.bounds.get().size
    }

    fn resize(&mut self, _size: Size<Pixels>) {
        // Resize is driven externally by the host via handle_resize
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor.get()
    }

    fn appearance(&self) -> WindowAppearance {
        if self.view.is_null() {
            return WindowAppearance::Light;
        }
        // SAFETY: `view` and every derived object are null-checked, so
        // only live objects are messaged; unknown states fall back to Light.
        unsafe {
            let effective: *mut Object = msg_send![self.view, effectiveAppearance];
            if effective.is_null() {
                return WindowAppearance::Light;
            }
            let name: *mut Object = msg_send![effective, name];
            if name.is_null() {
                return WindowAppearance::Light;
            }
            if crate::helpers::is_dark_aqua_appearance_name(name) {
                WindowAppearance::Dark
            } else {
                WindowAppearance::Light
            }
        }
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(Rc::new(AuDisplay::main()))
    }

    fn mouse_position(&self) -> Point<Pixels> {
        self.mouse_position.get()
    }

    fn modifiers(&self) -> Modifiers {
        self.modifiers.get()
    }

    fn capslock(&self) -> Capslock {
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
        _msg: &str,
        _detail: Option<&str>,
        _answers: &[PromptButton],
    ) -> Option<futures::channel::oneshot::Receiver<usize>> {
        // No prompt support in AU extensions
        None
    }

    fn activate(&self) {}

    fn is_active(&self) -> bool {
        self.active.get()
    }

    fn is_hovered(&self) -> bool {
        self.hovered.get()
    }

    fn set_title(&mut self, _title: &str) {}

    fn background_appearance(&self) -> WindowBackgroundAppearance {
        WindowBackgroundAppearance::Opaque
    }

    fn set_background_appearance(&self, _background_appearance: WindowBackgroundAppearance) {}

    fn minimize(&self) {}
    fn zoom(&self) {}
    fn toggle_fullscreen(&self) {}

    fn is_fullscreen(&self) -> bool {
        false
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        self.request_frame_callback.set(callback);
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> DispatchEventResult>) {
        self.input_callback.set(callback);
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.active.on_change(callback);
    }

    fn on_hover_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.hovered.on_change(callback);
    }

    fn on_resize(&self, mut callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        self.resize_callback
            .set(move |(size, scale)| callback(size, scale));
    }

    fn on_moved(&self, mut callback: Box<dyn FnMut()>) {
        self.moved_callback.set(move |()| callback());
    }

    fn on_should_close(&self, mut callback: Box<dyn FnMut() -> bool>) {
        self.should_close_callback.set(move |()| callback());
    }

    fn on_hit_test_window_control(
        &self,
        mut callback: Box<dyn FnMut() -> Option<WindowControlArea>>,
    ) {
        self.hit_test_callback.set(move |()| callback());
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        *self.close_callback.borrow_mut() = Some(callback);
    }

    fn on_appearance_changed(&self, mut callback: Box<dyn FnMut()>) {
        self.appearance_changed_callback.set(move |()| callback());
    }

    fn draw(&self, scene: &Scene) {
        self.ensure_renderer_initialized();
        self.apply_pending_surface_size();
        // Clone the renderer handle under a short lock so the mutex is not
        // held across the GPU work. When the lock (or the renderer) is
        // unavailable the previous frame is still in flight: drop this one
        // and count it instead of stalling the realtime thread.
        let renderer = self.renderer.try_lock().and_then(|guard| guard.clone());
        let Some(renderer) = renderer else {
            self.dropped_frames.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let Some(mut inner) = renderer.try_lock() else {
            self.dropped_frames.fetch_add(1, Ordering::Relaxed);
            return;
        };
        inner.draw(scene);
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        // The atlas handle is an `Arc` clone that stays valid after the
        // locks are released; a contended renderer falls back to the stub.
        if let Some(outer) = self.renderer.try_lock()
            && let Some(renderer) = outer.as_ref()
            && let Some(inner) = renderer.try_lock()
        {
            return inner.sprite_atlas().clone();
        }
        FALLBACK_ATLAS
            .get_or_init(|| Arc::new(FallbackAtlas::new()))
            .clone()
    }

    fn is_subpixel_rendering_supported(&self) -> bool {
        false
    }

    fn gpu_specs(&self) -> Option<GpuSpecs> {
        if let Some(outer) = self.renderer.try_lock()
            && let Some(renderer) = outer.as_ref()
            && let Some(inner) = renderer.try_lock()
        {
            return Some(inner.gpu_specs());
        }
        None
    }

    fn update_ime_position(&self, _bounds: Bounds<Pixels>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::with_au_window;

    fn empty_window() -> AuWindow {
        AuWindow {
            view: std::ptr::null_mut(),
            bounds: Cell::new(Bounds {
                origin: Default::default(),
                size: size(px(600.0), px(400.0)),
            }),
            scale_factor: Cell::new(2.0),
            input_handler: RefCell::new(None),
            request_frame_callback: CallbackSlot::new(),
            input_callback: CallbackSlot::new(),
            active: TransitionCell::new(true),
            hovered: TransitionCell::new(false),
            resize_callback: CallbackSlot::new(),
            moved_callback: CallbackSlot::new(),
            should_close_callback: CallbackSlot::new(),
            hit_test_callback: CallbackSlot::new(),
            close_callback: RefCell::new(None),
            appearance_changed_callback: CallbackSlot::new(),
            mouse_position: Cell::new(Point::default()),
            modifiers: Cell::new(Modifiers::default()),
            renderer: Mutex::new(None),
            gpu_context: Rc::new(RefCell::new(None)),
            pending_gpu_init: Mutex::new(None),
            surface_debounce: ResizeDebounce::default(),
            frame_throttle: FrameThrottle::default(),
            dropped_frames: AtomicUsize::new(0),
            parameters: AuParameterTree::with_default_plugin_params(),
        }
    }

    #[test]
    fn test_with_au_window_returns_none_when_unregistered() {
        // Ensure that with_au_window returns None when no window is registered
        let result = with_au_window(|_window| 42);
        assert!(result.is_none());
    }

    #[test]
    fn host_focus_and_hover_transitions_update_state_once() {
        let window = empty_window();
        let active_transitions = Rc::new(RefCell::new(Vec::new()));
        let hover_transitions = Rc::new(RefCell::new(Vec::new()));
        let active_capture = Rc::clone(&active_transitions);
        let hover_capture = Rc::clone(&hover_transitions);
        window.active.on_change(move |active| {
            active_capture.borrow_mut().push(active);
        });
        window.hovered.on_change(move |hovered| {
            hover_capture.borrow_mut().push(hovered);
        });

        window.update_active_status(false);
        window.update_active_status(false);
        window.update_hover_status(true);
        window.update_hover_status(true);

        assert_eq!(&*active_transitions.borrow(), &[false]);
        assert_eq!(&*hover_transitions.borrow(), &[true]);
        assert!(!PlatformWindow::is_active(&window));
        assert!(PlatformWindow::is_hovered(&window));
    }

    #[test]
    fn backspace_moves_over_a_complete_utf16_code_point() {
        assert_eq!(previous_utf16_code_point_start(1, "a"), 0);
        assert_eq!(previous_utf16_code_point_start(2, "😀"), 0);
        assert_eq!(previous_utf16_code_point_start(4, "a😀"), 2);
    }

    #[test]
    fn test_sprite_atlas_reuses_fallback() {
        let window = empty_window();
        let atlas1 = window.sprite_atlas();
        let atlas2 = window.sprite_atlas();
        assert!(Arc::ptr_eq(&atlas1, &atlas2));
    }

    #[test]
    fn rapid_resize_stashes_pending_size_until_debounce_elapses() {
        let window = empty_window();
        // No renderer yet: reconfigures cannot apply, so the newest size
        // is stashed for the next frame.
        window.handle_resize(800.0, 600.0, 2.0);
        window.handle_resize(1024.0, 768.0, 2.0);
        let pending = window
            .surface_debounce
            .pending()
            .expect("resize should stash a pending size");
        assert_eq!(pending, size(DevicePixels(2048), DevicePixels(1536)));
        // Bounds still track the latest host size immediately.
        assert_eq!(window.bounds.get().size, size(px(1024.0), px(768.0)));
    }

    #[test]
    fn frame_request_overdrive_is_coalesced() {
        let window = empty_window();
        let frames = Rc::new(Cell::new(0usize));
        let capture = Rc::clone(&frames);
        window
            .request_frame_callback
            .set(move |_| capture.set(capture.get() + 1));
        window.request_frame();
        window.request_frame();
        window.request_frame();
        assert_eq!(frames.get(), 1);
        assert_eq!(window.coalesced_frames(), 2);
    }

    #[test]
    fn draw_without_renderer_drops_frame_without_panicking() {
        let window = empty_window();
        // `empty_window` has a null view, so lazy init is skipped and the
        // frame is dropped against the missing renderer.
        window.draw(&Scene::default());
        assert_eq!(window.dropped_frames(), 1);
        assert!(window.gpu_specs().is_none());
    }

    #[test]
    fn window_exposes_default_plugin_parameters() {
        let window = empty_window();
        assert!(window.parameter_count() >= 2);
        assert_eq!(window.parameter_value(0), Some(0.8));
        assert!(window.set_parameter_value(0, 0.5));
        assert!(!window.set_parameter_value(9999, 0.5));
        assert!(window.register_parameter(10, "cutoff", 20.0, 20000.0, 440.0));
        assert!(!window.register_parameter(10, "dup", 0.0, 1.0, 0.5));
        let bytes = window.capture_plugin_state();
        assert!(window.set_parameter_value(0, 0.1));
        assert!(window.restore_plugin_state(&bytes));
        assert!(!window.restore_plugin_state(b"not-a-state"));
        assert_eq!(window.parameter_value(0), Some(0.5));
    }
}
