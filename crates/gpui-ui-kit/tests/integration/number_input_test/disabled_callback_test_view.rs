use gpui::{Context, Window, div, prelude::*};
use gpui_ui_kit::number_input::NumberInput;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Test that disabled input doesn't trigger callbacks on button click
pub(super) struct DisabledCallbackTestView {
    pub(super) change_count: Arc<AtomicUsize>,
}

impl Render for DisabledCallbackTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let change_count = self.change_count.clone();

        div().size_full().child(
            NumberInput::new("disabled-callback-input")
                .value(50.0)
                .disabled(true)
                .width(150.0)
                .on_change(move |_, _window, _cx| {
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}
