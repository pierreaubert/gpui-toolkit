//! Integration tests for Button component
//!
//! These tests verify that Button can be rendered in a real GPUI window.

use gpui::{Context, TestAppContext, Window, div, prelude::*};
use gpui_ui_kit::button::Button;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

// Simple test view that renders a button
struct ButtonTestView {
    button_label: String,
    click_handler: Option<Arc<AtomicBool>>,
}

impl ButtonTestView {
    fn new(label: impl Into<String>) -> Self {
        Self {
            button_label: label.into(),
            click_handler: None,
        }
    }

    fn with_click_handler(mut self, handler: Arc<AtomicBool>) -> Self {
        self.click_handler = Some(handler);
        self
    }
}

impl Render for ButtonTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut button = Button::new("test-button", &self.button_label);

        if let Some(handler) = self.click_handler.clone() {
            button = button.on_click(move |_event, _cx| {
                handler.store(true, Ordering::SeqCst);
            });
        }

        div().child(button)
    }
}

/// Minimal test that TestAppContext can be created
/// This matches the pattern used in Zed's test suite
#[gpui::test]
async fn test_context_creation(cx: &mut TestAppContext) {
    // Minimal test - just verify TestAppContext works
    cx.update(|_cx| {
        // Just a simple update to verify context works
    });
}

/// Test that a window can be created with a button
#[gpui::test]
async fn test_button_renders_in_window(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ButtonTestView::new("Click Me"));

    // If we got here without crashing, the window was created successfully
}

/// Test that a button can handle click events
#[gpui::test]
async fn test_button_click_handler(cx: &mut TestAppContext) {
    let clicked = Arc::new(AtomicBool::new(false));
    let clicked_clone = clicked.clone();

    let _window = cx.add_window(|_window, _cx| {
        ButtonTestView::new("Click Me").with_click_handler(clicked_clone)
    });

    // Note: Actually triggering the click in a test would require
    // simulating mouse events, which is complex. This test verifies
    // that the click handler can be attached without crashing.

    // Verify the handler wasn't called yet
    assert!(!clicked.load(Ordering::SeqCst));
}

/// Test that Button API compiles (without rendering)
#[test]
fn test_button_api_compiles() {
    let button = Button::new("test-button", "Click Me");
    drop(button);
}
