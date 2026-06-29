use gpui::{AtlasKey, AtlasTile};
use objc::runtime::Object;
use std::collections::HashMap;

/// Thread-local storage for the NSView pointer + dimensions,
/// set by `gpui_au_create` before calling `app.run()` so that
/// `AuWindow::new()` can read it during `open_window()`.
pub struct PendingViewInfo {
    pub ns_view: *mut Object,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
}

pub(super) struct FallbackAtlasState {
    pub(super) next_id: u32,
    pub(super) tiles: HashMap<AtlasKey, AtlasTile>,
}
