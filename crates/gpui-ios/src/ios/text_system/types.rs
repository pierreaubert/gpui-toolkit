use gpui::{FontFallbacks, FontFeatures, SharedString};

#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct FontKey {
    pub(super) font_family: SharedString,
    pub(super) font_features: FontFeatures,
    pub(super) font_fallbacks: Option<FontFallbacks>,
}
