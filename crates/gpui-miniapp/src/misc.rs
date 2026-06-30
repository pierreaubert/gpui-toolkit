use gpui::*;
use std::rc::Rc;

/// Construct the GPUI [`Platform`] backend for the current OS.
///
/// Centralizing this here keeps the per-platform `gpui_macos` / `gpui_linux` /
/// `gpui_windows` deps confined to this single (`publish = false`) crate so
/// publishable consumers don't have to declare them. Examples and demos that
/// need a custom `Application::with_platform(...)` should call this helper
/// instead of inlining the cfg-cascade themselves.
pub fn current_platform() -> Result<Rc<dyn gpui::Platform>, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(Rc::new(gpui_macos::MacPlatform::new(false)))
    }
    #[cfg(target_os = "linux")]
    {
        Ok(gpui_linux::current_platform(false))
    }
    #[cfg(target_os = "windows")]
    {
        gpui_windows::WindowsPlatform::new(false)
            .map(|p| Rc::new(p) as Rc<dyn gpui::Platform>)
            .map_err(|e| format!("failed to create Windows platform: {e:?}"))
    }
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        Ok(gpui_ios::current_platform(false))
    }
    #[cfg(target_os = "android")]
    {
        Ok(gpui_android::current_platform(false))
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows",
        target_os = "ios",
        target_os = "tvos",
        target_os = "android"
    )))]
    {
        compile_error!("unsupported platform for gpui-miniapp")
    }
}
