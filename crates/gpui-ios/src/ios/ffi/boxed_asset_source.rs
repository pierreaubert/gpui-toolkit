/// Wrapper that forwards `AssetSource` through a `Box<dyn AssetSource>`,
/// needed because `with_assets()` takes `impl AssetSource` and the trait
/// isn't automatically implemented for `Box<dyn AssetSource>`.
pub(super) struct BoxedAssetSource(pub(super) Box<dyn gpui::AssetSource>);

impl gpui::AssetSource for BoxedAssetSource {
    fn load(&self, path: &str) -> gpui::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        self.0.load(path)
    }
    fn list(&self, path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        self.0.list(path)
    }
}
