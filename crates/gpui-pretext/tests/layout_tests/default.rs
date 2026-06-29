#[doc = " Integration tests ported from the TypeScript pretext test suite."]
#[doc = ""]
#[doc = " Uses a deterministic \"fake canvas\" measure that mimics the original test's"]
#[doc = " character-width model:"]
#[doc = " - Space: 0.33× font_size (compressible)"]
#[doc = " - Tab: 1.32× font_size"]
#[doc = " - Punctuation (.,!?:;()[]{}\"'): 0.4× font_size"]
#[doc = " - Regular characters: 0.6× font_size"]
use gpui_pretext::{EngineProfile, PrepareOptions};

pub(super) fn default_profile() -> EngineProfile {
    EngineProfile::default()
}

pub(super) fn default_options() -> PrepareOptions {
    PrepareOptions::default()
}
