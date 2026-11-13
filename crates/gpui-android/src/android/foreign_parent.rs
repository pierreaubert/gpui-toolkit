//! Foreign-parent embedding for Android: GPUI editors in host `Surface`s.
//!
//! Implements the Android binding of `docs/foreign-parent-editor-embedding.md`.
//! A host (Kotlin/Java) view hands a `Surface` to
//! `Java_dev_gpui_mobile_GpuiForeign_nativeCreate`, pumps frames from its
//! `Choreographer`, and forwards resizes, focus, touches, and keys. There is
//! no `NativeActivity` event loop: the foreign platform runs synchronously.
//!
//! Touch mapping reuses the native `Pointer` pipeline: each host pointer
//! action becomes a `TouchPoint` delivered to `AndroidWindow::handle_touch`,
//! which the adapter converts to `PlatformInput::Pointer` (Down/Up/Move/
//! Cancel, Touch/Pen devices). History batching is not forwarded; the host
//! sends one call per pointer. IME routes through the shared global queue,
//! drained per window on its frame callback (single-editor assumption for
//! composed text; multi-editor IME routing is future work).
//!
//! Hover has no Android adapter support (`is_hovered` reports active, the
//! hover callback is dropped on the app path too), so this binding exposes
//! no hover entry point; hover state is defined as active.
//!
//! Threading: JNI calls may arrive on any attached thread, so all shared
//! editor state uses the `Sync` core policies. The one exception is the
//! `AppCell` pin, which lives in a thread-local map: create and destroy for
//! one editor must run on the same thread (normally the UI thread). A
//! cross-thread destroy logs loudly and leaks that editor's app state
//! rather than risking undefined behavior.
//!
//! Surface lifecycle: renderer construction is synchronous on the create
//! path, matching the app path (the renderer exposes no detachable-surface
//! API, so lazy construction would need the same blocking call on the first
//! frame instead). Hosts should call create off the UI thread when possible;
//! also see `nativeSurfaceCreated`/`nativeSurfaceDestroyed` for rotation and
//! backgrounding without a full re-create.

use super::dispatcher::AndroidDispatcher;
use super::jni::{get_string, with_env};
use super::platform::AndroidPlatform;
use super::window::AndroidWindow;
use super::{AndroidKeyEvent, TouchPoint, TouchTool};
use gpui::{
    App, AppCell, AppContext, Context, Entity, IntoElement, Render, Styled, Window, WindowOptions,
    div,
};
use gpui_foreign_parent::ffi_boundary;
use gpui_foreign_parent::sync::{SyncFrameThrottle, SyncResizeDebounce};
use jni::objects::JObject;
use jni::sys::{jfloat, jint, jlong};
use ndk::native_window::NativeWindow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{
    LazyLock, Mutex,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

/// One-shot handoff from the foreign create call to `open_window`, so the
/// platform wraps the new editor window instead of the primary window.
pub(crate) struct PendingForeign {
    pub(crate) editor_id: u64,
    pub(crate) window: std::sync::Arc<AndroidWindow>,
}

thread_local! {
    static PENDING_FOREIGN: RefCell<Option<PendingForeign>> = const { RefCell::new(None) };
}

/// Take the pending foreign window, if the last create stashed one.
pub(crate) fn take_pending_foreign() -> Option<PendingForeign> {
    PENDING_FOREIGN.with(|pending| pending.borrow_mut().take())
}

/// Shareable per-editor policy state.
#[derive(Debug, Default)]
struct ForeignPolicy {
    throttle: SyncFrameThrottle,
    debounce: SyncResizeDebounce<(i32, i32)>,
    dropped_frames: AtomicUsize,
}

/// One live foreign editor. Everything here is `Send + Sync`; the `AppCell`
/// pin lives separately in thread-local storage (see module docs).
struct ForeignEditor {
    window: std::sync::Arc<AndroidWindow>,
    dispatcher: std::sync::Arc<AndroidDispatcher>,
    policy: std::sync::Arc<ForeignPolicy>,
}

static FOREIGN_EDITORS: LazyLock<Mutex<HashMap<u64, ForeignEditor>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn lock_editors() -> std::sync::MutexGuard<'static, HashMap<u64, ForeignEditor>> {
    // Guarded state is plain owned data; a poisoned lock still holds a
    // usable map, so fail open instead of wedging the host.
    FOREIGN_EDITORS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

static NEXT_EDITOR_ID: AtomicU64 = AtomicU64::new(1);

/// Allocate a fresh editor id; `None` when the id space is exhausted
/// (never wraps to a reused id, and 0 stays invalid).
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

// `AppCell` pins, one per editor. Thread-local because `Rc` is `!Send`;
// create and destroy for one editor must share a thread.
thread_local! {
    static FOREIGN_APP_CELLS: RefCell<Option<HashMap<u64, Rc<AppCell>>>> =
        const { RefCell::new(None) };
}

/// Register a freshly opened foreign window under its editor id.
/// Called from `open_window` once the window reaches a stable address.
pub(crate) fn register_foreign_window(
    editor_id: u64,
    window: std::sync::Arc<AndroidWindow>,
    dispatcher: std::sync::Arc<AndroidDispatcher>,
) {
    lock_editors().insert(
        editor_id,
        ForeignEditor {
            window,
            dispatcher,
            policy: std::sync::Arc::new(ForeignPolicy::default()),
        },
    );
}

/// Clone one editor's shared handles. The map lock is released before the
/// caller runs anything, so frames never serialize on it.
fn clone_editor(editor_id: u64) -> Option<ForeignEditor> {
    lock_editors().get(&editor_id).map(|editor| ForeignEditor {
        window: std::sync::Arc::clone(&editor.window),
        dispatcher: std::sync::Arc::clone(&editor.dispatcher),
        policy: std::sync::Arc::clone(&editor.policy),
    })
}

/// Neutral placeholder root for the JNI entry point.
///
/// Hosts that need real editor content inject it through
/// [`android_foreign_create_with`].
struct PlaceholderRootView;

impl Render for PlaceholderRootView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

/// Create a GPUI context embedded in a host `Surface`, with content.
///
/// Rust-native constructor behind the JNI create entry point: returns the
/// editor id, or 0 when the surface is unusable or the window fails to open.
/// Geometry comes from the `Surface` itself; `scale` is the display density
/// relative to 160 dpi.
pub fn android_foreign_create_with<V: Render + 'static>(
    label: &str,
    scale: f32,
    surface: NativeWindow,
    build_root_view: impl FnOnce(&mut Window, &mut App) -> Entity<V> + 'static,
) -> u64 {
    if !scale.is_finite() || scale <= 0.0 {
        log::error!("GPUI Android foreign: invalid scale {scale}");
        return 0;
    }
    log::info!(
        "GPUI Android foreign: create label={label} size={}x{} @{scale:.1}x",
        surface.width(),
        surface.height(),
    );

    let platform = Rc::new(AndroidPlatform::new_foreign());
    let window = match platform.open_window(surface, scale, false) {
        Ok(window) => window,
        Err(e) => {
            log::error!("GPUI Android foreign: window init failed: {e:#}");
            return 0;
        }
    };

    let Some(editor_id) = alloc_editor_id() else {
        log::error!("GPUI Android foreign: editor id space exhausted");
        return 0;
    };
    PENDING_FOREIGN.with(|pending| {
        *pending.borrow_mut() = Some(PendingForeign {
            editor_id,
            window: std::sync::Arc::clone(&window),
        });
    });

    let app = gpui::Application::with_platform(platform);
    // The foreign `run()` invokes the launch callback synchronously, so
    // pin the app cell for the lifetime of the editor.
    let app_cell = app.clone_app_cell();

    let window_opened = Rc::new(std::cell::Cell::new(false));
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
                log::error!("GPUI Android foreign: open_window failed: {e:#}");
            }
        }
    });

    if !window_opened.get() {
        log::error!("GPUI Android foreign: returning 0 because open_window failed");
        PENDING_FOREIGN.with(|pending| *pending.borrow_mut() = None);
        lock_editors().remove(&editor_id);
        return 0;
    }

    FOREIGN_APP_CELLS.with(|cells| {
        cells
            .borrow_mut()
            .get_or_insert_with(HashMap::new)
            .insert(editor_id, app_cell);
    });
    log::info!("GPUI Android foreign: editor {editor_id} ready ({label})");
    editor_id
}

/// Destroy a foreign editor. Unknown ids are ignored. Releases the surface,
/// unregisters the editor, and drops its app state.
pub fn android_foreign_destroy(editor_id: u64) {
    let editor = lock_editors().remove(&editor_id);
    let Some(editor) = editor else {
        log::warn!("GPUI Android foreign: destroy of unknown editor {editor_id}");
        return;
    };
    editor.window.term_window();
    let pinned = FOREIGN_APP_CELLS.with(|cells| {
        cells
            .borrow_mut()
            .as_mut()
            .and_then(|cells| cells.remove(&editor_id))
    });
    if pinned.is_none() {
        log::error!(
            "GPUI Android foreign: editor {editor_id} destroyed on a different thread than its create; app state leaked"
        );
    }
    // `editor` and the pin drop here; the platform, windows, and adapter
    // tear down with the app state.
}

/// Request one frame. Call from the host `Choreographer` callback.
pub fn android_foreign_request_frame(editor_id: u64) {
    let Some(editor) = clone_editor(editor_id) else {
        return;
    };
    editor.dispatcher.flush_main_thread_tasks();
    if !editor.window.has_surface() {
        editor.policy.dropped_frames.fetch_add(1, Ordering::Relaxed);
        return;
    }
    // Converge any debounced resize before drawing.
    editor
        .policy
        .debounce
        .poll(|(width, height)| editor.window.reconfigure_surface(width, height));
    if !editor.policy.throttle.should_forward() {
        return;
    }
    editor.window.request_frame();
}

/// Host-driven resize. Non-positive sizes are clamped; a non-finite or
/// non-positive scale rejects the call.
pub fn android_foreign_resize(editor_id: u64, width: i32, height: i32, scale: f32) {
    if !scale.is_finite() || scale <= 0.0 {
        log::warn!("GPUI Android foreign: ignoring resize with invalid scale {scale}");
        return;
    }
    let Some(editor) = clone_editor(editor_id) else {
        return;
    };
    editor
        .window
        .resize_foreign(width.max(1), height.max(1), scale, &editor.policy.debounce);
}

/// Report host focus changes. Transition-guarded inside the window.
pub fn android_foreign_set_active(editor_id: u64, is_active: bool) {
    let Some(editor) = clone_editor(editor_id) else {
        return;
    };
    editor.window.set_active(is_active);
}

/// Map a host tool code to a touch tool: 0 = finger, 1 = stylus
/// (including eraser), anything else = compatibility path.
fn tool_from_int(tool: i32) -> TouchTool {
    match tool {
        0 => TouchTool::Finger,
        1 => TouchTool::Stylus,
        _ => TouchTool::Other,
    }
}

/// Deliver one host pointer action through the native `Pointer` pipeline.
///
/// Coordinates are physical pixels; non-finite values drop the event.
#[allow(clippy::too_many_arguments, reason = "JNI-ordered pointer fields")]
pub fn android_foreign_touch(
    editor_id: u64,
    pointer_id: i32,
    action: u32,
    x: f32,
    y: f32,
    timestamp_ns: u64,
    tool: TouchTool,
    pressure: Option<f32>,
) {
    if !x.is_finite() || !y.is_finite() {
        log::warn!("GPUI Android foreign: dropping touch with non-finite position");
        return;
    }
    let Some(editor) = clone_editor(editor_id) else {
        return;
    };
    editor.window.handle_touch(TouchPoint {
        id: pointer_id,
        x,
        y,
        action,
        timestamp_ns,
        tool,
        pressure: pressure.filter(|value| value.is_finite()),
        history: Vec::new(),
    });
}

/// Deliver one host key action. Only 0 (down) and 1 (up) are accepted.
pub fn android_foreign_key(
    editor_id: u64,
    key_code: i32,
    action: i32,
    meta_state: i32,
    unicode_char: u32,
) {
    if action != 0 && action != 1 {
        log::warn!("GPUI Android foreign: ignoring key action {action}");
        return;
    }
    let Some(editor) = clone_editor(editor_id) else {
        return;
    };
    editor.window.handle_key_event(AndroidKeyEvent {
        key_code,
        action,
        meta_state,
        unicode_char,
    });
}

/// Re-attach after a `Surface` loss (rotation, backgrounding) without a
/// full re-create. Reuses the stored GPU context and syncs geometry.
pub fn android_foreign_surface_created(editor_id: u64, surface: NativeWindow) {
    let Some(editor) = clone_editor(editor_id) else {
        return;
    };
    // The stored GPU context survives `term_window`, so this dummy is
    // ignored; `init_window` requires the parameter regardless.
    let ignored_gpu_context = std::rc::Rc::new(std::cell::RefCell::new(None));
    if let Err(e) = editor.window.init_window(surface, ignored_gpu_context) {
        log::error!("GPUI Android foreign: surface re-attach failed: {e:#}");
        return;
    }
    editor.window.handle_resize();
}

/// Release the surface ahead of its destruction. The logical window and
/// its callbacks stay alive for a later `surface_created`.
pub fn android_foreign_surface_destroyed(editor_id: u64) {
    let Some(editor) = clone_editor(editor_id) else {
        return;
    };
    editor.window.term_window();
}

/// Realtime health counters: frames dropped while headless and
/// `Choreographer` ticks coalesced by the frame throttle. Unknown ids
/// report zeros.
pub fn android_foreign_frame_stats(editor_id: u64) -> (usize, usize) {
    clone_editor(editor_id).map_or((0, 0), |editor| {
        (
            editor.policy.dropped_frames.load(Ordering::Relaxed),
            editor.policy.throttle.coalesced(),
        )
    })
}

// ── JNI entry points ─────────────────────────────────────────────────────────
// Raw-pointer convention matching the existing `GpuiActivity` bridges.
// IDs start at 1; 0 (or negative) is always invalid and ignored. Every
// export runs inside `ffi_boundary` so a guest panic cannot unwind
// into the JVM.

fn valid_editor_id(editor_id: jlong) -> Option<u64> {
    u64::try_from(editor_id).ok().filter(|id| *id != 0)
}

/// Create an editor in `surface`. Returns the editor id, or 0 on failure.
///
/// # Safety
///
/// Called by the JVM through JNI with the declared signature. `env` must
/// be the valid JNI environment for this call; `surface` must be null or
/// name a live `android.view.Surface`; `label` must be null or a live
/// string reference. Nulls are rejected before any dereference.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiForeign_nativeCreate(
    env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    surface: jni_sys::jobject,
    scale: jfloat,
    label: jni::sys::jobject,
) -> jlong {
    ffi_boundary(0, || {
        if env.is_null() || surface.is_null() || label.is_null() {
            log::error!("GPUI Android foreign: null argument to nativeCreate");
            return 0;
        }
        let label_text = with_env(|env| {
            // SAFETY: `label` is a JNI call-frame local ref; it is not dropped
            // here because the frame owns it.
            let label = unsafe { JObject::from_raw(env, label) };
            Ok(get_string(env, &label))
        });
        let Ok(label_text) = label_text else {
            log::error!("GPUI Android foreign: label conversion failed");
            return 0;
        };
        // SAFETY: `env` is the valid JNI environment for this call and
        // `surface` names a live `android.view.Surface`. `from_surface`
        // acquires an independent native reference, so the editor outlives
        // the Java object if needed.
        let surface = unsafe { NativeWindow::from_surface(env.cast::<jni_sys::JNIEnv>(), surface) };
        let Some(surface) = surface else {
            log::error!("GPUI Android foreign: Surface had no native window");
            return 0;
        };
        android_foreign_create_with(&label_text, scale, surface, |_, cx| {
            cx.new(|_| PlaceholderRootView)
        }) as jlong
    })
}

/// Destroy an editor. Unknown ids are ignored.///
/// # Safety
///
/// Called by the JVM through JNI with the declared signature. Scalar
/// arguments are validated before use.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiForeign_nativeDestroy(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    editor_id: jlong,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = valid_editor_id(editor_id) {
            android_foreign_destroy(editor_id);
        }
    });
}

/// Request one frame from the host `Choreographer` callback.///
/// # Safety
///
/// Called by the JVM through JNI with the declared signature. Scalar
/// arguments are validated before use.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiForeign_nativeRequestFrame(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    editor_id: jlong,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = valid_editor_id(editor_id) {
            android_foreign_request_frame(editor_id);
        }
    });
}

/// Host-driven resize in physical pixels plus density scale.///
/// # Safety
///
/// Called by the JVM through JNI with the declared signature. Scalar
/// arguments are validated before use.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiForeign_nativeResize(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    editor_id: jlong,
    width: jint,
    height: jint,
    scale: jfloat,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = valid_editor_id(editor_id) {
            android_foreign_resize(editor_id, width, height, scale);
        }
    });
}

/// Host focus report.///
/// # Safety
///
/// Called by the JVM through JNI with the declared signature. Scalar
/// arguments are validated before use.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiForeign_nativeSetActive(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    editor_id: jlong,
    is_active: u8,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = valid_editor_id(editor_id) {
            android_foreign_set_active(editor_id, is_active != 0);
        }
    });
}

/// One host pointer action in physical pixels. `action` uses the masked
/// `AMOTION_EVENT_ACTION_*` encoding (0 down, 1 up, 2 move, 3 cancel);
/// `tool` is 0 finger, 1 stylus, anything else compatibility.///
/// # Safety
///
/// Called by the JVM through JNI with the declared signature. Scalar
/// arguments are validated before use.
#[allow(
    clippy::too_many_arguments,
    reason = "JNI signature mirrors MotionEvent"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiForeign_nativeTouch(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    editor_id: jlong,
    pointer_id: jint,
    action: jint,
    x: jfloat,
    y: jfloat,
    timestamp_ns: jlong,
    tool: jint,
    pressure: jfloat,
) {
    ffi_boundary((), || {
        let Some(editor_id) = valid_editor_id(editor_id) else {
            return;
        };
        let Ok(action) = u32::try_from(action) else {
            log::warn!("GPUI Android foreign: ignoring negative touch action");
            return;
        };
        android_foreign_touch(
            editor_id,
            pointer_id,
            action,
            x,
            y,
            timestamp_ns.max(0) as u64,
            tool_from_int(tool),
            (pressure >= 0.0).then_some(pressure),
        );
    });
}

/// One host key action. `action` is 0 (down) or 1 (up).
///
/// # Safety
///
/// Called by the JVM through JNI with the declared signature. Scalar
/// arguments are validated before use.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiForeign_nativeKey(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    editor_id: jlong,
    key_code: jint,
    action: jint,
    meta_state: jint,
    unicode_char: jint,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = valid_editor_id(editor_id) {
            android_foreign_key(
                editor_id,
                key_code,
                action,
                meta_state,
                unicode_char.max(0) as u32,
            );
        }
    });
}

/// Re-attach after a `Surface` loss without a full re-create.
///
/// # Safety
///
/// Called by the JVM through JNI with the declared signature. `env` must
/// be the valid JNI environment for this call; `surface` must be null or
/// name a live `android.view.Surface`. Nulls are rejected before any
/// dereference.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiForeign_nativeSurfaceCreated(
    env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    editor_id: jlong,
    surface: jni_sys::jobject,
) {
    ffi_boundary((), || {
        let Some(editor_id) = valid_editor_id(editor_id) else {
            return;
        };
        if env.is_null() || surface.is_null() {
            log::error!("GPUI Android foreign: null argument to nativeSurfaceCreated");
            return;
        }
        // SAFETY: same contract as create: valid JNI environment and a live
        // `Surface`; `from_surface` acquires an independent reference.
        let surface = unsafe { NativeWindow::from_surface(env.cast::<jni_sys::JNIEnv>(), surface) };
        if let Some(surface) = surface {
            android_foreign_surface_created(editor_id, surface);
        } else {
            log::error!("GPUI Android foreign: re-attached Surface had no native window");
        }
    });
}

/// Release the surface ahead of its destruction (rotation/backgrounding).
///
/// # Safety
///
/// Called by the JVM through JNI with the declared signature. Scalar
/// arguments are validated before use.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiForeign_nativeSurfaceDestroyed(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    editor_id: jlong,
) {
    ffi_boundary((), || {
        if let Some(editor_id) = valid_editor_id(editor_id) {
            android_foreign_surface_destroyed(editor_id);
        }
    });
}

/// Frames dropped while headless.///
/// # Safety
///
/// Called by the JVM through JNI with the declared signature. Scalar
/// arguments are validated before use.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiForeign_nativeDroppedFrames(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    editor_id: jlong,
) -> jlong {
    ffi_boundary(0, || {
        valid_editor_id(editor_id)
            .map(|id| android_foreign_frame_stats(id).0 as jlong)
            .unwrap_or(0)
    })
}

/// `Choreographer` ticks coalesced by the frame throttle.
///
/// # Safety
///
/// Called by the JVM through JNI with the declared signature. Scalar
/// arguments are validated before use.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiForeign_nativeCoalescedFrames(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    editor_id: jlong,
) -> jlong {
    ffi_boundary(0, || {
        valid_editor_id(editor_id)
            .map(|id| android_foreign_frame_stats(id).1 as jlong)
            .unwrap_or(0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_ids_are_rejected() {
        assert_eq!(valid_editor_id(0), None);
        assert_eq!(valid_editor_id(-1), None);
        assert_eq!(valid_editor_id(7), Some(7));
    }

    #[test]
    fn tool_codes_map_to_touch_tools() {
        assert_eq!(tool_from_int(0), TouchTool::Finger);
        assert_eq!(tool_from_int(1), TouchTool::Stylus);
        assert_eq!(tool_from_int(2), TouchTool::Other);
        assert_eq!(tool_from_int(-1), TouchTool::Other);
    }

    #[test]
    fn unknown_editors_are_ignored() {
        // No editor 12345 exists; every call must no-op without panicking.
        android_foreign_destroy(12345);
        android_foreign_request_frame(12345);
        android_foreign_resize(12345, 100, 100, 2.0);
        android_foreign_set_active(12345, true);
        android_foreign_touch(12345, 0, 0, 1.0, 1.0, 0, TouchTool::Finger, None);
        android_foreign_key(12345, 0, 0, 0, 0);
        android_foreign_surface_destroyed(12345);
        assert_eq!(android_foreign_frame_stats(12345), (0, 0));
    }

    #[test]
    fn invalid_input_is_rejected() {
        // Invalid geometry and actions are dropped before any map lookup.
        android_foreign_resize(12345, 100, 100, f32::NAN);
        android_foreign_resize(12345, 100, 100, 0.0);
        android_foreign_touch(12345, 0, 0, f32::NAN, 1.0, 0, TouchTool::Finger, None);
        android_foreign_key(12345, 0, 7, 0, 0);
    }
}
