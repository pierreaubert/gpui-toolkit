use super::boxed_asset_source::BoxedAssetSource;
use super::consts::APP_CALLBACK;
use super::consts::ASSET_SOURCE;
use gpui::App;

pub(super) fn take_app_callback() -> Option<Box<dyn FnOnce(&mut App)>> {
    APP_CALLBACK
        .get()
        .and_then(|cell| unsafe { (*cell.0.get()).take() })
}

pub(super) fn take_asset_source() -> Option<BoxedAssetSource> {
    ASSET_SOURCE
        .get()
        .and_then(|cell| unsafe { (*cell.0.get()).take() })
        .map(BoxedAssetSource)
}
