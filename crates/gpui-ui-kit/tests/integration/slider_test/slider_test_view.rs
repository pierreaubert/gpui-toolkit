use gpui::{Context, Window, div, prelude::*};
use gpui_ui_kit::slider::Slider;

pub(super) struct SliderTestView;

impl Render for SliderTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Slider::new("test-slider").value(0.5).min(0.0).max(1.0))
    }
}
