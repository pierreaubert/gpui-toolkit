use cosmic_text::{Font as CosmicTextFont, FontFeatures as CosmicFontFeatures};
use std::sync::Arc;

pub(super) struct LoadedFont {
    pub(super) font: Arc<CosmicTextFont>,
    pub(super) features: CosmicFontFeatures,
    pub(super) is_known_emoji_font: bool,
}
