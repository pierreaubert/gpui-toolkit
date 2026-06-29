use gpui::{Context, Window, div, prelude::*};
use gpui_ui_kit::number_input::NumberInput;

pub(super) struct NumberInputTestView;

impl Render for NumberInputTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            NumberInput::new("test-number-input")
                .value(50.0)
                .min(0.0)
                .max(100.0),
        )
    }
}
