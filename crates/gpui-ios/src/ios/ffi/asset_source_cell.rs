pub(super) struct AssetSourceCell(
    pub(super) std::cell::UnsafeCell<Option<Box<dyn gpui::AssetSource>>>,
);

unsafe impl Send for AssetSourceCell {}

unsafe impl Sync for AssetSourceCell {}
