use raw_window_handle::{
    AppKitDisplayHandle, AppKitWindowHandle, HasDisplayHandle, HasWindowHandle,
};
use std::{ffi::c_void, ptr::NonNull};

/// Lightweight handle struct passed to `WgpuRenderer::new()`.
///
/// `WgpuRenderer::new` requires `W: HasWindowHandle + HasDisplayHandle + Debug +
/// Send + Sync + Clone + 'static`. `AuWindow` itself cannot satisfy those bounds
/// (it contains callbacks, `RefCells`, etc.), so we extract just the `NSView` pointer
/// into this small struct -- the same pattern used by `gpui_linux`'s `RawWindow`.
#[derive(Debug, Clone, Copy)]
pub(super) struct AuRawWindow {
    pub(super) ns_view: *mut c_void,
}

// SAFETY: The raw pointer is only dereferenced on the main thread by wgpu's
// Metal backend during surface creation. The pointer's lifetime is guaranteed
// by the Swift `AUViewController` that owns the `NSView`.
unsafe impl Send for AuRawWindow {}

// SAFETY: same as `Send` above; the `NSView` outlives any handle.
unsafe impl Sync for AuRawWindow {}

impl HasWindowHandle for AuRawWindow {
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        let view = NonNull::new(self.ns_view).ok_or(raw_window_handle::HandleError::Unavailable)?;
        let handle = AppKitWindowHandle::new(view);
        // SAFETY: `view` is a non-null live `NSView` for the borrowed
        // lifetime; the handle only names it for surface creation.
        let raw = unsafe { raw_window_handle::WindowHandle::borrow_raw(handle.into()) };
        Ok(raw)
    }
}

impl HasDisplayHandle for AuRawWindow {
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        let handle = AppKitDisplayHandle::new();
        // SAFETY: the AppKit display handle is a process-global unit value
        // with no lifetime obligations beyond the borrow.
        let raw = unsafe { raw_window_handle::DisplayHandle::borrow_raw(handle.into()) };
        Ok(raw)
    }
}
