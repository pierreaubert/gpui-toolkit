use gpui::{FontFallbacks, FontFeatures, SharedString};

#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct FontKey {
    pub(super) family: SharedString,
    pub(super) features: FontFeatures,
    pub(super) fallbacks: Option<FontFallbacks>,
}
