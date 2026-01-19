//! Integration tests for Accordion component
//!
//! Tests the accordion component including:
//! - Basic rendering with items
//! - Single vs Multiple expand modes
//! - Expand/collapse via click
//! - Different orientations (Vertical, Horizontal, Side)
//! - Disabled items
//! - on_change callback

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
};
use gpui_ui_kit::accordion::{Accordion, AccordionItem, AccordionMode, AccordionOrientation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct AccordionTestView;

impl Render for AccordionTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Accordion::new().items(vec![
            AccordionItem::new("item1", "Section 1").content("Content 1"),
            AccordionItem::new("item2", "Section 2").content("Content 2"),
        ]))
    }
}

#[gpui::test]
async fn test_accordion_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| AccordionTestView);
}

// ============================================================================
// Expand/Collapse Tests
// ============================================================================

/// View that tracks expand/collapse events
struct AccordionExpandTestView {
    expanded: Rc<RefCell<Vec<String>>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for AccordionExpandTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let expanded_items: Vec<gpui::SharedString> = self
            .expanded
            .borrow()
            .iter()
            .map(|s| gpui::SharedString::from(s.clone()))
            .collect();

        let expanded_rc = self.expanded.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            Accordion::new()
                .items(vec![
                    AccordionItem::new("section-a", "Section A").content(div().child("Content A")),
                    AccordionItem::new("section-b", "Section B").content(div().child("Content B")),
                    AccordionItem::new("section-c", "Section C").content(div().child("Content C")),
                ])
                .expanded(expanded_items)
                .on_change(move |item_id, is_expanded, _window, _cx| {
                    change_count.fetch_add(1, Ordering::SeqCst);
                    let mut expanded = expanded_rc.borrow_mut();
                    if is_expanded {
                        if !expanded.contains(&item_id.to_string()) {
                            expanded.push(item_id.to_string());
                        }
                    } else {
                        expanded.retain(|id| id != &item_id.to_string());
                    }
                }),
        )
    }
}

#[gpui::test]
async fn test_accordion_click_to_expand(cx: &mut TestAppContext) {
    let expanded: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let change_count = Arc::new(AtomicUsize::new(0));

    let expanded_clone = expanded.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| AccordionExpandTestView {
        expanded: expanded_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initially nothing expanded
    assert!(
        expanded.borrow().is_empty(),
        "Nothing should be expanded initially"
    );

    // Click on first section header to expand
    if let Some(bounds) = cx.debug_bounds("accordion-header-section-a") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            change_count.load(Ordering::SeqCst),
            1,
            "on_change should have been called"
        );
        // Note: The callback should have added "section-a" to expanded
    }
}

// ============================================================================
// Single Mode Tests
// ============================================================================

#[gpui::test]
async fn test_accordion_single_mode(cx: &mut TestAppContext) {
    struct SingleModeView;

    impl Render for SingleModeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Accordion::new()
                    .mode(AccordionMode::Single)
                    .items(vec![
                        AccordionItem::new("s1", "Section 1").content("Content 1"),
                        AccordionItem::new("s2", "Section 2").content("Content 2"),
                    ])
                    .expanded(vec!["s1".into()]), // Only one expanded in single mode
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SingleModeView);
}

// ============================================================================
// Multiple Mode Tests
// ============================================================================

#[gpui::test]
async fn test_accordion_multiple_mode(cx: &mut TestAppContext) {
    struct MultipleModeView;

    impl Render for MultipleModeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Accordion::new()
                    .mode(AccordionMode::Multiple)
                    .items(vec![
                        AccordionItem::new("m1", "Multi 1").content("Content 1"),
                        AccordionItem::new("m2", "Multi 2").content("Content 2"),
                        AccordionItem::new("m3", "Multi 3").content("Content 3"),
                    ])
                    .expanded(vec!["m1".into(), "m3".into()]), // Multiple expanded
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| MultipleModeView);
}

// ============================================================================
// Orientation Tests
// ============================================================================

#[gpui::test]
async fn test_accordion_vertical_orientation(cx: &mut TestAppContext) {
    struct VerticalView;

    impl Render for VerticalView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Accordion::new()
                    .orientation(AccordionOrientation::Vertical)
                    .items(vec![
                        AccordionItem::new("v1", "Vertical 1").content("Content 1"),
                        AccordionItem::new("v2", "Vertical 2").content("Content 2"),
                    ])
                    .expanded(vec!["v1".into()]),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| VerticalView);
}

#[gpui::test]
async fn test_accordion_horizontal_orientation(cx: &mut TestAppContext) {
    struct HorizontalView;

    impl Render for HorizontalView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Accordion::new()
                    .orientation(AccordionOrientation::Horizontal)
                    .items(vec![
                        AccordionItem::new("h1", "Horiz 1").content("Content 1"),
                        AccordionItem::new("h2", "Horiz 2").content("Content 2"),
                    ])
                    .expanded(vec!["h1".into()]),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| HorizontalView);
}

#[gpui::test]
async fn test_accordion_side_orientation(cx: &mut TestAppContext) {
    struct SideView;

    impl Render for SideView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().h(gpui::px(300.0)).child(
                Accordion::new()
                    .orientation(AccordionOrientation::Side)
                    .items(vec![
                        AccordionItem::new("side1", "Side 1").content("Side content 1"),
                        AccordionItem::new("side2", "Side 2").content("Side content 2"),
                    ])
                    .expanded(vec!["side1".into()]),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SideView);
}

// ============================================================================
// Disabled Item Tests
// ============================================================================

/// View with disabled items that tracks click attempts
struct DisabledItemTestView {
    change_count: Arc<AtomicUsize>,
}

impl Render for DisabledItemTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let change_count = self.change_count.clone();

        div().size_full().child(
            Accordion::new()
                .items(vec![
                    AccordionItem::new("enabled-item", "Enabled").content("This can be clicked"),
                    AccordionItem::new("disabled-item", "Disabled")
                        .content("This cannot be clicked")
                        .disabled(true),
                ])
                .on_change(move |_id, _expanded, _window, _cx| {
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_accordion_disabled_item(cx: &mut TestAppContext) {
    let change_count = Arc::new(AtomicUsize::new(0));
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| DisabledItemTestView {
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Try clicking the disabled item
    if let Some(bounds) = cx.debug_bounds("accordion-header-disabled-item") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // on_change should NOT have been called for disabled item
        assert_eq!(
            change_count.load(Ordering::SeqCst),
            0,
            "Disabled item should not trigger on_change"
        );
    }
}

// ============================================================================
// Content Tests
// ============================================================================

#[gpui::test]
async fn test_accordion_with_rich_content(cx: &mut TestAppContext) {
    struct RichContentView;

    impl Render for RichContentView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Accordion::new()
                    .items(vec![
                        AccordionItem::new("rich1", "Rich Content 1").content(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(div().child("Line 1"))
                                .child(div().child("Line 2"))
                                .child(div().child("Line 3")),
                        ),
                        AccordionItem::new("rich2", "Rich Content 2").content(
                            div()
                                .flex()
                                .gap_2()
                                .child(div().bg(gpui::rgb(0xff0000)).size(gpui::px(20.0)))
                                .child(div().bg(gpui::rgb(0x00ff00)).size(gpui::px(20.0)))
                                .child(div().bg(gpui::rgb(0x0000ff)).size(gpui::px(20.0))),
                        ),
                    ])
                    .expanded(vec!["rich1".into(), "rich2".into()]),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| RichContentView);
}

// ============================================================================
// Item API Tests
// ============================================================================

#[gpui::test]
async fn test_accordion_item_api(cx: &mut TestAppContext) {
    // Test AccordionItem builder methods
    let item = AccordionItem::new("test-id", "Test Title")
        .content("Test content")
        .disabled(true);

    assert_eq!(item.id(), "test-id");

    let _ = cx; // Satisfy async requirement
}

#[gpui::test]
async fn test_accordion_add_single_item(cx: &mut TestAppContext) {
    struct SingleItemView;

    impl Render for SingleItemView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Accordion::new()
                    .item(AccordionItem::new("single1", "Single 1").content("Content 1"))
                    .item(AccordionItem::new("single2", "Single 2").content("Content 2")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SingleItemView);
}

// ============================================================================
// Theming Tests
// ============================================================================

#[gpui::test]
async fn test_accordion_with_custom_theme(cx: &mut TestAppContext) {
    use gpui_ui_kit::accordion::AccordionTheme;

    struct ThemedAccordionView;

    impl Render for ThemedAccordionView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let custom_theme = AccordionTheme {
                header_bg: gpui::rgb(0x0066cc),
                header_hover_bg: gpui::rgb(0x0077ee),
                content_bg: gpui::rgb(0x001133),
                border: gpui::rgb(0x0055aa),
                title_color: gpui::rgb(0xffffff),
                indicator_color: gpui::rgb(0xcccccc),
            };

            div().child(
                Accordion::new()
                    .theme(custom_theme)
                    .items(vec![
                        AccordionItem::new("themed1", "Themed 1").content("Content 1"),
                        AccordionItem::new("themed2", "Themed 2").content("Content 2"),
                    ])
                    .expanded(vec!["themed1".into()]),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ThemedAccordionView);
}

// ============================================================================
// Collapse Toggle Tests
// ============================================================================

/// Test toggling an already expanded item to collapse it
struct CollapseTestView {
    expanded: Rc<RefCell<Vec<String>>>,
}

impl Render for CollapseTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let expanded_items: Vec<gpui::SharedString> = self
            .expanded
            .borrow()
            .iter()
            .map(|s| gpui::SharedString::from(s.clone()))
            .collect();

        let expanded_rc = self.expanded.clone();

        div().size_full().child(
            Accordion::new()
                .items(vec![
                    AccordionItem::new("collapse-test", "Toggle Me").content("I can be collapsed"),
                ])
                .expanded(expanded_items)
                .on_change(move |item_id, is_expanded, _window, _cx| {
                    let mut expanded = expanded_rc.borrow_mut();
                    if is_expanded {
                        expanded.push(item_id.to_string());
                    } else {
                        expanded.retain(|id| id != &item_id.to_string());
                    }
                }),
        )
    }
}

#[gpui::test]
async fn test_accordion_collapse_expanded_item(cx: &mut TestAppContext) {
    // Start with item expanded
    let expanded: Rc<RefCell<Vec<String>>> =
        Rc::new(RefCell::new(vec!["collapse-test".to_string()]));
    let expanded_clone = expanded.clone();

    let window = cx.add_window(move |_window, _cx| CollapseTestView {
        expanded: expanded_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initially expanded
    assert!(
        !expanded.borrow().is_empty(),
        "Item should be expanded initially"
    );

    // Click to collapse
    if let Some(bounds) = cx.debug_bounds("accordion-header-collapse-test") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Callback should have removed the item from expanded list
    }
}

// ============================================================================
// Empty Accordion Tests
// ============================================================================

#[gpui::test]
async fn test_accordion_empty(cx: &mut TestAppContext) {
    struct EmptyAccordionView;

    impl Render for EmptyAccordionView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Accordion::new())
        }
    }

    let _window = cx.add_window(|_window, _cx| EmptyAccordionView);
}
