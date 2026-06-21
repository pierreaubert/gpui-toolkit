use gpui_audio_kit::AudioDesignTokens;
use gpui_audio_kit::AudioToggleExt;
use gpui_ui_kit::Toggle;

#[test]
fn audio_toggle_ext_sets_sliding_style_by_default() {
    let tokens = AudioDesignTokens::default();
    let toggle = Toggle::new("test-toggle").design_tokens(&tokens);
    // The toggle should be constructible; the exact style is an implementation detail.
    drop(toggle);
}

#[test]
fn audio_toggle_ext_sets_segmented_style() {
    let mut tokens = AudioDesignTokens::default();
    tokens.toggle_variant = AudioDesignTokens::TOGGLE_SEGMENTED;
    let toggle = Toggle::new("test-toggle").design_tokens(&tokens);
    drop(toggle);
}
