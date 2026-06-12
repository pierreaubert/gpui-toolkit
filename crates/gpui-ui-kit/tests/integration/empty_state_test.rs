//! Integration tests for EmptyState component
//!
//! Tests the EmptyState component including:
//! - Basic rendering
//! - With description
//! - With icon
//! - With action button
//! - Full configuration

use gpui::{Context, IntoElement, ParentElement, Render, TestAppContext, Window, div};
use gpui_ui_kit::Button;
use gpui_ui_kit::empty_state::EmptyState;

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct EmptyStateTestView;

impl Render for EmptyStateTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(EmptyState::new("No albums found"))
    }
}

#[gpui::test]
async fn test_empty_state_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| EmptyStateTestView);
}

// ============================================================================
// Description Tests
// ============================================================================

#[gpui::test]
async fn test_empty_state_with_description(cx: &mut TestAppContext) {
    struct DescriptionView;

    impl Render for DescriptionView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                EmptyState::new("No results").description("Try adjusting your search filters"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| DescriptionView);
}

// ============================================================================
// Icon Tests
// ============================================================================

#[gpui::test]
async fn test_empty_state_with_icon(cx: &mut TestAppContext) {
    struct IconView;

    impl Render for IconView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(EmptyState::new("No files").icon("F"))
        }
    }

    let _window = cx.add_window(|_window, _cx| IconView);
}

// ============================================================================
// Action Tests
// ============================================================================

#[gpui::test]
async fn test_empty_state_with_action(cx: &mut TestAppContext) {
    struct ActionView;

    impl Render for ActionView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                EmptyState::new("No albums found").action(Button::new("add-album", "Add Album")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ActionView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_empty_state_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                EmptyState::new("No albums found")
                    .description("Your library is empty. Add some music to get started.")
                    .icon("M")
                    .action(Button::new("scan-library", "Scan Library")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}

#[gpui::test]
async fn test_empty_state_title_only(cx: &mut TestAppContext) {
    struct TitleOnlyView;

    impl Render for TitleOnlyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(EmptyState::new("Nothing here yet"))
        }
    }

    let _window = cx.add_window(|_window, _cx| TitleOnlyView);
}
