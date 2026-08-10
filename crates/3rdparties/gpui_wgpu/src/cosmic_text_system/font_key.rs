use gpui::{FontFeatures, SharedString};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct FontKey {
    pub(super) family: SharedString,
    pub(super) features: FontFeatures,
}

impl FontKey {
    pub(super) fn new(family: SharedString, features: FontFeatures) -> Self {
        Self { family, features }
    }
}
