use gpui::{Context, Window, div, prelude::*};
use gpui_audio_kit::audio::vertical_slider::VerticalSlider;

pub(super) struct VerticalSliderTestView;

impl Render for VerticalSliderTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            VerticalSlider::new("test-vslider")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .label("Volume"),
        )
    }
}
