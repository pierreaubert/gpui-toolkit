//! Integration tests for SearchBar component
//!
//! Tests the SearchBar component including:
//! - Basic rendering
//! - With value
//! - With placeholder
//! - All sizes
//! - With handlers
//! - Full configuration

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, point, px,
};
use gpui_ui_kit::search_bar::{SearchBar, SearchBarSize};
use std::cell::RefCell;
use std::rc::Rc;

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct SearchBarTestView;

impl Render for SearchBarTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(SearchBar::new("test-search"))
    }
}

#[gpui::test]
async fn test_search_bar_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| SearchBarTestView);
}

// ============================================================================
// Value Tests
// ============================================================================

#[gpui::test]
async fn test_search_bar_with_value(cx: &mut TestAppContext) {
    struct ValueView;

    impl Render for ValueView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(SearchBar::new("value-search").value("beethoven"))
        }
    }

    let _window = cx.add_window(|_window, _cx| ValueView);
}

#[gpui::test]
async fn test_search_bar_with_placeholder(cx: &mut TestAppContext) {
    struct PlaceholderView;

    impl Render for PlaceholderView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(SearchBar::new("placeholder-search").placeholder("Search albums..."))
        }
    }

    let _window = cx.add_window(|_window, _cx| PlaceholderView);
}

// ============================================================================
// Size Tests
// ============================================================================

#[gpui::test]
async fn test_search_bar_size_sm(cx: &mut TestAppContext) {
    struct SmView;

    impl Render for SmView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(SearchBar::new("sm-search").size(SearchBarSize::Sm))
        }
    }

    let _window = cx.add_window(|_window, _cx| SmView);
}

#[gpui::test]
async fn test_search_bar_size_md(cx: &mut TestAppContext) {
    struct MdView;

    impl Render for MdView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(SearchBar::new("md-search").size(SearchBarSize::Md))
        }
    }

    let _window = cx.add_window(|_window, _cx| MdView);
}

#[gpui::test]
async fn test_search_bar_size_lg(cx: &mut TestAppContext) {
    struct LgView;

    impl Render for LgView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(SearchBar::new("lg-search").size(SearchBarSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| LgView);
}

#[gpui::test]
async fn test_search_bar_all_sizes(cx: &mut TestAppContext) {
    struct AllSizesView;

    impl Render for AllSizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    SearchBar::new("sm")
                        .size(SearchBarSize::Sm)
                        .placeholder("Small"),
                )
                .child(
                    SearchBar::new("md")
                        .size(SearchBarSize::Md)
                        .placeholder("Medium"),
                )
                .child(
                    SearchBar::new("lg")
                        .size(SearchBarSize::Lg)
                        .placeholder("Large"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllSizesView);
}

// ============================================================================
// Handler Tests
// ============================================================================

#[gpui::test]
async fn test_search_bar_with_handlers(cx: &mut TestAppContext) {
    struct HandlersView;

    impl Render for HandlersView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                SearchBar::new("handlers-search")
                    .value("test")
                    .on_change(|_query, _window, _cx| {})
                    .on_submit(|_query, _window, _cx| {})
                    .on_escape(|_window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| HandlersView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_search_bar_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                SearchBar::new("full-search")
                    .value("mozart")
                    .placeholder("Search library...")
                    .size(SearchBarSize::Md)
                    .show_icon(true)
                    .show_clear(true)
                    .on_change(|_query, _window, _cx| {})
                    .on_submit(|_query, _window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}

#[gpui::test]
async fn test_search_bar_typing_changes_query(cx: &mut TestAppContext) {
    struct TypingView {
        query: Rc<RefCell<String>>,
        changes: Rc<RefCell<Vec<String>>>,
    }

    impl Render for TypingView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let query = self.query.clone();
            let changes = self.changes.clone();
            let value = query.borrow().clone();

            div().size_full().p_4().child(div().w(px(300.)).child(
                SearchBar::new("typing-search").value(value).on_change(
                    move |next, _window, _cx| {
                        *query.borrow_mut() = next.to_string();
                        changes.borrow_mut().push(next.to_string());
                    },
                ),
            ))
        }
    }

    let query = Rc::new(RefCell::new(String::new()));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let window = cx.add_window({
        let query = query.clone();
        let changes = changes.clone();

        move |_window, _cx| TypingView { query, changes }
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let center = cx
        .debug_bounds("typing-search")
        .map(|bounds| bounds.center())
        .unwrap_or_else(|| point(px(150.), px(24.)));

    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();

    cx.simulate_input("eee");
    cx.run_until_parked();

    assert_eq!(
        query.borrow().as_str(),
        "eee",
        "Typing e e e into SearchBar should update the query; changes: {:?}",
        changes.borrow().as_slice()
    );
    assert!(
        !changes.borrow().is_empty(),
        "SearchBar on_change should be called while typing"
    );
}

#[gpui::test]
async fn test_search_bar_no_icon_no_clear(cx: &mut TestAppContext) {
    struct MinimalView;

    impl Render for MinimalView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                SearchBar::new("minimal-search")
                    .show_icon(false)
                    .show_clear(false),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| MinimalView);
}
