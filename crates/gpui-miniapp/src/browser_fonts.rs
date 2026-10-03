// Rust guideline compliant 2026-02-21
use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;
pub(crate) struct BrowserFonts;
impl AssetSource for BrowserFonts {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(match path {
            "fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf" => Some(Cow::Borrowed(include_bytes!(
                "../../assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf"
            ))),
            "fonts/lilex/Lilex-Regular.ttf" => Some(Cow::Borrowed(include_bytes!(
                "../../assets/fonts/lilex/Lilex-Regular.ttf"
            ))),
            _ => None,
        })
    }
    fn list(&self, _: &str) -> Result<Vec<SharedString>> {
        Ok(vec![])
    }
}
