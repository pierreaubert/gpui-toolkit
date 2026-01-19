//! Integration test for Progress component

use gpui::{Context, TestAppContext, Window, div, prelude::*};
use gpui_ui_kit::progress::Progress;

struct ProgressTestView;

impl Render for ProgressTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Progress::new(0.75))
    }
}

#[gpui::test]
async fn test_progress_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ProgressTestView);
}
