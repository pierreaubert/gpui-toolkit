//! Android entry point and event loop using the `android-activity` crate.
//!
//! This module replaces the previous hand-rolled `ANativeActivity_onCreate`,
//! `JNI_OnLoad`, and lifecycle callback implementations with the higher-level
//! `android-activity` glue layer.
//!
//! ## Entry sequence
//!
//! ```text
//! android-activity loads the .so and calls android_main(app: AndroidApp)
//!   └── We store the AndroidApp globally
//!       └── Call the user-supplied gpui_android_main(app)
//! ```
//!
//! ## Threading model
//!
//! `android-activity` spawns a dedicated native thread and calls `android_main`
//! on it.  All GPUI draw / event callbacks run on this thread.  The
//! `AndroidApp` handle is `Send + Sync` and can be shared across threads.
//!
//! ## User entry point
//!
//! Applications must define:
//!
//! ```rust,no_run
//! #[no_mangle]
//! fn android_main(app: android_activity::AndroidApp) {
//!     // Initialise GPUI and run the application.
//! }
//! ```
//!
//! ## Event handling
//!
//! Lifecycle events (window creation/destruction, focus changes, etc.) are
//! delivered via `AndroidApp::poll_events()`.  Input events are obtained via
//! `AndroidApp::input_events_iter()`.

#![allow(unsafe_code)]
#![allow(non_snake_case)]

use std::{
    ffi::c_void,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// Whether the deferred init-window callback has already been invoked.
///
/// Reset to `false` on `TerminateWindow` so that when the surface is
/// recreated on resume the init callbacks run again.
static INIT_WINDOW_DONE: AtomicBool = AtomicBool::new(false);

/// Whether the GPUI native library has completed initialization.
///
/// Set to `true` after the first frame is rendered. This can be queried
/// via JNI by a custom Activity to dismiss the splash screen, although
/// with NativeActivity the system splash handles this automatically.
pub static NATIVE_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Deferred lifecycle flags.
///
/// We must NOT call `win.set_active()` or `platform.did_enter_background()`
/// inside `handle_main_event` (which runs within `poll_events`).
/// The android-activity crate's Java-side callbacks block on a condvar
/// waiting for the native thread to finish processing the command.
/// If our handler tries to acquire the window `state` lock (a
/// `parking_lot::Mutex`), and a background thread is holding it (e.g.
/// during a render pass), we deadlock: native waits on the lock,
/// the lock holder waits for native rendering to complete, but native
/// is stuck.
///
/// Instead, the handlers set these flags and the main loop body
/// processes them AFTER `poll_events` returns.
static PAUSE_PENDING: AtomicBool = AtomicBool::new(false);
static RESUME_PENDING: AtomicBool = AtomicBool::new(false);
static TERM_WINDOW_PENDING: AtomicBool = AtomicBool::new(false);
static INIT_WINDOW_PENDING: AtomicBool = AtomicBool::new(false);
static WINDOW_RESIZED_PENDING: AtomicBool = AtomicBool::new(false);
static CONFIG_CHANGED_PENDING: AtomicBool = AtomicBool::new(false);

use android_activity::{AndroidApp, MainEvent, PollEvent};

use super::platform::{AndroidPlatform, SharedPlatform};

use jni::JavaVM;
use jni::objects::{Global, JObject, JString, JValue};

// ── JNI helpers (safe `jni` crate wrappers) ──────────────────────────────────

static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

/// Get or create the static `JavaVM` wrapper.
fn java_vm_safe() -> Result<&'static JavaVM, String> {
    if let Some(vm) = JAVA_VM.get() {
        return Ok(vm);
    }
    let ptr = java_vm();
    if ptr.is_null() {
        return Err("JavaVM not available".into());
    }
    Ok(JAVA_VM.get_or_init(|| unsafe { JavaVM::from_raw(ptr as *mut jni::sys::JavaVM) }))
}

/// Run a closure with an attached `jni::Env` for the current thread.
///
/// In jni 0.22 the `attach_current_thread` API is closure-based.
/// The thread is auto-detached when the closure returns (if it was
/// not already attached).
pub fn with_env<T>(f: impl FnOnce(&mut jni::Env) -> Result<T, String>) -> Result<T, String> {
    let vm = java_vm_safe()?;
    let mut result: Option<Result<T, String>> = None;
    vm.attach_current_thread(|env: &mut jni::Env| -> Result<(), jni::errors::Error> {
        result = Some(f(env));
        Ok(())
    })
    .map_err(|e: jni::errors::Error| e.to_string())?;
    result.unwrap()
}

/// Convenience alias: kept so existing callers that import `obtain_env`
/// compile with minimal changes. Returns a result by running the given
/// closure inside `with_env`.
#[inline]
pub fn obtain_env<T>(f: impl FnOnce(&mut jni::Env) -> Result<T, String>) -> Result<T, String> {
    with_env(f)
}

/// Get the Activity as a [`JObject`].
///
/// `activity_as_ptr()` returns a JNI global reference from `android-activity`
/// that is valid for the lifetime of the app. We wrap it in a `JObject`.
///
/// Requires `&Env` because jni 0.22's `JObject::from_raw` binds the
/// local-reference-frame lifetime.
pub fn activity<'local>(env: &jni::Env<'local>) -> Result<JObject<'local>, String> {
    let ptr = activity_as_ptr();
    if ptr.is_null() {
        return Err("Activity not available".into());
    }
    Ok(unsafe { JObject::from_raw(env, ptr as jni::sys::jobject) })
}

/// Convert a Java String (`JObject` wrapping a `java.lang.String`) to a Rust `String`.
///
/// Returns an empty string on null or error.
pub fn get_string(env: &mut jni::Env<'_>, obj: &JObject<'_>) -> String {
    if obj.is_null() {
        return String::new();
    }
    let jstr = unsafe { JString::from_raw(env, obj.as_raw()) };
    jstr.to_string()
}

/// Extension trait for converting `jni::errors::Result<T>` to `Result<T, String>`.
pub(crate) trait JniExt<T> {
    fn e(self) -> Result<T, String>;
}

impl<T> JniExt<T> for jni::errors::Result<T> {
    fn e(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}

/// Find an application class by name using the Activity's classloader.
///
/// From native threads, `JNIEnv::FindClass` uses the system classloader
/// which doesn't know about application classes.  This helper uses the
/// Activity's classloader via `activity.getClass().getClassLoader().loadClass(name)`.
///
/// `class_name` uses Java dot notation (e.g. `"dev.gpui.mobile.GpuiHelper"`).
pub fn find_app_class<'local>(
    env: &mut jni::Env<'local>,
    class_name: &str,
) -> Result<jni::objects::JClass<'local>, String> {
    let act = activity(env)?;

    // activity.getClassLoader() — call on the Context instance directly.
    // Do NOT use activity.getClass().getClassLoader(): NativeActivity is a
    // framework class loaded by BootClassLoader, which cannot see app classes.
    let class_loader = env
        .call_method(
            &act,
            jni::jni_str!("getClassLoader"),
            jni::jni_sig!("()Ljava/lang/ClassLoader;"),
            &[],
        )
        .and_then(|v| v.l())
        .map_err(|e| {
            env.exception_clear();
            let msg = format!("getClassLoader failed: {e}");
            log::error!("find_app_class({class_name}): {msg}");
            msg
        })?;

    // classLoader.loadClass("dev.gpui.mobile.GpuiHelper")
    let jname = env.new_string(class_name).e()?;
    let loaded = env
        .call_method(
            &class_loader,
            jni::jni_str!("loadClass"),
            jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"),
            &[JValue::Object(&jname)],
        )
        .and_then(|v| v.l())
        .map_err(|e| {
            // Print full Java stack trace to logcat, then clear.
            env.exception_describe();
            env.exception_clear();
            let msg = format!("loadClass({class_name}) failed: {e}");
            log::error!("{msg}");
            msg
        })?;

    log::debug!("find_app_class: loaded {class_name}");
    Ok(unsafe { jni::objects::JClass::from_raw(env, loaded.as_raw()) })
}

// ── cached application-class lookup ─────────────────────────────────────────
//
// `find_app_class` goes through the Activity classloader (`loadClass`) on
// every call. Platform-view bounds/visibility updates and the lifecycle
// helpers call it repeatedly, so the resolved class is retained as a JNI
// global reference (same pattern as `KEY_CHARACTER_MAP` above and the IME
// cache in `window.rs`). A failed call through a cached class invalidates the
// entry so the next call transparently falls back to a fresh lookup.

/// Application classes resolved via the Activity classloader, by dot name.
///
/// The global references are owned by this map; readers only borrow the raw
/// pointer for the duration of one `Env` call.
static CACHED_APP_CLASSES: std::sync::Mutex<
    std::collections::HashMap<String, Global<JObject<'static>>>,
> = std::sync::Mutex::new(std::collections::HashMap::new());

/// Resolve an application class, reusing a cached global reference.
///
/// Falls back to [`find_app_class`] on a cache miss. Callers should invoke
/// [`invalidate_app_class_cache`] when a call through the returned class
/// fails, in case the retained reference went stale.
pub fn find_cached_app_class<'local>(
    env: &mut jni::Env<'local>,
    class_name: &str,
) -> Result<jni::objects::JClass<'local>, String> {
    if let Some(raw) = CACHED_APP_CLASSES
        .lock()
        .unwrap()
        .get(class_name)
        .map(|global| global.as_obj().as_raw())
    {
        if !raw.is_null() {
            return Ok(unsafe { jni::objects::JClass::from_raw(env, raw) });
        }
        CACHED_APP_CLASSES.lock().unwrap().remove(class_name);
    }
    let class = find_app_class(env, class_name)?;
    // Best-effort: retain a global reference for subsequent calls. `class`
    // converts into a plain `JObject` view; the global ref owns an
    // independent lifetime, so the locals may drop at frame pop as usual.
    let obj: JObject<'_> = class.into();
    let raw = obj.as_raw();
    if !raw.is_null()
        && let Ok(global) = env.new_global_ref(&obj)
    {
        CACHED_APP_CLASSES
            .lock()
            .unwrap()
            .insert(class_name.to_string(), global);
    }
    Ok(unsafe { jni::objects::JClass::from_raw(env, raw) })
}

/// Drop a cached application class, forcing a fresh lookup next time.
///
/// Call this when a JNI call through a cached class fails: the retained
/// reference may predate an Activity recreation.
pub fn invalidate_app_class_cache(class_name: &str) {
    CACHED_APP_CLASSES.lock().unwrap().remove(class_name);
}

// ── global state ─────────────────────────────────────────────────────────────

/// The `AndroidApp` handle from `android-activity`.
///
/// Set once in `android_main`; read-only thereafter.
static ANDROID_APP: OnceLock<AndroidApp> = OnceLock::new();

/// Process-global `AndroidPlatform` instance.
///
/// Initialised once during `android_main`; read-only thereafter.
static PLATFORM: OnceLock<Arc<AndroidPlatform>> = OnceLock::new();

/// Immutable virtual-keyboard character map shared by hardware key events.
static KEY_CHARACTER_MAP: OnceLock<Global<JObject<'static>>> = OnceLock::new();

/// Get the unicode character produced by an Android key event via JNI.
///
/// Uses a retained `KeyCharacterMap`, avoiding a Java `KeyEvent` allocation on
/// every transition. Returns 0 on failure.
pub fn unicode_char_for_key_event(key_code: i32, _action: i32, meta_state: i32) -> u32 {
    with_env(|env| {
        if KEY_CHARACTER_MAP.get().is_none() {
            let local = match env.call_static_method(
                jni::jni_str!("android/view/KeyCharacterMap"),
                jni::jni_str!("load"),
                jni::jni_sig!("(I)Landroid/view/KeyCharacterMap;"),
                &[JValue::Int(-1)],
            ) {
                Ok(value) => value.l().map_err(|error| error.to_string())?,
                Err(error) => return Err(error.to_string()),
            };
            let global = env
                .new_global_ref(local)
                .map_err(|error| error.to_string())?;
            let _ = KEY_CHARACTER_MAP.set(global);
        }
        let Some(key_character_map) = KEY_CHARACTER_MAP.get() else {
            return Ok(0);
        };
        match env.call_method(
            key_character_map.as_obj(),
            jni::jni_str!("get"),
            jni::jni_sig!("(II)I"),
            &[JValue::Int(key_code), JValue::Int(meta_state)],
        ) {
            Ok(v) => {
                let c = v.i().unwrap_or(0);
                Ok(if c > 0 { c as u32 } else { 0 })
            }
            Err(_) => {
                env.exception_clear();
                Ok(0)
            }
        }
    })
    .unwrap_or(0)
}

// ── public accessors ──────────────────────────────────────────────────────────

/// Public accessor for the JavaVM pointer.
///
/// Uses `AndroidApp::vm_as_ptr()` from the stored `AndroidApp`.
/// Used by `platform.rs` for JNI calls.
pub fn java_vm() -> *mut c_void {
    ANDROID_APP
        .get()
        .map(|app| app.vm_as_ptr())
        .unwrap_or(std::ptr::null_mut())
}

/// Public accessor for the current Activity's JNI object reference.
///
/// Uses `AndroidApp::activity_as_ptr()` from the stored `AndroidApp`.
/// Used by `platform.rs` and `window.rs` for JNI calls that require the
/// activity's jobject.
///
/// NOTE: This returns a jobject (JNI global ref), NOT an `ANativeActivity *`.
/// Code that previously used `(*activity).clazz` should use this directly
/// as the activity jobject.
pub fn activity_as_ptr() -> *mut c_void {
    ANDROID_APP
        .get()
        .map(|app| app.activity_as_ptr())
        .unwrap_or(std::ptr::null_mut())
}

/// Returns a clone of the stored `AndroidApp`, if initialised.
pub fn android_app() -> Option<AndroidApp> {
    ANDROID_APP.get().cloned()
}

/// Returns a reference to the global `AndroidPlatform`, if initialised.
///
/// Returns `None` before `android_main` has set it up.
pub fn platform() -> Option<&'static Arc<AndroidPlatform>> {
    PLATFORM.get()
}

/// Returns a [`SharedPlatform`] wrapping the global `Arc<AndroidPlatform>`.
///
/// This is the value you hand to `Application::with_platform(...)`:
///
/// ```rust,no_run
/// let platform = jni::shared_platform().unwrap();
/// Application::with_platform(platform.into_rc()).run(|cx| { … });
/// ```
///
/// Returns `None` before `init_platform` has been called.
pub fn shared_platform() -> Option<SharedPlatform> {
    PLATFORM
        .get()
        .map(|arc| SharedPlatform::new(Arc::clone(arc)))
}

// ── input event types ─────────────────────────────────────────────────────────

// ── night mode query via NDK Configuration ───────────────────────────────────

/// Query the current night mode using the NDK Configuration API.
///
/// Returns `true` if the system is in dark mode.
pub fn query_night_mode_via_jni() -> bool {
    let app = match android_app() {
        Some(app) => app,
        None => return false,
    };

    // Build an ndk::configuration::Configuration from the app's asset manager.
    let config = ndk::configuration::Configuration::from_asset_manager(&app.asset_manager());
    let is_dark = config.ui_mode_night() == ndk::configuration::UiModeNight::Yes;

    log::debug!("query_night_mode (ndk): is_dark={}", is_dark);
    is_dark
}

// ── input event processing ────────────────────────────────────────────────────

/// Map an NDK motion action onto the testable [`crate::event_stages::MotionKind`].
///
/// This is the **decode** stage of the input pipeline: everything downstream
/// (`decode_motion_slot`) is pure and unit-tested on any host.
fn motion_kind(action: android_activity::input::MotionAction) -> crate::event_stages::MotionKind {
    use crate::event_stages::MotionKind as Kind;
    use android_activity::input::MotionAction as Native;
    match action {
        Native::Down => Kind::Down,
        Native::PointerDown => Kind::PointerDown,
        Native::Up => Kind::Up,
        Native::PointerUp => Kind::PointerUp,
        Native::Move => Kind::Move,
        Native::Cancel => Kind::Cancel,
        _ => Kind::Other,
    }
}

/// Process input events from the `AndroidApp` and dispatch them to the window.
fn process_input_events(app: &AndroidApp) {
    let platform = match PLATFORM.get() {
        Some(p) => p,
        None => {
            log::trace!("process_input_events: no platform yet");
            return;
        }
    };

    let win = match platform.primary_window() {
        Some(w) => w,
        None => {
            log::trace!("process_input_events: no primary window yet");
            return;
        }
    };

    match app.input_events_iter() {
        Ok(mut iter) => {
            loop {
                let read_input = iter.next(|event| {
                    use android_activity::input::{InputEvent, MotionAction};

                    match event {
                        InputEvent::MotionEvent(motion_event) => {
                            let action = motion_event.action();
                            let pointer_count = motion_event.pointer_count();

                            log::debug!(
                                "process_input_events: MotionEvent action={:?} pointers={}",
                                action,
                                pointer_count,
                            );

                            // Check if this touch lands on a platform view.
                            //
                            // On Android with NativeActivity, ALL touch events go
                            // to the native surface first. Platform views are real
                            // Java Views in a FrameLayout overlay, but they won't
                            // receive touches unless we skip GPUI dispatch and let
                            // Android's view hierarchy handle the event instead.
                            //
                            // We use the primary pointer for the hit-test on DOWN
                            // actions. Physical pixel coordinates are converted to
                            // logical pixels to match PlatformViewBounds.
                            let hits_platform_view = {
                                let registry = crate::platform_view::PlatformViewRegistry::global();
                                if registry.active_view_count() > 0 {
                                    let primary = motion_event.pointer_at_index(
                                        match action {
                                            MotionAction::PointerDown | MotionAction::PointerUp => {
                                                motion_event.pointer_index()
                                            }
                                            _ => 0,
                                        }
                                    );
                                    let scale = win.scale_factor();
                                    let logical_x = primary.x() / scale;
                                    let logical_y = primary.y() / scale;
                                    registry.hit_test(logical_x, logical_y)
                                } else {
                                    false
                                }
                            };

                            if hits_platform_view {
                                log::debug!(
                                    "process_input_events: touch hits platform view, skipping GPUI dispatch",
                                );
                                // Return Unhandled so android-activity can pass
                                // the event back to the Java view hierarchy where
                                // the platform view's FrameLayout lives.
                                return android_activity::InputStatus::Unhandled;
                            }

                            // Decode stage: one masked action per pointer slot.
                            // Hover/scroll/button actions decode to `None` and
                            // are skipped so the OS can route them elsewhere.
                            let kind = motion_kind(action);
                            let pointer_index = motion_event.pointer_index();
                            for i in 0..pointer_count {
                                let Some(touch_action) =
                                    crate::event_stages::decode_motion_slot(
                                        kind,
                                        pointer_index,
                                        i,
                                    )
                                else {
                                    continue;
                                };
                                let pointer = motion_event.pointer_at_index(i);

                                let touch = crate::android::TouchPoint {
                                    id: pointer.pointer_id(),
                                    x: pointer.x(),
                                    y: pointer.y(),
                                    action: touch_action,
                                };

                                log::debug!(
                                    "process_input_events: dispatching touch id={} x={:.0} y={:.0} action={}",
                                    touch.id, touch.x, touch.y, touch.action,
                                );

                                win.handle_touch(touch);
                            }

                            android_activity::InputStatus::Handled
                        }
                        InputEvent::KeyEvent(key_event) => {
                            use android_activity::input::KeyAction;
                            use crate::event_stages::{KeyDirection, decode_key_action};

                            let direction = match key_event.action() {
                                KeyAction::Down => KeyDirection::Press,
                                KeyAction::Up => KeyDirection::Release,
                                _ => return android_activity::InputStatus::Unhandled,
                            };
                            // Single source of truth for the ACTION_DOWN = 0 /
                            // ACTION_UP = 1 encoding stored in AndroidKeyEvent.
                            let action = direction.action_int();
                            debug_assert_eq!(decode_key_action(action), Some(direction));

                            let key_code: u32 = key_event.key_code().into();
                            let meta_state: u32 = key_event.meta_state().0;

                            let unicode_char = unicode_char_for_key_event(
                                key_code as i32,
                                action,
                                meta_state as i32,
                            );

                            if unicode_char != 0 {
                                log::trace!(
                                    "dispatch_key_event: code={} action={} meta={:#x} → unicode=U+{:04X}",
                                    key_code,
                                    action,
                                    meta_state,
                                    unicode_char
                                );
                            }

                            let key_event = crate::android::AndroidKeyEvent {
                                key_code: key_code as i32,
                                action,
                                meta_state: meta_state as i32,
                                unicode_char,
                            };

                            win.handle_key_event(key_event);
                            android_activity::InputStatus::Handled
                        }
                        _ => android_activity::InputStatus::Unhandled,
                    }
                });

                if !read_input {
                    break;
                }
            }
        }
        Err(err) => {
            log::error!("Failed to get input events iterator: {err:?}");
        }
    }
}

// ── main event loop ───────────────────────────────────────────────────────────

/// Frame cadence while momentum scrolling or a fling animation is active.
/// The tested twin lives in `crate::event_stages`.
const FRAME_POLL_INTERVAL: Duration = Duration::from_millis(8);

fn event_loop_poll_timeout(
    needs_frame_pump: bool,
    next_delayed_task: Option<Duration>,
) -> Option<Duration> {
    // Tested on every host in `crate::event_stages`; kept as a thin alias so
    // the loop reads in domain terms.
    crate::event_stages::frame_poll_timeout(needs_frame_pump, next_delayed_task)
}

/// Drain lifecycle events queued while a deferred handler ran.
///
/// Java UI-thread callbacks block on a condvar until the native thread drains
/// the looper, so each deferred stage re-polls with a zero timeout before
/// returning. This keeps the condvar wait short and prevents the
/// InputDispatcher ANR (10 s timeout on MotionEvent delivery).
fn drain_main_events(app: &AndroidApp) {
    app.poll_events(Some(Duration::ZERO), |event| {
        if let PollEvent::Main(main_event) = event {
            handle_main_event(app, main_event);
        }
    });
}

/// Deferred `TerminateWindow` stage: drop the renderer/surface, keep the
/// window struct (and its GPUI callbacks) alive for the next `InitWindow`.
fn apply_term_window(iteration: u64) {
    log::info!("deferred: TerminateWindow (iter={})", iteration);
    INIT_WINDOW_DONE.store(false, Ordering::Relaxed);
    *LAST_CHROME_STYLE.lock().unwrap() = None;
    if let Some(platform) = PLATFORM.get() {
        if let Some(win) = platform.primary_window() {
            win.term_window();
        }
    }
}

/// Deferred `InitWindow` stage: bind the new surface onto the existing
/// renderer, or open the primary window when there is none yet.
fn apply_init_window(app: &AndroidApp, attempted_existing_window_init: &mut bool, iteration: u64) {
    log::info!("deferred: InitWindow (iter={})", iteration);
    *attempted_existing_window_init = true;
    if let Some(platform) = PLATFORM.get() {
        if let Some(native_window) = app.native_window() {
            let width = native_window.width();
            let height = native_window.height();
            log::info!("InitWindow: {}×{}", width, height);

            platform.update_primary_display(&native_window, &app.asset_manager());

            let scale_factor = platform
                .primary_display()
                .map(|d| d.scale_factor())
                .unwrap_or(1.0);

            if let Some(existing) = platform.primary_window() {
                let gpu_ctx = platform.gpu_context();
                match existing.init_window(native_window, gpu_ctx) {
                    Ok(()) => {
                        log::info!("InitWindow: reinitialised existing window");
                        existing.set_active(true);
                    }
                    Err(e) => {
                        log::error!("failed to reinit window surface: {e:#}");
                    }
                }

                // Trigger GPUI resize so layout adapts to new dimensions.
                existing.handle_resize();

                let cr = app.content_rect();
                existing.update_safe_area_from_content_rect(cr.left, cr.top, cr.right, cr.bottom);

                INIT_WINDOW_DONE.store(true, Ordering::Relaxed);
            } else {
                match platform.open_window(native_window, scale_factor, false) {
                    Ok(win) => {
                        log::info!(
                            "window opened — id={:#x} scale={:.1}",
                            win.id(),
                            scale_factor
                        );
                        win.set_active(true);

                        let cr = app.content_rect();
                        win.update_safe_area_from_content_rect(
                            cr.left, cr.top, cr.right, cr.bottom,
                        );
                    }
                    Err(e) => {
                        log::error!("failed to open window: {e:#}");
                    }
                }
            }
        }
    }
}

/// Deferred `WindowResized` stage: reconfigure the surface and safe area.
fn apply_window_resized(app: &AndroidApp) {
    log::debug!("deferred: WindowResized");
    if let Some(platform) = PLATFORM.get() {
        if let Some(win) = platform.primary_window() {
            win.handle_resize();
            let cr = app.content_rect();
            win.update_safe_area_from_content_rect(cr.left, cr.top, cr.right, cr.bottom);
        }
    }
}

/// Deferred `ConfigChanged` stage: refresh keyboard layout and dark-mode.
fn apply_config_changed() {
    log::debug!("deferred: ConfigChanged");
    if let Some(platform) = PLATFORM.get() {
        platform.notify_keyboard_layout_change();
        let is_dark = query_night_mode_via_jni();
        if let Some(win) = platform.primary_window() {
            let appearance = if is_dark {
                crate::android::window::WindowAppearance::Dark
            } else {
                crate::android::window::WindowAppearance::Light
            };
            win.set_appearance(appearance);
        }
    }
}

/// Deferred `Pause` stage: mark backgrounded and pause platform views.
fn apply_pause(iteration: u64) {
    log::info!("deferred: Pause (iter={})", iteration);
    if let Some(platform) = PLATFORM.get() {
        platform.did_enter_background();
        if let Some(win) = platform.primary_window() {
            win.set_active(false);
        }
    }
    // Pause platform views
    pause_platform_views();
}

/// Deferred `Resume` stage: mark foregrounded and resume platform views.
fn apply_resume(iteration: u64) {
    log::info!("deferred: Resume (iter={})", iteration);
    if let Some(platform) = PLATFORM.get() {
        platform.did_become_active();
        if let Some(win) = platform.primary_window() {
            win.set_active(true);
        }
    }
    // Resume platform views
    resume_platform_views();
}

/// Sync the loop's view of foreground state with the window.
fn track_active_state(app_is_active: &mut bool, iteration: u64) {
    if let Some(platform) = PLATFORM.get() {
        if let Some(win) = platform.primary_window() {
            let is_active = win.is_active();
            if is_active != *app_is_active {
                log::info!(
                    "run_event_loop: active {} -> {} (iter={})",
                    *app_is_active,
                    is_active,
                    iteration,
                );
                *app_is_active = is_active;
            }
        }
    }
}

/// Run the one-shot finish-launching / init-window callbacks, then pump the
/// first frame immediately so the surface never idles on black.
fn maybe_run_init_callbacks(iteration: u64) {
    if INIT_WINDOW_DONE.load(Ordering::Relaxed) {
        return;
    }
    let Some(platform) = PLATFORM.get() else {
        return;
    };
    if platform.primary_window().is_none() {
        return;
    }
    if let Some(finish_cb) = platform.take_finish_launching_callback() {
        log::info!("invoking finish_launching callback (iter={})", iteration);
        finish_cb();
    }

    if let Some(init_cb) = platform.take_on_init_window_callback() {
        let win = platform.primary_window().unwrap();
        log::info!("invoking on_init_window callback (iter={})", iteration);
        init_cb(win);
    }

    INIT_WINDOW_DONE.store(true, Ordering::Relaxed);
    NATIVE_INITIALIZED.store(true, Ordering::Release);
    log::info!("NATIVE_INITIALIZED = true");

    // Render first frame immediately.
    platform.flush_main_thread_tasks();
    if let Some(win) = platform.primary_window() {
        win.request_frame();
    }
}

/// Render stage: pump one GPUI frame when a wake source fired.
///
/// The gate decision itself (`should_pump_frame`) is pure and unit-tested in
/// `crate::event_stages`; this function only performs the effects. Note that
/// `take_main_thread_wake` is consumed every iteration even when no frame is
/// pumped, so a dispatched task is never lost.
fn pump_frame_if_needed(
    app: &AndroidApp,
    force_frame_after_poll: bool,
    platform_event_woke_frame: bool,
    needs_frame_pump: bool,
    app_is_active: bool,
) {
    let Some(platform) = PLATFORM.get() else {
        return;
    };
    let main_wake = platform.take_main_thread_wake();
    if !crate::event_stages::should_pump_frame(
        INIT_WINDOW_DONE.load(Ordering::Relaxed),
        app_is_active,
        force_frame_after_poll,
        platform_event_woke_frame,
        main_wake,
        needs_frame_pump,
    ) {
        return;
    }
    platform.flush_main_thread_tasks();
    if let Some(win) = platform.primary_window() {
        win.request_frame();
    }

    // Drain lifecycle events that arrived during rendering
    // (e.g. rotation triggers TerminateWindow while we were
    // in get_current_texture / present).
    drain_main_events(app);
    process_input_events(app);
}

/// The event loop that processes `android-activity` events and drives the
/// platform.
///
/// Called from `android_main` after the platform is initialised.
/// Runs until the platform requests quit or the activity is destroyed.
/// Keep momentum scrolling responsive, but otherwise let the Android looper
/// sleep until a platform event, foreground-task wake, or delayed task is due.
///
/// The loop is staged as **poll → decode → dispatch → draw**: lifecycle work
/// is deferred to the `apply_*` stage handlers, input mapping goes through
/// the tested `crate::event_stages` decoders, and frames pump through
/// `pump_frame_if_needed`.
pub fn run_event_loop(app: &AndroidApp) {
    log::info!("run_event_loop: entering main loop");

    // Track whether the on_init_window callback has already been invoked.
    // We do NOT invoke it inside handle_main_event (which runs inside
    // poll_events) because the callback can be heavy (shader compilation,
    // GPUI Application setup).  Running it there blocks the event loop and
    // prevents the system's FocusEvent from being consumed, triggering an
    // ANR after 10 seconds.
    //
    // Instead we check each loop iteration: if a primary window exists and
    // the callback is still pending, invoke it *after* poll_events has
    // returned so focus/input events have already been drained.
    INIT_WINDOW_DONE.store(false, Ordering::Relaxed);
    let mut iteration: u64 = 0;
    let mut last_heartbeat = std::time::Instant::now();
    let mut app_is_active = false;
    let mut attempted_existing_window_init = false;

    loop {
        iteration += 1;

        // Log a heartbeat every 5 seconds so we can tell if the loop is alive.
        let now = std::time::Instant::now();
        if now.duration_since(last_heartbeat) >= Duration::from_secs(5) {
            log::info!(
                "run_event_loop: heartbeat — iteration={}, init_done={}, active={}",
                iteration,
                INIT_WINDOW_DONE.load(Ordering::Relaxed),
                app_is_active,
            );
            last_heartbeat = now;
        }

        // Check if quit was requested.
        if let Some(platform) = PLATFORM.get() {
            if platform.should_quit() {
                platform.run_pending_quit_callback();
                log::info!("run_event_loop: platform requested quit");
                dispose_all_platform_views();
                break;
            }
            platform.tick();
        }

        let (poll_timeout, force_frame_after_poll, needs_frame_pump) = PLATFORM
            .get()
            .map(|platform| {
                let needs_frame_pump = app_is_active
                    && platform
                        .primary_window()
                        .is_some_and(|window| window.needs_frame_pump());
                (
                    event_loop_poll_timeout(
                        needs_frame_pump,
                        platform.next_delayed_task_in(Instant::now()),
                    ),
                    !INIT_WINDOW_DONE.load(Ordering::Relaxed),
                    needs_frame_pump,
                )
            })
            .unwrap_or((None, false, false));
        let mut platform_event_woke_frame = false;

        // ── Poll for events (non-blocking) ──
        //
        // Non-blocking poll: process any pending events then immediately
        // continue to rendering. No sleep — the GPU present call
        // (get_current_texture / Mailbox) provides natural frame pacing.
        app.poll_events(poll_timeout, |event| {
            platform_event_woke_frame |= !matches!(&event, PollEvent::Timeout);
            match event {
                PollEvent::Main(main_event) => {
                    handle_main_event(app, main_event);
                }
                PollEvent::Wake => {}
                PollEvent::Timeout => {}
                _ => {}
            }
        });

        // Some NativeActivity launches can enter `android_main` after the
        // system has already installed the native window, without delivering a
        // fresh InitWindow event to this loop.  Treat a visible native window
        // as an init signal so GPUI can bind a renderer instead of idling on a
        // black surface forever.
        if !attempted_existing_window_init
            && !INIT_WINDOW_DONE.load(Ordering::Relaxed)
            && !INIT_WINDOW_PENDING.load(Ordering::Relaxed)
            && app.native_window().is_some()
        {
            log::info!("run_event_loop: synthesizing InitWindow from existing native window");
            INIT_WINDOW_PENDING.store(true, Ordering::Relaxed);
            attempted_existing_window_init = true;
        }

        // ── Deferred lifecycle processing ──
        //
        // Between stages the loop re-polls with a zero timeout
        // (`drain_main_events`) to drain events the Java UI thread queued
        // while the stage ran. This keeps the condvar wait short and prevents
        // the InputDispatcher ANR (10s timeout on MotionEvent delivery).

        // 1. TerminateWindow — unconfigure surface, release native window
        if TERM_WINDOW_PENDING.swap(false, Ordering::Relaxed) {
            apply_term_window(iteration);
            // Drain immediately — Java thread may be blocked on InitWindow condvar.
            drain_main_events(app);
        }

        // 2. InitWindow — replace surface on existing renderer, or create new
        if INIT_WINDOW_PENDING.swap(false, Ordering::Relaxed) {
            apply_init_window(app, &mut attempted_existing_window_init, iteration);
            // Drain — Java thread may have queued ConfigChanged/WindowResized.
            drain_main_events(app);
        }

        // 3. WindowResized
        if WINDOW_RESIZED_PENDING.swap(false, Ordering::Relaxed) {
            apply_window_resized(app);
        }

        // 4. ConfigChanged
        if CONFIG_CHANGED_PENDING.swap(false, Ordering::Relaxed) {
            apply_config_changed();
        }

        // 5. Pause / background
        if PAUSE_PENDING.swap(false, Ordering::Relaxed) {
            apply_pause(iteration);
        }

        // 6. Resume / foreground
        if RESUME_PENDING.swap(false, Ordering::Relaxed) {
            apply_resume(iteration);
        }

        // Track active/focused state.
        track_active_state(&mut app_is_active, iteration);

        // Process input events.
        process_input_events(app);

        // Deferred initialisation callbacks (runs once).
        maybe_run_init_callbacks(iteration);

        // ── Render ──
        pump_frame_if_needed(
            app,
            force_frame_after_poll,
            platform_event_woke_frame,
            needs_frame_pump,
            app_is_active,
        );
    }

    log::info!("run_event_loop: exiting main loop");
}

/// Pause all platform views when the app goes to background.
fn pause_platform_views() {
    let _ = with_env(|env| {
        if let Ok(helper_class) = find_cached_app_class(env, "dev.gpui.mobile.GpuiPlatformView") {
            let _ = env.call_static_method(
                &helper_class,
                jni::jni_str!("pauseAll"),
                jni::jni_sig!("()V"),
                &[],
            );
            env.exception_clear();
        }
        Ok(())
    });
}

/// Resume all platform views when the app returns to foreground.
fn resume_platform_views() {
    let _ = with_env(|env| {
        if let Ok(helper_class) = find_cached_app_class(env, "dev.gpui.mobile.GpuiPlatformView") {
            let _ = env.call_static_method(
                &helper_class,
                jni::jni_str!("resumeAll"),
                jni::jni_sig!("()V"),
                &[],
            );
            env.exception_clear();
        }
        Ok(())
    });
}

/// Dispose all platform views during app shutdown.
fn dispose_all_platform_views() {
    let _ = with_env(|env| {
        if let Ok(helper_class) = find_cached_app_class(env, "dev.gpui.mobile.GpuiPlatformView") {
            let _ = env.call_static_method(
                &helper_class,
                jni::jni_str!("disposeAll"),
                jni::jni_sig!("()V"),
                &[],
            );
            env.exception_clear();
        }
        Ok(())
    });
}

/// Handle a single `MainEvent` from `android-activity`.
fn handle_main_event(_app: &AndroidApp, event: MainEvent<'_>) {
    match event {
        MainEvent::InitWindow { .. } => {
            log::info!("MainEvent::InitWindow");
            // Defer to after poll_events to avoid deadlock with state lock.
            INIT_WINDOW_PENDING.store(true, Ordering::Relaxed);
        }

        MainEvent::TerminateWindow { .. } => {
            log::info!("MainEvent::TerminateWindow");
            // Defer to after poll_events to avoid deadlock with state lock.
            TERM_WINDOW_PENDING.store(true, Ordering::Relaxed);
        }

        MainEvent::WindowResized { .. } => {
            log::debug!("MainEvent::WindowResized");
            // Defer to after poll_events to avoid deadlock with state lock.
            WINDOW_RESIZED_PENDING.store(true, Ordering::Relaxed);
        }

        MainEvent::GainedFocus => {
            log::info!("MainEvent::GainedFocus");
            RESUME_PENDING.store(true, Ordering::Relaxed);
        }

        MainEvent::LostFocus => {
            log::info!("MainEvent::LostFocus");
            PAUSE_PENDING.store(true, Ordering::Relaxed);
        }

        MainEvent::Resume { .. } => {
            log::info!("MainEvent::Resume");
            RESUME_PENDING.store(true, Ordering::Relaxed);
        }

        MainEvent::Pause => {
            log::info!("MainEvent::Pause");
            // set_active uses AtomicBool so it never blocks.
            PAUSE_PENDING.store(true, Ordering::Relaxed);
        }

        MainEvent::ConfigChanged { .. } => {
            log::debug!("MainEvent::ConfigChanged");
            CONFIG_CHANGED_PENDING.store(true, Ordering::Relaxed);
        }

        MainEvent::Start => {
            log::info!("MainEvent::Start");
        }

        MainEvent::Stop => {
            log::info!("MainEvent::Stop");
        }

        MainEvent::SaveState { .. } => {
            log::info!("MainEvent::SaveState");
        }

        MainEvent::LowMemory => {
            log::warn!("MainEvent::LowMemory — consider releasing cached resources");
        }

        MainEvent::Destroy => {
            log::info!("MainEvent::Destroy");

            if let Some(platform) = PLATFORM.get() {
                platform.quit();
            }
        }

        MainEvent::InsetsChanged { .. } => {
            log::debug!("MainEvent::InsetsChanged");
            WINDOW_RESIZED_PENDING.store(true, Ordering::Relaxed);
        }

        MainEvent::ContentRectChanged { .. } => {
            log::debug!("MainEvent::ContentRectChanged");
            WINDOW_RESIZED_PENDING.store(true, Ordering::Relaxed);
        }

        _ => {
            log::trace!("MainEvent: other variant");
        }
    }
}

// ── main loop helper (compat with existing code) ──────────────────────────────

/// Run one iteration of the event loop.
///
/// This is a compatibility wrapper for code that uses a manual poll loop.
/// Prefer `run_event_loop` for the standard event loop.
///
/// `timeout_ms` — how long to block waiting for events (milliseconds).
/// Pass `0` for non-blocking, `-1` to block indefinitely.
///
/// Returns `true` if the application should exit.
pub fn poll_events(timeout_ms: i32) -> bool {
    if let Some(platform) = PLATFORM.get() {
        if platform.should_quit() {
            return true;
        }
        platform.tick();
    }

    let app = match ANDROID_APP.get() {
        Some(app) => app,
        None => return false,
    };

    let timeout = if timeout_ms < 0 {
        None
    } else {
        Some(Duration::from_millis(timeout_ms as u64))
    };

    app.poll_events(timeout, |event| match event {
        PollEvent::Main(main_event) => {
            handle_main_event(app, main_event);
        }
        PollEvent::Wake => {}
        _ => {}
    });

    process_input_events(app);

    // Drive the GPUI rendering pipeline (same as run_event_loop).
    if let Some(platform) = PLATFORM.get() {
        platform.flush_main_thread_tasks();
        if let Some(win) = platform.primary_window() {
            win.request_frame();
        }
    }

    false
}

// ── public init / run helpers ─────────────────────────────────────────────────

/// Install a panic hook that routes panic messages to logcat.
///
/// Call this early in `android_main` so that any subsequent panic is
/// visible via `adb logcat`.  Safe to call multiple times — each call
/// replaces the previous hook.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any>".to_string()
        };
        if let Some(loc) = info.location() {
            log::error!(
                "PANIC at {}:{}:{}: {}",
                loc.file(),
                loc.line(),
                loc.column(),
                payload
            );
        } else {
            log::error!("PANIC: {}", payload);
        }
    }));
}

/// Store the `AndroidApp` globally and create the `AndroidPlatform`.
///
/// Must be called exactly once from `android_main` before
/// `run_event_loop`.  Returns a reference to the platform so the caller
/// can register callbacks (e.g. `set_on_init_window`) before entering
/// the event loop.
pub fn init_platform(app: &AndroidApp) -> &'static Arc<AndroidPlatform> {
    let _ = ANDROID_APP.set(app.clone());
    log::info!("init_platform: stored AndroidApp");

    let platform = Arc::new(AndroidPlatform::new(false));
    log::info!("init_platform: AndroidPlatform created");

    PLATFORM
        .set(Arc::clone(&platform))
        .unwrap_or_else(|_| log::warn!("PLATFORM already set — duplicate init_platform?"));

    // SAFETY: we just set it above.
    PLATFORM.get().unwrap()
}

// ── system chrome (status bar / navigation bar) ───────────────────────────────

/// Cached last-applied system chrome style.
///
/// `set_system_chrome` is called on every frame render.  The JNI calls it
/// makes (getWindow, setStatusBarColor, etc.) are View operations that can
/// contend with the Android UI thread and intermittently deadlock.
/// By caching the last applied style we skip the JNI calls entirely when
/// nothing changed — which is the common case.
#[allow(clippy::type_complexity)]
static LAST_CHROME_STYLE: std::sync::Mutex<
    Option<(Option<u32>, Option<u32>, crate::StatusBarContentStyle)>,
> = std::sync::Mutex::new(None);

/// Apply system chrome styling on Android.
///
/// Sets the status bar color, navigation bar color, and light/dark
/// status bar icons via JNI calls to `Window` and `WindowInsetsController`.
///
/// Must be called from the main (native) thread that has JNI access.
pub fn set_system_chrome(style: &crate::SystemChromeStyle) {
    let status_bar_color = style.status_bar_color;
    let navigation_bar_color = style.navigation_bar_color;
    let status_bar_style = style.status_bar_style;

    // Skip the (expensive, potentially deadlocking) JNI calls when nothing changed.
    {
        let key = (status_bar_color, navigation_bar_color, status_bar_style);
        let mut last = LAST_CHROME_STYLE.lock().unwrap();
        if *last == Some(key) {
            return;
        }
        *last = Some(key);
    }

    let result = with_env(|env| {
        let activity_obj = activity(env)?;

        // 1. Get the Window: activity.getWindow()
        let window = env
            .call_method(
                &activity_obj,
                jni::jni_str!("getWindow"),
                jni::jni_sig!("()Landroid/view/Window;"),
                &[],
            )
            .and_then(|v: jni::objects::JValueOwned| v.l())
            .map_err(|e| {
                env.exception_clear();
                e.to_string()
            })?;
        if window.is_null() {
            return Err("getWindow returned null".into());
        }

        // 2. Set status bar color if provided
        if let Some(color) = status_bar_color {
            let argb = (0xFF000000_u32 | color) as i32;
            let _ = env.call_method(
                &window,
                jni::jni_str!("setStatusBarColor"),
                jni::jni_sig!("(I)V"),
                &[JValue::Int(argb)],
            );
            env.exception_clear();
        }

        // 3. Set navigation bar color if provided
        if let Some(color) = navigation_bar_color {
            let argb = (0xFF000000_u32 | color) as i32;
            let _ = env.call_method(
                &window,
                jni::jni_str!("setNavigationBarColor"),
                jni::jni_sig!("(I)V"),
                &[JValue::Int(argb)],
            );
            env.exception_clear();
        }

        // 4. Set light/dark status bar icons via WindowInsetsController (API 30+)
        let insetsctl = env.call_method(
            &window,
            jni::jni_str!("getInsetsController"),
            jni::jni_sig!("()Landroid/view/WindowInsetsController;"),
            &[],
        );

        if let Ok(v) = insetsctl {
            if let Ok(ctl) = v.l() {
                if !ctl.is_null() {
                    let mask: i32 = 0x00000008;
                    let appearance: i32 = match status_bar_style {
                        crate::StatusBarContentStyle::Dark => 0x00000008,
                        crate::StatusBarContentStyle::Light => 0,
                    };
                    let _ = env.call_method(
                        &ctl,
                        jni::jni_str!("setSystemBarsAppearance"),
                        jni::jni_sig!("(II)V"),
                        &[JValue::Int(appearance), JValue::Int(mask)],
                    );
                    env.exception_clear();
                }
            }
        } else {
            env.exception_clear();

            if let Ok(decor) = env
                .call_method(
                    &window,
                    jni::jni_str!("getDecorView"),
                    jni::jni_sig!("()Landroid/view/View;"),
                    &[],
                )
                .and_then(|v: jni::objects::JValueOwned| v.l())
            {
                if !decor.is_null() {
                    if let Ok(current) = env
                        .call_method(
                            &decor,
                            jni::jni_str!("getSystemUiVisibility"),
                            jni::jni_sig!("()I"),
                            &[],
                        )
                        .and_then(|v: jni::objects::JValueOwned| v.i())
                    {
                        let new_flags = match status_bar_style {
                            crate::StatusBarContentStyle::Dark => current | 0x00002000,
                            crate::StatusBarContentStyle::Light => current & !0x00002000,
                        };
                        let _ = env.call_method(
                            &decor,
                            jni::jni_str!("setSystemUiVisibility"),
                            jni::jni_sig!("(I)V"),
                            &[JValue::Int(new_flags)],
                        );
                        env.exception_clear();
                    }
                }
            }
        }

        Ok(())
    });

    if let Err(e) = result {
        log::warn!("set_system_chrome: {e}");
    }

    log::info!(
        "set_system_chrome: status_bar_color={:?}, nav_bar_color={:?}, style={:?}",
        style.status_bar_color,
        style.navigation_bar_color,
        style.status_bar_style
    );
}

// ── software keyboard (IME) control ───────────────────────────────────────────

/// Show the software keyboard on Android.
///
/// The activity owns a real editor view and dispatches the UI-thread work.
pub fn show_keyboard_android(keyboard_type: crate::KeyboardType) {
    let input_type = match keyboard_type {
        crate::KeyboardType::Default => 1,
        crate::KeyboardType::EmailAddress => 33,
        crate::KeyboardType::Phone => 3,
        crate::KeyboardType::NumberPad => 2,
        crate::KeyboardType::URL => 17,
        crate::KeyboardType::Decimal => 8194,
    };
    let result = with_env(|env| {
        let activity = activity(env)?;
        env.call_method(
            &activity,
            jni::jni_str!("gpuiShowKeyboard"),
            jni::jni_sig!("(I)V"),
            &[JValue::Int(input_type)],
        )
        .e()?;
        Ok(())
    });
    if let Err(error) = result {
        log::warn!("gpuiShowKeyboard failed: {error}");
    }
}

/// Hide the software keyboard on Android.
///
pub fn hide_keyboard_android() {
    let result = with_env(|env| {
        let activity = activity(env)?;
        env.call_method(
            &activity,
            jni::jni_str!("gpuiHideKeyboard"),
            jni::jni_sig!("()V"),
            &[],
        )
        .e()?;
        Ok(())
    });
    if let Err(error) = result {
        log::warn!("gpuiHideKeyboard failed: {error}");
    }
}

pub fn notify_accessibility_changed() {
    let result = with_env(|env| {
        let activity = activity(env)?;
        env.call_method(
            &activity,
            jni::jni_str!("gpuiAccessibilityChanged"),
            jni::jni_sig!("()V"),
            &[],
        )
        .e()?;
        Ok(())
    });
    if let Err(error) = result {
        log::warn!("gpuiAccessibilityChanged failed: {error}");
    }
}

/// Reset the Android IME's bounded local context after GPUI switches the
/// focused input handler. The shadow never determines Rust-side edit ranges.
pub fn reset_ime_shadow() {
    let result = with_env(|env| {
        let activity = activity(env)?;
        env.call_method(
            &activity,
            jni::jni_str!("gpuiResetInputShadow"),
            jni::jni_sig!("()V"),
            &[],
        )
        .e()?;
        Ok(())
    });
    if let Err(error) = result {
        log::warn!("gpuiResetInputShadow failed: {error}");
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

// ── JNI export for GpuiActivity splash screen ────────────────────────────────

/// Called from `GpuiActivity.nativeIsInitialized()` to check whether the
/// native library has finished initializing (first frame rendered).
///
/// The AndroidX SplashScreen API calls this via `setKeepOnScreenCondition`
/// to hold the splash visible until GPUI is ready.
///
/// # Safety
/// Must only be called from the JVM on a valid JNI thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiActivity_nativeIsInitialized(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
) -> u8 {
    if NATIVE_INITIALIZED.load(Ordering::Acquire) {
        1 // JNI_TRUE
    } else {
        0 // JNI_FALSE
    }
}

/// JNI bridge: receive a deeplink URL from `GpuiActivity.onNewIntent()`.
///
/// When the app is already running and a deeplink is opened (e.g. via
/// `adb shell am start -d gpui://video_player`), the Java side calls
/// this to notify the Rust deeplink handler.
///
/// # Safety
/// Must only be called from the JVM on a valid JNI thread with a valid `url` jobject.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiActivity_nativeOnDeepLink(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    url: *mut std::ffi::c_void,
) {
    // We already have a JVM attached on this thread (UI thread).
    // Use with_env to get a properly wrapped Env handle.
    let url_raw = url as jni::sys::jobject;
    let _ = with_env(|env| {
        let url_obj = unsafe { JObject::from_raw(env, url_raw) };
        let url_string = get_string(env, &url_obj);
        // Don't let the JObject be dropped (it's owned by the JNI call frame).
        if url_string.is_empty() {
            return Ok(());
        }
        log::info!("nativeOnDeepLink: {}", url_string);

        #[cfg(feature = "deeplink")]
        {
            crate::packages::deeplink::notify_deep_link(&url_string);
        }
        Ok(())
    });
}

fn enqueue_string_ime_event(
    value: *mut std::ffi::c_void,
    make_event: impl FnOnce(String) -> crate::ImeEvent,
) {
    let value_raw = value as jni::sys::jobject;
    let queued = with_env(|env| {
        let value = unsafe { JObject::from_raw(env, value_raw) };
        crate::enqueue_ime_event(make_event(get_string(env, &value)));
        Ok(())
    })
    .is_ok();
    if queued && let Some(platform) = PLATFORM.get() {
        // IME callbacks can arrive while the native loop is idle. Wake its
        // registered looper pipe so the next frame drains the queued event.
        platform
            .foreground_executor()
            .dispatch_on_main_thread(|| {});
    }
}

/// Commit finalized text from Android's `InputConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiActivity_nativeCommitText(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    text: *mut std::ffi::c_void,
) {
    enqueue_string_ime_event(text, crate::ImeEvent::Commit);
}

/// Update the active marked/composing text.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiActivity_nativeSetComposingText(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    text: *mut std::ffi::c_void,
) {
    enqueue_string_ime_event(text, crate::ImeEvent::SetComposing);
}

/// Finish the active marked-text session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiActivity_nativeFinishComposingText(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
) {
    crate::enqueue_ime_event(crate::ImeEvent::FinishComposing);
}

/// Delete UTF-16 code units around the current selection.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiActivity_nativeDeleteSurroundingText(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    before: i32,
    after: i32,
) {
    crate::enqueue_ime_event(crate::ImeEvent::DeleteSurrounding {
        before: before.max(0) as usize,
        after: after.max(0) as usize,
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiActivity_nativeAccessibilitySnapshot(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
) -> *mut std::ffi::c_void {
    let snapshot = crate::accessibility::snapshot();
    let mut result = std::ptr::null_mut();
    let _ = with_env(|env| {
        result = env.new_string(snapshot).e()?.into_raw().cast();
        Ok(())
    });
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiActivity_nativeAccessibilityAction(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    node_id: i64,
    action: i32,
) -> u8 {
    u8::from(node_id >= 0 && crate::accessibility::perform_action(node_id as u64, action))
}

/// JNI bridge: receive a media action from `GpuiMediaSession` system controls.
///
/// Actions: "play", "pause", "stop", "next", "previous"
///
/// # Safety
/// Must only be called from the JVM on a valid JNI thread with a valid `action` jobject.
#[cfg(feature = "media_session")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiMediaSession_nativeMediaAction(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    action: *mut std::ffi::c_void,
) {
    let action_raw = action as jni::sys::jobject;
    let _ = with_env(|env| {
        let action_obj = unsafe { JObject::from_raw(env, action_raw) };
        let action_str = get_string(env, &action_obj);

        let Some(media_action) = crate::packages::media_session::parse_media_action(&action_str)
        else {
            log::warn!("nativeMediaAction: unknown action '{action_str}'");
            return Ok(());
        };

        log::info!("nativeMediaAction: {:?}", media_action);
        crate::packages::media_session::notify_action(media_action);
        Ok(())
    });
}

/// JNI bridge: receive a seek request from `GpuiMediaSession` system controls.
///
/// # Safety
/// Must only be called from the JVM on a valid JNI thread.
#[cfg(feature = "media_session")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_dev_gpui_mobile_GpuiMediaSession_nativeMediaSeek(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    position_ms: i64,
) {
    log::info!("nativeMediaSeek: {}ms", position_ms);
    crate::packages::media_session::notify_seek(position_ms as u64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_loop_blocks_without_frame_or_delayed_work() {
        assert_eq!(event_loop_poll_timeout(false, None), None);
        assert_eq!(
            event_loop_poll_timeout(false, Some(Duration::from_millis(25))),
            Some(Duration::from_millis(25))
        );
        assert_eq!(
            event_loop_poll_timeout(true, Some(Duration::from_secs(1))),
            Some(FRAME_POLL_INTERVAL)
        );
    }

    #[test]
    fn poll_events_returns_false_when_no_platform() {
        // PLATFORM is not set in a unit-test context, so poll_events should
        // be a safe no-op and return false (don't quit).
        let result = poll_events(0);
        let _ = result;
    }

    #[test]
    fn java_vm_returns_null_before_init() {
        // Before android_main is called, java_vm() should return null.
        let vm = java_vm();
        assert!(vm.is_null());
    }

    #[test]
    fn activity_as_ptr_returns_null_before_init() {
        let ptr = activity_as_ptr();
        assert!(ptr.is_null());
    }

    #[test]
    fn android_app_returns_none_before_init() {
        assert!(android_app().is_none());
    }

    #[test]
    fn platform_returns_none_before_init() {
        assert!(platform().is_none());
    }
}
