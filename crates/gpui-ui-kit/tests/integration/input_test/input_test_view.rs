use gpui::{Context, Window, div, prelude::*};
use gpui_ui_kit::input::Input;

pub(super) struct InputTestView;

impl Render for InputTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            Input::new("test-input")
                .placeholder("Enter text...")
                .value("Hello"),
        )
    }
}
