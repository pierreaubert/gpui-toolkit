use gpui::{Context, Window, div, prelude::*};
use gpui_audio_kit::audio::vertical_slider::VerticalSlider;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// View for disabled scroll wheel test
pub(super) struct SliderScrollWheelDisabledView {
    pub(super) change_count: Arc<AtomicUsize>,
}

impl Render for SliderScrollWheelDisabledView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let change_count = self.change_count.clone();

        div().size_full().child(
            VerticalSlider::new("scroll-wheel-disabled-slider")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .label("Disabled Scroll")
                .disabled(true)
                .on_change(move |_, _window, _cx| {
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}
