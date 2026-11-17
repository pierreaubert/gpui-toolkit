//! Android host for Python-authored GPUI apps.
//!
//! `NativeActivity` loads this `.so` and calls `android_main`, which
//! extracts the bundled `CPython` home + app assets into internal storage
//! (mirroring the `CPython` Android testbed), points the interpreter at
//! them, and launches the shared Python host from
//! `gpui-python-runtime` (`bin/showcase.rs`, included by path exactly
//! like the desktop `gpui-python-host` binary does).
//!
//! APK contract (assembled by `scripts/stage-android-resources.sh`):
//! - `assets/python/` — `CPython` home (`lib/python3.15` stdlib).
//! - `assets/app/main_app.py` — the Python app entry (`build_app()`/`app`).
//! - `assets/app/gpui_toolkit/` — toolkit package incl. `_native.abi3.so`.
//! - `assets/gpui_bundle_manifest.txt` — one relative path per line for
//!   every file under `python/` and `app/` (asset extraction worklist).
//! - `jniLibs/arm64-v8a/` — this library plus `libpython3.15.so` and the
//!   `lib*_python.so` support libraries it depends on.

// Rust guideline compliant 2026-10-09

/// The shared GPUI host for Python-authored apps, reused verbatim from
/// the desktop host binary so mobile and desktop render identically.
#[cfg(target_os = "android")]
#[path = "../../../bin/showcase.rs"]
mod showcase;

#[cfg(target_os = "android")]
mod imp {
    use std::ffi::CString;
    use std::io::Read;
    use std::path::Path;

    use android_activity::AndroidApp;
    use gpui::App;

    /// FNV-1a over the manifest bytes; identifies one exact bundle build.
    fn manifest_hash(bytes: &[u8]) -> String {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        format!("{hash:016x}")
    }

    fn read_asset(app: &AndroidApp, name: &str) -> Option<Vec<u8>> {
        let filename = CString::new(name).ok()?;
        let mut asset = app.asset_manager().open(&filename)?;
        let mut contents = Vec::new();
        asset.read_to_end(&mut contents).ok()?;
        Some(contents)
    }

    /// Extracts `python/` and `app/` assets under `root`, once per build.
    ///
    /// Re-extraction is keyed by a marker file holding the manifest hash,
    /// so app upgrades with changed assets refresh the tree while normal
    /// launches skip straight to interpreter startup.
    fn extract_bundle(app: &AndroidApp, root: &Path) -> Result<(), String> {
        let manifest = read_asset(app, "gpui_bundle_manifest.txt")
            .ok_or("missing gpui_bundle_manifest.txt asset")?;
        let marker = root.join(".gpui_bundle_hash");
        let expected = manifest_hash(&manifest);
        if std::fs::read_to_string(&marker).is_ok_and(|stored| stored == expected)
            && root.join("app/main_app.py").is_file()
        {
            return Ok(());
        }

        let asset_manager = app.asset_manager();
        for relative in String::from_utf8_lossy(&manifest).lines() {
            let relative = relative.trim();
            if relative.is_empty() {
                continue;
            }
            let filename = CString::new(relative)
                .map_err(|_| format!("non-UTF8 asset path in manifest: {relative}"))?;
            let mut asset = asset_manager
                .open(&filename)
                .ok_or_else(|| format!("missing bundled asset: {relative}"))?;
            let destination = root.join(relative);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("create {}: {error}", parent.display()))?;
            }
            let mut file = std::fs::File::create(&destination)
                .map_err(|error| format!("create {}: {error}", destination.display()))?;
            std::io::copy(&mut asset, &mut file)
                .map_err(|error| format!("extract {relative}: {error}"))?;
        }
        std::fs::write(&marker, &expected)
            .map_err(|error| format!("write bundle marker: {error}"))?;
        Ok(())
    }

    #[unsafe(no_mangle)]
    pub fn android_main(app: AndroidApp) {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("gpui-python-mobile"),
        );
        gpui_android::android::jni::install_panic_hook();

        let Some(internal) = app.internal_data_path() else {
            log::error!("android_main: internal_data_path() returned None");
            return;
        };
        let bundle_root = internal.join("gpui-python-bundle");
        if let Err(error) = std::fs::create_dir_all(&bundle_root) {
            log::error!(
                "android_main: cannot create {}: {error}",
                bundle_root.display()
            );
            return;
        }
        if let Err(error) = extract_bundle(&app, &bundle_root) {
            log::error!("android_main: bundle extraction failed: {error}");
            return;
        }
        let script = bundle_root.join("app/main_app.py");
        let python_paths = vec![bundle_root.join("app")];

        // Point CPython at the extracted home before the interpreter is
        // first initialized (on the session thread spawned later).
        // SAFETY: android_main runs once at process launch, before GPUI
        // or Python threads exist.
        unsafe {
            std::env::set_var("PYTHONHOME", bundle_root.join("python"));
            std::env::set_var("PYTHONDONTWRITEBYTECODE", "1");
            std::env::set_var("GPUI_TOOLKIT_DATA_DIR", &internal);
        }

        // GPUI runs on the first android_main call only; recreated
        // activities hand their surface to it and state is preserved.
        gpui_android::run_android_app(app, move |cx: &mut App| {
            crate::showcase::run_embedded_app(cx, script.clone(), python_paths.clone());
        });
    }
}
