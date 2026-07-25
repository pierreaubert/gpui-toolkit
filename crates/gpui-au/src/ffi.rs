//! C-compatible FFI functions for embedding GPUI in macOS Audio Unit ViewControllers.

use crate::helpers::nslog;
use crate::window::{PENDING_VIEW, PendingViewInfo, with_au_window};
use gpui::{
    App, AppCell, AppContext, Context, ElementId, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, PlatformInput, Render, RequestFrameOptions, SharedString,
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
    plugin_label: SharedString,
    click_count: usize,
    click_label: SharedString,
}

impl AuRootView {
    fn new(plugin_type: impl AsRef<str>) -> Self {
        let plugin_label = SharedString::from(format!("SOTF: {}", plugin_type.as_ref()));
        Self {
            plugin_label,
            click_count: 0,
            click_label: SharedString::from("Clicks: 0"),
        }
    }

    fn refresh_click_label(&mut self) {
        self.click_label = SharedString::from(format!("Clicks: {}", self.click_count));
    }
}

impl Render for AuRootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(self.plugin_label.clone()),
            )
            .child(
                div()
                    .id(ElementId::Name("click-target".into()))
                    .mt(px(16.0))
                    .px(px(16.0))
                    .py(px(8.0))
                    .bg(rgb(0x3366ff))
                    .text_color(rgb(0xffffff))
                    .child(self.click_label.clone())
                    .on_click(cx.listener(|this, _event, _window, _cx| {
                        this.click_count += 1;
                        this.refresh_click_label();
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
            |_window, cx| cx.new(|_| AuRootView::new(pt)),
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

fn modifiers_from_ns_event(flags: u32) -> gpui::Modifiers {
    gpui::Modifiers {
        control: flags & (1 << 18) != 0,
        alt: flags & (1 << 19) != 0,
        shift: flags & (1 << 17) != 0,
        platform: flags & (1 << 20) != 0,
        function: flags & (1 << 23) != 0,
    }
}

fn mac_key_code_to_key(key_code: u16, characters: Option<&str>) -> String {
    let named = match key_code {
        36 => Some("enter"),
        48 => Some("tab"),
        49 => Some("space"),
        51 => Some("backspace"),
        53 => Some("escape"),
        115 => Some("home"),
        116 => Some("pageup"),
        117 => Some("delete"),
        119 => Some("end"),
        121 => Some("pagedown"),
        123 => Some("left"),
        124 => Some("right"),
        125 => Some("down"),
        126 => Some("up"),
        _ => None,
    };
    named
        .map(str::to_owned)
        .or_else(|| {
            characters
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("keycode-{key_code}"))
}

fn optional_c_string(value: *const c_char) -> Option<String> {
    if value.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(value).to_str().ok().map(str::to_owned) }
    }
}

fn key_event(key_code: u16, characters: *const c_char, modifier_flags: u32) -> gpui::Keystroke {
    let characters = optional_c_string(characters);
    let key = mac_key_code_to_key(key_code, characters.as_deref());
    let key_char = characters.filter(|value| !value.is_empty() && key.chars().count() == 1);
    gpui::Keystroke {
        modifiers: modifiers_from_ns_event(modifier_flags),
        key,
        key_char,
    }
}

/// Forward an NSEvent keyDown event from the host NSView.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_key_down(
    context: *mut AuContext,
    key_code: u16,
    characters: *const c_char,
    modifier_flags: u32,
    is_repeat: bool,
) {
    if context.is_null() {
        return;
    }
    dispatch_to_window(PlatformInput::KeyDown(gpui::KeyDownEvent {
        keystroke: key_event(key_code, characters, modifier_flags),
        is_held: is_repeat,
        prefer_character_input: false,
    }));
}

/// Forward an NSEvent keyUp event from the host NSView.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_key_up(
    context: *mut AuContext,
    key_code: u16,
    characters: *const c_char,
    modifier_flags: u32,
) {
    if context.is_null() {
        return;
    }
    dispatch_to_window(PlatformInput::KeyUp(gpui::KeyUpEvent {
        keystroke: key_event(key_code, characters, modifier_flags),
    }));
}

/// Commit UTF-8 text from `NSTextInputClient::insertText`.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_insert_text(context: *mut AuContext, text: *const c_char) {
    if context.is_null() {
        return;
    }
    if let Some(text) = optional_c_string(text) {
        with_au_window(|window| window.insert_text(&text));
    }
}

/// Forward an in-progress marked-text composition from AppKit.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_set_marked_text(
    context: *mut AuContext,
    text: *const c_char,
    selected_location: usize,
    selected_length: usize,
) {
    if context.is_null() {
        return;
    }
    if let Some(text) = optional_c_string(text) {
        with_au_window(|window| {
            window.set_marked_text(&text, selected_location, selected_length);
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_unmark_text(context: *mut AuContext) {
    if !context.is_null() {
        with_au_window(|window| window.unmark_text());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_delete_backward(context: *mut AuContext) {
    if !context.is_null() {
        with_au_window(|window| window.delete_backward());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_root_view_renders(cx: &mut TestAppContext) {
        let _window = cx.add_window(|_window, _cx| AuRootView::new("test-plugin"));
    }

    #[test]
    fn test_root_view_caches_labels() {
        let mut view = AuRootView::new("MyPlugin");
        assert_eq!(view.plugin_label.as_ref(), "SOTF: MyPlugin");
        assert_eq!(view.click_label.as_ref(), "Clicks: 0");

        view.click_count = 3;
        view.refresh_click_label();
        assert_eq!(view.click_label.as_ref(), "Clicks: 3");
    }

    #[test]
    fn mac_key_codes_and_modifiers_map_to_gpui_keystrokes() {
        assert_eq!(mac_key_code_to_key(123, None), "left");
        assert_eq!(mac_key_code_to_key(0, Some("a")), "a");
        assert_eq!(mac_key_code_to_key(999, None), "keycode-999");

        let modifiers = modifiers_from_ns_event((1 << 17) | (1 << 20));
        assert!(modifiers.shift);
        assert!(modifiers.platform);
        assert!(!modifiers.control);
    }
}
