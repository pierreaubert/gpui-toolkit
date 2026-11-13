//! Foreign-parent FFI for iOS: C entry points for host view controllers.
//!
//! FFI functions necessarily dereference raw pointers from C callers; every
//! dereference carries a SAFETY comment instead.
//!
//! Implements the host/guest boundary of
//! `docs/foreign-parent-editor-embedding.md` for `AUv3` plugin views and
//! other hosts that own the `UIView`. The host pumps frames from its
//! `CADisplayLink`, forwards resizes, and reports focus/hover; touches,
//! hardware keys, and marked-text sessions route through the shared view
//! classes with no per-event forwarding.
//!
//! Each editor instance is keyed by id in a thread-local lease registry
//! (multi-editor hosts work); the opaque context pins the `AppCell` past
//! the synchronous platform `run()`. GPUI owns the window box, so — as
//! in the AU binding — the registry leases the window per callback:
//! destroy closes the slot immediately but defers dropping the context
//! (and with it the `AppCell` keeping the window alive) until active
//! leases drain, so reentrant host teardown cannot dangle a borrow.
//! Every export is `unsafe` with a documented contract and runs inside
//! `ffi_boundary`.

use super::super::window::{IosWindow, PENDING_PARENT, ParentViewInfo};
use gpui::{
    App, AppCell, AppContext, Context, Entity, IntoElement, Render, Styled, Window, WindowOptions,
    div,
};
use gpui_foreign_parent::{ffi_boundary, optional_c_string};
use objc::runtime::Object;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(not(test))]
fn is_main_thread() -> bool {
    use objc::{class, msg_send, runtime::BOOL, sel, sel_impl};
    // SAFETY: `NSThread` and its `isMainThread` class method always
    // exist; the message takes no arguments and returns a `BOOL`.
    unsafe {
        let is_main: BOOL = msg_send![class!(NSThread), isMainThread];
        is_main
    }
}

#[cfg(test)]
fn is_main_thread() -> bool {
    true
}

fn assert_main_thread() {
    assert!(
        is_main_thread(),
        "foreign iOS windows must be used on the main thread"
    );
}

/// Runs once the last lease on a closed slot drains (or immediately when idle).
type DeferredDrop = Box<dyn FnOnce()>;

#[derive(Default)]
struct ForeignSlot {
    window: Option<NonNull<IosWindow>>,
    leases: usize,
    closing: bool,
    deferred: Vec<DeferredDrop>,
}

thread_local! {
    /// Live foreign editor instances, keyed by editor id. Thread-local:
    /// foreign threads see an empty map and resolve to "no window".
    static FOREIGN_WINDOWS: RefCell<HashMap<u64, ForeignSlot>> = RefCell::new(HashMap::new());
}

fn drain_deferred(slot: &mut ForeignSlot) {
    if slot.leases == 0 && slot.closing {
        let deferred = std::mem::take(&mut slot.deferred);
        for drop_deferred in deferred {
            drop_deferred();
        }
    }
}

struct LeaseGuard(u64);

impl Drop for LeaseGuard {
    fn drop(&mut self) {
        FOREIGN_WINDOWS.with(|windows| {
            let mut windows = windows.borrow_mut();
            if let Some(slot) = windows.get_mut(&self.0) {
                slot.leases = slot.leases.saturating_sub(1);
                drain_deferred(slot);
                if slot.closing && slot.leases == 0 {
                    windows.remove(&self.0);
                }
            }
        });
    }
}

/// Register the boxed window for `editor_id` (from `open_window`).
pub(crate) fn register_foreign_window(editor_id: u64, window: &IosWindow) {
    assert_main_thread();
    FOREIGN_WINDOWS.with(|windows| {
        windows.borrow_mut().insert(
            editor_id,
            ForeignSlot {
                window: Some(NonNull::from(window)),
                ..ForeignSlot::default()
            },
        );
    });
}

/// Close `editor_id`'s slot: later lookups resolve to "no window";
/// deferred destruction runs once active leases drain.
pub(crate) fn unregister_foreign_window(editor_id: u64) {
    if !is_main_thread() {
        return;
    }
    FOREIGN_WINDOWS.with(|windows| {
        let mut windows = windows.borrow_mut();
        if let Some(slot) = windows.get_mut(&editor_id) {
            slot.closing = true;
            slot.window = None;
            drain_deferred(slot);
            if slot.leases == 0 {
                windows.remove(&editor_id);
            }
        }
    });
}

/// Run `drop_deferred` now when `editor_id` has no active lease, or
/// once its last lease returns after close.
pub(crate) fn defer_until_idle(editor_id: u64, drop_deferred: impl FnOnce() + 'static) {
    if !is_main_thread() {
        drop_deferred();
        return;
    }
    let deferred = FOREIGN_WINDOWS.with(|windows| {
        let mut windows = windows.borrow_mut();
        match windows.get_mut(&editor_id) {
            Some(slot) if slot.leases > 0 => {
                slot.deferred.push(Box::new(drop_deferred));
                None
            }
            _ => Some(Box::new(drop_deferred) as DeferredDrop),
        }
    });
    if let Some(drop_deferred) = deferred {
        drop_deferred();
    }
}

static NEXT_EDITOR_ID: AtomicU64 = AtomicU64::new(1);

/// Allocate a fresh editor id; `None` when the id space is exhausted
/// (never wraps to a reused id).
fn alloc_editor_id() -> Option<u64> {
    let mut id = NEXT_EDITOR_ID.load(Ordering::Relaxed);
    loop {
        if id == 0 {
            return None;
        }
        let next = id.checked_add(1)?;
        match NEXT_EDITOR_ID.compare_exchange_weak(id, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return Some(id),
            Err(current) => id = current,
        }
    }
}

/// Opaque context handle passed to/from the host.
pub struct IosForeignContext {
    _label: String,
    editor_id: u64,
    /// Pins the `AppCell` past the synchronous foreign `run()`, mirroring
    /// the AU backend's lifetime management.
    _app_cell: Rc<AppCell>,
}

/// Neutral placeholder root for the C entry point.
///
/// Rust hosts inject real editor content through
/// [`gpui_ios_foreign_create_with`].
struct PlaceholderRootView;

impl Render for PlaceholderRootView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

fn with_foreign_window<R>(editor_id: u64, f: impl FnOnce(&IosWindow) -> R) -> Option<R> {
    if !is_main_thread() {
        return None;
    }
    let window = FOREIGN_WINDOWS.with(|windows| {
        let mut windows = windows.borrow_mut();
        let slot = windows.get_mut(&editor_id)?;
        if slot.closing {
            return None;
        }
        let window = slot.window?;
        slot.leases = slot.leases.saturating_add(1);
        Some(window)
    })?;
    let _lease = LeaseGuard(editor_id);
    // SAFETY: `window` names the live `IosWindow` boxed by GPUI in
    // `open_window`; the lease defers the context (and with it the
    // `AppCell` keeping that box alive) past this call, and slot use is
    // confined to the main thread.
    Some(f(unsafe { window.as_ref() }))
}

/// Create a GPUI context embedded in a host `UIView`, with content.
///
/// Rust-native constructor behind [`gpui_ios_foreign_create`]: plugin
/// crates call this with their own root-view factory instead of going
/// through the C ABI. Returns null when the view is null or the window
/// fails to open.
///
/// # Safety
///
/// `parent_view` must be null or point to a live `UIView` that outlives
/// the returned context. All calls must happen on the host main thread.
pub unsafe fn gpui_ios_foreign_create_with<V: Render + 'static>(
    parent_view: *mut Object,
    width: f32,
    height: f32,
    scale: f32,
    label: &str,
    build_root_view: impl FnOnce(&mut Window, &mut App) -> Entity<V> + 'static,
) -> *mut IosForeignContext {
    if parent_view.is_null() {
        log::error!("GPUI iOS foreign: null parent view");
        return std::ptr::null_mut();
    }
    log::info!(
        "GPUI iOS foreign: create label={label} size={width}x{height} @{scale:.1}x view={parent_view:p}"
    );

    let Some(editor_id) = alloc_editor_id() else {
        log::error!("GPUI iOS foreign: editor id space exhausted");
        return std::ptr::null_mut();
    };

    // One-shot handoff so `open_window` routes to the foreign constructor.
    PENDING_PARENT.with(|pending| {
        *pending.borrow_mut() = Some(ParentViewInfo {
            parent_view,
            width,
            height,
            scale,
            editor_id,
        });
    });

    let platform = Rc::new(super::super::platform::IosPlatform::new_foreign());
    let app = gpui::Application::with_platform(platform);
    // The foreign `run()` invokes the launch callback synchronously, so
    // pin the app cell for the lifetime of the editor.
    let app_cell = app.clone_app_cell();

    let window_opened = Rc::new(Cell::new(false));
    let window_opened_clone = Rc::clone(&window_opened);
    app.run(move |cx: &mut App| {
        match cx.open_window(
            WindowOptions {
                window_bounds: None,
                ..Default::default()
            },
            build_root_view,
        ) {
            Ok(_handle) => {
                window_opened_clone.set(true);
            }
            Err(e) => {
                log::error!("GPUI iOS foreign: open_window failed: {e:#}");
            }
        }
    });

    if !window_opened.get() {
        log::error!("GPUI iOS foreign: returning null because open_window failed");
        PENDING_PARENT.with(|pending| *pending.borrow_mut() = None);
        unregister_foreign_window(editor_id);
        return std::ptr::null_mut();
    }

    Box::into_raw(Box::new(IosForeignContext {
        _label: label.to_string(),
        editor_id,
        _app_cell: app_cell,
    }))
}

/// Create a GPUI context embedded in a host `UIView`.
///
/// Null pointers are rejected; otherwise `parent_view` must be a live
/// `UIView` and `label` a valid C string. Renders the neutral placeholder
/// root; Rust hosts inject real content through
/// [`gpui_ios_foreign_create_with`].
/// # Safety
///
/// `parent_view` must be null or a live `UIView` outliving the
/// context, `label` null or a valid NUL-terminated C string. Call
/// on the host main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_ios_foreign_create(
    parent_view: *mut c_void,
    width: f32,
    height: f32,
    scale: f32,
    label: *const c_char,
) -> *mut IosForeignContext {
    ffi_boundary(std::ptr::null_mut(), || {
        if parent_view.is_null() || label.is_null() {
            log::error!("GPUI iOS foreign: null pointer argument");
            return std::ptr::null_mut();
        }
        // SAFETY: `label` is null-checked above; the host passes a valid C
        // string whose bytes are copied out immediately.
        let Some(label) = (unsafe { optional_c_string(label) }) else {
            log::error!("GPUI iOS foreign: invalid UTF-8 label");
            return std::ptr::null_mut();
        };
        // SAFETY: `parent_view` is null-checked above and documented as a
        // live `UIView` outliving the context; hosts call on the main thread.
        unsafe {
            gpui_ios_foreign_create_with(
                parent_view.cast::<Object>(),
                width,
                height,
                scale,
                &label,
                |_, cx| cx.new(|_| PlaceholderRootView),
            )
        }
    })
}

/// Destroy a foreign GPUI context. Null-safe; each context is destroyed
/// at most once.
/// # Safety
///
/// `context` must be null or a live context from
/// `gpui_ios_foreign_create`; call on the host main thread.
/// Each context must be destroyed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_ios_foreign_destroy(context: *mut IosForeignContext) {
    ffi_boundary((), || {
        if context.is_null() {
            return;
        }
        // SAFETY: non-null context created by the foreign create entry
        // point; only the id is read before ownership is taken below.
        let editor_id = unsafe { (*context).editor_id };
        unregister_foreign_window(editor_id);
        // SAFETY: `context` came from `Box::into_raw` in the foreign create
        // entry point and the host destroys each context at most once.
        let context = unsafe { Box::from_raw(context) };
        // Defer the context (AppCell) drop past any active window lease.
        defer_until_idle(editor_id, move || drop(context));
    });
}

/// Request one frame of GPUI rendering.
/// Call from the host `CADisplayLink` callback on the main thread.
/// # Safety
///
/// `context` must be null or a live context from
/// `gpui_ios_foreign_create`; call on the host main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_ios_foreign_request_frame(context: *mut IosForeignContext) {
    ffi_boundary((), || {
        if context.is_null() {
            return;
        }
        // SAFETY: non-null context created by the foreign create entry point;
        // only the id is read.
        let editor_id = unsafe { (*context).editor_id };
        with_foreign_window(editor_id, IosWindow::request_frame_foreign);
    });
}

/// Handle view resize from the host.
/// # Safety
///
/// `context` must be null or a live context from
/// `gpui_ios_foreign_create`; call on the host main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_ios_foreign_resize(
    context: *mut IosForeignContext,
    width: f32,
    height: f32,
    scale: f32,
) {
    ffi_boundary((), || {
        if context.is_null() {
            return;
        }
        // SAFETY: non-null context created by the foreign create entry point;
        // only the id is read.
        let editor_id = unsafe { (*context).editor_id };
        with_foreign_window(editor_id, |window| {
            window.resize_foreign(width, height, scale);
        });
    });
}

/// Report host focus changes for GPUI window activation state.
/// # Safety
///
/// `context` must be null or a live context from
/// `gpui_ios_foreign_create`; call on the host main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_ios_foreign_set_active(
    context: *mut IosForeignContext,
    is_active: bool,
) {
    ffi_boundary((), || {
        if context.is_null() {
            return;
        }
        // SAFETY: non-null context created by the foreign create entry point;
        // only the id is read.
        let editor_id = unsafe { (*context).editor_id };
        with_foreign_window(editor_id, |window| window.set_active_foreign(is_active));
    });
}

/// Report whether the pointer is currently inside the host view.
/// # Safety
///
/// `context` must be null or a live context from
/// `gpui_ios_foreign_create`; call on the host main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_ios_foreign_set_hovered(
    context: *mut IosForeignContext,
    is_hovered: bool,
) {
    ffi_boundary((), || {
        if context.is_null() {
            return;
        }
        // SAFETY: non-null context created by the foreign create entry point;
        // only the id is read.
        let editor_id = unsafe { (*context).editor_id };
        with_foreign_window(editor_id, |window| {
            window.set_hovered_foreign(is_hovered);
        });
    });
}

/// Realtime health counters: frames dropped on a busy renderer and
/// display-link ticks coalesced by the frame throttle. Each out-pointer is
/// optional; a null context leaves all outputs untouched.
/// # Safety
///
/// `context` must be null or a live context from
/// `gpui_ios_foreign_create`; call on the host main thread.
/// Out-count pointers must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_ios_foreign_frame_stats(
    context: *mut IosForeignContext,
    dropped: *mut usize,
    coalesced: *mut usize,
) {
    ffi_boundary((), || {
        if context.is_null() {
            return;
        }
        // SAFETY: non-null context created by the foreign create entry point;
        // only the id is read.
        let editor_id = unsafe { (*context).editor_id };
        let (dropped_frames, coalesced_frames) =
            with_foreign_window(editor_id, IosWindow::foreign_frame_stats).unwrap_or((0, 0));
        if !dropped.is_null() {
            // SAFETY: `dropped` is null-checked; the host passes a writable
            // out counter.
            unsafe {
                *dropped = dropped_frames;
            }
        }
        if !coalesced.is_null() {
            // SAFETY: `coalesced` is null-checked; the host passes a writable
            // out counter.
            unsafe {
                *coalesced = coalesced_frames;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreign_entry_points_are_null_safe() {
        // SAFETY: every call below passes only null pointers, which each
        // export's contract explicitly accepts as a no-op/query.
        unsafe {
            let context = std::ptr::null_mut();
            gpui_ios_foreign_destroy(context);
            gpui_ios_foreign_request_frame(context);
            gpui_ios_foreign_resize(context, 320.0, 200.0, 2.0);
            gpui_ios_foreign_set_active(context, true);
            gpui_ios_foreign_set_hovered(context, true);
            let (mut dropped, mut coalesced) = (usize::MAX, usize::MAX);
            gpui_ios_foreign_frame_stats(context, &raw mut dropped, &raw mut coalesced);
            assert_eq!((dropped, coalesced), (usize::MAX, usize::MAX));
            gpui_ios_foreign_frame_stats(context, std::ptr::null_mut(), std::ptr::null_mut());
            assert!(
                gpui_ios_foreign_create(std::ptr::null_mut(), 320.0, 200.0, 2.0, std::ptr::null())
                    .is_null()
            );
        }
    }

    #[test]
    fn foreign_create_with_rejects_null_parent() {
        // SAFETY: a null parent is explicitly rejected without dereference.
        let context = unsafe {
            gpui_ios_foreign_create_with(
                std::ptr::null_mut(),
                320.0,
                200.0,
                2.0,
                "test",
                |_, cx| cx.new(|_| PlaceholderRootView),
            )
        };
        assert!(context.is_null());
    }
}
