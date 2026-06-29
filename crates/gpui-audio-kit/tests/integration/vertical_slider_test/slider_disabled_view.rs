use gpui::{Context, Window, div, prelude::*};
use gpui_audio_kit::audio::vertical_slider::VerticalSlider;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// View with disabled slider that tracks interactions
pub(super) struct SliderDisabledView {
    pub(super) change_count: Arc<AtomicUsize>,
    pub(super) select_count: Arc<AtomicUsize>,
}

impl Render for SliderDisabledView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let change_count = self.change_count.clone();
        let select_count = self.select_count.clone();

        div().size_full().child(
            VerticalSlider::new("disabled-interaction-slider")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .label("Disabled")
                .disabled(true)
                .on_change(move |_, _window, _cx| {
                    change_count.fetch_add(1, Ordering::SeqCst);
                })
                .on_select(move |_window, _cx| {
                    select_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}
