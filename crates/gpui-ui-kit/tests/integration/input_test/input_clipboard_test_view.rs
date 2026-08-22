use gpui::{Context, Window, div, prelude::*};
use gpui_ui_kit::input::Input;
use std::cell::RefCell;
use std::sync::Arc;

/// View for testing clipboard operations
pub(super) struct InputClipboardTestView {
    pub(super) text_changes: Arc<RefCell<Vec<String>>>,
}

impl Render for InputClipboardTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let text_changes = self.text_changes.clone();

        div()
            .size_full()
            .child(Input::new("clipboard-test-input").value("").on_text_change(
                move |text, _window, _cx| {
                    text_changes.borrow_mut().push(text.to_string());
                },
            ))
    }
}
