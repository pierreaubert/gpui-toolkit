//! iOS host for Python-authored GPUI apps.
//!
//! This crate compiles to a static library the Xcode project links. The
//! Swift `AppDelegate` calls `python_mobile_ios_start()`, which points
//! `CPython` at the bundled `python/` home, registers the shared Python
//! host from `gpui-python-runtime` (`bin/showcase.rs`, included by path
//! exactly like the desktop `gpui-python-host` binary does), and hands
//! the run loop to `gpui-ios`.
//!
//! Bundle contract (assembled by `scripts/stage-ios-resources.sh` and the
//! Xcode `install_python` phase):
//! - `python/` — `CPython` home (`lib/python3.15` stdlib).
//! - `app/main_app.py` — the Python app entry (`build_app()` or `app`).
//! - `app/gpui_toolkit/` — the toolkit package, incl. `_native.abi3.so`
//!   (converted to a signed framework by the Xcode phase).
//! - `Python.xcframework` embedded as `Python.framework`.

// Rust guideline compliant 2026-10-09

/// The shared GPUI host for Python-authored apps, reused verbatim from
/// the desktop host binary so mobile and desktop render identically.
#[cfg(any(target_os = "ios", target_os = "tvos"))]
#[path = "../../../bin/showcase.rs"]
mod showcase;

#[cfg(any(target_os = "ios", target_os = "tvos"))]
mod imp {
    use std::path::PathBuf;

    use super::misc::ffi_guard;

    /// Bundle resource directory, resolved from the running executable
    /// (iOS app executables live at the bundle root next to resources).
    fn resources_dir() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// Called from Swift to start the embedded Python GPUI application.
    #[unsafe(no_mangle)]
    pub extern "C" fn python_mobile_ios_start() {
        ffi_guard(|| {
            oslog::OsLogger::new("org.spinorama.gpui-python-mobile")
                .level_filter(log::LevelFilter::Info)
                .init()
                .ok();

            let resources = resources_dir();
            let script = resources.join("app/main_app.py");
            let python_paths = vec![resources.join("app")];

            // Point CPython at the bundled home before the interpreter is
            // first initialized (lazily, on the embedded session thread
            // spawned from the app callback below). SAFETY: this runs once
            // at process launch, before GPUI or Python threads exist.
            unsafe {
                std::env::set_var("PYTHONHOME", resources.join("python"));
                std::env::set_var("PYTHONDONTWRITEBYTECODE", "1");
            }

            log::info!("python_mobile_ios_start: registering app callback");
            gpui_ios::ios::ffi::set_app_callback(Box::new(move |cx| {
                crate::showcase::run_embedded_app(cx, script.clone(), python_paths.clone());
            }));

            log::info!("python_mobile_ios_start: calling run_app");
            gpui_ios::ios::ffi::run_app();
        });
    }
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
mod misc {
    /// FFI panic guard, mirroring `gpui-showcase-ios` so a Rust panic
    /// never unwinds across the C ABI.
    pub(super) fn ffi_guard<F, R>(f: F) -> R
    where
        F: FnOnce() -> R + std::panic::UnwindSafe,
        R: Default,
    {
        match std::panic::catch_unwind(f) {
            Ok(result) => result,
            Err(_) => {
                log::error!("[iOS] FFI call panicked; returning default");
                R::default()
            }
        }
    }
}
