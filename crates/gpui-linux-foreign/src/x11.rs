//! X11 foreign-parent binding: GPUI editors in host windows (VST3, CLAP-x11).
//!
//! Attach hands the host XID, logical size, and backing scale to
//! [`linux_x11_create_with`]; the fork backend creates one undecorated
//! child at parent origin (0, 0) and selects input on it, so pointer,
//! keyboard (via XIM), and `ConfigureNotify` arrive over the guest's own
//! X connection without host forwarding. The `mouse_*`/`scroll`/`key`
//! entries exist for hosts that prefer forwarding raw events instead.
//!
//! [`linux_x11_create_with`]: crate::x11::linux_x11_create_with

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, KeyDownEvent, KeyUpEvent, Keystroke,
    MouseDownEvent, MouseMoveEvent, PlatformInput, Render, ScrollDelta, ScrollWheelEvent,
    Styled as _, Window, WindowOptions, div, point, px,
};
use gpui_foreign_parent::{FrameThrottle, ResizeDebounce, TransitionCell};
use gpui_linux::foreign_parent::{
    ForeignParentPlatform, X11ForeignParent, clear_pending_x11_foreign_parent,
    stash_pending_x11_foreign_parent,
};

use crate::editor::{
    ForeignBackend, LinuxForeignContext, LinuxForeignEditor, forward_input, pin_app_cell,
    register_editor, unpin_app_cell, unregister_editor, with_editor,
};
use crate::geometry::ForeignBounds;
use crate::input::{
    next_click_count, resolve_key_name, x11_button_to_gpui, x11_state_to_modifiers,
};

/// Neutral placeholder root for the C entry point.
///
/// The C ABI cannot carry a Rust factory, so `gpui_linux_x11_create`
/// renders an empty root. Hosts that need real editor content inject it
/// through [`linux_x11_create_with`].
struct PlaceholderRootView;

impl Render for PlaceholderRootView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

/// Create a GPUI context embedded in a host X11 window, with content.
///
/// Rust-native constructor behind `gpui_linux_x11_create`: plugin crates
/// call this with their own root-view factory instead of going through the
/// C ABI. Returns null when the host record is invalid, no X display is
/// reachable, or the window fails to open. All calls must happen on the
/// host UI thread.
pub fn linux_x11_create_with<V: Render + 'static>(
    parent: u32,
    width: u32,
    height: u32,
    scale: f32,
    label: &str,
    build_root_view: impl FnOnce(&mut Window, &mut App) -> Entity<V> + 'static,
) -> *mut LinuxForeignContext {
    let (Some(host), Some(bounds)) = (
        X11ForeignParent::new(parent, width, height, scale),
        ForeignBounds::new(width, height, scale),
    ) else {
        return std::ptr::null_mut();
    };
    let platform = match ForeignParentPlatform::new_x11() {
        Ok(platform) => Rc::new(platform),
        Err(_) => return std::ptr::null_mut(),
    };

    // Stash the host record so `open_window` takes it one-shot during
    // `app.run()`.
    stash_pending_x11_foreign_parent(host);
    let app = gpui::Application::with_platform(platform.clone());
    // The foreign `run()` invokes the launch callback synchronously, so
    // pin the app cell for the lifetime of the editor.
    let app_cell = app.clone_app_cell();

    let opened = Rc::new(Cell::new(None));
    let opened_capture = Rc::clone(&opened);
    let create_label = label.to_owned();
    app.run(move |cx: &mut App| {
        match cx.open_window(
            WindowOptions {
                window_bounds: None,
                ..Default::default()
            },
            build_root_view,
        ) {
            // `WindowHandle` derefs to its `AnyWindowHandle`.
            Ok(handle) => opened_capture.set(Some(*handle)),
            Err(err) => {
                eprintln!("gpui-linux-foreign: X11 open_window failed for {create_label}: {err:#}");
            }
        }
    });

    let Some(window) = opened.get() else {
        clear_pending_x11_foreign_parent();
        return std::ptr::null_mut();
    };

    let active = TransitionCell::new(false);
    let hovered = TransitionCell::new(false);
    let platform_capture = Rc::clone(&platform);
    active.on_change(move |value| platform_capture.set_foreign_active(window, value));
    let platform_capture = Rc::clone(&platform);
    hovered.on_change(move |value| platform_capture.set_foreign_hovered(window, value));

    let editor = LinuxForeignEditor {
        platform,
        window,
        backend: ForeignBackend::X11,
        label: label.to_owned(),
        bounds: Cell::new(bounds),
        throttle: FrameThrottle::default(),
        debounce: ResizeDebounce::default(),
        dropped_frames: AtomicUsize::new(0),
        active,
        hovered,
        last_click_ms: Cell::new(0),
        last_click_button: Cell::new(None),
        last_click_at: Cell::new((0.0, 0.0)),
        click_count: Cell::new(0),
    };
    // Ownership moves to the owned registry until `linux_x11_destroy`.
    let Some(editor_id) = register_editor(editor) else {
        return std::ptr::null_mut();
    };
    pin_app_cell(editor_id, app_cell);

    Box::into_raw(Box::new(LinuxForeignContext::new(editor_id)))
}

/// Destroy an X11 foreign editor. Unknown ids are ignored. Unregisters
/// the instance, drops GPU resources with the app state, then drops the
/// context — late host calls resolve to "no instance" and no-op.
pub fn linux_x11_destroy(editor_id: u64) {
    let describe = crate::editor::describe_editor(editor_id);
    unregister_editor(editor_id);
    if !unpin_app_cell(editor_id)
        && let Some((backend, label)) = describe
    {
        eprintln!(
            "gpui-linux-foreign: {backend:?} editor {editor_id} ({label}) destroyed on a different thread than its create; app state leaked"
        );
    }
}

/// Request one frame. Call from the host frame timer (VST3 `IRunLoop` or
/// equivalent): pumps native events, converges any debounced resize, then
/// forwards one throttled frame request — overdrive is coalesced, and
/// pumps while windowless are dropped and counted.
pub fn linux_x11_request_frame(editor_id: u64) {
    with_editor(editor_id, |editor| {
        editor.platform.pump_events();
        if !editor.platform.has_window(editor.window) {
            editor.dropped_frames.fetch_add(1, Ordering::Relaxed);
            return;
        }
        // Converge any debounced resize before drawing.
        editor.debounce.poll(|(width, height, scale)| {
            editor
                .platform
                .resize_foreign(editor.window, width, height, scale);
            true
        });
        if !editor.throttle.should_forward() {
            return;
        }
        editor.platform.request_frame(editor.window);
    });
}

/// Host-driven resize. Invalid geometry is ignored; bounds track the
/// newest host size immediately while the surface reconfigure is debounced.
pub fn linux_x11_resize(editor_id: u64, width: u32, height: u32, scale: f32) {
    let Some(bounds) = ForeignBounds::new(width, height, scale) else {
        return;
    };
    with_editor(editor_id, |editor| {
        editor.bounds.set(bounds);
        editor
            .debounce
            .submit((width, height, scale), |(width, height, scale)| {
                editor
                    .platform
                    .resize_foreign(editor.window, width, height, scale);
                true
            });
    });
}

/// Report host focus changes; transitions only reach GPUI.
pub fn linux_x11_set_active(editor_id: u64, active: bool) {
    with_editor(editor_id, |editor| {
        editor.active.update(active);
    });
}

/// Report host pointer enter/leave; transitions only reach GPUI.
pub fn linux_x11_set_hovered(editor_id: u64, hovered: bool) {
    with_editor(editor_id, |editor| {
        editor.hovered.update(hovered);
    });
}

/// Forwarded pointer motion in logical pixels plus the X11 state mask.
pub fn linux_x11_mouse_move(editor_id: u64, x: f32, y: f32, state_mask: u16) {
    if !x.is_finite() || !y.is_finite() {
        return;
    }
    forward_input(
        editor_id,
        PlatformInput::MouseMove(MouseMoveEvent {
            position: point(px(x), px(y)),
            pressed_button: None,
            modifiers: x11_state_to_modifiers(state_mask),
        }),
    );
}

/// Forwarded button press/release: X11 detail, logical position, state
/// mask, and a millisecond clock for click counts.
pub fn linux_x11_mouse_button(
    editor_id: u64,
    detail: u32,
    pressed: bool,
    x: f32,
    y: f32,
    state_mask: u16,
    now_ms: u64,
) {
    if !x.is_finite() || !y.is_finite() {
        return;
    }
    with_editor(editor_id, |editor| {
        let button = x11_button_to_gpui(detail);
        let modifiers = x11_state_to_modifiers(state_mask);
        let position = point(px(x), px(y));
        if pressed {
            let (last_x, last_y) = editor.last_click_at.get();
            let distance = ((x - last_x).powi(2) + (y - last_y).powi(2)).sqrt();
            let count = next_click_count(
                editor.click_count.get(),
                editor.last_click_button.get(),
                button,
                now_ms.saturating_sub(editor.last_click_ms.get()),
                distance,
            );
            editor.click_count.set(count);
            editor.last_click_button.set(Some(button));
            editor.last_click_ms.set(now_ms);
            editor.last_click_at.set((x, y));
            forward_input(
                editor_id,
                PlatformInput::MouseDown(MouseDownEvent {
                    button,
                    position,
                    modifiers,
                    click_count: count,
                    first_mouse: false,
                }),
            );
        } else {
            forward_input(
                editor_id,
                PlatformInput::MouseUp(gpui::MouseUpEvent {
                    button,
                    position,
                    modifiers,
                    click_count: editor.click_count.get(),
                }),
            );
        }
    });
}

/// Forwarded scroll in lines plus the X11 state mask.
pub fn linux_x11_scroll(
    editor_id: u64,
    x: f32,
    y: f32,
    delta_x: f32,
    delta_y: f32,
    state_mask: u16,
) {
    if !x.is_finite() || !y.is_finite() || !delta_x.is_finite() || !delta_y.is_finite() {
        return;
    }
    forward_input(
        editor_id,
        PlatformInput::ScrollWheel(ScrollWheelEvent {
            position: point(px(x), px(y)),
            delta: ScrollDelta::Lines(point(delta_x, delta_y)),
            modifiers: x11_state_to_modifiers(state_mask),
            touch_phase: gpui::TouchPhase::Moved,
        }),
    );
}

/// Forwarded key press/release: X11 keysym, optional commit text, and the
/// X11 state mask. Repeats are held presses (`is_repeat`).
pub fn linux_x11_key(
    editor_id: u64,
    keysym: u32,
    pressed: bool,
    is_repeat: bool,
    text: Option<&str>,
    state_mask: u16,
) {
    let modifiers = x11_state_to_modifiers(state_mask);
    let key = resolve_key_name(keysym, text);
    let key_char = if pressed {
        text.map(str::to_owned)
    } else {
        None
    };
    if pressed {
        forward_input(
            editor_id,
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke {
                    modifiers,
                    key,
                    key_char,
                },
                is_held: is_repeat,
                prefer_character_input: false,
            }),
        );
    } else {
        forward_input(
            editor_id,
            PlatformInput::KeyUp(KeyUpEvent {
                keystroke: Keystroke {
                    modifiers,
                    key,
                    key_char: None,
                },
            }),
        );
    }
}

/// Placeholder-backed create for the C entry point (see [`linux_x11_create_with`]).
pub(crate) fn linux_x11_create_placeholder(
    parent: u32,
    width: u32,
    height: u32,
    scale: f32,
    label: &str,
) -> *mut LinuxForeignContext {
    linux_x11_create_with(parent, width, height, scale, label, |_, cx| {
        cx.new(|_| PlaceholderRootView)
    })
}
