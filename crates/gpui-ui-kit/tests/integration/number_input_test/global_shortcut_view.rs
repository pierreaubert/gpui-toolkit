use gpui::{Context, Window, div, prelude::*};
use gpui_ui_kit::number_input::NumberInput;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) struct GlobalShortcutView {
    pub(super) shortcut_triggered: Arc<AtomicBool>,
}

impl Render for GlobalShortcutView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let triggered = self.shortcut_triggered.clone();

        div()
            .id("root")
            .size_full()
            // Simulate a global shortcut handler at a parent level (e.g. Play/Pause toggle)
            .on_key_down(move |event, _window, _cx| {
                if event.keystroke.key == "space" {
                    triggered.store(true, Ordering::SeqCst);
                }
            })
            .child(
                div()
                    .p_4()
                    .child(NumberInput::new("test-input").value(50.0)),
            )
    }
}
