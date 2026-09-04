//! C-compatible FFI functions for embedding GPUI in macOS Audio Unit ViewControllers.

use crate::helpers::{nslog, nslog_verbose};
use crate::window::{PENDING_VIEW, PendingViewInfo, with_au_window};
use gpui::{
    App, AppCell, AppContext, Context, ElementId, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, PlatformInput, Render, SharedString,
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
    // Keep the app state alive after Application::run returns because
    // AuPlatform::run invokes the launch callback synchronously.
    app.clone_app_cell()
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
    nslog_verbose(b"SOTF gpui_au_create: entry");

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
    nslog_verbose(msg.as_bytes());

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

    nslog_verbose(b"SOTF gpui_au_create: creating GPUI Application");
    let platform = Rc::new(crate::AuPlatform::new());
    let app = gpui::Application::with_platform(platform);

    let app_cell = clone_application_cell(&app);
    nslog_verbose(b"SOTF gpui_au_create: Rc<AppCell> cloned for lifetime management");

    let window_opened = std::rc::Rc::new(std::cell::Cell::new(false));
    let window_opened_clone = window_opened.clone();
    let pt = plugin_type_str.clone();
    app.run(move |cx: &mut App| {
        nslog_verbose(b"SOTF gpui_au_create: inside app.run callback");
        match cx.open_window(
            WindowOptions {
                window_bounds: None,
                ..Default::default()
            },
            |_window, cx| cx.new(|_| AuRootView::new(pt)),
        ) {
            Ok(_handle) => {
                nslog_verbose(b"SOTF gpui_au_create: window opened OK");
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

    nslog_verbose(b"SOTF gpui_au_create: app.run() returned, context ready");

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
        nslog_verbose(b"SOTF gpui_au_destroy: cleaning up");
        crate::window::unregister_au_window();
        unsafe {
            drop(Box::from_raw(context));
        }
        nslog_verbose(b"SOTF gpui_au_destroy: done");
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
    with_au_window(|window| window.request_frame());
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

/// Report AU host focus changes for GPUI window activation state.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_set_active(context: *mut AuContext, is_active: bool) {
    if context.is_null() {
        return;
    }
    with_au_window(|window| window.update_active_status(is_active));
}

/// Report whether the pointer is currently inside the AU view.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_set_hovered(context: *mut AuContext, is_hovered: bool) {
    if context.is_null() {
        return;
    }
    with_au_window(|window| window.update_hover_status(is_hovered));
}

// ── Mouse Events ──────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_mouse_down(
    context: *mut AuContext,
    x: f32,
    y: f32,
    button: i32,
    click_count: i32,
    modifier_flags: u32,
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
        modifiers: modifiers_from_ns_event(modifier_flags),
        click_count: click_count as usize,
        first_mouse: false,
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_mouse_up(
    context: *mut AuContext,
    x: f32,
    y: f32,
    button: i32,
    modifier_flags: u32,
) {
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
        modifiers: modifiers_from_ns_event(modifier_flags),
        click_count: 1,
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_mouse_moved(
    context: *mut AuContext,
    x: f32,
    y: f32,
    modifier_flags: u32,
) {
    if context.is_null() {
        return;
    }
    dispatch_to_window(PlatformInput::MouseMove(gpui::MouseMoveEvent {
        position: point(px(x), px(y)),
        pressed_button: None,
        modifiers: modifiers_from_ns_event(modifier_flags),
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_mouse_dragged(
    context: *mut AuContext,
    x: f32,
    y: f32,
    button: i32,
    modifier_flags: u32,
) {
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
        modifiers: modifiers_from_ns_event(modifier_flags),
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_scroll_wheel(
    context: *mut AuContext,
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    modifier_flags: u32,
) {
    if context.is_null() {
        return;
    }
    dispatch_to_window(PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
        position: point(px(x), px(y)),
        delta: gpui::ScrollDelta::Pixels(point(px(dx), px(dy))),
        modifiers: modifiers_from_ns_event(modifier_flags),
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

fn mac_key_code_to_key(key_code: u16) -> Option<&'static str> {
    match key_code {
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
    }
}

fn optional_c_string(value: *const c_char) -> Option<String> {
    if value.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(value).to_str().ok().map(str::to_owned) }
    }
}

fn key_event(
    key_code: u16,
    characters: *const c_char,
    characters_ignoring_modifiers: *const c_char,
    modifier_flags: u32,
) -> gpui::Keystroke {
    let named_key = mac_key_code_to_key(key_code);
    let characters = named_key
        .is_none()
        .then(|| optional_c_string(characters))
        .flatten();
    let characters_ignoring_modifiers = named_key
        .is_none()
        .then(|| optional_c_string(characters_ignoring_modifiers))
        .flatten();
    let key = named_key
        .map(str::to_owned)
        .or_else(|| {
            characters_ignoring_modifiers
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(str::to_lowercase)
        })
        // Unknown hardware codes are not useful binding identifiers. Keep a
        // stable fallback and avoid formatting a fresh diagnostic string on
        // every repeat event.
        .unwrap_or_else(|| "unknown".to_owned());
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
    characters_ignoring_modifiers: *const c_char,
    modifier_flags: u32,
    is_repeat: bool,
) {
    if context.is_null() {
        return;
    }
    dispatch_to_window(PlatformInput::KeyDown(gpui::KeyDownEvent {
        keystroke: key_event(
            key_code,
            characters,
            characters_ignoring_modifiers,
            modifier_flags,
        ),
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
    characters_ignoring_modifiers: *const c_char,
    modifier_flags: u32,
) {
    if context.is_null() {
        return;
    }
    dispatch_to_window(PlatformInput::KeyUp(gpui::KeyUpEvent {
        keystroke: key_event(
            key_code,
            characters,
            characters_ignoring_modifiers,
            modifier_flags,
        ),
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

// ── Parameters & State ─────────────────────────────────────────────────────
// Minimal `AUParameterTree` / `fullState` bridge: the plugin window owns an
// `AuParameterTree` (gain + bypass placeholders; hosts extend it), and these
// entry points expose count/get/set/register plus versioned state save/load
// to Swift.

/// Number of parameters in the plugin's parameter tree.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_parameter_count(context: *mut AuContext) -> usize {
    if context.is_null() {
        return 0;
    }
    with_au_window(|window| window.parameter_count()).unwrap_or(0)
}

/// Current value of a parameter; `ok` (when non-null) reports id lookup.
/// Unknown ids (or a null context) yield `0.0` with `ok` set to false.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_parameter_value(context: *mut AuContext, id: u32, ok: *mut bool) -> f32 {
    let value = if context.is_null() {
        None
    } else {
        with_au_window(|window| window.parameter_value(id)).flatten()
    };
    if !ok.is_null() {
        unsafe {
            *ok = value.is_some();
        }
    }
    value.unwrap_or(0.0)
}

/// Store a (clamped) parameter value. Returns false for a null context or
/// an unknown id.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_parameter_set(context: *mut AuContext, id: u32, value: f32) -> bool {
    if context.is_null() {
        return false;
    }
    with_au_window(|window| window.set_parameter_value(id, value)).unwrap_or(false)
}

/// Register a host parameter. A null `name` falls back to `"param-{id}"`.
/// Returns false for a null context, duplicate ids, or invalid ranges.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_parameter_register(
    context: *mut AuContext,
    id: u32,
    min_value: f32,
    max_value: f32,
    default_value: f32,
    name: *const c_char,
) -> bool {
    if context.is_null() {
        return false;
    }
    let name = optional_c_string(name).unwrap_or_else(|| format!("param-{id}"));
    with_au_window(|window| {
        window.register_parameter(id, &name, min_value, max_value, default_value)
    })
    .unwrap_or(false)
}

/// Serialize the plugin state (`fullState` analogue).
///
/// When `out` is null or `capacity` is 0, no bytes are written and the
/// required size is returned. Otherwise up to `capacity` bytes are written,
/// `*written` receives the count, and the return value is the total required
/// size (greater than `capacity` means the caller's buffer was too small).
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_save_state(
    context: *mut AuContext,
    out: *mut u8,
    capacity: usize,
    written: *mut usize,
) -> usize {
    let bytes = if context.is_null() {
        Vec::new()
    } else {
        with_au_window(|window| window.capture_plugin_state()).unwrap_or_default()
    };
    let required = bytes.len();
    if !written.is_null() {
        unsafe {
            *written = 0;
        }
    }
    if out.is_null() || capacity == 0 {
        return required;
    }
    let count = required.min(capacity);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, count);
        if !written.is_null() {
            *written = count;
        }
    }
    required
}

/// Realtime health counters: frames dropped on a busy renderer and
/// display-link ticks coalesced by the frame throttle. Each out-pointer is
/// optional; a null context leaves all outputs untouched.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_frame_stats(
    context: *mut AuContext,
    dropped: *mut usize,
    coalesced: *mut usize,
) {
    if context.is_null() {
        return;
    }
    let (dropped_frames, coalesced_frames) =
        with_au_window(|window| (window.dropped_frames(), window.coalesced_frames()))
            .unwrap_or((0, 0));
    if !dropped.is_null() {
        unsafe {
            *dropped = dropped_frames;
        }
    }
    if !coalesced.is_null() {
        unsafe {
            *coalesced = coalesced_frames;
        }
    }
}

/// Restore plugin state previously produced by `gpui_au_save_state`.
/// Returns false for a null context, null data, or a corrupt payload.
///
/// # Safety
/// `data` must point to `len` readable bytes when non-null.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_au_load_state(context: *mut AuContext, data: *const u8, len: usize) -> bool {
    if context.is_null() || data.is_null() {
        return false;
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    with_au_window(|window| window.restore_plugin_state(bytes)).unwrap_or(false)
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
        assert_eq!(mac_key_code_to_key(123), Some("left"));
        assert_eq!(mac_key_code_to_key(0), None);
        assert_eq!(mac_key_code_to_key(999), None);

        let modifiers = modifiers_from_ns_event((1 << 17) | (1 << 20));
        assert!(modifiers.shift);
        assert!(modifiers.platform);
        assert!(!modifiers.control);
    }

    #[test]
    fn modified_characters_keep_the_unmodified_binding_key() {
        let shifted = std::ffi::CString::new("P").unwrap();
        let unmodified = std::ffi::CString::new("p").unwrap();

        let keystroke = key_event(0, shifted.as_ptr(), unmodified.as_ptr(), 1 << 17);

        assert_eq!(keystroke.key, "p");
        assert_eq!(keystroke.key_char.as_deref(), Some("P"));
        assert!(keystroke.modifiers.shift);
    }

    #[test]
    fn exported_host_entry_points_are_null_safe() {
        let context = std::ptr::null_mut();
        gpui_au_destroy(context);
        gpui_au_request_frame(context);
        gpui_au_resize(context, 320.0, 200.0, 2.0);
        gpui_au_set_active(context, false);
        gpui_au_set_hovered(context, false);
        gpui_au_mouse_down(context, 0.0, 0.0, 0, 1, 0);
        gpui_au_mouse_up(context, 0.0, 0.0, 0, 0);
        gpui_au_mouse_moved(context, 0.0, 0.0, 0);
        gpui_au_mouse_dragged(context, 0.0, 0.0, 0, 0);
        gpui_au_scroll_wheel(context, 0.0, 0.0, 0.0, 0.0, 0);
        gpui_au_key_down(context, 0, std::ptr::null(), std::ptr::null(), 0, false);
        gpui_au_key_up(context, 0, std::ptr::null(), std::ptr::null(), 0);
        gpui_au_insert_text(context, std::ptr::null());
        gpui_au_set_marked_text(context, std::ptr::null(), 0, 0);
        gpui_au_unmark_text(context);
        gpui_au_delete_backward(context);
        assert_eq!(gpui_au_parameter_count(context), 0);
        let mut ok = true;
        assert_eq!(gpui_au_parameter_value(context, 0, &mut ok), 0.0);
        assert!(!ok);
        assert_eq!(
            gpui_au_parameter_value(context, 0, std::ptr::null_mut()),
            0.0
        );
        assert!(!gpui_au_parameter_set(context, 0, 0.5));
        assert!(!gpui_au_parameter_register(
            context,
            0,
            0.0,
            1.0,
            0.5,
            std::ptr::null()
        ));
        let mut written = 0usize;
        assert_eq!(
            gpui_au_save_state(context, std::ptr::null_mut(), 0, &mut written),
            0
        );
        assert_eq!(written, 0);
        assert!(!gpui_au_load_state(context, std::ptr::null(), 0));
        let (mut dropped, mut coalesced) = (usize::MAX, usize::MAX);
        gpui_au_frame_stats(context, &mut dropped, &mut coalesced);
        assert_eq!((dropped, coalesced), (usize::MAX, usize::MAX));
        gpui_au_frame_stats(context, std::ptr::null_mut(), std::ptr::null_mut());
    }
}
