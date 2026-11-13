//! Shared editor lifecycle for the Linux bindings (contract §§4, 8, 9).
//!
//! One [`LinuxForeignEditor`] per host editor: the platform handle, the
//! GPUI window id, the newest host bounds, and the shared host/guest
//! policies (frame throttle, resize debounce, transition-only focus and
//! hover, health counters). Instances are owned by a thread-local
//! [`OwnedEditorRegistry`] under non-reused checked tokens; the numeric
//! editor id in the FFI context is that token, never a pointer. Closing
//! a token during a callback defers destruction until the callback
//! returns, so reentrant host teardown cannot dangle a live borrow.
//! The `AppCell` pin also lives in thread-local storage because `Rc` is
//! `!Send`.
//!
//! Affinity: every entry runs on the host UI thread. The first call pins
//! the UI thread id; calls from other threads are rejected (resolve to
//! "no instance") in every build profile, not just debug.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::{ThreadId, current};

use gpui::{AnyWindowHandle, AppCell, PlatformInput};
use gpui_foreign_parent::{
    EditorHandle, FrameThrottle, OwnedEditorRegistry, ResizeDebounce, TransitionCell,
};
use gpui_linux::foreign_parent::ForeignParentPlatform;

use crate::geometry::ForeignBounds;

/// Which native parent backs an editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForeignBackend {
    /// Host X11 window (VST3, CLAP-x11).
    X11,
    /// Host Wayland surface (CLAP-wayland).
    Wayland,
}

/// Opaque host handle for one editor; passed back verbatim.
pub struct LinuxForeignContext {
    editor_id: u64,
}

impl LinuxForeignContext {
    pub(crate) fn new(editor_id: u64) -> Self {
        Self { editor_id }
    }

    pub(crate) fn editor_id(&self) -> u64 {
        self.editor_id
    }
}

/// One live foreign editor. Main-thread use only; shared reads go through
/// the registry with the lock released before any callback runs.
pub(crate) struct LinuxForeignEditor {
    pub platform: Rc<ForeignParentPlatform>,
    pub window: AnyWindowHandle,
    pub backend: ForeignBackend,
    pub label: String,
    /// Newest host geometry; updated synchronously on every resize while
    /// the surface reconfigure is debounced.
    pub bounds: Cell<ForeignBounds>,
    pub throttle: FrameThrottle,
    pub debounce: ResizeDebounce<(u32, u32, f32)>,
    pub dropped_frames: AtomicUsize,
    pub active: TransitionCell,
    pub hovered: TransitionCell,
    // Host-forwarded click tracking (contract §7 click counts).
    pub last_click_ms: Cell<u64>,
    pub last_click_button: Cell<Option<gpui::MouseButton>>,
    pub last_click_at: Cell<(f32, f32)>,
    pub click_count: Cell<usize>,
}

/// Whether the current thread is the pinned host UI thread.
///
/// The first caller pins the thread. Every entry point checks this in
/// release builds too and rejects foreign threads by resolving to "no
/// instance" — editor state is `!Send` and must never be touched
/// cross-thread.
pub(crate) fn on_ui_thread() -> bool {
    static UI_THREAD: LazyLock<ThreadId> = LazyLock::new(|| current().id());
    current().id() == *UI_THREAD
}

thread_local! {
    static EDITORS: OwnedEditorRegistry<LinuxForeignEditor> = OwnedEditorRegistry::new();
    static APP_CELLS: RefCell<Option<HashMap<u64, Rc<AppCell>>>> = const { RefCell::new(None) };
}

fn handle_of(id: u64) -> Option<EditorHandle> {
    EditorHandle::from_usize(usize::try_from(id).ok()?)
}

/// Move `editor` into the owned registry; returns its checked token as
/// the numeric editor id. `None` off the UI thread or on token
/// exhaustion (never wraps).
pub(crate) fn register_editor(editor: LinuxForeignEditor) -> Option<u64> {
    if !on_ui_thread() {
        return None;
    }
    EDITORS.with(|editors| {
        editors
            .insert(editor)
            .ok()
            .map(|handle| handle.as_usize() as u64)
    })
}

/// Close `id`'s token, dropping its editor once active callbacks return.
pub(crate) fn unregister_editor(id: u64) {
    if !on_ui_thread() {
        return;
    }
    if let Some(handle) = handle_of(id) {
        EDITORS.with(|editors| editors.close(handle));
    }
}

/// Run `f` against the editor registered under `id`, if any.
pub(crate) fn with_editor<R>(id: u64, f: impl FnOnce(&LinuxForeignEditor) -> R) -> Option<R> {
    if !on_ui_thread() {
        return None;
    }
    let handle = handle_of(id)?;
    EDITORS.with(|editors| editors.with(handle, f))
}

/// Backend and label of the editor registered under `id`, if any.
/// Used for destroy-path diagnostics.
pub(crate) fn describe_editor(id: u64) -> Option<(ForeignBackend, String)> {
    with_editor(id, |editor| (editor.backend, editor.label.clone()))
}

/// Pin the app state for `id` on this thread.
pub(crate) fn pin_app_cell(id: u64, cell: Rc<AppCell>) {
    APP_CELLS.with(|cells| {
        cells
            .borrow_mut()
            .get_or_insert_with(HashMap::new)
            .insert(id, cell);
    });
}

/// Release the app-state pin for `id`. False when no pin exists on this
/// thread (a cross-thread destroy, which leaks that editor's app state
/// rather than risking undefined behavior).
pub(crate) fn unpin_app_cell(id: u64) -> bool {
    APP_CELLS.with(|cells| {
        cells
            .borrow_mut()
            .as_mut()
            .and_then(|cells| cells.remove(&id))
            .is_some()
    })
}

/// Forward one normalized input to an editor. Unknown ids are ignored.
pub(crate) fn forward_input(id: u64, input: PlatformInput) {
    with_editor(id, |editor| {
        editor.platform.input_foreign(editor.window, input);
    });
}

/// Boot time in milliseconds, the clock for forwarded click counts.
pub(crate) fn boot_ms() -> u64 {
    static BOOT: std::sync::LazyLock<std::time::Instant> =
        std::sync::LazyLock::new(std::time::Instant::now);
    BOOT.elapsed().as_millis() as u64
}

/// Realtime health counters: frames dropped while windowless and host
/// ticks coalesced by the frame throttle. Unknown ids report zeros, and a
/// null context leaves outputs untouched (see `ffi`).
pub(crate) fn frame_stats(id: u64) -> (usize, usize) {
    with_editor(id, |editor| {
        (
            editor.dropped_frames.load(Ordering::Relaxed),
            editor.throttle.coalesced(),
        )
    })
    .unwrap_or((0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_and_garbage_ids_resolve_to_no_instance() {
        assert!(with_editor(0, |_| ()).is_none());
        assert!(with_editor(u64::MAX, |_| ()).is_none());
        unregister_editor(0);
        unregister_editor(u64::MAX);
        assert_eq!(frame_stats(0), (0, 0));
    }

    #[test]
    fn foreign_threads_are_rejected_in_every_profile() {
        assert!(on_ui_thread());
        let resolved = std::thread::spawn(|| (on_ui_thread(), with_editor(1, |_| ()).is_some()))
            .join()
            .unwrap();
        assert_eq!(resolved, (false, false));
    }
}
