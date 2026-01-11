//! Integration test for Checkbox component

use gpui::{Context, TestAppContext, Window, div, prelude::*};
use gpui_ui_kit::checkbox::Checkbox;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

struct CheckboxTestView {
    checked: Arc<AtomicBool>,
}

impl CheckboxTestView {
    fn new(checked: Arc<AtomicBool>) -> Self {
        Self { checked }
    }
}

impl Render for CheckboxTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let checked_clone = self.checked.clone();
        div().child(
            Checkbox::new("test-checkbox")
                .label("Test Checkbox")
                .checked(false)
                .on_change(move |new_state, _window, _cx| {
                    checked_clone.store(new_state, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_checkbox_renders(cx: &mut TestAppContext) {
    let checked = Arc::new(AtomicBool::new(false));
    let _window = cx.add_window(|_window, _cx| CheckboxTestView::new(checked));
}
