use gpui::{Context, TestAppContext, Window, div, prelude::*};
use gpui_audio_kit::{
    AudioDesignTokens, Potentiometer, PotentiometerSize, VerticalSlider,
    audio::potentiometer::PotentiometerTheme,
    audio::vertical_slider::VerticalSliderTheme,
};

#[gpui::test]
async fn test_potentiometer_underlined_label_renders(cx: &mut TestAppContext) {
    struct UnderlinedPotView;

    impl Render for UnderlinedPotView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut tokens = AudioDesignTokens::default();
            tokens.knob_label_style = AudioDesignTokens::LABEL_UNDERLINED;

            div().child(
                Potentiometer::new("underlined-pot")
                    .value(50.0)
                    .min(0.0)
                    .max(100.0)
                    .label("Gain")
                    .size(PotentiometerSize::Md)
                    .design_tokens(tokens),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| UnderlinedPotView);
}

#[gpui::test]
async fn test_potentiometer_indicator_styles_renders(cx: &mut TestAppContext) {
    struct IndicatorStylesView;

    impl Render for IndicatorStylesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut tick_tokens = AudioDesignTokens::default();
            tick_tokens.knob_indicator_style = AudioDesignTokens::INDICATOR_TICK;
            let mut arrow_tokens = AudioDesignTokens::default();
            arrow_tokens.knob_indicator_style = AudioDesignTokens::INDICATOR_ARROW;

            div()
                .flex()
                .gap_4()
                .child(
                    Potentiometer::new("tick-pot")
                        .value(30.0)
                        .label("Tick")
                        .design_tokens(tick_tokens),
                )
                .child(
                    Potentiometer::new("arrow-pot")
                        .value(60.0)
                        .label("Arrow")
                        .design_tokens(arrow_tokens),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| IndicatorStylesView);
}

#[gpui::test]
async fn test_potentiometer_custom_theme_and_accent_renders(cx: &mut TestAppContext) {
    struct CustomThemePotView;

    impl Render for CustomThemePotView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let theme = PotentiometerTheme {
                surface: gpui::rgba(0x1a1a1aff),
                surface_hover: gpui::rgba(0x2a2a2aff),
                knob_bg: gpui::rgba(0x333333ff),
                accent: gpui::rgba(0xff6600ff),
                accent_muted: gpui::rgba(0xff660033),
                border: gpui::rgba(0x444444ff),
                text_secondary: gpui::rgba(0xaaaaaaff),
                text_primary: gpui::rgba(0xffffffff),
                text_muted: gpui::rgba(0x888888ff),
                text_on_accent: gpui::rgba(0xffffffff),
                background_secondary: gpui::rgba(0x222222ff),
            };

            div().child(
                Potentiometer::new("accent-pot")
                    .value(50.0)
                    .min(0.0)
                    .max(100.0)
                    .label("Accent")
                    .theme(theme)
                    .accent_color(gpui::rgba(0x00ff00ff))
                    .aria_label("Custom accent potentiometer")
                    .aria_role(gpui_ui_kit::accessibility::AriaRole::Slider),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomThemePotView);
}

#[gpui::test]
async fn test_vertical_slider_underlined_label_renders(cx: &mut TestAppContext) {
    struct UnderlinedSliderView;

    impl Render for UnderlinedSliderView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut tokens = AudioDesignTokens::default();
            tokens.meter_label_style = AudioDesignTokens::LABEL_UNDERLINED;
            tokens.meter_glow = 1.0;

            div().child(
                VerticalSlider::new("underlined-slider")
                    .value(50.0)
                    .min(0.0)
                    .max(100.0)
                    .label("Glow")
                    .peak(Some(80.0))
                    .with_ticks()
                    .design_tokens(tokens),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| UnderlinedSliderView);
}

#[gpui::test]
async fn test_vertical_slider_custom_theme_renders(cx: &mut TestAppContext) {
    struct CustomThemeSliderView;

    impl Render for CustomThemeSliderView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let theme = VerticalSliderTheme {
                surface: gpui::rgba(0x1a1a1aff),
                surface_hover: gpui::rgba(0x2a2a2aff),
                track_bg: gpui::rgba(0x111111ff),
                accent: gpui::rgba(0xff6600ff),
                accent_muted: gpui::rgba(0xff660033),
                border: gpui::rgba(0x444444ff),
                text_secondary: gpui::rgba(0xaaaaaaff),
                text_primary: gpui::rgba(0xffffffff),
                text_muted: gpui::rgba(0x888888ff),
                text_on_accent: gpui::rgba(0xffffffff),
                background_secondary: gpui::rgba(0x222222ff),
                peak_marker: gpui::rgba(0xff0000ff),
            };

            div().child(
                VerticalSlider::new("themed-slider")
                    .value(40.0)
                    .min(0.0)
                    .max(100.0)
                    .label("Themed")
                    .theme(theme)
                    .aria_label("Custom themed slider")
                    .aria_role(gpui_ui_kit::accessibility::AriaRole::Slider),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomThemeSliderView);
}
