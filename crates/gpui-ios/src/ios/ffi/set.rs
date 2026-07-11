use super::app_callback_cell::AppCallbackCell;
use super::asset_source_cell::AssetSourceCell;
use super::consts::APP_CALLBACK;
use super::consts::ASSET_SOURCE;
use super::consts::IOS_APP_STATE;
use gpui::App;

pub(crate) fn set_finish_launching_callback(callback: Box<dyn FnOnce()>) {
    if let Some(state) = IOS_APP_STATE.get() {
        unsafe {
            *state.finish_launching.get() = Some(callback);
        }
    }
}

pub fn set_app_callback(cb: Box<dyn FnOnce(&mut App)>) {
    let cell = APP_CALLBACK.get_or_init(|| AppCallbackCell(std::cell::UnsafeCell::new(None)));
    unsafe {
        *cell.0.get() = Some(cb);
    }
}

/// Register an asset source (SVG icons, fonts, images) before calling `run_app()`.
/// Without this, `svg()` elements will fail to load their paths.
pub fn set_asset_source(source: impl gpui::AssetSource + 'static) {
    let cell = ASSET_SOURCE.get_or_init(|| AssetSourceCell(std::cell::UnsafeCell::new(None)));
    unsafe {
        *cell.0.get() = Some(Box::new(source));
    }
}
