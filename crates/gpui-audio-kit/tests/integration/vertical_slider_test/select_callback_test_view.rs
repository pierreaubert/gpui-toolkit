use gpui::{Context, Window, div, prelude::*};
use gpui_audio_kit::audio::vertical_slider::VerticalSlider;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Test on_select callback
pub(super) struct SelectCallbackTestView {
    pub(super) select_count: Arc<AtomicUsize>,
}

impl Render for SelectCallbackTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let select_count = self.select_count.clone();

        div().size_full().child(
            VerticalSlider::new("select-callback-slider")
                .value(50.0)
                .on_select(move |_window, _cx| {
                    select_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}
