use gpui::{Context, Window, div, prelude::*};
use gpui_audio_kit::audio::volume_knob::VolumeKnob;

pub(super) struct VolumeKnobTestView;

impl Render for VolumeKnobTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            VolumeKnob::new()
                .id("test-volume-knob")
                .value(0.7)
                .label("VOL"),
        )
    }
}
