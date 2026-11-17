//! C ABI for host-owned Linux editors (contract §4).
//!
//! One entry point per lifecycle transition, each idempotent and
//! null-safe: `create` returns null on failure, `destroy` on null is a
//! no-op, and stats leave out-pointers untouched for null contexts.
//!
//! # Host notes
//!
//! ## X11 (`gpui_linux_x11_*`; VST3, CLAP-x11)
//!
//! - Attach: pass the host window XID, the logical size, the backing
//!   scale, and a label. The guest creates one undecorated child at the
//!   parent origin and selects input on it — pointer, keyboard (XIM), and
//!   `ConfigureNotify` arrive over the guest connection, so most hosts
//!   need no `mouse_*`/`key` forwarding (those entries exist for hosts
//!   that capture input and prefer forwarding).
//! - Pump: call `request_frame` from a frame timer (VST3 `IRunLoop` or
//!   equivalent). Ticks faster than 4 ms coalesce; count them with
//!   `frame_stats` to distinguish idle from starved.
//! - Resize: call `resize` with the newest logical size and scale. Bounds
//!   track immediately; the surface reconfigures within ~16 ms. The guest
//!   also honors host-initiated `ConfigureNotify` on the child.
//! - Focus/hover: `set_active`/`set_hovered` forward on transition only.
//! - Detach: close the opaque token. Repeated/stale calls are harmless. Keep
//!   the host parent alive through callbacks already in progress.
//!
//! ## Wayland (`gpui_linux_wayland_*`; CLAP-wayland)
//!
//! - Attach: pass the host `wl_display*` and parent `wl_surface*` (both
//!   live for the editor lifetime), the logical size, backing scale, and a
//!   label. The guest adopts the display in guest mode (never closed) and
//!   creates a desync subsurface plus its own seat/pointer/keyboard, so
//!   input arrives from the compositor directly. Keep dispatching the host
//!   queue as usual; attach/detach/pump all run on the host UI thread.
//! - Pump, resize, focus/hover, and detach match the X11 notes above.
//! - The host owns geometry: compositor scale advisories are ignored in
//!   favor of the `resize` scale. Fractional scales use `wp_viewporter`
//!   when the compositor offers it.

// Rust guideline compliant 2026-02-21

use std::ffi::{c_char, c_void};

use crate::lifecycle::context_handle;
use gpui_foreign_parent::{ffi_boundary, optional_c_string};

use crate::editor::{
    ForeignBackend, LinuxForeignContext, frame_stats, permits_create, with_editor,
};
use crate::wayland::{
    linux_wayland_create_placeholder, linux_wayland_destroy, linux_wayland_key,
    linux_wayland_modifiers, linux_wayland_mouse_button, linux_wayland_mouse_move,
    linux_wayland_request_frame, linux_wayland_resize, linux_wayland_scroll,
    linux_wayland_set_active, linux_wayland_set_hovered,
};
use crate::x11::{
    linux_x11_create_placeholder, linux_x11_destroy, linux_x11_key, linux_x11_mouse_button,
    linux_x11_mouse_move, linux_x11_request_frame, linux_x11_resize, linux_x11_scroll,
    linux_x11_set_active, linux_x11_set_hovered,
};

/// Decodes an opaque numeric token without touching caller memory.
fn editor_id(context: *mut LinuxForeignContext) -> Option<u64> {
    u64::try_from(context_handle(context)?.as_usize()).ok()
}

/// Copies a label out of caller-owned C memory.
///
/// # Safety
///
/// Label must be null or a readable NUL-terminated C string.
unsafe fn label_text(label: *const c_char) -> String {
    // SAFETY: the caller guarantees null or a valid NUL-terminated string.
    unsafe { optional_c_string(label) }.unwrap_or_default()
}

// ── X11 ──────────────────────────────────────────────────────────────────────

/// Create an editor in a host X11 window. Null on invalid input or when no
/// X display is reachable. Renders the neutral placeholder; Rust hosts
/// inject content through `linux_x11_create_with`.
///
/// # Safety
///
/// `label` must be null or point to a valid NUL-terminated C string. Call
/// on the host UI thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_linux_x11_create(
    parent: u32,
    width: u32,
    height: u32,
    scale: f32,
    label: *const c_char,
) -> *mut LinuxForeignContext {
    ffi_boundary(std::ptr::null_mut(), || {
        if !permits_create() {
            return std::ptr::null_mut();
        }
        // SAFETY: null or a readable NUL-terminated label by the export contract.
        let label = unsafe { label_text(label) };
        linux_x11_create_placeholder(parent, width, height, scale, &label)
    })
}

/// Closes this backend's opaque editor token; active callbacks retain its resources.
///
/// Null, stale, wrong-backend and wrong-thread tokens do nothing. The host parent
/// must outlive callbacks already in progress when close is requested.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_x11_destroy(context: *mut LinuxForeignContext) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_x11_destroy(editor_id);
        }
    });
}

/// Pump native events and forward one throttled frame request.
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_x11_request_frame(context: *mut LinuxForeignContext) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_x11_request_frame(editor_id);
        }
    });
}

/// Host-driven resize in logical pixels plus backing scale.
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_x11_resize(
    context: *mut LinuxForeignContext,
    width: u32,
    height: u32,
    scale: f32,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_x11_resize(editor_id, width, height, scale);
        }
    });
}

/// Report host focus changes (`active` != 0 means focused).
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_x11_set_active(context: *mut LinuxForeignContext, active: bool) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_x11_set_active(editor_id, active);
        }
    });
}

/// Report host pointer enter/leave (`hovered` != 0 means inside).
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_x11_set_hovered(context: *mut LinuxForeignContext, hovered: bool) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_x11_set_hovered(editor_id, hovered);
        }
    });
}

/// Forwarded pointer motion: logical position plus the X11 state mask.
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_x11_mouse_move(
    context: *mut LinuxForeignContext,
    x: f32,
    y: f32,
    state_mask: u16,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_x11_mouse_move(editor_id, x, y, state_mask);
        }
    });
}

/// Forwarded button press/release: X11 detail (1/2/3/8/9), logical
/// position, and the X11 state mask.
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_x11_mouse_button(
    context: *mut LinuxForeignContext,
    detail: u32,
    pressed: bool,
    x: f32,
    y: f32,
    state_mask: u16,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_x11_mouse_button(
                editor_id,
                detail,
                pressed,
                x,
                y,
                state_mask,
                crate::editor::boot_ms(),
            );
        }
    });
}

/// Forwarded scroll in lines at a logical position, plus the X11 state mask.
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_x11_scroll(
    context: *mut LinuxForeignContext,
    x: f32,
    y: f32,
    delta_x: f32,
    delta_y: f32,
    state_mask: u16,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_x11_scroll(editor_id, x, y, delta_x, delta_y, state_mask);
        }
    });
}

/// Forwarded key press/release: X11 keysym, optional commit text, and the
/// X11 state mask. `is_repeat` marks held presses.
///
/// # Safety
///
/// Non-null `text` must point to a readable NUL-terminated C string for the call.
/// Context is an opaque token, never dereferenced.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_linux_x11_key(
    context: *mut LinuxForeignContext,
    keysym: u32,
    pressed: bool,
    is_repeat: bool,
    text: *const c_char,
    state_mask: u16,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            with_editor(editor_id, ForeignBackend::X11, |_| {
                // SAFETY: null or a valid string by contract; copied out immediately.
                let text = unsafe { optional_c_string(text) };
                linux_x11_key(
                    editor_id,
                    keysym,
                    pressed,
                    is_repeat,
                    text.as_deref(),
                    state_mask,
                );
            });
        }
    });
}

/// Health counters: frames dropped while windowless and host ticks
/// coalesced by the frame throttle. A null context leaves outputs
/// untouched.
///
/// # Safety
///
/// Non-null output pointers must be aligned and writable for one usize without
/// violating Rust aliasing rules. Unknown, stale, wrong-backend and wrong-thread
/// contexts leave outputs untouched.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_linux_x11_frame_stats(
    context: *mut LinuxForeignContext,
    dropped: *mut usize,
    coalesced: *mut usize,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            let Some((dropped_frames, coalesced_frames)) =
                frame_stats(editor_id, ForeignBackend::X11)
            else {
                return;
            };
            if !dropped.is_null() {
                // SAFETY: null-checked; the caller guarantees a writable out count.
                unsafe {
                    *dropped = dropped_frames;
                }
            }
            if !coalesced.is_null() {
                // SAFETY: null-checked; the caller guarantees a writable out count.
                unsafe {
                    *coalesced = coalesced_frames;
                }
            }
        }
    });
}

// ── Wayland ──────────────────────────────────────────────────────────────────

/// Create an editor in a host Wayland surface. Null on invalid input or
/// when a required global is missing. Renders the neutral placeholder;
/// Rust hosts inject content through `linux_wayland_create_with`.
///
/// # Safety
///
/// `display`/`parent_surface` must be null or live host objects that
/// outlive the editor and callbacks active at close; `label` must be null or a valid NUL-terminated C
/// string. Call on the host UI thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_linux_wayland_create(
    display: *mut c_void,
    parent_surface: *mut c_void,
    width: u32,
    height: u32,
    scale: f32,
    label: *const c_char,
) -> *mut LinuxForeignContext {
    ffi_boundary(std::ptr::null_mut(), || {
        if !permits_create() {
            return std::ptr::null_mut();
        }
        // SAFETY: null or a readable NUL-terminated label by the export contract.
        let label = unsafe { label_text(label) };
        // SAFETY: forwarded from the caller contract above.
        unsafe {
            linux_wayland_create_placeholder(display, parent_surface, width, height, scale, &label)
        }
    })
}

/// Closes this backend's opaque editor token; active callbacks retain its resources.
///
/// Null, stale, wrong-backend and wrong-thread tokens do nothing. The host parent
/// must outlive callbacks already in progress when close is requested.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_wayland_destroy(context: *mut LinuxForeignContext) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_wayland_destroy(editor_id);
        }
    });
}

/// Pump native events and forward one throttled frame request.
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_wayland_request_frame(context: *mut LinuxForeignContext) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_wayland_request_frame(editor_id);
        }
    });
}

/// Host-driven resize in logical pixels plus backing scale.
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_wayland_resize(
    context: *mut LinuxForeignContext,
    width: u32,
    height: u32,
    scale: f32,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_wayland_resize(editor_id, width, height, scale);
        }
    });
}

/// Report host focus changes (`active` != 0 means focused).
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_wayland_set_active(context: *mut LinuxForeignContext, active: bool) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_wayland_set_active(editor_id, active);
        }
    });
}

/// Report host pointer enter/leave (`hovered` != 0 means inside).
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_wayland_set_hovered(context: *mut LinuxForeignContext, hovered: bool) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_wayland_set_hovered(editor_id, hovered);
        }
    });
}

/// Forwarded pointer motion at a logical position plus modifier bits
/// (0x01 shift, 0x02 control, 0x04 alt, 0x08 platform).
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_wayland_mouse_move(
    context: *mut LinuxForeignContext,
    x: f32,
    y: f32,
    modifiers: u8,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_wayland_mouse_move(editor_id, x, y, linux_wayland_modifiers(modifiers));
        }
    });
}

/// Forwarded button press/release: evdev code, logical position, and
/// modifier bits.
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_wayland_mouse_button(
    context: *mut LinuxForeignContext,
    button_code: u32,
    pressed: bool,
    x: f32,
    y: f32,
    modifiers: u8,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_wayland_mouse_button(
                editor_id,
                button_code,
                pressed,
                x,
                y,
                linux_wayland_modifiers(modifiers),
                crate::editor::boot_ms(),
            );
        }
    });
}

/// Forwarded scroll in lines at a logical position, plus modifier bits.
/// Null, stale, wrong-backend and wrong-thread tokens do nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_linux_wayland_scroll(
    context: *mut LinuxForeignContext,
    x: f32,
    y: f32,
    delta_x: f32,
    delta_y: f32,
    modifiers: u8,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            linux_wayland_scroll(
                editor_id,
                x,
                y,
                delta_x,
                delta_y,
                linux_wayland_modifiers(modifiers),
            );
        }
    });
}

/// Forwarded key press/release: XKB keysym, optional commit text, and
/// modifier bits. `is_repeat` marks held presses.
///
/// # Safety
///
/// Non-null `text` must point to a readable NUL-terminated C string for the call.
/// Context is an opaque token, never dereferenced.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_linux_wayland_key(
    context: *mut LinuxForeignContext,
    keysym: u32,
    pressed: bool,
    is_repeat: bool,
    text: *const c_char,
    modifiers: u8,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            with_editor(editor_id, ForeignBackend::Wayland, |_| {
                // SAFETY: null or a valid string by contract; copied out immediately.
                let text = unsafe { optional_c_string(text) };
                linux_wayland_key(
                    editor_id,
                    keysym,
                    pressed,
                    is_repeat,
                    text.as_deref(),
                    linux_wayland_modifiers(modifiers),
                );
            });
        }
    });
}

/// Health counters: frames dropped while windowless and host ticks
/// coalesced by the frame throttle. A null context leaves outputs
/// untouched.
///
/// # Safety
///
/// Non-null output pointers must be aligned and writable for one usize without
/// violating Rust aliasing rules. Unknown, stale, wrong-backend and wrong-thread
/// contexts leave outputs untouched.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_linux_wayland_frame_stats(
    context: *mut LinuxForeignContext,
    dropped: *mut usize,
    coalesced: *mut usize,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = editor_id(context) {
            let Some((dropped_frames, coalesced_frames)) =
                frame_stats(editor_id, ForeignBackend::Wayland)
            else {
                return;
            };
            if !dropped.is_null() {
                // SAFETY: null-checked; the caller guarantees a writable out count.
                unsafe {
                    *dropped = dropped_frames;
                }
            }
            if !coalesced.is_null() {
                // SAFETY: null-checked; the caller guarantees a writable out count.
                unsafe {
                    *coalesced = coalesced_frames;
                }
            }
        }
    });
}
