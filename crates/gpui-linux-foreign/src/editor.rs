//! Shared Linux editor lifetime and UI-thread entry guards.
//!
//! Each typed lease retains editor policy, platform and its `AppCell` pin through
//! active callbacks. Closing removes the token immediately. Context addresses
//! are numeric tokens, never allocations owned by the caller.

// Rust guideline compliant 2026-02-21

use crate::geometry::ForeignBounds;
use crate::lifecycle::{EditorState, LocalEditors, ThreadAffinity, context_from_handle, id_handle};
#[doc(inline)]
pub use crate::lifecycle::{ForeignBackend, LinuxForeignContext};
use gpui::{AnyWindowHandle, AppCell, PlatformInput};
use gpui_foreign_parent::{FrameScheduler, TransitionCell};
use gpui_linux::foreign_parent::{
    ForeignParentPlatform, clear_pending_wayland_foreign_parent, clear_pending_x11_foreign_parent,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// One live foreign editor, borrowed through a typed UI-thread lease.
pub(crate) struct LinuxForeignEditor {
    pub platform: Rc<ForeignParentPlatform>,
    pub window: AnyWindowHandle,
    /// Newest host geometry; updated synchronously on every resize while
    /// the surface reconfigure is debounced.
    pub bounds: Cell<ForeignBounds>,
    /// Common frame policy: host pumps invalidate, the scheduler
    /// throttles/coalesces, resizes debounce through it.
    pub scheduler: FrameScheduler<(u32, u32, f32)>,
    pub dropped_frames: AtomicUsize,
    pub active: TransitionCell,
    pub hovered: TransitionCell,
    // Host-forwarded click tracking (contract §7 click counts).
    pub last_click_ms: Cell<u64>,
    pub last_click_button: Cell<Option<gpui::MouseButton>>,
    pub last_click_at: Cell<(f32, f32)>,
    pub click_count: Cell<usize>,
}

type LinuxEditorState = EditorState<LinuxForeignEditor, Rc<AppCell>>;
static UI_THREAD: ThreadAffinity = ThreadAffinity::new();
thread_local! {
    static EDITORS: LocalEditors<LinuxForeignEditor, Rc<AppCell>> = LocalEditors::new();
}

pub(crate) fn permits_create() -> bool {
    UI_THREAD.permits_create()
}
pub(crate) fn claim_ui_thread() -> bool {
    UI_THREAD.claim()
}

/// Clears pending parent records before releasing a failed attach's app pin.
pub(crate) struct PendingAttach {
    pub pin: Option<Rc<AppCell>>,
}
impl PendingAttach {
    pub(crate) fn new() -> Self {
        Self { pin: None }
    }
}
impl Drop for PendingAttach {
    fn drop(&mut self) {
        clear_pending_x11_foreign_parent();
        clear_pending_wayland_foreign_parent();
    }
}

/// Registers the editor and pin as one owned lifetime.
pub(crate) fn register_editor(
    backend: ForeignBackend,
    editor: LinuxForeignEditor,
    pin: Rc<AppCell>,
) -> *mut LinuxForeignContext {
    if !UI_THREAD.is_current() {
        return std::ptr::null_mut();
    }
    EDITORS
        .try_with(|editors| editors.insert(backend, editor, pin))
        .ok()
        .and_then(Result::ok)
        .map_or(std::ptr::null_mut(), context_from_handle)
}

pub(crate) fn unregister_editor(id: u64, backend: ForeignBackend) {
    if !UI_THREAD.is_current() {
        if UI_THREAD.claimed_elsewhere() {
            // The editor lives in the owner thread's TLS registry and
            // cannot be reached from here; say so instead of silently
            // leaking it (the host violated the UI-thread contract).
            log::warn!(
                "gpui-linux-foreign: {backend:?} editor {id} destroyed on a different thread than its create; app state leaked"
            );
        }
        return;
    }
    if let Some(handle) = id_handle(id) {
        let _ = EDITORS.try_with(|editors| editors.close(handle, backend));
    }
}

pub(crate) fn with_editor<R>(
    id: u64,
    backend: ForeignBackend,
    f: impl FnOnce(&LinuxEditorState) -> R,
) -> Option<R> {
    // Only the claimed UI thread can register state. This !Send/!Sync registry
    // stays in TLS, so another thread cannot resolve a live native instance.
    // Lookups need no extra thread-id syscall/TLS read on every host tick.
    let handle = id_handle(id)?;
    EDITORS
        .try_with(|editors| editors.with(handle, backend, f))
        .ok()
        .flatten()
}

/// Forwards normalized input while retaining its editor and app state.
pub(crate) fn forward_input(id: u64, backend: ForeignBackend, input: PlatformInput) {
    with_editor(id, backend, |editor| {
        editor.platform.input_foreign(editor.window, input);
    });
}

/// Monotonic milliseconds used only for forwarded click counts.
pub(crate) fn boot_ms() -> u64 {
    static BOOT: std::sync::LazyLock<std::time::Instant> =
        std::sync::LazyLock::new(std::time::Instant::now);
    u64::try_from(BOOT.elapsed().as_millis()).unwrap_or(u64::MAX)
}

pub(crate) fn frame_stats(id: u64, backend: ForeignBackend) -> Option<(usize, usize)> {
    with_editor(id, backend, |editor| {
        (
            editor.dropped_frames.load(Ordering::Relaxed),
            editor.scheduler.coalesced(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_and_garbage_ids_resolve_to_no_instance() {
        assert!(with_editor(0, ForeignBackend::X11, |_| ()).is_none());
        assert!(with_editor(u64::MAX, ForeignBackend::Wayland, |_| ()).is_none());
        unregister_editor(0, ForeignBackend::X11);
        unregister_editor(u64::MAX, ForeignBackend::Wayland);
        assert_eq!(frame_stats(0, ForeignBackend::X11), None);
    }

    #[test]
    fn foreign_threads_cannot_resolve_editors() {
        // The registry is thread-local to the claimed UI thread, so a
        // lookup from any other thread resolves to no instance.
        let resolved = std::thread::spawn(|| with_editor(1, ForeignBackend::X11, |_| ()).is_some())
            .join()
            .unwrap();
        assert!(!resolved);
    }
}
