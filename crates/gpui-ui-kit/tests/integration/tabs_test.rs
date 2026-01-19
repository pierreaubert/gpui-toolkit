//! Integration tests for Tabs component
//!
//! Tests the Tabs and TabItem components including:
//! - All variants (Underline, Enclosed, Pills, VerticalCard)
//! - Tab selection and callbacks
//! - Disabled tabs
//! - Closeable tabs with callback
//! - Badges and icons

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
};
use gpui_ui_kit::tabs::{TabItem, TabVariant, Tabs};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct TabsTestView;

impl Render for TabsTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Tabs::new("tabs").tabs(vec![
            TabItem::new("tab1", "Tab 1"),
            TabItem::new("tab2", "Tab 2"),
            TabItem::new("tab3", "Tab 3"),
        ]))
    }
}

#[gpui::test]
async fn test_tabs_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| TabsTestView);
}

// ============================================================================
// Variant Tests
// ============================================================================

#[gpui::test]
async fn test_tabs_underline_variant(cx: &mut TestAppContext) {
    struct UnderlineView;

    impl Render for UnderlineView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tabs::new("tabs").variant(TabVariant::Underline).tabs(vec![
                TabItem::new("tab1", "Tab 1"),
                TabItem::new("tab2", "Tab 2"),
            ]))
        }
    }

    let _window = cx.add_window(|_window, _cx| UnderlineView);
}

#[gpui::test]
async fn test_tabs_enclosed_variant(cx: &mut TestAppContext) {
    struct EnclosedView;

    impl Render for EnclosedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tabs::new("tabs").variant(TabVariant::Enclosed).tabs(vec![
                TabItem::new("tab1", "Tab 1"),
                TabItem::new("tab2", "Tab 2"),
            ]))
        }
    }

    let _window = cx.add_window(|_window, _cx| EnclosedView);
}

#[gpui::test]
async fn test_tabs_pills_variant(cx: &mut TestAppContext) {
    struct PillsView;

    impl Render for PillsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tabs::new("tabs").variant(TabVariant::Pills).tabs(vec![
                TabItem::new("tab1", "Tab 1"),
                TabItem::new("tab2", "Tab 2"),
            ]))
        }
    }

    let _window = cx.add_window(|_window, _cx| PillsView);
}

#[gpui::test]
async fn test_tabs_vertical_card_variant(cx: &mut TestAppContext) {
    struct VerticalCardView;

    impl Render for VerticalCardView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Tabs::new("tabs")
                    .variant(TabVariant::VerticalCard)
                    .tabs(vec![
                        TabItem::new("tab1", "Tab 1").icon("📄"),
                        TabItem::new("tab2", "Tab 2").icon("📁"),
                    ]),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| VerticalCardView);
}

// ============================================================================
// Selection Tests
// ============================================================================

#[gpui::test]
async fn test_tabs_selected_index(cx: &mut TestAppContext) {
    struct SelectedIndexView;

    impl Render for SelectedIndexView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Tabs::new("tabs")
                    .tabs(vec![
                        TabItem::new("tab1", "Tab 1"),
                        TabItem::new("tab2", "Tab 2"),
                        TabItem::new("tab3", "Tab 3"),
                    ])
                    .selected_index(1), // Second tab selected
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SelectedIndexView);
}

struct SelectableTabsView {
    selected_index: Rc<RefCell<usize>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for SelectableTabsView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_index = *self.selected_index.borrow();
        let selected_index = self.selected_index.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            Tabs::new("tabs")
                .tabs(vec![
                    TabItem::new("tab1", "Tab 1"),
                    TabItem::new("tab2", "Tab 2"),
                ])
                .selected_index(current_index)
                .on_change(move |index, _window, _cx| {
                    *selected_index.borrow_mut() = index;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_tabs_selection_callback(cx: &mut TestAppContext) {
    let selected_index: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let selected_index_clone = selected_index.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SelectableTabsView {
        selected_index: selected_index_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Click on second tab
    if let Some(bounds) = cx.debug_bounds("tab-tab2") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            *selected_index.borrow(),
            1,
            "Selected index should be 1 after clicking second tab"
        );
        assert_eq!(
            change_count.load(Ordering::SeqCst),
            1,
            "on_change should have been called"
        );
    }
}

// ============================================================================
// Disabled Tab Tests
// ============================================================================

#[gpui::test]
async fn test_tabs_disabled(cx: &mut TestAppContext) {
    struct DisabledTabView;

    impl Render for DisabledTabView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tabs::new("tabs").tabs(vec![
                TabItem::new("tab1", "Tab 1"),
                TabItem::new("tab2", "Tab 2").disabled(true),
                TabItem::new("tab3", "Tab 3"),
            ]))
        }
    }

    let _window = cx.add_window(|_window, _cx| DisabledTabView);
}

// ============================================================================
// Closeable Tab Tests
// ============================================================================

struct CloseableTabsView {
    close_count: Arc<AtomicUsize>,
    last_closed: Rc<RefCell<Option<String>>>,
}

impl Render for CloseableTabsView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let close_count = self.close_count.clone();
        let last_closed = self.last_closed.clone();

        div().size_full().child(
            Tabs::new("tabs")
                .tabs(vec![
                    TabItem::new("tab1", "Tab 1").closeable(true),
                    TabItem::new("tab2", "Tab 2").closeable(true),
                ])
                .on_close(move |id, _window, _cx| {
                    close_count.fetch_add(1, Ordering::SeqCst);
                    *last_closed.borrow_mut() = Some(id.to_string());
                }),
        )
    }
}

#[gpui::test]
async fn test_tabs_closeable(cx: &mut TestAppContext) {
    let close_count = Arc::new(AtomicUsize::new(0));
    let last_closed: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let close_count_clone = close_count.clone();
    let last_closed_clone = last_closed.clone();

    let window = cx.add_window(move |_window, _cx| CloseableTabsView {
        close_count: close_count_clone,
        last_closed: last_closed_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Click on close button of first tab
    if let Some(bounds) = cx.debug_bounds("tab-close-tab1") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            close_count.load(Ordering::SeqCst),
            1,
            "on_close should have been called"
        );
        assert_eq!(
            *last_closed.borrow(),
            Some("tab1".to_string()),
            "Closed tab should be tab1"
        );
    }
}

// ============================================================================
// Icon and Badge Tests
// ============================================================================

#[gpui::test]
async fn test_tabs_with_icons(cx: &mut TestAppContext) {
    struct IconTabsView;

    impl Render for IconTabsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tabs::new("tabs").tabs(vec![
                TabItem::new("home", "Home").icon("🏠"),
                TabItem::new("settings", "Settings").icon("⚙️"),
                TabItem::new("profile", "Profile").icon("👤"),
            ]))
        }
    }

    let _window = cx.add_window(|_window, _cx| IconTabsView);
}

#[gpui::test]
async fn test_tabs_with_badges(cx: &mut TestAppContext) {
    struct BadgeTabsView;

    impl Render for BadgeTabsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tabs::new("tabs").tabs(vec![
                TabItem::new("inbox", "Inbox").badge("5"),
                TabItem::new("sent", "Sent").badge("12"),
                TabItem::new("drafts", "Drafts"),
            ]))
        }
    }

    let _window = cx.add_window(|_window, _cx| BadgeTabsView);
}

#[gpui::test]
async fn test_tabs_with_icons_and_badges(cx: &mut TestAppContext) {
    struct IconBadgeTabsView;

    impl Render for IconBadgeTabsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tabs::new("tabs").tabs(vec![
                    TabItem::new("notifications", "Notifications")
                        .icon("🔔")
                        .badge("3"),
                    TabItem::new("messages", "Messages").icon("💬").badge("99+"),
                ]))
        }
    }

    let _window = cx.add_window(|_window, _cx| IconBadgeTabsView);
}

// ============================================================================
// Default Tests
// ============================================================================

#[gpui::test]
async fn test_tabs_default(cx: &mut TestAppContext) {
    struct DefaultView;

    impl Render for DefaultView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tabs::default())
        }
    }

    let _window = cx.add_window(|_window, _cx| DefaultView);
}

// ============================================================================
// Complex Tabs Tests
// ============================================================================

#[gpui::test]
async fn test_tabs_complex(cx: &mut TestAppContext) {
    struct ComplexTabsView;

    impl Render for ComplexTabsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Tabs::new("tabs")
                    .variant(TabVariant::Pills)
                    .tabs(vec![
                        TabItem::new("active", "Active").icon("✓").badge("12"),
                        TabItem::new("pending", "Pending").icon("⏳").badge("3"),
                        TabItem::new("archived", "Archived")
                            .icon("📦")
                            .disabled(true),
                    ])
                    .selected_index(0),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ComplexTabsView);
}
