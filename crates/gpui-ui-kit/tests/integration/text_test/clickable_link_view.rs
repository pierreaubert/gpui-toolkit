use gpui::{Context, Window, div, prelude::*};
use gpui_ui_kit::text::Link;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) struct ClickableLinkView {
    pub(super) click_count: Arc<AtomicUsize>,
}

impl Render for ClickableLinkView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let click_count = self.click_count.clone();

        div()
            .size_full()
            .child(
                Link::new("clickable-link", "Click me").on_click(move |_window, _cx| {
                    click_count.fetch_add(1, Ordering::SeqCst);
                }),
            )
    }
}
