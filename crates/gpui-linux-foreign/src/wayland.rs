//! Wayland foreign-parent binding: GPUI editors in host surfaces (CLAP-wayland).
//!
//! Attach hands the host `wl_display*` and parent `wl_surface*` plus the
//! logical size and backing scale to [`linux_wayland_create_with`]; the
//! fork backend adopts the display in guest mode and creates its own
//! desync subsurface, seat, pointer, and keyboard on the host connection,
//! so input arrives from the compositor directly without host forwarding.
//! The `mouse_*`/`scroll`/`key` entries exist for hosts that prefer
//! forwarding raw events instead.
//!
//! [`linux_wayland_create_with`]: crate::wayland::linux_wayland_create_with

use std::cell::Cell;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, KeyDownEvent, KeyUpEvent, Keystroke,
    Modifiers, MouseDownEvent, MouseMoveEvent, PlatformInput, Render, ScrollDelta,
    ScrollWheelEvent, Styled as _, TouchPhase, Window, WindowOptions, div, point, px,
};
use gpui_foreign_parent::{FrameThrottle, ResizeDebounce, TransitionCell};
use gpui_linux::foreign_parent::{
    ForeignParentPlatform, WaylandForeignParent, clear_pending_wayland_foreign_parent,
    stash_pending_wayland_foreign_parent,
};

use crate::editor::{
    ForeignBackend, LinuxForeignContext, LinuxForeignEditor, forward_input, pin_app_cell,
    register_editor, unpin_app_cell, unregister_editor, with_editor,
};
use crate::geometry::ForeignBounds;
use crate::input::{
    modifiers_from_bits, next_click_count, resolve_key_name, wayland_button_to_gpui,
};

/// Neutral placeholder root for the C entry point.
///
/// The C ABI cannot carry a Rust factory, so `gpui_linux_wayland_create`
/// renders an empty root. Hosts that need real editor content inject it
/// through [`linux_wayland_create_with`].
struct PlaceholderRootView;

impl Render for PlaceholderRootView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

/// Create a GPUI context embedded in a host Wayland surface, with content.
///
/// Rust-native constructor behind `gpui_linux_wayland_create`: plugin
/// crates call this with their own root-view factory instead of going
/// through the C ABI. Returns null when the host record is invalid, a
/// required Wayland global is missing, or the window fails to open. All
/// calls must happen on the host UI thread.
///
/// # Safety
///
/// `display` must be null or a live `wl_display*`, and `parent_surface`
/// must be null or a live `wl_surface*` on that display; both must outlive
/// the returned context.
pub unsafe fn linux_wayland_create_with<V: Render + 'static>(
    display: *mut c_void,
    parent_surface: *mut c_void,
    width: u32,
    height: u32,
    scale: f32,
    label: &str,
    build_root_view: impl FnOnce(&mut Window, &mut App) -> Entity<V> + 'static,
) -> *mut LinuxForeignContext {
    // SAFETY: null-checked here; the caller guarantees the liveness documented above.
    let (Some(host), Some(bounds)) = (
        unsafe { WaylandForeignParent::new(display, parent_surface, width, height, scale) },
        ForeignBounds::new(width, height, scale),
    ) else {
        return std::ptr::null_mut();
    };
    let platform = match ForeignParentPlatform::new_wayland(&host) {
        Ok(platform) => Rc::new(platform),
        Err(_) => return std::ptr::null_mut(),
    };

    // Stash the host record so `open_window` takes it one-shot during
    // `app.run()`.
    stash_pending_wayland_foreign_parent(host);
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
                eprintln!(
                    "gpui-linux-foreign: Wayland open_window failed for {create_label}: {err:#}"
                );
            }
        }
    });

    let Some(window) = opened.get() else {
        clear_pending_wayland_foreign_parent();
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
        backend: ForeignBackend::Wayland,
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
    // Ownership moves to the owned registry until `linux_wayland_destroy`.
    let Some(editor_id) = register_editor(editor) else {
        return std::ptr::null_mut();
    };
    pin_app_cell(editor_id, app_cell);

    Box::into_raw(Box::new(LinuxForeignContext::new(editor_id)))
}

/// Destroy a Wayland foreign editor. Unknown ids are ignored. Unregisters
/// the instance, drops GPU resources with the app state, then drops the
/// context — late host calls resolve to "no instance" and no-op.
pub fn linux_wayland_destroy(editor_id: u64) {
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

/// Request one frame. Call from the host frame timer: pumps native events,
/// converges any debounced resize, then forwards one throttled frame
/// request — overdrive is coalesced, and pumps while windowless are
/// dropped and counted.
pub fn linux_wayland_request_frame(editor_id: u64) {
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
pub fn linux_wayland_resize(editor_id: u64, width: u32, height: u32, scale: f32) {
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
pub fn linux_wayland_set_active(editor_id: u64, active: bool) {
    with_editor(editor_id, |editor| {
        editor.active.update(active);
    });
}

/// Report host pointer enter/leave; transitions only reach GPUI.
pub fn linux_wayland_set_hovered(editor_id: u64, hovered: bool) {
    with_editor(editor_id, |editor| {
        editor.hovered.update(hovered);
    });
}

/// Forwarded pointer motion in logical pixels plus modifier bits.
pub fn linux_wayland_mouse_move(editor_id: u64, x: f32, y: f32, modifiers: Modifiers) {
    if !x.is_finite() || !y.is_finite() {
        return;
    }
    forward_input(
        editor_id,
        PlatformInput::MouseMove(MouseMoveEvent {
            position: point(px(x), px(y)),
            pressed_button: None,
            modifiers,
        }),
    );
}

/// Forwarded button press/release: evdev code, logical position, modifier
/// bits, and a millisecond clock for click counts.
pub fn linux_wayland_mouse_button(
    editor_id: u64,
    button_code: u32,
    pressed: bool,
    x: f32,
    y: f32,
    modifiers: Modifiers,
    now_ms: u64,
) {
    if !x.is_finite() || !y.is_finite() {
        return;
    }
    with_editor(editor_id, |editor| {
        let button = wayland_button_to_gpui(button_code);
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

/// Forwarded scroll in lines plus modifier bits.
pub fn linux_wayland_scroll(
    editor_id: u64,
    x: f32,
    y: f32,
    delta_x: f32,
    delta_y: f32,
    modifiers: Modifiers,
) {
    if !x.is_finite() || !y.is_finite() || !delta_x.is_finite() || !delta_y.is_finite() {
        return;
    }
    forward_input(
        editor_id,
        PlatformInput::ScrollWheel(ScrollWheelEvent {
            position: point(px(x), px(y)),
            delta: ScrollDelta::Lines(point(delta_x, delta_y)),
            modifiers,
            touch_phase: TouchPhase::Moved,
        }),
    );
}

/// Forwarded key press/release: XKB keysym, optional commit text, and
/// modifier bits. Repeats are held presses (`is_repeat`).
pub fn linux_wayland_key(
    editor_id: u64,
    keysym: u32,
    pressed: bool,
    is_repeat: bool,
    text: Option<&str>,
    modifiers: Modifiers,
) {
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

/// Decode C-ABI modifier bits for the forwarded-input entries.
pub fn linux_wayland_modifiers(bits: u8) -> Modifiers {
    modifiers_from_bits(bits)
}

/// Placeholder-backed create for the C entry point (see [`linux_wayland_create_with`]).
///
/// # Safety
///
/// See [`linux_wayland_create_with`].
pub(crate) unsafe fn linux_wayland_create_placeholder(
    display: *mut c_void,
    parent_surface: *mut c_void,
    width: u32,
    height: u32,
    scale: f32,
    label: &str,
) -> *mut LinuxForeignContext {
    // SAFETY: forwarded from the caller contract (see the function docs).
    unsafe {
        linux_wayland_create_with(
            display,
            parent_surface,
            width,
            height,
            scale,
            label,
            |_, cx| cx.new(|_| PlaceholderRootView),
        )
    }
}
