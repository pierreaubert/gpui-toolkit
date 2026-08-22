use gpui::{Context, Window, div, prelude::*};
use gpui_ui_kit::input::Input;
use std::cell::RefCell;
use std::sync::Arc;

/// View for testing mouse drag selection
pub(super) struct InputDragSelectTestView {
    pub(super) text_changes: Arc<RefCell<Vec<String>>>,
}

impl Render for InputDragSelectTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let text_changes = self.text_changes.clone();

        div().size_full().child(
            Input::new("drag-select-input")
                .value("Select some text here")
                .on_text_change(move |text, _window, _cx| {
                    text_changes.borrow_mut().push(text.to_string());
                }),
        )
    }
}
