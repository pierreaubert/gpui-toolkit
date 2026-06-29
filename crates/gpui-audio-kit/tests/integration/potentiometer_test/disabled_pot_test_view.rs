use gpui::{Context, Window, div, prelude::*};
use gpui_audio_kit::audio::potentiometer::Potentiometer;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// View with disabled potentiometer that tracks interactions
pub(super) struct DisabledPotTestView {
    pub(super) change_count: Arc<AtomicUsize>,
}

impl Render for DisabledPotTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let change_count = self.change_count.clone();

        div().size_full().child(
            Potentiometer::new("disabled-pot")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .disabled(true)
                .label("Disabled")
                .on_change(move |_, _window, _cx| {
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}
