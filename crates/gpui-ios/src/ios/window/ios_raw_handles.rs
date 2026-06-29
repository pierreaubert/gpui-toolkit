use super::super::events::*;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

/// Lightweight, owned, `Debug + Clone + Send + Sync + 'static` wrapper around
/// the raw iOS window/display handles. `WgpuRenderer::new` requires these
/// trait bounds on the window argument; `IosWindow` itself can't satisfy them
/// because of its interior mutability and raw `*mut Object` pointers.
#[derive(Clone, Debug)]
pub(crate) struct IosRawHandles {
    pub(super) window: raw_window_handle::RawWindowHandle,
    pub(super) display: raw_window_handle::RawDisplayHandle,
}

unsafe impl Send for IosRawHandles {}

unsafe impl Sync for IosRawHandles {}

impl HasWindowHandle for IosRawHandles {
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(self.window) })
    }
}

impl HasDisplayHandle for IosRawHandles {
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(self.display) })
    }
}
