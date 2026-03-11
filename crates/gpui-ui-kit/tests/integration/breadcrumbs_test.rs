//! Integration tests for Breadcrumbs component
//!
//! Tests the Breadcrumbs component including:
//! - Click callback on breadcrumb items
//! - Separator variants
//! - Items with icons
//! - Single item
//! - Empty breadcrumbs

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
};
use gpui_ui_kit::breadcrumbs::{BreadcrumbItem, BreadcrumbSeparator, Breadcrumbs};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct BreadcrumbsTestView;

impl Render for BreadcrumbsTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Breadcrumbs::new().items(vec![
            BreadcrumbItem::new("home", "Home"),
            BreadcrumbItem::new("docs", "Docs"),
            BreadcrumbItem::new("api", "API"),
        ]))
    }
}

#[gpui::test]
async fn test_breadcrumbs_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| BreadcrumbsTestView);
}

// ============================================================================
// Click Callback Tests
// ============================================================================

struct ClickableBreadcrumbsView {
    click_count: Arc<AtomicUsize>,
    last_clicked: Rc<RefCell<Option<String>>>,
}

impl Render for ClickableBreadcrumbsView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let click_count = self.click_count.clone();
        let last_clicked = self.last_clicked.clone();

        div().size_full().child(
            Breadcrumbs::new()
                .items(vec![
                    BreadcrumbItem::new("home", "Home"),
                    BreadcrumbItem::new("docs", "Docs"),
                    BreadcrumbItem::new("api", "API"),
                ])
                .on_click(move |id, _window, _cx| {
                    click_count.fetch_add(1, Ordering::SeqCst);
                    *last_clicked.borrow_mut() = Some(id.to_string());
                }),
        )
    }
}

#[gpui::test]
async fn test_breadcrumbs_click_item_callback(cx: &mut TestAppContext) {
    let click_count = Arc::new(AtomicUsize::new(0));
    let last_clicked: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let click_count_clone = click_count.clone();
    let last_clicked_clone = last_clicked.clone();

    let window = cx.add_window(move |_window, _cx| ClickableBreadcrumbsView {
        click_count: click_count_clone,
        last_clicked: last_clicked_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Click on the "home" breadcrumb (non-last item should fire callback)
    if let Some(bounds) = cx.debug_bounds("breadcrumb-home") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            click_count.load(Ordering::SeqCst),
            1,
            "on_click should have been called"
        );
        assert_eq!(
            *last_clicked.borrow(),
            Some("home".to_string()),
            "Clicked item should be 'home'"
        );
    }
}

// ============================================================================
// Separator Variant Tests
// ============================================================================

#[gpui::test]
async fn test_breadcrumbs_separator_slash(cx: &mut TestAppContext) {
    struct SlashSeparatorView;

    impl Render for SlashSeparatorView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Breadcrumbs::new()
                    .items(vec![
                        BreadcrumbItem::new("a", "A"),
                        BreadcrumbItem::new("b", "B"),
                    ])
                    .separator(BreadcrumbSeparator::Slash),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SlashSeparatorView);
}

#[gpui::test]
async fn test_breadcrumbs_separator_chevron(cx: &mut TestAppContext) {
    struct ChevronSeparatorView;

    impl Render for ChevronSeparatorView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Breadcrumbs::new()
                    .items(vec![
                        BreadcrumbItem::new("a", "A"),
                        BreadcrumbItem::new("b", "B"),
                    ])
                    .separator(BreadcrumbSeparator::Chevron),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ChevronSeparatorView);
}

#[gpui::test]
async fn test_breadcrumbs_separator_dot(cx: &mut TestAppContext) {
    struct DotSeparatorView;

    impl Render for DotSeparatorView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Breadcrumbs::new()
                    .items(vec![
                        BreadcrumbItem::new("a", "A"),
                        BreadcrumbItem::new("b", "B"),
                    ])
                    .separator(BreadcrumbSeparator::Dot),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| DotSeparatorView);
}

// ============================================================================
// Items with Icons Tests
// ============================================================================

#[gpui::test]
async fn test_breadcrumbs_with_icons(cx: &mut TestAppContext) {
    struct IconBreadcrumbsView;

    impl Render for IconBreadcrumbsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Breadcrumbs::new().items(vec![
                BreadcrumbItem::new("home", "Home").icon("🏠"),
                BreadcrumbItem::new("settings", "Settings").icon("⚙"),
            ]))
        }
    }

    let _window = cx.add_window(|_window, _cx| IconBreadcrumbsView);
}

// ============================================================================
// Single Item Test
// ============================================================================

#[gpui::test]
async fn test_breadcrumbs_single_item(cx: &mut TestAppContext) {
    struct SingleItemView;

    impl Render for SingleItemView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Breadcrumbs::new().items(vec![BreadcrumbItem::new("home", "Home")]))
        }
    }

    let _window = cx.add_window(|_window, _cx| SingleItemView);
}

// ============================================================================
// Empty Breadcrumbs Test
// ============================================================================

#[gpui::test]
async fn test_breadcrumbs_empty(cx: &mut TestAppContext) {
    struct EmptyBreadcrumbsView;

    impl Render for EmptyBreadcrumbsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Breadcrumbs::new())
        }
    }

    let _window = cx.add_window(|_window, _cx| EmptyBreadcrumbsView);
}
