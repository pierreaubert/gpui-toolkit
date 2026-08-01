//! Android platform implementation for GPUI.
//!
//! `AndroidPlatform` is the top-level struct that ties together all Android-
//! specific sub-systems and implements the platform-level contract that GPUI
//! expects.  It mirrors the role played by `gpui_linux::LinuxPlatform` /
//! `gpui_linux::HeadlessClient` on the desktop side.
//!
//! ## Architecture
//!
//! ```text
//! AndroidPlatform
//!   ├── AndroidDispatcher  — ALooper foreground queue + Rust thread-pool
//!   ├── AndroidTextSystem  — cosmic-text + swash shaping / rasterisation
//!   ├── WgpuContext        — shared wgpu device + queue (lazily initialised)
//!   ├── WindowList         — live AndroidWindow instances
//!   └── DisplayList        — connected AndroidDisplay instances
//! ```
//!
//! ## Lifecycle
//!
//! 1. `AndroidPlatform::new(headless)` is called from `current_platform()` on
//!    the Android main thread (inside `ANativeActivity_onCreate`).
//! 2. `run(on_finish_launching)` stores the finish-launching callback; it is
//!    invoked as soon as the platform is set up.
//! 3. `APP_CMD_INIT_WINDOW` → `open_window` or `window.init_window()`.
//! 4. `APP_CMD_TERM_WINDOW` → `window.term_window()`.
//! 5. `quit()` sets the shutdown flag; the main loop exits on the next tick.
//!
//! ## No GPUI workspace dependency
//!
//! All GPUI trait / type references are **stubbed out** locally so this file
//! compiles in isolation.  Replace the stub section with `use gpui::*` when
//! building inside the full workspace.

use anyhow::Result;
use futures::channel::oneshot;
use gpui::{
    Action, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle, ForegroundExecutor,
    KeybindingKeystroke, Keymap, Keystroke, Menu, MenuItem, PathPromptOptions, Platform,
    PlatformDisplay, PlatformKeyboardLayout, PlatformKeyboardMapper, PlatformTextSystem,
    PlatformWindow, Task, ThermalState, WindowAppearance, WindowParams,
};
use gpui_wgpu::CosmicTextSystem;
use parking_lot::Mutex;
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use super::{
    AndroidBackend,
    dispatcher::AndroidDispatcher,
    display::{AndroidDisplay, DisplayList},
    window::{AndroidWindow, WindowList},
};
use gpui_wgpu::GpuContext;

type PlatformCallbackId = u64;

#[derive(Default)]
struct PlatformCallbacks {
    finish_launching: Option<Box<dyn FnOnce()>>,
    on_init_window: Option<Box<dyn FnOnce(Arc<AndroidWindow>)>>,
    quit: Option<Box<dyn FnMut()>>,
    reopen: Option<Box<dyn FnMut()>>,
    open_urls: Option<Box<dyn FnMut(Vec<String>)>>,
    keyboard_layout: Option<Box<dyn FnMut()>>,
    thermal_state: Option<Box<dyn FnMut()>>,
}

static NEXT_PLATFORM_CALLBACK_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static PLATFORM_CALLBACKS: RefCell<HashMap<PlatformCallbackId, PlatformCallbacks>> =
        RefCell::new(HashMap::new());
}

/// Android clipboard (thin wrapper over `ClipboardManager` via JNI).
#[derive(Default)]
pub struct AndroidClipboard {
    #[cfg(not(target_os = "android"))]
    contents: Option<String>,
}

impl AndroidClipboard {
    pub fn read(&self) -> Option<String> {
        #[cfg(not(target_os = "android"))]
        {
            self.contents.clone()
        }

        #[cfg(target_os = "android")]
        {
            self.read_native()
                .inspect_err(|error| log::warn!("failed to read Android clipboard: {error}"))
                .ok()
                .flatten()
        }
    }

    pub fn write(&mut self, text: String) {
        #[cfg(not(target_os = "android"))]
        {
            self.contents = Some(text);
        }

        #[cfg(target_os = "android")]
        if let Err(error) = self.write_native(&text) {
            log::warn!("failed to write Android clipboard: {error}");
        }
    }

    pub fn clear(&mut self) {
        #[cfg(not(target_os = "android"))]
        {
            self.contents = None;
        }

        #[cfg(target_os = "android")]
        if let Err(error) = self.write_native("") {
            log::warn!("failed to clear Android clipboard: {error}");
        }
    }

    #[cfg(target_os = "android")]
    fn write_native(&self, text: &str) -> std::result::Result<(), String> {
        use crate::android::jni::{self as jni_helpers, JniExt};
        use jni::objects::JValue;

        jni_helpers::with_env(|env| {
            let activity = jni_helpers::activity(env)?;
            let service_name = env.new_string("clipboard").e()?;
            let clipboard = env
                .call_method(
                    &activity,
                    jni::jni_str!("getSystemService"),
                    jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                    &[JValue::Object(&service_name)],
                )
                .and_then(|value| value.l())
                .e()?;
            if clipboard.is_null() {
                return Err("ClipboardManager is unavailable".into());
            }

            let label = env.new_string("GPUI").e()?;
            let value = env.new_string(text).e()?;
            let clip_data = env
                .call_static_method(
                    jni::jni_str!("android/content/ClipData"),
                    jni::jni_str!("newPlainText"),
                    jni::jni_sig!(
                        "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;"
                    ),
                    &[JValue::Object(&label), JValue::Object(&value)],
                )
                .and_then(|value| value.l())
                .e()?;
            env.call_method(
                &clipboard,
                jni::jni_str!("setPrimaryClip"),
                jni::jni_sig!("(Landroid/content/ClipData;)V"),
                &[JValue::Object(&clip_data)],
            )
            .e()?;
            Ok(())
        })
    }

    #[cfg(target_os = "android")]
    fn read_native(&self) -> std::result::Result<Option<String>, String> {
        use crate::android::jni::{self as jni_helpers, JniExt};
        use jni::objects::JValue;

        jni_helpers::with_env(|env| {
            let activity = jni_helpers::activity(env)?;
            let service_name = env.new_string("clipboard").e()?;
            let clipboard = env
                .call_method(
                    &activity,
                    jni::jni_str!("getSystemService"),
                    jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                    &[JValue::Object(&service_name)],
                )
                .and_then(|value| value.l())
                .e()?;
            if clipboard.is_null() {
                return Ok(None);
            }
            let has_clip = env
                .call_method(
                    &clipboard,
                    jni::jni_str!("hasPrimaryClip"),
                    jni::jni_sig!("()Z"),
                    &[],
                )
                .and_then(|value| value.z())
                .e()?;
            if !has_clip {
                return Ok(None);
            }
            let clip = env
                .call_method(
                    &clipboard,
                    jni::jni_str!("getPrimaryClip"),
                    jni::jni_sig!("()Landroid/content/ClipData;"),
                    &[],
                )
                .and_then(|value| value.l())
                .e()?;
            if clip.is_null() {
                return Ok(None);
            }
            let item = env
                .call_method(
                    &clip,
                    jni::jni_str!("getItemAt"),
                    jni::jni_sig!("(I)Landroid/content/ClipData$Item;"),
                    &[JValue::Int(0)],
                )
                .and_then(|value| value.l())
                .e()?;
            let text = env
                .call_method(
                    &item,
                    jni::jni_str!("coerceToText"),
                    jni::jni_sig!("(Landroid/content/Context;)Ljava/lang/CharSequence;"),
                    &[JValue::Object(&activity)],
                )
                .and_then(|value| value.l())
                .e()?;
            if text.is_null() {
                return Ok(None);
            }
            let string = env
                .call_method(
                    &text,
                    jni::jni_str!("toString"),
                    jni::jni_sig!("()Ljava/lang/String;"),
                    &[],
                )
                .and_then(|value| value.l())
                .e()?;
            Ok(Some(jni_helpers::get_string(env, &string)))
        })
    }
}

/// Android credential store backed by the Android Keystore system.
///
/// Host builds use an in-memory test double because Android framework services
/// are unavailable there. Android builds delegate to the platform Keystore.
#[derive(Default)]
pub struct AndroidCredentialStore {
    #[cfg(not(target_os = "android"))]
    store: std::collections::HashMap<(String, String), Vec<u8>>,
}

impl AndroidCredentialStore {
    pub fn write(&mut self, service: &str, username: &str, password: &[u8]) -> Result<()> {
        #[cfg(not(target_os = "android"))]
        {
            self.store.insert(
                (service.to_string(), username.to_string()),
                password.to_vec(),
            );
            Ok(())
        }

        #[cfg(target_os = "android")]
        {
            self.write_native(&crate::credential_alias(service, username), password)
                .map_err(anyhow::Error::msg)
        }
    }

    pub fn read(&self, service: &str, username: &str) -> Result<Option<Vec<u8>>> {
        #[cfg(not(target_os = "android"))]
        {
            Ok(self
                .store
                .get(&(service.to_string(), username.to_string()))
                .cloned())
        }

        #[cfg(target_os = "android")]
        {
            self.read_native(&crate::credential_alias(service, username))
                .map_err(anyhow::Error::msg)
        }
    }

    pub fn delete(&mut self, service: &str, username: &str) -> Result<()> {
        #[cfg(not(target_os = "android"))]
        {
            self.store
                .remove(&(service.to_string(), username.to_string()));
            Ok(())
        }

        #[cfg(target_os = "android")]
        {
            self.delete_native(&crate::credential_alias(service, username))
                .map_err(anyhow::Error::msg)
        }
    }

    #[cfg(target_os = "android")]
    fn write_native(&self, alias: &str, secret: &[u8]) -> std::result::Result<(), String> {
        use crate::android::jni::{self as jni_helpers, JniExt};
        use jni::objects::JValue;

        jni_helpers::with_env(|env| {
            let activity = jni_helpers::activity(env)?;
            let alias = env.new_string(alias).e()?;
            let secret_array = env.new_byte_array(secret.len()).e()?;
            let signed_secret: &[i8] =
                unsafe { std::slice::from_raw_parts(secret.as_ptr().cast::<i8>(), secret.len()) };
            secret_array.set_region(env, 0, signed_secret).e()?;
            env.call_method(
                &activity,
                jni::jni_str!("gpuiWriteCredential"),
                jni::jni_sig!("(Ljava/lang/String;[B)V"),
                &[JValue::Object(&alias), JValue::Object(&secret_array)],
            )
            .e()?;
            Ok(())
        })
    }

    #[cfg(target_os = "android")]
    fn read_native(&self, alias: &str) -> std::result::Result<Option<Vec<u8>>, String> {
        use crate::android::jni::{self as jni_helpers, JniExt};
        use jni::objects::{JByteArray, JValue};

        jni_helpers::with_env(|env| {
            let activity = jni_helpers::activity(env)?;
            let alias = env.new_string(alias).e()?;
            let value = env
                .call_method(
                    &activity,
                    jni::jni_str!("gpuiReadCredential"),
                    jni::jni_sig!("(Ljava/lang/String;)[B"),
                    &[JValue::Object(&alias)],
                )
                .and_then(|value| value.l())
                .e()?;
            if value.is_null() {
                return Ok(None);
            }
            let array = unsafe { JByteArray::from_raw(env, value.as_raw()) };
            env.convert_byte_array(&array).map(Some).e()
        })
    }

    #[cfg(target_os = "android")]
    fn delete_native(&self, alias: &str) -> std::result::Result<(), String> {
        use crate::android::jni::{self as jni_helpers, JniExt};
        use jni::objects::JValue;

        jni_helpers::with_env(|env| {
            let activity = jni_helpers::activity(env)?;
            let alias = env.new_string(alias).e()?;
            env.call_method(
                &activity,
                jni::jni_str!("gpuiDeleteCredential"),
                jni::jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(&alias)],
            )
            .e()?;
            Ok(())
        })
    }
}

// ── platform state ────────────────────────────────────────────────────────────

struct AndroidPlatformState {
    // ── executors ─────────────────────────────────────────────────────────────
    /// Shared between `background_executor` and `foreground_executor`.
    dispatcher: Arc<AndroidDispatcher>,

    // ── rendering ─────────────────────────────────────────────────────────────
    /// Shared wgpu device + queue.  `None` until the first window is opened.
    gpu_context: GpuContext,

    // ── windows / displays ────────────────────────────────────────────────────
    windows: WindowList,
    displays: DisplayList,

    // ── text ──────────────────────────────────────────────────────────────────
    text_system: Arc<CosmicTextSystem>,

    // ── I/O ───────────────────────────────────────────────────────────────────
    clipboard: AndroidClipboard,
    credentials: AndroidCredentialStore,

    /// Last known Android thermal state.
    thermal_state: ThermalState,

    // ── miscellaneous ─────────────────────────────────────────────────────────
    /// `true` while the app is active (foreground).
    is_active: bool,

    /// Whether the platform is running headless (no real surface).
    headless: bool,

    /// Preferred GPU backend.
    preferred_backend: AndroidBackend,
}

// SAFETY: `AndroidPlatformState` is only ever accessed while holding the
// `Mutex<AndroidPlatformState>` lock, and all GPU work (including any use of
// `GpuContext = Rc<RefCell<Option<WgpuContext>>>`) happens exclusively on the
// Android main thread.  The `Rc` never escapes to another thread; we just need
// `Send` so that `Arc<AndroidPlatform>` satisfies the `Sync` bound required by
// `static OnceLock<Arc<AndroidPlatform>>`.
unsafe impl Send for AndroidPlatformState {}

// ── AndroidPlatform ───────────────────────────────────────────────────────────

/// The GPUI platform implementation for Android.
///
/// Constructed via `current_platform(headless)` in `mod.rs`.
///
/// ## Thread safety
///
/// `AndroidPlatform` uses `Mutex<AndroidPlatformState>` internally.  The outer
/// shell is wrapped in `Rc` (not `Arc`) by the caller, matching GPUI's
/// expectation that the platform is single-threaded at the Rc level.
pub struct AndroidPlatform {
    state: Mutex<AndroidPlatformState>,
    /// Set to `true` when `quit()` is called; the main loop checks this.
    should_quit: AtomicBool,
    /// Set when `quit()` is requested from another thread. The callback is
    /// then delivered by the Android main event loop.
    quit_callback_pending: AtomicBool,
    callback_id: PlatformCallbackId,
    callback_thread: std::thread::ThreadId,
}

/// Check whether a TrueType/OpenType font file contains CBDT (Color Bitmap
/// Data Table) tables — the bitmap emoji format that swash can render.
///
/// Returns `false` for COLR-only fonts (v0 or v1), SVG-only fonts, or if the
/// file is too small / malformed to parse the table directory.
fn font_has_cbdt_tables(data: &[u8]) -> bool {
    // Minimum size: 12-byte offset table header.
    if data.len() < 12 {
        return false;
    }
    let num_tables = u16::from_be_bytes([data[4], data[5]]) as usize;
    // Each table record is 16 bytes, starting at offset 12.
    let table_dir_end = 12 + num_tables * 16;
    if data.len() < table_dir_end {
        return false;
    }
    for i in 0..num_tables {
        let offset = 12 + i * 16;
        if &data[offset..offset + 4] == b"CBDT" {
            return true;
        }
    }
    false
}

/// Load a file from the Android APK assets directory via the NDK
/// `AAssetManager`.
///
/// `path` is relative to the `assets/` directory in the APK (e.g.
/// `"fonts/NotoColorEmoji.ttf"`).
fn load_asset_bytes(app: &android_activity::AndroidApp, path: &str) -> anyhow::Result<Vec<u8>> {
    use std::ffi::CString;

    let asset_manager = app.asset_manager();
    let c_path = CString::new(path)?;

    // Open the asset.  `AssetManager::open` returns `Option<Asset>`.
    let mut asset = asset_manager
        .open(&c_path)
        .ok_or_else(|| anyhow::anyhow!("asset not found: {path}"))?;

    // `Asset::buffer` returns `io::Result<&[u8]>` — the entire file mapped
    // into memory.
    let bytes = asset
        .buffer()
        .map_err(|e| anyhow::anyhow!("failed to read asset buffer for {path}: {e}"))?;

    Ok(bytes.to_vec())
}

impl AndroidPlatform {
    // ── constructors ─────────────────────────────────────────────────────────

    /// Create a new `AndroidPlatform`.
    ///
    /// `headless` — when `true`, no real `ANativeWindow` / wgpu surface is
    /// created.  Useful for off-screen rendering and unit tests.
    ///
    /// # Panics
    ///
    /// Panics if called from a thread that is not the Android main thread
    /// (i.e. `ALooper_forThread()` returns null) — unless `headless` is `true`.
    pub fn new(headless: bool) -> Self {
        let callback_id = NEXT_PLATFORM_CALLBACK_ID.fetch_add(1, Ordering::Relaxed);
        PLATFORM_CALLBACKS.with(|callbacks| {
            callbacks
                .borrow_mut()
                .insert(callback_id, PlatformCallbacks::default());
        });

        crate::android::init_logger();

        // Build a dispatcher appropriate for the current context.
        let dispatcher = if headless {
            // In headless mode we create a minimal dispatcher without a real
            // ALooper.  Background tasks still work; foreground dispatch is
            // handled by `flush_main_thread_tasks`.
            AndroidDispatcher::new_headless()
        } else {
            AndroidDispatcher::new()
        };

        let text_system = Arc::new(CosmicTextSystem::new("Roboto"));

        // Load Android system fonts so GPUI's fallback chain can resolve.
        // GPUI tries ".SystemUIFont" → ".ZedMono" → … → "Noto Sans" → "DejaVu Sans" → "Arial".
        // Android ships Roboto and DroidSans in /system/fonts/.  We load them
        // explicitly because cosmic-text's fontdb may not scan Android paths.
        {
            let font_paths: &[&str] = &[
                // Core UI fonts
                "/system/fonts/Roboto-Regular.ttf",
                "/system/fonts/Roboto-Bold.ttf",
                "/system/fonts/Roboto-Italic.ttf",
                "/system/fonts/Roboto-BoldItalic.ttf",
                "/system/fonts/Roboto-Medium.ttf",
                "/system/fonts/Roboto-Light.ttf",
                "/system/fonts/Roboto-Thin.ttf",
                "/system/fonts/RobotoFlex-Regular.ttf",
                // Monospace
                "/system/fonts/DroidSans.ttf",
                "/system/fonts/DroidSans-Bold.ttf",
                "/system/fonts/DroidSansMono.ttf",
                "/system/fonts/RobotoMono-Regular.ttf",
                // Noto Sans (wide Unicode coverage)
                "/system/fonts/NotoSans-Regular.ttf",
                "/system/fonts/NotoSans-Bold.ttf",
                "/system/fonts/NotoSans-Italic.ttf",
                "/system/fonts/NotoSans-BoldItalic.ttf",
                "/system/fonts/NotoSansCJK-Regular.ttc",
                "/system/fonts/NotoSansDevanagari-Regular.otf",
                "/system/fonts/NotoSansArabic-Regular.ttf",
                "/system/fonts/NotoSansHebrew-Regular.ttf",
                "/system/fonts/NotoSansThai-Regular.ttf",
                // Emoji fonts — loaded so glyphs like 🦀 ✅ ❤️ render.
                // NOTE: On Android 13+ (API 33+) the system NotoColorEmoji.ttf
                // uses COLR v1 color outlines which swash cannot render.  We
                // detect this below and load a bundled CBDT-based fallback
                // instead.  The flag font is still useful regardless.
                "/system/fonts/NotoColorEmojiFlags.ttf",
                // Serif (fallback)
                "/system/fonts/NotoSerif-Regular.ttf",
                "/system/fonts/NotoSerif-Bold.ttf",
            ];
            let mut font_data: Vec<std::borrow::Cow<'static, [u8]>> = Vec::new();
            for path in font_paths {
                match std::fs::read(path) {
                    Ok(bytes) => {
                        log::info!("loaded system font: {path} ({} bytes)", bytes.len());
                        font_data.push(std::borrow::Cow::Owned(bytes));
                    }
                    Err(e) => {
                        log::debug!("skipping system font {path}: {e}");
                    }
                }
            }

            // ── Emoji font: prefer CBDT (bitmap) over COLR v1 ───────────
            //
            // swash can render CBDT/CBLC bitmap emoji and COLR v0 colour
            // outlines, but NOT COLR v1 (used by Android 13+ / API 33+).
            // If the system font lacks CBDT tables we load a bundled
            // CBDT-based NotoColorEmoji from the APK assets instead.
            let system_emoji_path = "/system/fonts/NotoColorEmoji.ttf";
            let mut emoji_loaded = false;

            if let Ok(system_emoji_bytes) = std::fs::read(system_emoji_path) {
                if font_has_cbdt_tables(&system_emoji_bytes) {
                    log::info!(
                        "system emoji font has CBDT tables — using it ({} bytes)",
                        system_emoji_bytes.len()
                    );
                    font_data.push(std::borrow::Cow::Owned(system_emoji_bytes));
                    emoji_loaded = true;
                } else {
                    log::info!(
                        "system emoji font is COLR v1 (no CBDT) — will try bundled fallback"
                    );
                }
            }

            if !emoji_loaded {
                // Try loading the bundled CBDT NotoColorEmoji from APK assets.
                if let Some(app) = crate::android::jni::android_app() {
                    match load_asset_bytes(&app, "fonts/NotoColorEmoji.ttf") {
                        Ok(bytes) => {
                            log::info!(
                                "loaded bundled CBDT emoji font from assets ({} bytes)",
                                bytes.len()
                            );
                            font_data.push(std::borrow::Cow::Owned(bytes));
                            emoji_loaded = true;
                        }
                        Err(e) => {
                            log::warn!("failed to load bundled emoji font from assets: {e:#}");
                        }
                    }
                } else {
                    log::debug!("no AndroidApp available — cannot load bundled emoji font");
                }
            }

            if !emoji_loaded {
                log::warn!("no compatible emoji font loaded — emoji glyphs may not render");
            }

            if !font_data.is_empty() {
                if let Err(e) = text_system.add_fonts(font_data) {
                    log::warn!("failed to add system fonts: {e:#}");
                }
            } else {
                log::warn!("no system fonts found in /system/fonts/");
            }
        }

        let displays = if headless {
            // Provide a synthetic 1080×1920 display for headless builds.
            DisplayList::single(AndroidDisplay::headless(1080, 1920))
        } else {
            // Real display geometry is filled in when the first
            // `APP_CMD_INIT_WINDOW` arrives.
            DisplayList::single(AndroidDisplay::headless(0, 0))
        };

        log::info!("AndroidPlatform::new — headless={headless}");

        Self {
            state: Mutex::new(AndroidPlatformState {
                dispatcher,
                gpu_context: Rc::new(RefCell::new(None)),
                windows: WindowList::default(),
                displays,
                text_system,
                clipboard: AndroidClipboard::default(),
                credentials: AndroidCredentialStore::default(),
                thermal_state: ThermalState::Nominal,
                is_active: false,
                headless,
                preferred_backend: AndroidBackend::Vulkan,
            }),
            should_quit: AtomicBool::new(false),
            quit_callback_pending: AtomicBool::new(false),
            callback_id,
            callback_thread: std::thread::current().id(),
        }
    }

    fn on_callback_thread(&self) -> bool {
        std::thread::current().id() == self.callback_thread
    }

    fn with_callbacks<R>(&self, f: impl FnOnce(&mut PlatformCallbacks) -> R) -> R {
        assert!(
            self.on_callback_thread(),
            "AndroidPlatform callbacks must be accessed on their owner thread"
        );

        PLATFORM_CALLBACKS.with(|callbacks| {
            let mut callbacks = callbacks.borrow_mut();
            let callbacks = callbacks
                .get_mut(&self.callback_id)
                .expect("AndroidPlatform callback registry entry is missing");
            f(callbacks)
        })
    }

    fn invoke_quit_callback(&self) {
        self.with_callbacks(|callbacks| {
            if let Some(callback) = callbacks.quit.as_mut() {
                callback();
            }
        });
    }

    /// Deliver a quit callback requested from another thread.
    ///
    /// This must be called from the Android main event loop. The callback is
    /// deliberately kept out of the `Send` state shared with other threads.
    pub fn run_pending_quit_callback(&self) {
        if self.quit_callback_pending.swap(false, Ordering::AcqRel) {
            self.invoke_quit_callback();
        }
    }

    // ── executor access ───────────────────────────────────────────────────────

    /// Returns the background-task executor (Rust thread-pool).
    pub fn background_executor(&self) -> Arc<AndroidDispatcher> {
        self.state.lock().dispatcher.clone()
    }

    /// Returns the foreground-task executor (ALooper main-thread queue).
    pub fn foreground_executor(&self) -> Arc<AndroidDispatcher> {
        self.state.lock().dispatcher.clone()
    }

    // ── text system ───────────────────────────────────────────────────────────

    /// Returns the text system.
    pub fn text_system(&self) -> Arc<CosmicTextSystem> {
        self.state.lock().text_system.clone()
    }

    // ── lifecycle ─────────────────────────────────────────────────────────────

    /// Store the `on_finish_launching` callback and invoke it immediately.
    ///
    /// Register a callback to be invoked when `MainEvent::InitWindow` has
    /// successfully created the native window.  The callback receives the
    /// newly created `Arc<AndroidWindow>`.
    ///
    /// This is the primary mechanism for deferred window setup on Android:
    /// instead of calling `cx.open_window(...)` directly (which is not
    /// supported), the application registers its view setup here.
    pub fn set_on_init_window<F>(&self, callback: F)
    where
        F: FnOnce(Arc<AndroidWindow>) + 'static,
    {
        self.with_callbacks(|callbacks| {
            callbacks.on_init_window = Some(Box::new(callback));
        });
    }

    /// Take the `on_init_window` callback (if any).
    ///
    /// Called internally by `handle_main_event` after the window is created.
    pub fn take_on_init_window_callback(&self) -> Option<Box<dyn FnOnce(Arc<AndroidWindow>)>> {
        self.with_callbacks(|callbacks| callbacks.on_init_window.take())
    }

    /// Store the finish-launching callback and drive the event loop.
    ///
    /// On Android, `Platform::run` **blocks** until the app quits — matching
    /// the semantics of macOS / Linux.  This keeps the `Application` (and its
    /// internal `Rc<RefCell<AppContext>>`) alive on the caller's stack for the
    /// entire lifetime of the event loop.
    ///
    /// The stored callback is invoked once the native window is ready (i.e.
    /// after `MainEvent::InitWindow` has been processed and an
    /// `AndroidWindow` exists).  At that point `cx.open_window(...)` will
    /// find the existing primary window.
    pub fn run(&self, on_finish_launching: Box<dyn FnOnce() + 'static>) {
        {
            self.with_callbacks(|callbacks| {
                callbacks.finish_launching = Some(on_finish_launching);
            });
        }
        log::info!("AndroidPlatform::run — callback stored, entering event loop");

        // Drive the event loop.  This blocks until quit() is called or
        // the activity is destroyed.  The finish_launching callback will
        // be invoked from inside the event loop when the window is ready.
        if let Some(app) = super::jni::android_app() {
            super::jni::run_event_loop(&app);
        } else {
            // Headless / test mode — just invoke the callback immediately.
            let cb = self.take_finish_launching_callback();
            if let Some(cb) = cb {
                cb();
            }
        }

        log::info!("AndroidPlatform::run — event loop exited");
    }

    /// Take the stored `on_finish_launching` callback, if any.
    ///
    /// Called by `run_event_loop` when the native window is ready so that
    /// the GPUI `Application` context can open its first window.
    pub fn take_finish_launching_callback(&self) -> Option<Box<dyn FnOnce()>> {
        self.with_callbacks(|callbacks| callbacks.finish_launching.take())
    }

    /// Request a graceful quit.
    ///
    /// Sets the `should_quit` flag; the main loop will exit on the next tick.
    /// Invokes the registered quit callback before returning.
    pub fn quit(&self) {
        log::info!("AndroidPlatform::quit");
        self.should_quit.store(true, Ordering::SeqCst);

        if self.on_callback_thread() {
            self.invoke_quit_callback();
        } else {
            self.quit_callback_pending.store(true, Ordering::Release);
        }
    }

    /// Returns `true` if `quit()` has been called.
    pub fn should_quit(&self) -> bool {
        self.should_quit.load(Ordering::Relaxed)
    }

    /// Called by the app delegate when the app moves to the foreground.
    pub fn did_become_active(&self) {
        log::debug!("AndroidPlatform::did_become_active");
        self.state.lock().is_active = true;
    }

    /// Called by the app delegate when the app moves to the background.
    pub fn did_enter_background(&self) {
        log::debug!("AndroidPlatform::did_enter_background");
        self.state.lock().is_active = false;
    }

    /// Deliver URLs from an implicit or explicit intent.
    pub fn deliver_open_urls(&self, urls: Vec<String>) {
        log::debug!("AndroidPlatform: delivering {} URL(s)", urls.len());
        self.with_callbacks(|callbacks| {
            if let Some(callback) = callbacks.open_urls.as_mut() {
                callback(urls);
            }
        });
    }

    /// Notify the platform that the keyboard layout has changed.
    pub fn notify_keyboard_layout_change(&self) {
        self.with_callbacks(|callbacks| {
            if let Some(callback) = callbacks.keyboard_layout.as_mut() {
                callback();
            }
        });
    }

    /// Deliver a "reopen" event (app tapped in recents while already running).
    pub fn deliver_reopen(&self) {
        self.with_callbacks(|callbacks| {
            if let Some(callback) = callbacks.reopen.as_mut() {
                callback();
            }
        });
    }

    // ── window management ─────────────────────────────────────────────────────

    /// Open a new window backed by `native_window`.
    pub fn open_window(
        &self,
        native_window: ndk::native_window::NativeWindow,
        scale_factor: f32,
        transparent: bool,
    ) -> Result<Arc<AndroidWindow>> {
        let mut state = self.state.lock();
        let window = AndroidWindow::new(
            native_window,
            Rc::clone(&state.gpu_context),
            scale_factor,
            transparent,
        )?;
        state.windows.push(Arc::clone(&window));
        log::info!(
            "AndroidPlatform::open_window — id={:#x} scale={:.1}",
            window.id(),
            scale_factor
        );
        Ok(window)
    }

    /// Remove and return the window identified by `id`.
    pub fn close_window(&self, id: u64) -> Option<Arc<AndroidWindow>> {
        self.state.lock().windows.remove(id)
    }

    /// Returns the primary (first) window, if any.
    pub fn primary_window(&self) -> Option<Arc<AndroidWindow>> {
        self.state.lock().windows.primary().cloned()
    }

    /// Returns the number of live windows.
    pub fn window_count(&self) -> usize {
        self.state.lock().windows.len()
    }

    /// Get a clone of the shared GPU context handle.
    pub fn gpu_context(&self) -> GpuContext {
        Rc::clone(&self.state.lock().gpu_context)
    }

    // ── display management ────────────────────────────────────────────────────

    /// Returns all connected displays.
    pub fn displays(&self) -> Vec<AndroidDisplay> {
        self.state.lock().displays.all().to_vec()
    }

    /// Returns the primary display.
    pub fn primary_display(&self) -> Option<AndroidDisplay> {
        self.state.lock().displays.primary().cloned()
    }

    /// Update the primary display from a new `NativeWindow`.
    ///
    /// Called when `APP_CMD_INIT_WINDOW` delivers a new surface.
    pub fn update_primary_display(
        &self,
        native_window: &ndk::native_window::NativeWindow,
        asset_manager: &ndk::asset::AssetManager,
    ) {
        let display = AndroidDisplay::from_activity(native_window, asset_manager);
        let mut state = self.state.lock();
        state.displays = DisplayList::single(display);
    }

    // ── clipboard ─────────────────────────────────────────────────────────────

    /// Write `text` to the clipboard.
    pub fn write_to_clipboard(&self, text: String) {
        self.state.lock().clipboard.write(text);
    }

    /// Read the current clipboard contents.
    pub fn read_from_clipboard(&self) -> Option<String> {
        self.state.lock().clipboard.read()
    }

    // ── credential store ──────────────────────────────────────────────────────

    /// Store credentials in the Android Keystore.
    pub fn write_credentials(&self, service: &str, username: &str, password: &[u8]) -> Result<()> {
        self.state
            .lock()
            .credentials
            .write(service, username, password)
    }

    /// Read credentials from the Android Keystore.
    pub fn read_credentials(&self, service: &str, username: &str) -> Result<Option<Vec<u8>>> {
        self.state.lock().credentials.read(service, username)
    }

    /// Delete credentials from the Android Keystore.
    pub fn delete_credentials(&self, service: &str, username: &str) -> Result<()> {
        self.state.lock().credentials.delete(service, username)
    }

    // ── misc platform queries ─────────────────────────────────────────────────

    /// Whether the platform is running in headless mode.
    pub fn is_headless(&self) -> bool {
        self.state.lock().headless
    }

    /// Whether the app is currently in the foreground.
    pub fn is_active(&self) -> bool {
        self.state.lock().is_active
    }

    /// Whether scrollbars should auto-hide.
    ///
    /// Android always hides scrollbars after a short delay.
    pub fn should_auto_hide_scrollbars(&self) -> bool {
        true
    }

    /// The path to the current executable / APK.
    ///
    /// On Android this resolves to `/proc/self/exe` (symlink to the process
    /// binary) or falls back to an empty path.
    pub fn app_path(&self) -> Result<PathBuf> {
        let exe = std::fs::read_link("/proc/self/exe").unwrap_or_else(|_| PathBuf::from(""));
        Ok(exe)
    }

    /// Android apps do not have auxiliary executables.
    pub fn path_for_auxiliary_executable(&self, _name: &str) -> Result<PathBuf> {
        anyhow::bail!("auxiliary executables are not supported on Android")
    }

    /// Android apps cannot select mixed files/dirs via the system file picker.
    pub fn can_select_mixed_files_and_dirs(&self) -> bool {
        false
    }

    /// Returns the current keyboard layout identifier.
    ///
    /// On Android this queries the `InputMethodManager` via JNI to get the
    /// current input method's locale/subtype tag (e.g. `"en-US"`).
    /// Falls back to `"en-US"` if the JNI call fails.
    pub fn keyboard_layout_id(&self) -> String {
        self.query_keyboard_layout_id_via_jni()
            .unwrap_or_else(|| "en-US".to_string())
    }

    /// Query the keyboard layout ID from `InputMethodManager` via JNI.
    ///
    /// Calls:
    /// ```java
    /// Context ctx = activity;
    /// InputMethodManager imm = (InputMethodManager) ctx.getSystemService(Context.INPUT_METHOD_SERVICE);
    /// InputMethodSubtype subtype = imm.getCurrentInputMethodSubtype();
    /// if (subtype != null) return subtype.getLocale(); // e.g. "en_US"
    /// ```
    ///
    /// Returns `None` if the JNI environment is unavailable or the call fails.
    fn query_keyboard_layout_id_via_jni(&self) -> Option<String> {
        use crate::android::jni::{self as jni_helpers, get_string};
        use jni::objects::JValue;

        jni_helpers::with_env(|env| {
            let activity = jni_helpers::activity(env)?;

            // activity.getSystemService("input_method")
            let service_name = env.new_string("input_method").map_err(|e| e.to_string())?;
            let imm = env
                .call_method(
                    &activity,
                    jni::jni_str!("getSystemService"),
                    jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                    &[JValue::Object(&service_name)],
                )
                .and_then(|v| v.l())
                .map_err(|e| e.to_string())?;
            if imm.is_null() {
                return Ok(None);
            }

            // imm.getCurrentInputMethodSubtype()
            let subtype = env
                .call_method(
                    &imm,
                    jni::jni_str!("getCurrentInputMethodSubtype"),
                    jni::jni_sig!("()Landroid/view/inputmethod/InputMethodSubtype;"),
                    &[],
                )
                .and_then(|v| v.l())
                .map_err(|e| e.to_string())?;
            if subtype.is_null() {
                return Ok(None);
            }

            // subtype.getLocale()
            let locale_obj = env
                .call_method(
                    &subtype,
                    jni::jni_str!("getLocale"),
                    jni::jni_sig!("()Ljava/lang/String;"),
                    &[],
                )
                .and_then(|v| v.l())
                .map_err(|e| e.to_string())?;

            let result = get_string(env, &locale_obj).replace('_', "-");
            if result.is_empty() {
                Ok(None)
            } else {
                log::debug!("keyboard_layout_id via JNI: {}", result);
                Ok(Some(result))
            }
        })
        .ok()
        .flatten()
    }

    /// Register a thermal status listener via JNI using PowerManager.
    ///
    /// On Android 29+ (API Q), calls:
    /// ```java
    /// PowerManager pm = (PowerManager) activity.getSystemService(Context.POWER_SERVICE);
    /// pm.addThermalStatusListener(executor, listener);
    /// ```
    ///
    /// For simplicity, we poll the thermal status periodically from the main
    /// loop tick rather than setting up a full JNI callback bridge.  The
    /// callback is stored and invoked when the thermal state changes.
    fn register_thermal_listener(&self, callback: Box<dyn FnMut()>) {
        // A fully async approach would use `PowerManager.addThermalStatusListener`
        // with a JNI callback proxy. The polling approach is simpler and avoids
        // the complexity of bridging Java-to-Rust callbacks.
        let current_state = self
            .current_thermal_state_via_jni()
            .unwrap_or(ThermalState::Nominal);
        self.state.lock().thermal_state = current_state;
        self.with_callbacks(|callbacks| {
            callbacks.thermal_state = Some(callback);
        });
        log::debug!("register_thermal_listener: polling thermal state from tick");
    }

    /// Query the current thermal status via JNI.
    ///
    /// Calls `PowerManager.getCurrentThermalStatus()` (API 29+).
    /// Returns the raw int status:
    /// - 0 = THERMAL_STATUS_NONE
    /// - 1 = THERMAL_STATUS_LIGHT
    /// - 2 = THERMAL_STATUS_MODERATE
    /// - 3 = THERMAL_STATUS_SEVERE
    /// - 4 = THERMAL_STATUS_CRITICAL
    /// - 5 = THERMAL_STATUS_EMERGENCY
    /// - 6 = THERMAL_STATUS_SHUTDOWN
    ///
    /// Returns -1 on failure (JNI unavailable, API < 29, etc.).
    #[allow(dead_code)]
    fn query_thermal_status_via_jni(&self) -> i32 {
        use crate::android::jni as jni_helpers;
        use jni::objects::JValue;

        jni_helpers::with_env(|env| {
            let activity = jni_helpers::activity(env)?;

            // activity.getSystemService("power")
            let service_name = env.new_string("power").map_err(|e| e.to_string())?;
            let pm = match env
                .call_method(
                    &activity,
                    jni::jni_str!("getSystemService"),
                    jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                    &[JValue::Object(&service_name)],
                )
                .and_then(|v| v.l())
            {
                Ok(o) if !o.is_null() => o,
                _ => {
                    env.exception_clear();
                    return Err("getSystemService(power) failed or returned null".to_string());
                }
            };

            // pm.getCurrentThermalStatus() — API 29+
            let status = match env
                .call_method(
                    &pm,
                    jni::jni_str!("getCurrentThermalStatus"),
                    jni::jni_sig!("()I"),
                    &[],
                )
                .and_then(|v| v.i())
            {
                Ok(s) => s,
                Err(_) => {
                    env.exception_clear();
                    return Err("getCurrentThermalStatus() failed".to_string());
                }
            };

            log::trace!("query_thermal_status_via_jni: status={}", status);
            Ok(status)
        })
        .unwrap_or(-1)
    }

    fn thermal_state_from_android_status(status: i32) -> Option<ThermalState> {
        match status {
            0 => Some(ThermalState::Nominal),
            1 => Some(ThermalState::Fair),
            2 | 3 => Some(ThermalState::Serious),
            4..=6 => Some(ThermalState::Critical),
            _ => None,
        }
    }

    fn current_thermal_state_via_jni(&self) -> Option<ThermalState> {
        Self::thermal_state_from_android_status(self.query_thermal_status_via_jni())
    }

    fn check_thermal_state(&self) {
        let Some(next_thermal_state) = self.current_thermal_state_via_jni() else {
            return;
        };

        let mut callback = {
            let mut state = self.state.lock();
            if state.thermal_state == next_thermal_state {
                return;
            }

            state.thermal_state = next_thermal_state;
            self.with_callbacks(|callbacks| callbacks.thermal_state.take())
        };

        if let Some(mut callback) = callback.take() {
            callback();

            self.with_callbacks(|callbacks| {
                if callbacks.thermal_state.is_none() {
                    callbacks.thermal_state = Some(callback);
                }
            });
        }
    }

    /// Preferred wgpu backend.
    pub fn preferred_backend(&self) -> AndroidBackend {
        self.state.lock().preferred_backend
    }

    /// Override the preferred wgpu backend.
    pub fn set_preferred_backend(&self, backend: AndroidBackend) {
        self.state.lock().preferred_backend = backend;
    }

    // ── callback registration ─────────────────────────────────────────────────

    /// Register a callback invoked when the app is about to quit.
    pub fn on_quit<F>(&self, cb: F)
    where
        F: FnMut() + 'static,
    {
        self.with_callbacks(|callbacks| {
            callbacks.quit = Some(Box::new(cb));
        });
    }

    /// Register a callback invoked when the app is re-opened.
    pub fn on_reopen<F>(&self, cb: F)
    where
        F: FnMut() + 'static,
    {
        self.with_callbacks(|callbacks| {
            callbacks.reopen = Some(Box::new(cb));
        });
    }

    /// Register a callback invoked when the OS delivers URLs to open.
    pub fn on_open_urls<F>(&self, cb: F)
    where
        F: FnMut(Vec<String>) + 'static,
    {
        self.with_callbacks(|callbacks| {
            callbacks.open_urls = Some(Box::new(cb));
        });
    }

    /// Register a callback invoked when the keyboard layout changes.
    pub fn on_keyboard_layout_change<F>(&self, cb: F)
    where
        F: FnMut() + 'static,
    {
        self.with_callbacks(|callbacks| {
            callbacks.keyboard_layout = Some(Box::new(cb));
        });
    }

    // ── dispatcher tick ───────────────────────────────────────────────────────

    /// Advance the platform's dispatcher by one tick.
    ///
    /// Should be called from the native-activity main loop on every iteration
    /// to process delayed background tasks.
    pub fn tick(&self) {
        let dispatcher = self.state.lock().dispatcher.clone();
        dispatcher.tick();
        self.check_thermal_state();
    }

    /// Drain all pending main-thread tasks synchronously.
    ///
    /// Useful in headless tests where there is no real ALooper.
    pub fn flush_main_thread_tasks(&self) {
        self.state.lock().dispatcher.flush_main_thread_tasks();
    }
}

impl Drop for AndroidPlatform {
    fn drop(&mut self) {
        if self.on_callback_thread() {
            PLATFORM_CALLBACKS.with(|callbacks| {
                callbacks.borrow_mut().remove(&self.callback_id);
            });
        }
    }
}

// ── impl Platform ─────────────────────────────────────────────────────────────
//
// Implementation of the GPUI `Platform` trait for Android.
//
// Methods that have a meaningful Android implementation are wired to the
// existing `AndroidPlatform` helpers.  Desktop-only methods (menus, dock,
// file pickers, etc.) are no-ops or return sensible defaults.

impl Platform for AndroidPlatform {
    fn background_executor(&self) -> BackgroundExecutor {
        let dispatcher: Arc<dyn gpui::PlatformDispatcher> = self.state.lock().dispatcher.clone();
        BackgroundExecutor::new(dispatcher)
    }

    fn foreground_executor(&self) -> ForegroundExecutor {
        let dispatcher: Arc<dyn gpui::PlatformDispatcher> = self.state.lock().dispatcher.clone();
        ForegroundExecutor::new(dispatcher)
    }

    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.state.lock().text_system.clone()
    }

    fn run(&self, on_finish_launching: Box<dyn 'static + FnOnce()>) {
        self.run(on_finish_launching);
    }

    fn quit(&self) {
        AndroidPlatform::quit(self);
    }

    fn restart(&self, _binary_path: Option<PathBuf>) {
        log::warn!("AndroidPlatform::restart — not supported on Android");
    }

    fn activate(&self, _ignoring_other_apps: bool) {
        self.state.lock().is_active = true;
    }

    fn hide(&self) {
        // No-op: Android apps don't "hide" in the macOS sense.
    }

    fn hide_other_apps(&self) {
        // No-op: not applicable on Android.
    }

    fn unhide_other_apps(&self) {
        // No-op: not applicable on Android.
    }

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        let state = self.state.lock();
        state
            .displays
            .all()
            .iter()
            .map(|d| Rc::new(d.clone()) as Rc<dyn PlatformDisplay>)
            .collect()
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        self.state
            .lock()
            .displays
            .primary()
            .map(|d| Rc::new(d.clone()) as Rc<dyn PlatformDisplay>)
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        // Android does not have a window-stack concept — there is at most one
        // active window (the primary window).  GPUI uses `AnyWindowHandle`
        // which requires an entity id we don't have from the platform layer,
        // so return `None` and let the GPUI app context track the active window.
        None
    }

    fn open_window(
        &self,
        _handle: AnyWindowHandle,
        _options: WindowParams,
    ) -> anyhow::Result<Box<dyn PlatformWindow>> {
        // On Android the native window is created by the system via
        // MainEvent::InitWindow, not by the application.  If a window
        // already exists we wrap it in an AndroidPlatformWindow and hand
        // it to GPUI.
        let window = self.primary_window().ok_or_else(|| {
            anyhow::anyhow!(
                "AndroidPlatform::open_window — no native window available yet. \
                 Call this from the on_init_window callback after the surface is ready."
            )
        })?;

        let display = self
            .state
            .lock()
            .displays
            .primary()
            .map(|d| Rc::new(d.clone()) as Rc<dyn PlatformDisplay>);

        Ok(Box::new(super::window::AndroidPlatformWindow::new(
            window, display,
        )))
    }

    fn window_appearance(&self) -> WindowAppearance {
        WindowAppearance::Dark
    }

    fn open_url(&self, url: &str) {
        // A full implementation would call startActivity with an ACTION_VIEW
        // Intent via JNI.  For now, log the request.
        log::info!("AndroidPlatform::open_url({url}) — Intent launch not yet implemented");
    }

    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        self.with_callbacks(|callbacks| {
            callbacks.open_urls = Some(callback);
        });
    }

    fn register_url_scheme(&self, _url: &str) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }

    fn prompt_for_paths(
        &self,
        _options: PathPromptOptions,
    ) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(Ok(None));
        rx
    }

    fn prompt_for_new_path(
        &self,
        _directory: &Path,
        _suggested_name: Option<&str>,
    ) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(Ok(None));
        rx
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        false
    }

    fn reveal_path(&self, _path: &Path) {
        log::info!("AndroidPlatform::reveal_path — not supported on Android");
    }

    fn open_with_system(&self, path: &Path) {
        #[cfg(target_os = "android")]
        {
            use crate::android::jni::{self as jni_helpers, JniExt};
            use jni::objects::JValue;

            let Some(path) = path.to_str() else {
                log::warn!("AndroidPlatform::open_with_system — path is not valid UTF-8");
                return;
            };

            let result = jni_helpers::with_env(|env| {
                let activity = jni_helpers::activity(env)?;
                let path = env.new_string(path).e()?;
                env.call_method(
                    &activity,
                    jni::jni_str!("gpuiOpenWithSystem"),
                    jni::jni_sig!("(Ljava/lang/String;)V"),
                    &[JValue::Object(&path)],
                )
                .e()?;
                Ok(())
            });

            if let Err(error) = result {
                log::warn!("AndroidPlatform::open_with_system failed: {error}");
            }
        }

        #[cfg(not(target_os = "android"))]
        {
            let _ = path;
            log::debug!("AndroidPlatform::open_with_system is only available on Android");
        }
    }

    fn on_quit(&self, callback: Box<dyn FnMut()>) {
        self.with_callbacks(|callbacks| {
            callbacks.quit = Some(callback);
        });
    }

    fn on_reopen(&self, callback: Box<dyn FnMut()>) {
        self.with_callbacks(|callbacks| {
            callbacks.reopen = Some(callback);
        });
    }

    fn set_menus(&self, _menus: Vec<Menu>, _keymap: &Keymap) {
        // No-op: Android doesn't have macOS-style menus.
    }

    fn set_dock_menu(&self, _menu: Vec<MenuItem>, _keymap: &Keymap) {
        // No-op: Android doesn't have a dock.
    }

    fn on_app_menu_action(&self, _callback: Box<dyn FnMut(&dyn Action)>) {
        // No-op: no app menu on Android.
    }

    fn on_will_open_app_menu(&self, _callback: Box<dyn FnMut()>) {
        // No-op.
    }

    fn on_validate_app_menu_command(&self, _callback: Box<dyn FnMut(&dyn Action) -> bool>) {
        // No-op.
    }

    fn thermal_state(&self) -> ThermalState {
        self.state.lock().thermal_state
    }

    fn on_thermal_state_change(&self, callback: Box<dyn FnMut()>) {
        // Poll `PowerManager.getCurrentThermalStatus()` from `tick()` and call
        // back when the mapped GPUI thermal state changes. Devices below API 29
        // or without a JNI environment keep the default nominal state.
        self.register_thermal_listener(callback);
    }

    fn app_path(&self) -> Result<PathBuf> {
        let exe = std::fs::read_link("/proc/self/exe").unwrap_or_else(|_| PathBuf::from(""));
        Ok(exe)
    }

    fn path_for_auxiliary_executable(&self, _name: &str) -> Result<PathBuf> {
        anyhow::bail!("auxiliary executables are not supported on Android")
    }

    fn set_cursor_style(&self, _style: CursorStyle) {
        // No-op: Android uses touch, not mouse cursors.
    }

    fn should_auto_hide_scrollbars(&self) -> bool {
        true
    }

    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        self.state
            .lock()
            .clipboard
            .read()
            .map(ClipboardItem::new_string)
    }

    fn write_to_clipboard(&self, item: ClipboardItem) {
        self.state
            .lock()
            .clipboard
            .write(item.text().unwrap_or_default());
    }

    fn write_credentials(&self, url: &str, username: &str, password: &[u8]) -> Task<Result<()>> {
        let result = (|| {
            let mut state = self.state.lock();
            state.credentials.write(url, username, password)?;
            state
                .credentials
                .write(url, "__gpui_username_index", username.as_bytes())
        })();
        Task::ready(result)
    }

    fn read_credentials(&self, url: &str) -> Task<Result<Option<(String, Vec<u8>)>>> {
        let result = (|| {
            let state = self.state.lock();
            let Some(username) = state
                .credentials
                .read(url, "__gpui_username_index")?
                .map(String::from_utf8)
                .transpose()?
            else {
                return Ok(None);
            };
            Ok(state
                .credentials
                .read(url, &username)?
                .map(|password| (username, password)))
        })();
        Task::ready(result)
    }

    fn delete_credentials(&self, url: &str) -> Task<Result<()>> {
        let result = (|| {
            let mut state = self.state.lock();
            if let Some(username) = state
                .credentials
                .read(url, "__gpui_username_index")?
                .map(String::from_utf8)
                .transpose()?
            {
                state.credentials.delete(url, &username)?;
            }
            state.credentials.delete(url, "__gpui_username_index")
        })();
        Task::ready(result)
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(crate::android::keyboard::AndroidKeyboardLayout::new(
            "en-US",
        ))
    }

    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        Rc::new(AndroidKeyboardMapper)
    }

    fn on_keyboard_layout_change(&self, callback: Box<dyn FnMut()>) {
        self.with_callbacks(|callbacks| {
            callbacks.keyboard_layout = Some(callback);
        });
    }

    fn hide_cursor_until_mouse_moves(&self) {
        // No-op on Android; cursor visibility is not mouse-driven.
    }

    fn is_cursor_visible(&self) -> bool {
        true
    }
}

// ── Android keyboard mapper (stub) ───────────────────────────────────────────

/// Minimal `PlatformKeyboardMapper` for Android.
///
/// On Android there are no macOS-style key equivalents; we pass keystrokes
/// through unchanged.
struct AndroidKeyboardMapper;

impl PlatformKeyboardMapper for AndroidKeyboardMapper {
    fn map_key_equivalent(
        &self,
        keystroke: Keystroke,
        _use_key_equivalents: bool,
    ) -> KeybindingKeystroke {
        KeybindingKeystroke::from_keystroke(keystroke)
    }

    fn get_key_equivalents(&self) -> Option<&HashMap<char, char, rustc_hash::FxBuildHasher>> {
        None
    }
}

// ── SharedPlatform — Rc-friendly wrapper around Arc<AndroidPlatform> ─────────
//
// GPUI's `Application::with_platform` requires `Rc<dyn Platform>`.
// The global `PLATFORM` in `jni` stores `Arc<AndroidPlatform>`.
// `SharedPlatform` bridges the two: it holds `Arc<AndroidPlatform>` and
// implements `Platform` by delegating every call, so it can be wrapped
// in `Rc` and handed to GPUI while sharing the same underlying state.

/// A thin wrapper so that `Arc<AndroidPlatform>` can be used as
/// `Rc<dyn Platform>` by GPUI's `Application::with_platform`.
pub struct SharedPlatform(pub Arc<AndroidPlatform>);

impl SharedPlatform {
    pub fn new(inner: Arc<AndroidPlatform>) -> Self {
        Self(inner)
    }

    /// Convenience: wrap in `Rc<dyn Platform>` for GPUI.
    pub fn into_rc(self) -> Rc<dyn Platform> {
        Rc::new(self)
    }
}

/// Route every `Platform` trait call through to `self.0: Arc<AndroidPlatform>`.
///
/// We use fully-qualified `<AndroidPlatform as Platform>::method(...)` syntax
/// so the compiler never accidentally picks the inherent method (which may
/// have a different return type).
impl Platform for SharedPlatform {
    fn background_executor(&self) -> BackgroundExecutor {
        <AndroidPlatform as Platform>::background_executor(&self.0)
    }
    fn foreground_executor(&self) -> ForegroundExecutor {
        <AndroidPlatform as Platform>::foreground_executor(&self.0)
    }
    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        <AndroidPlatform as Platform>::text_system(&self.0)
    }
    fn run(&self, on_finish_launching: Box<dyn 'static + FnOnce()>) {
        <AndroidPlatform as Platform>::run(&self.0, on_finish_launching)
    }
    fn quit(&self) {
        <AndroidPlatform as Platform>::quit(&self.0)
    }
    fn restart(&self, binary_path: Option<PathBuf>) {
        <AndroidPlatform as Platform>::restart(&self.0, binary_path)
    }
    fn activate(&self, ignoring_other_apps: bool) {
        <AndroidPlatform as Platform>::activate(&self.0, ignoring_other_apps)
    }
    fn hide(&self) {
        <AndroidPlatform as Platform>::hide(&self.0)
    }
    fn hide_other_apps(&self) {
        <AndroidPlatform as Platform>::hide_other_apps(&self.0)
    }
    fn unhide_other_apps(&self) {
        <AndroidPlatform as Platform>::unhide_other_apps(&self.0)
    }
    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        <AndroidPlatform as Platform>::displays(&self.0)
    }
    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        <AndroidPlatform as Platform>::primary_display(&self.0)
    }
    fn active_window(&self) -> Option<AnyWindowHandle> {
        <AndroidPlatform as Platform>::active_window(&self.0)
    }
    fn open_window(
        &self,
        handle: AnyWindowHandle,
        options: WindowParams,
    ) -> anyhow::Result<Box<dyn PlatformWindow>> {
        <AndroidPlatform as Platform>::open_window(&self.0, handle, options)
    }
    fn window_appearance(&self) -> WindowAppearance {
        <AndroidPlatform as Platform>::window_appearance(&self.0)
    }
    fn open_url(&self, url: &str) {
        <AndroidPlatform as Platform>::open_url(&self.0, url)
    }
    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        <AndroidPlatform as Platform>::on_open_urls(&self.0, callback)
    }
    fn register_url_scheme(&self, url: &str) -> Task<Result<()>> {
        <AndroidPlatform as Platform>::register_url_scheme(&self.0, url)
    }
    fn prompt_for_paths(
        &self,
        options: PathPromptOptions,
    ) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        <AndroidPlatform as Platform>::prompt_for_paths(&self.0, options)
    }
    fn prompt_for_new_path(
        &self,
        directory: &Path,
        suggested_name: Option<&str>,
    ) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        <AndroidPlatform as Platform>::prompt_for_new_path(&self.0, directory, suggested_name)
    }
    fn can_select_mixed_files_and_dirs(&self) -> bool {
        <AndroidPlatform as Platform>::can_select_mixed_files_and_dirs(&self.0)
    }
    fn reveal_path(&self, path: &Path) {
        <AndroidPlatform as Platform>::reveal_path(&self.0, path)
    }
    fn open_with_system(&self, path: &Path) {
        <AndroidPlatform as Platform>::open_with_system(&self.0, path)
    }
    fn on_quit(&self, callback: Box<dyn FnMut()>) {
        <AndroidPlatform as Platform>::on_quit(&self.0, callback)
    }
    fn on_reopen(&self, callback: Box<dyn FnMut()>) {
        <AndroidPlatform as Platform>::on_reopen(&self.0, callback)
    }
    fn set_menus(&self, menus: Vec<Menu>, keymap: &Keymap) {
        <AndroidPlatform as Platform>::set_menus(&self.0, menus, keymap)
    }
    fn set_dock_menu(&self, menu: Vec<MenuItem>, keymap: &Keymap) {
        <AndroidPlatform as Platform>::set_dock_menu(&self.0, menu, keymap)
    }
    fn on_app_menu_action(&self, callback: Box<dyn FnMut(&dyn Action)>) {
        <AndroidPlatform as Platform>::on_app_menu_action(&self.0, callback)
    }
    fn on_will_open_app_menu(&self, callback: Box<dyn FnMut()>) {
        <AndroidPlatform as Platform>::on_will_open_app_menu(&self.0, callback)
    }
    fn on_validate_app_menu_command(&self, callback: Box<dyn FnMut(&dyn Action) -> bool>) {
        <AndroidPlatform as Platform>::on_validate_app_menu_command(&self.0, callback)
    }
    fn thermal_state(&self) -> ThermalState {
        <AndroidPlatform as Platform>::thermal_state(&self.0)
    }
    fn on_thermal_state_change(&self, callback: Box<dyn FnMut()>) {
        <AndroidPlatform as Platform>::on_thermal_state_change(&self.0, callback)
    }
    fn app_path(&self) -> Result<PathBuf> {
        <AndroidPlatform as Platform>::app_path(&self.0)
    }
    fn path_for_auxiliary_executable(&self, name: &str) -> Result<PathBuf> {
        <AndroidPlatform as Platform>::path_for_auxiliary_executable(&self.0, name)
    }
    fn set_cursor_style(&self, style: CursorStyle) {
        <AndroidPlatform as Platform>::set_cursor_style(&self.0, style)
    }
    fn should_auto_hide_scrollbars(&self) -> bool {
        <AndroidPlatform as Platform>::should_auto_hide_scrollbars(&self.0)
    }
    fn write_to_clipboard(&self, item: ClipboardItem) {
        <AndroidPlatform as Platform>::write_to_clipboard(&self.0, item)
    }
    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        <AndroidPlatform as Platform>::read_from_clipboard(&self.0)
    }
    fn write_credentials(&self, url: &str, username: &str, password: &[u8]) -> Task<Result<()>> {
        <AndroidPlatform as Platform>::write_credentials(&self.0, url, username, password)
    }
    fn read_credentials(&self, url: &str) -> Task<Result<Option<(String, Vec<u8>)>>> {
        <AndroidPlatform as Platform>::read_credentials(&self.0, url)
    }
    fn delete_credentials(&self, url: &str) -> Task<Result<()>> {
        <AndroidPlatform as Platform>::delete_credentials(&self.0, url)
    }
    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        <AndroidPlatform as Platform>::keyboard_layout(&self.0)
    }
    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        <AndroidPlatform as Platform>::keyboard_mapper(&self.0)
    }
    fn on_keyboard_layout_change(&self, callback: Box<dyn FnMut()>) {
        <AndroidPlatform as Platform>::on_keyboard_layout_change(&self.0, callback)
    }

    fn hide_cursor_until_mouse_moves(&self) {
        <AndroidPlatform as Platform>::hide_cursor_until_mouse_moves(&self.0)
    }

    fn is_cursor_visible(&self) -> bool {
        <AndroidPlatform as Platform>::is_cursor_visible(&self.0)
    }
}

impl Default for AndroidPlatform {
    fn default() -> Self {
        Self::new(false)
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn headless() -> AndroidPlatform {
        AndroidPlatform::new(true)
    }

    // ── basic construction ────────────────────────────────────────────────────

    #[test]
    fn platform_constructs_headless() {
        let p = headless();
        assert!(p.is_headless());
        assert!(!p.is_active());
        assert!(!p.should_quit());
    }

    // ── lifecycle ─────────────────────────────────────────────────────────────

    #[test]
    fn run_invokes_finish_launching_callback() {
        let p = headless();
        let ran = Arc::new(AtomicBool::new(false));
        let ran2 = ran.clone();
        p.run(Box::new(move || {
            ran2.store(true, Ordering::Relaxed);
        }));
        assert!(ran.load(Ordering::Relaxed));
    }

    #[test]
    fn quit_sets_should_quit() {
        let p = headless();
        assert!(!p.should_quit());
        p.quit();
        assert!(p.should_quit());
    }

    #[test]
    fn quit_callback_fires() {
        let p = headless();
        let fired = Arc::new(AtomicBool::new(false));
        let f2 = fired.clone();
        p.on_quit(move || {
            f2.store(true, Ordering::Relaxed);
        });
        p.quit();
        assert!(fired.load(Ordering::Relaxed));
    }

    #[test]
    fn non_send_quit_callback_fires_on_owner_thread() {
        let p = headless();
        let fired = Rc::new(std::cell::Cell::new(false));
        let f2 = Rc::clone(&fired);
        p.on_quit(move || f2.set(true));
        p.quit();
        assert!(fired.get());
    }

    #[test]
    fn off_thread_quit_callback_is_delivered_on_owner_thread() {
        let p = Arc::new(headless());
        let fired = Rc::new(std::cell::Cell::new(false));
        let f2 = Rc::clone(&fired);
        p.on_quit(move || f2.set(true));

        let p2 = Arc::clone(&p);
        std::thread::spawn(move || p2.quit())
            .join()
            .expect("quit request thread panicked");
        assert!(!fired.get());

        p.run_pending_quit_callback();
        assert!(fired.get());
    }

    #[test]
    fn did_become_active_sets_flag() {
        let p = headless();
        p.did_become_active();
        assert!(p.is_active());
        p.did_enter_background();
        assert!(!p.is_active());
    }

    // ── clipboard ─────────────────────────────────────────────────────────────

    #[test]
    fn clipboard_round_trip() {
        let p = headless();
        assert!(p.read_from_clipboard().is_none());
        p.write_to_clipboard("hello android".to_string());
        assert_eq!(p.read_from_clipboard().as_deref(), Some("hello android"));
    }

    #[test]
    fn clipboard_overwrite() {
        let p = headless();
        p.write_to_clipboard("first".to_string());
        p.write_to_clipboard("second".to_string());
        assert_eq!(p.read_from_clipboard().as_deref(), Some("second"));
    }

    // ── credentials ───────────────────────────────────────────────────────────

    #[test]
    fn credentials_round_trip() {
        let p = headless();
        p.write_credentials("svc", "user", b"pass123").unwrap();
        let result = p.read_credentials("svc", "user").unwrap();
        assert_eq!(result.as_deref(), Some(b"pass123".as_slice()));
    }

    #[test]
    fn credentials_delete() {
        let p = headless();
        p.write_credentials("svc", "user", b"pass").unwrap();
        p.delete_credentials("svc", "user").unwrap();
        assert!(p.read_credentials("svc", "user").unwrap().is_none());
    }

    #[test]
    fn credentials_missing_returns_none() {
        let p = headless();
        assert!(
            p.read_credentials("no-such-service", "user")
                .unwrap()
                .is_none()
        );
    }

    // ── misc platform queries ─────────────────────────────────────────────────

    #[test]
    fn should_auto_hide_scrollbars_true() {
        assert!(headless().should_auto_hide_scrollbars());
    }

    #[test]
    fn can_select_mixed_files_false() {
        assert!(!headless().can_select_mixed_files_and_dirs());
    }

    #[test]
    fn keyboard_layout_id_non_empty() {
        let id = headless().keyboard_layout_id();
        assert!(!id.is_empty());
    }

    #[test]
    fn path_for_auxiliary_executable_errors() {
        let result = headless().path_for_auxiliary_executable("foo");
        assert!(result.is_err());
    }

    // ── display ───────────────────────────────────────────────────────────────

    #[test]
    fn headless_has_one_display() {
        let p = headless();
        assert_eq!(p.displays().len(), 1);
        assert!(p.primary_display().is_some());
    }

    // ── preferred backend ─────────────────────────────────────────────────────

    #[test]
    fn default_backend_is_vulkan() {
        assert_eq!(headless().preferred_backend(), AndroidBackend::Vulkan);
    }

    #[test]
    fn backend_override() {
        let p = headless();
        p.set_preferred_backend(AndroidBackend::Gles);
        assert_eq!(p.preferred_backend(), AndroidBackend::Gles);
    }

    // ── text system ───────────────────────────────────────────────────────────

    #[test]
    fn text_system_accessible() {
        let p = headless();
        // Just confirm the text system can be retrieved without panicking.
        let _ts = p.text_system();
    }

    // ── thermal state ────────────────────────────────────────────────────────

    #[test]
    fn android_thermal_status_maps_to_gpui_state() {
        assert_eq!(
            AndroidPlatform::thermal_state_from_android_status(0),
            Some(ThermalState::Nominal)
        );
        assert_eq!(
            AndroidPlatform::thermal_state_from_android_status(1),
            Some(ThermalState::Fair)
        );
        assert_eq!(
            AndroidPlatform::thermal_state_from_android_status(2),
            Some(ThermalState::Serious)
        );
        assert_eq!(
            AndroidPlatform::thermal_state_from_android_status(3),
            Some(ThermalState::Serious)
        );
        assert_eq!(
            AndroidPlatform::thermal_state_from_android_status(4),
            Some(ThermalState::Critical)
        );
        assert_eq!(
            AndroidPlatform::thermal_state_from_android_status(6),
            Some(ThermalState::Critical)
        );
        assert_eq!(AndroidPlatform::thermal_state_from_android_status(-1), None);
        assert_eq!(AndroidPlatform::thermal_state_from_android_status(99), None);
    }

    #[test]
    fn thermal_listener_is_retained() {
        let p = headless();
        p.register_thermal_listener(Box::new(|| {}));
        assert!(p.with_callbacks(|callbacks| callbacks.thermal_state.is_some()));
    }

    // ── open-URLs callback ────────────────────────────────────────────────────

    #[test]
    fn open_urls_callback_fires() {
        let p = headless();
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let r2 = received.clone();
        p.on_open_urls(move |urls| {
            r2.lock().extend(urls);
        });
        p.deliver_open_urls(vec!["gpui://test".to_string()]);
        assert_eq!(received.lock().as_slice(), &["gpui://test"]);
    }

    // ── reopen callback ───────────────────────────────────────────────────────

    #[test]
    fn reopen_callback_fires() {
        let p = headless();
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c2 = count.clone();
        p.on_reopen(move || {
            c2.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });
        p.deliver_reopen();
        p.deliver_reopen();
        assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 2);
    }

    // ── headless window count ─────────────────────────────────────────────────

    #[test]
    fn initial_window_count_is_zero() {
        assert_eq!(headless().window_count(), 0);
        assert!(headless().primary_window().is_none());
    }

    // ── flush main thread tasks ───────────────────────────────────────────────

    #[test]
    fn flush_main_thread_tasks_no_panic() {
        // There are no tasks queued, so this should be a no-op.
        headless().flush_main_thread_tasks();
    }

    // ── tick ─────────────────────────────────────────────────────────────────

    #[test]
    fn tick_no_panic() {
        headless().tick();
    }
}
