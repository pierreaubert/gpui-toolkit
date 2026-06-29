use gpui::{Context, Window, div, prelude::*};
use gpui_audio_kit::audio::potentiometer::Potentiometer;

pub(super) struct PotentiometerTestView;

impl Render for PotentiometerTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            Potentiometer::new("test-pot")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .unit("%")
                .label("Volume"),
        )
    }
}
