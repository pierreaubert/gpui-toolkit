//! Integration test for IconButton component

use gpui::{Context, TestAppContext, Window, div, prelude::*};
use gpui_ui_kit::icon_button::IconButton;

struct IconButtonTestView;

impl Render for IconButtonTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(IconButton::new("test-icon-button", "✓"))
    }
}

#[gpui::test]
async fn test_icon_button_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| IconButtonTestView);
}
