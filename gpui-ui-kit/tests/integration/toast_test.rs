//! Integration test for Toast component

use gpui::{Context, TestAppContext, Window, div, prelude::*};
use gpui_ui_kit::toast::Toast;

struct ToastTestView;

impl Render for ToastTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Toast::new("test-toast", "Notification message"))
    }
}

#[gpui::test]
async fn test_toast_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ToastTestView);
}
