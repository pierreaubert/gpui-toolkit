//! Foreign-parent embedding for Windows: GPUI editors in host `HWND`s.
//!
//! Implements the Windows binding of
//! `docs/foreign-parent-editor-embedding.md` (§10, VST3 row). A host (DAW)
//! hands a parent `HWND` to [`gpui_win_create`], pumps frames from a
//! timer/DWM callback through [`gpui_win_request_frame`], and forwards
//! resizes and focus. Each editor gets its own `Application` (own
//! `AppCell`), keyed in a shared [`OwnedEditorRegistry`] by editor id so
//! multi-editor hosts work.
//!
//! The host owns the message loop: mouse, keyboard, wheel, and IME traffic
//! arrives at the `WS_CHILD` window as ordinary Win32 messages dispatched
//! by the host `GetMessage` loop, so — unlike the AU binding — this crate
//! needs no input-forwarding FFI. What the host must drive explicitly:
//!
//! - Frames: call [`gpui_win_request_frame`] on the UI thread (a steady
//!   timer or DWM-driven tick). Ticks faster than the 4 ms minimum
//!   interval coalesce instead of queueing.
//! - Resize: call [`gpui_win_resize`]; bounds track the newest host size
//!   immediately while the swapchain reconfigure debounces (16 ms).
//! - Focus/hover: call [`gpui_win_set_active`]/[`gpui_win_set_hovered`];
//!   child windows never see `WM_ACTIVATE`, and delivery is
//!   transition-only.
//! - Detach: call [`gpui_win_destroy`]. It closes the token immediately;
//!   native resources and the app pin survive callbacks already in progress.
//!
//! Context pointers are opaque numeric tokens, never dereferenced. Stale,
//! unknown and wrong-thread calls are rejected. Closing removes the token
//! immediately; native teardown waits for any callback already in progress.
//! The host parent must outlive those callbacks. Unwinding panics produce a
//! primitive failure result and invalidate the callback's token.

// Rust guideline compliant 2026-02-21

use std::cell::Cell;
use std::ffi::{c_char, c_void};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Once, OnceLock};

use gpui::{
    App, AppCell, AppContext, Context, Entity, IntoElement, Render, Styled, Window, WindowOptions,
    div,
};
use gpui_foreign_parent::{
    EditorHandle, FrameThrottle, OwnedEditorRegistry, ResizeDebounce, TransitionCell, ffi_boundary,
    optional_c_string,
};
use gpui_windows::{
    ForeignParentInfo, ForeignWindow, WindowsPlatform, clear_pending_foreign,
    stash_pending_foreign, take_opened_foreign,
};

use crate::policy::{CreationGuard, ForeignGeometry, validate_geometry};

static INIT_LOGGER: Once = Once::new();

fn init_logger() {
    INIT_LOGGER.call_once(|| {
        let _ = env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .try_init();
    });
}

/// Host UI thread, captured at the first attach.
static UI_THREAD: OnceLock<std::thread::ThreadId> = OnceLock::new();

fn is_ui_thread() -> bool {
    UI_THREAD
        .get()
        .is_none_or(|expected| *expected == std::thread::current().id())
}

thread_local! {
    static WIN_EDITORS: OwnedEditorRegistry<WinState> = OwnedEditorRegistry::new();
}

fn with_editor<R>(handle: EditorHandle, callback: impl FnOnce(&WinEditor) -> R) -> Option<R> {
    if !is_ui_thread() {
        return None;
    }
    WIN_EDITORS
        .try_with(|editors| editors.with(handle, |state| callback(&state.editor)))
        .ok()
        .flatten()
}

fn editor_id_of(context: *mut WinContext) -> Option<EditorHandle> {
    EditorHandle::from_usize(context.addr())
}

// ── Editor ───────────────────────────────────────────────────────────────────

/// One live foreign editor: the child window plus its host/guest policy.
struct WinEditor {
    foreign: Option<ForeignWindow>,
    throttle: FrameThrottle,
    debounce: ResizeDebounce<(i32, i32)>,
    /// Newest host geometry; tracks immediately while the surface
    /// reconfigure lags within the debounce window (contract §5).
    latest: Cell<ForeignGeometry>,
    active: TransitionCell,
    hovered: TransitionCell,
    dropped_frames: AtomicUsize,
}

impl WinEditor {
    fn foreign(&self) -> &ForeignWindow {
        self.foreign.as_ref().expect("live editor callback")
    }

    fn new(foreign: ForeignWindow, geometry: ForeignGeometry) -> Self {
        Self {
            foreign: Some(foreign),
            throttle: FrameThrottle::default(),
            debounce: ResizeDebounce::default(),
            latest: Cell::new(geometry),
            // A fresh child is neither active nor hovered, matching the
            // backend cells (`is_active` compares against the top-level
            // foreground window, which a child never is).
            active: TransitionCell::new(false),
            hovered: TransitionCell::new(false),
            dropped_frames: AtomicUsize::new(0),
        }
    }
}

/// Opaque C token; this address must never be dereferenced or freed by hosts.
#[derive(Debug)]
#[repr(C)]
pub struct WinContext {
    _private: [u8; 0],
}

struct WinState {
    editor: WinEditor,
    _label: String,
    // Dropped after editor policy/native resources, including deferred close.
    _app_cell: Rc<AppCell>,
}

impl Drop for WinState {
    fn drop(&mut self) {
        if let Some(foreign) = self.editor.foreign.take() {
            foreign.destroy();
        }
    }
}

/// Keeps the app alive while a failed or panicking attach releases its child.
struct PendingForeign {
    pin: Option<Rc<AppCell>>,
}

impl Drop for PendingForeign {
    fn drop(&mut self) {
        clear_pending_foreign();
        if let Some(foreign) = take_opened_foreign() {
            foreign.destroy();
        }
    }
}

/// Neutral placeholder root for the C entry point.
///
/// The C ABI cannot carry a Rust factory, so [`gpui_win_create`] renders
/// an empty root. Hosts that need real editor content inject it through
/// [`gpui_win_create_with`].
struct PlaceholderRootView;

impl Render for PlaceholderRootView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

// ── Lifecycle ────────────────────────────────────────────────────────────────

/// Create a GPUI context embedded in a host `HWND`, with injected content.
///
/// Rust-native constructor behind [`gpui_win_create`]: plugin crates call
/// this with their own root-view factory instead of going through the C
/// ABI. Returns null when the parent is null, the geometry is invalid, or
/// the window fails to open.
///
/// # Safety
///
/// `parent` must be null or point to a live host `HWND` (as
/// `*mut c_void`) that outlives the returned context. All calls must
/// happen on the host UI thread. Parent must also outlive callbacks active
/// when destruction is requested.
pub unsafe fn gpui_win_create_with<V: Render + 'static>(
    parent: *mut c_void,
    width: f32,
    height: f32,
    scale: f32,
    label: &str,
    build_root_view: impl FnOnce(&mut Window, &mut App) -> Entity<V> + 'static,
) -> *mut WinContext {
    ffi_boundary(std::ptr::null_mut(), || {
        init_logger();
        let expected = UI_THREAD.get_or_init(|| std::thread::current().id());
        if *expected != std::thread::current().id() {
            return std::ptr::null_mut();
        }
        let Some(_creation) = CreationGuard::enter() else {
            return std::ptr::null_mut();
        };

        if parent.is_null() {
            log::error!("GPUI Windows foreign: null parent HWND");
            return std::ptr::null_mut();
        }
        let Some(geometry) = validate_geometry(width, height, scale) else {
            log::error!("GPUI Windows foreign: invalid geometry {width}x{height} @{scale}");
            return std::ptr::null_mut();
        };

        log::info!(
            "GPUI Windows foreign: create label={label} size={}x{} @{:.1}x parent={parent:p}",
            geometry.width,
            geometry.height,
            geometry.scale,
        );

        stash_pending_foreign(ForeignParentInfo {
            parent: parent as isize,
            width: geometry.width,
            height: geometry.height,
            scale: geometry.scale,
        });

        let mut pending = PendingForeign { pin: None };
        let platform = match WindowsPlatform::new_foreign() {
            Ok(platform) => Rc::new(platform),
            Err(e) => {
                clear_pending_foreign();
                log::error!("GPUI Windows foreign: platform init failed: {e:#}");
                return std::ptr::null_mut();
            }
        };
        let app = gpui::Application::with_platform(platform);
        // The foreign `run()` invokes the launch callback synchronously, so
        // pin the app cell for the lifetime of the editor.
        let app_cell = app.clone_app_cell();
        pending.pin = Some(app_cell.clone());

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
                    log::error!("GPUI Windows foreign: open_window failed: {e:#}");
                }
            }
        });

        if !window_opened.get() {
            clear_pending_foreign();
            log::error!("GPUI Windows foreign: returning null because open_window failed");
            return std::ptr::null_mut();
        }
        let Some(foreign) = take_opened_foreign() else {
            clear_pending_foreign();
            log::error!("GPUI Windows foreign: open_window did not yield a foreign window");
            return std::ptr::null_mut();
        };

        let state = WinState {
            editor: WinEditor::new(foreign, geometry),
            _label: label.to_owned(),
            _app_cell: app_cell,
        };
        match WIN_EDITORS.try_with(|editors| editors.insert(state)) {
            Ok(Ok(handle)) => std::ptr::without_provenance_mut(handle.as_usize()),
            _ => std::ptr::null_mut(),
        }
    })
}

// ── C ABI ────────────────────────────────────────────────────────────────────

/// Create a GPUI context embedded in a host `HWND`.
///
/// `parent` must be a live host window (as `*mut c_void`) and `label`
/// must be a valid C string; nulls are rejected. Renders the neutral
/// placeholder root; Rust hosts inject real content through
/// [`gpui_win_create_with`]. Returns null on failure or an unwinding panic.
///
/// # Safety
///
/// Non-null label must point to a readable NUL-terminated C string. Parent
/// must be a valid HWND on its UI thread, outliving the editor and callbacks
/// active when destroy is called. The returned token must never be dereferenced.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_win_create(
    parent: *mut c_void,
    width: f32,
    height: f32,
    scale: f32,
    label: *const c_char,
) -> *mut WinContext {
    ffi_boundary(std::ptr::null_mut(), || {
        if !is_ui_thread() {
            return std::ptr::null_mut();
        }

        init_logger();

        if parent.is_null() || label.is_null() {
            log::error!("GPUI Windows foreign: null pointer argument to create");
            return std::ptr::null_mut();
        }
        // SAFETY: `label` is null-checked above and documented as a valid C
        // string; invalid UTF-8 is rejected without further dereference.
        let label_text = unsafe { optional_c_string(label) };
        let Some(label_text) = label_text else {
            log::error!("GPUI Windows foreign: invalid UTF-8 label");
            return std::ptr::null_mut();
        };
        // SAFETY: `parent` is null-checked above and documented as a live
        // host `HWND` outliving the context; hosts call on the UI thread.
        unsafe {
            gpui_win_create_with(parent, width, height, scale, &label_text, |_, cx| {
                cx.new(|_| PlaceholderRootView)
            })
        }
    })
}

/// Closes a Windows editor token; stale/null/wrong-thread tokens do nothing.
///
/// Destruction waits for callbacks already in progress. The parent must remain
/// alive through their return; no new callbacks resolve the closed token.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_win_destroy(context: *mut WinContext) {
    ffi_boundary((), || {
        if !is_ui_thread() {
            return;
        }
        if let Some(handle) = editor_id_of(context) {
            let _ = WIN_EDITORS.try_with(|editors| editors.close(handle));
        }
    });
}

/// Request one frame of GPUI rendering.
/// Call from a host timer/DWM tick on the UI thread (§10 pump).
#[unsafe(no_mangle)]
pub extern "C" fn gpui_win_request_frame(context: *mut WinContext) {
    ffi_boundary((), || {
        if !is_ui_thread() {
            return;
        }

        let Some(editor_id) = editor_id_of(context) else {
            return;
        };
        with_editor(editor_id, |editor| {
            // Converge a debounced resize before drawing (§6).
            editor.debounce.poll(|(width, height)| {
                editor
                    .foreign()
                    .resize_foreign(width, height, editor.latest.get().scale);
                true
            });
            if !editor.throttle.should_forward() {
                return;
            }
            if !editor.foreign().request_frame() {
                editor.dropped_frames.fetch_add(1, Ordering::Relaxed);
            }
        });
    });
}

/// Handle host resize: logical size plus backing scale.
///
/// Bounds track the newest host size immediately; only the surface
/// reconfigure (`SetWindowPos` → swapchain resize) is debounced (§5).
/// Non-finite input or a bad scale ignores the call.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_win_resize(context: *mut WinContext, width: f32, height: f32, scale: f32) {
    ffi_boundary((), || {
        if !is_ui_thread() {
            return;
        }

        let Some(editor_id) = editor_id_of(context) else {
            return;
        };
        let Some(geometry) = validate_geometry(width, height, scale) else {
            log::warn!("GPUI Windows foreign: ignoring resize with invalid geometry");
            return;
        };
        with_editor(editor_id, |editor| {
            editor.latest.set(geometry);
            editor
                .debounce
                .submit((geometry.width, geometry.height), |(w, h)| {
                    editor.foreign().resize_foreign(w, h, geometry.scale);
                    true
                });
        });
    });
}

/// Report host focus changes for GPUI window activation state.
/// Fires the GPUI callback on transition only (§7).
#[unsafe(no_mangle)]
pub extern "C" fn gpui_win_set_active(context: *mut WinContext, is_active: bool) {
    ffi_boundary((), || {
        if !is_ui_thread() {
            return;
        }

        let Some(editor_id) = editor_id_of(context) else {
            return;
        };
        with_editor(editor_id, |editor| {
            if editor.active.get() != is_active {
                editor.active.update(is_active);
                editor.foreign().set_foreign_active(is_active);
            }
        });
    });
}

/// Report whether the pointer is currently inside the child view.
/// Fires the GPUI callback on transition only (§7).
#[unsafe(no_mangle)]
pub extern "C" fn gpui_win_set_hovered(context: *mut WinContext, is_hovered: bool) {
    ffi_boundary((), || {
        if !is_ui_thread() {
            return;
        }

        let Some(editor_id) = editor_id_of(context) else {
            return;
        };
        with_editor(editor_id, |editor| {
            if editor.hovered.get() != is_hovered {
                editor.hovered.update(is_hovered);
                editor.foreign().set_foreign_hovered(is_hovered);
            }
        });
    });
}

/// Realtime health counters: frames dropped on a gone child window and
/// host ticks coalesced by the frame throttle. Each out-pointer is
/// optional; a null context leaves all outputs untouched.
///
/// # Safety
///
/// Non-null output pointers must be aligned and writable for one usize without
/// violating Rust aliasing rules. Context is only an opaque token.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gpui_win_frame_stats(
    context: *mut WinContext,
    dropped: *mut usize,
    coalesced: *mut usize,
) {
    ffi_boundary((), || {
        if !is_ui_thread() {
            return;
        }

        if context.is_null() {
            return;
        }
        let Some(editor_id) = editor_id_of(context) else {
            return;
        };
        let (dropped_frames, coalesced_frames) = with_editor(editor_id, |editor| {
            (
                editor.dropped_frames.load(Ordering::Relaxed),
                editor.throttle.coalesced(),
            )
        })
        .unwrap_or((0, 0));
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

/// Query the GPUI child `HWND` (as `*mut c_void`) for host-side plumbing:
/// z-order, explicit show/hide, or diagnostics. Returns null for a null
/// context or a gone editor.
#[unsafe(no_mangle)]
pub extern "C" fn gpui_win_child_hwnd(context: *mut WinContext) -> *mut c_void {
    ffi_boundary(std::ptr::null_mut(), || {
        if !is_ui_thread() {
            return std::ptr::null_mut();
        }

        let Some(editor_id) = editor_id_of(context) else {
            return std::ptr::null_mut();
        };
        with_editor(editor_id, |editor| {
            editor.foreign().child_hwnd() as *mut c_void
        })
        .unwrap_or(std::ptr::null_mut())
    })
}
