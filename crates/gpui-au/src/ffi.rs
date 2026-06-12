//! C-compatible FFI functions for embedding GPUI in macOS Audio Unit ViewControllers.

use crate::helpers::nslog;
use crate::window::{PENDING_VIEW, PendingViewInfo, with_au_window};
use gpui::{
    App, AppCell, AppContext, Context, ElementId, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, PlatformInput, Render, RequestFrameOptions,
    StatefulInteractiveElement as _, Styled, Window, WindowOptions, div, point, px, rgb,
};
use objc::runtime::Object;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::rc::Rc;
use std::sync::Once;

static INIT_LOGGER: Once = Once::new();

fn init_logger() {
    INIT_LOGGER.call_once(|| {
        let _ = env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .try_init();
    });
}

// ── Root View ────────────────────────────────────────────────────────────────

struct AuRootView {
    plugin_type: String,
    click_count: usize,
}

impl Render for AuRootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let click_count = self.click_count;
        let plugin_type = self.plugin_type.clone();

        div()
            .size_full()
            .bg(rgb(0x1a1a2e))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_color(rgb(0xffffff))
                    .text_xl()
                    .child(format!("SOTF: {plugin_type}")),
            )
            .child(
                div()
                    .id(ElementId::Name("click-target".into()))
                    .mt(px(16.0))
                    .px(px(16.0))
                    .py(px(8.0))
                    .bg(rgb(0x3366ff))
                    .text_color(rgb(0xffffff))
                    .child(format!("Clicks: {click_count}"))
                    .on_click(cx.listener(|this, _event, _window, _cx| {
                        this.click_count += 1;
                    })),
            )
    }
}

// ── Context ──────────────────────────────────────────────────────────────────

/// Opaque context handle passed to/from Swift.
pub struct AuContext {
    _plugin_type: String,
    /// Prevents GPUI's AppCell from being deallocated after Application::run() returns.
    ///
    /// Application::run(self, callback) consumes self and the callback's captured Rc<AppCell>
    /// is dropped after the callback completes. Since AuPlatform::run() calls the callback
    /// immediately (unlike macOS/iOS platforms which block or defer), all Rc references would
    /// reach zero and AppCell would be deallocated. This clone keeps the refcount positive
    /// for the lifetime of the AU plugin view.
    _app_cell: Rc<AppCell>,
}

impl AuContext {
    /// Create a new AU context (for use by external crates like plugins-ffi).
    pub fn new(plugin_type: String, app_cell: Rc<AppCell>) -> Self {
        Self {
            _plugin_type: plugin_type,
            _app_cell: app_cell,
        }
    }
}

fn clone_application_cell(app: &gpui::Application) -> Rc<AppCell> {
    // GPUI does not expose the AppCell handle, but AU embeddings need to keep
    // it alive after Application::run returns because AuPlatform::run invokes
    // the launch callback synchronously. Clone the inner Rc; never copy it
    // bitwise, or the refcount is not incremented.
    debug_assert_eq!(
        std::mem::size_of::<gpui::Application>(),
        std::mem::size_of::<Rc<AppCell>>(),
        "Application layout changed -- AU AppCell clone assumption broken"
    );
    unsafe {
        let rc: &Rc<AppCell> = std::mem::transmute(app);
        Rc::clone(rc)
    }
}

// ── Lifecycle ─────────────────────────────────────────────────────────────────

/// Create a GPUI context embedded in an NSView.
///
/// # Safety
/// `ns_view` must be a valid NSView pointer. `plugin_type` must be a valid C string.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_create(
    ns_view: *mut Object,
    width: f32,
    height: f32,
    scale: f32,
    plugin_type: *const c_char,
) -> *mut AuContext {
    init_logger();
    nslog(b"SOTF gpui_au_create: entry");

    if ns_view.is_null() || plugin_type.is_null() {
        nslog(b"SOTF gpui_au_create: null pointer argument!");
        return std::ptr::null_mut();
    }

    let plugin_type_str = unsafe {
        match CStr::from_ptr(plugin_type).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                nslog(b"SOTF gpui_au_create: invalid UTF-8");
                return std::ptr::null_mut();
            }
        }
    };

    let msg = format!(
        "SOTF gpui_au_create: plugin={}, size={}x{} @{:.1}x, view={:p}",
        plugin_type_str, width, height, scale, ns_view
    );
    nslog(msg.as_bytes());

    // Store the NSView info in a thread-local so AuWindow::new() can read it
    // during the open_window() call inside app.run().
    PENDING_VIEW.with(|pv| {
        *pv.borrow_mut() = Some(PendingViewInfo {
            ns_view,
            width,
            height,
            scale,
        });
    });

    nslog(b"SOTF gpui_au_create: creating GPUI Application");
    let platform = Rc::new(crate::AuPlatform::new());
    let app = gpui::Application::with_platform(platform);

    let app_cell = clone_application_cell(&app);
    nslog(b"SOTF gpui_au_create: Rc<AppCell> cloned for lifetime management");

    let window_opened = std::rc::Rc::new(std::cell::Cell::new(false));
    let window_opened_clone = window_opened.clone();
    let pt = plugin_type_str.clone();
    app.run(move |cx: &mut App| {
        nslog(b"SOTF gpui_au_create: inside app.run callback");
        match cx.open_window(
            WindowOptions {
                window_bounds: None,
                ..Default::default()
            },
            |_window, cx| {
                cx.new(|_| AuRootView {
                    plugin_type: pt,
                    click_count: 0,
                })
            },
        ) {
            Ok(_handle) => {
                nslog(b"SOTF gpui_au_create: window opened OK");
                window_opened_clone.set(true);
            }
            Err(e) => {
                let msg = format!("SOTF gpui_au_create: open_window FAILED: {e:#}");
                nslog(msg.as_bytes());
            }
        }
    });

    if !window_opened.get() {
        nslog(b"SOTF gpui_au_create: returning null because open_window failed");
        return std::ptr::null_mut();
    }

    nslog(b"SOTF gpui_au_create: app.run() returned, context ready");

    let context = Box::new(AuContext {
        _plugin_type: plugin_type_str,
        _app_cell: app_cell,
    });
    Box::into_raw(context)
}

/// Destroy a GPUI AU context.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_destroy(context: *mut AuContext) {
    if !context.is_null() {
        nslog(b"SOTF gpui_au_destroy: cleaning up");
        crate::window::unregister_au_window();
        unsafe {
            drop(Box::from_raw(context));
        }
        nslog(b"SOTF gpui_au_destroy: done");
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

/// Request one frame of GPUI rendering.
/// Call from a timer/CVDisplayLink callback on the main thread.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_request_frame(context: *mut AuContext) {
    if context.is_null() {
        return;
    }
    with_au_window(|window| {
        let cb = window.request_frame_callback.borrow_mut().take();
        if let Some(mut cb) = cb {
            cb(RequestFrameOptions::default());
            window.request_frame_callback.borrow_mut().replace(cb);
        }
    });
}

/// Handle view resize from the host.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_resize(context: *mut AuContext, width: f32, height: f32, scale: f32) {
    if context.is_null() {
        return;
    }
    with_au_window(|window| {
        window.handle_resize(width, height, scale);
    });
}

// ── Mouse Events ──────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_mouse_down(
    context: *mut AuContext,
    x: f32,
    y: f32,
    button: i32,
    click_count: i32,
) {
    if context.is_null() {
        return;
    }
    let mouse_button = match button {
        1 => MouseButton::Right,
        2 => MouseButton::Middle,
        _ => MouseButton::Left,
    };
    dispatch_to_window(PlatformInput::MouseDown(gpui::MouseDownEvent {
        button: mouse_button,
        position: point(px(x), px(y)),
        modifiers: gpui::Modifiers::default(),
        click_count: click_count as usize,
        first_mouse: false,
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_mouse_up(context: *mut AuContext, x: f32, y: f32, button: i32) {
    if context.is_null() {
        return;
    }
    let mouse_button = match button {
        1 => MouseButton::Right,
        2 => MouseButton::Middle,
        _ => MouseButton::Left,
    };
    dispatch_to_window(PlatformInput::MouseUp(gpui::MouseUpEvent {
        button: mouse_button,
        position: point(px(x), px(y)),
        modifiers: gpui::Modifiers::default(),
        click_count: 1,
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_mouse_moved(context: *mut AuContext, x: f32, y: f32) {
    if context.is_null() {
        return;
    }
    dispatch_to_window(PlatformInput::MouseMove(gpui::MouseMoveEvent {
        position: point(px(x), px(y)),
        pressed_button: None,
        modifiers: gpui::Modifiers::default(),
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_mouse_dragged(context: *mut AuContext, x: f32, y: f32, button: i32) {
    if context.is_null() {
        return;
    }
    let mouse_button = match button {
        1 => MouseButton::Right,
        2 => MouseButton::Middle,
        _ => MouseButton::Left,
    };
    dispatch_to_window(PlatformInput::MouseMove(gpui::MouseMoveEvent {
        position: point(px(x), px(y)),
        pressed_button: Some(mouse_button),
        modifiers: gpui::Modifiers::default(),
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_scroll_wheel(context: *mut AuContext, x: f32, y: f32, dx: f32, dy: f32) {
    if context.is_null() {
        return;
    }
    dispatch_to_window(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
        position: point(px(x), px(y)),
        delta: gpui::ScrollDelta::Pixels(point(px(dx), px(dy))),
        modifiers: gpui::Modifiers::default(),
        touch_phase: gpui::TouchPhase::Moved,
    }));
}

fn dispatch_to_window(event: PlatformInput) {
    with_au_window(|window| {
        window.dispatch_input(event);
    });
}
