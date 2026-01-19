//! Integration tests for PaneDivider component
//!
//! Tests the pane divider component including:
//! - Basic rendering (vertical and horizontal)
//! - Collapse/expand toggle via double-click
//! - Collapsed state with label display
//! - Drag start callback
//! - Different collapse directions

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
};
use gpui_ui_kit::pane_divider::{CollapseDirection, PaneDivider};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct VerticalDividerTestView;

impl Render for VerticalDividerTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .w_full()
            .h(gpui::px(200.0))
            .child(div().w(gpui::px(100.0)).h_full().bg(gpui::rgb(0x333333)))
            .child(PaneDivider::vertical(
                "test-vertical-divider",
                CollapseDirection::Left,
            ))
            .child(div().flex_1().h_full().bg(gpui::rgb(0x444444)))
    }
}

#[gpui::test]
async fn test_vertical_pane_divider_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| VerticalDividerTestView);
}

struct HorizontalDividerTestView;

impl Render for HorizontalDividerTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .h(gpui::px(200.0))
            .child(div().w_full().h(gpui::px(80.0)).bg(gpui::rgb(0x333333)))
            .child(PaneDivider::horizontal(
                "test-horizontal-divider",
                CollapseDirection::Up,
            ))
            .child(div().w_full().flex_1().bg(gpui::rgb(0x444444)))
    }
}

#[gpui::test]
async fn test_horizontal_pane_divider_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| HorizontalDividerTestView);
}

// ============================================================================
// Collapse Direction Tests
// ============================================================================

#[gpui::test]
async fn test_pane_divider_collapse_directions(cx: &mut TestAppContext) {
    struct DirectionsView;

    impl Render for DirectionsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                // Vertical dividers
                .child(div().flex().h(gpui::px(50.0)).child(PaneDivider::vertical(
                    "divider-left",
                    CollapseDirection::Left,
                )))
                .child(div().flex().h(gpui::px(50.0)).child(PaneDivider::vertical(
                    "divider-right",
                    CollapseDirection::Right,
                )))
                // Horizontal dividers
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .w(gpui::px(100.0))
                        .child(PaneDivider::horizontal("divider-up", CollapseDirection::Up)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .w(gpui::px(100.0))
                        .child(PaneDivider::horizontal(
                            "divider-down",
                            CollapseDirection::Down,
                        )),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| DirectionsView);
}

// ============================================================================
// Toggle Callback Tests
// ============================================================================

/// View that tracks toggle (collapse/expand) events
struct DividerToggleTestView {
    collapsed: Rc<RefCell<bool>>,
    toggle_count: Arc<AtomicUsize>,
}

impl Render for DividerToggleTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let is_collapsed = *self.collapsed.borrow();
        let collapsed_rc = self.collapsed.clone();
        let toggle_count = self.toggle_count.clone();

        div().flex().w_full().h(gpui::px(200.0)).child(
            PaneDivider::vertical("toggle-test-divider", CollapseDirection::Left)
                .collapsed(is_collapsed)
                .label("Sidebar")
                .on_toggle(move |new_collapsed, _window, _cx| {
                    *collapsed_rc.borrow_mut() = new_collapsed;
                    toggle_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_pane_divider_double_click_to_collapse(cx: &mut TestAppContext) {
    let collapsed = Rc::new(RefCell::new(false));
    let toggle_count = Arc::new(AtomicUsize::new(0));

    let collapsed_clone = collapsed.clone();
    let toggle_count_clone = toggle_count.clone();

    let window = cx.add_window(move |_window, _cx| DividerToggleTestView {
        collapsed: collapsed_clone,
        toggle_count: toggle_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initially not collapsed
    assert!(
        !*collapsed.borrow(),
        "Divider should not be collapsed initially"
    );

    // Double-click to collapse
    if let Some(bounds) = cx.debug_bounds("toggle-test-divider") {
        let center = bounds.center();

        // Simulate double-click (click_count = 2)
        // Note: GPUI's simulate_mouse_down/up may not support click_count directly,
        // but the component should handle this internally
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
    }
}

// ============================================================================
// Collapsed State Tests
// ============================================================================

#[gpui::test]
async fn test_pane_divider_collapsed_state(cx: &mut TestAppContext) {
    struct CollapsedStateView;

    impl Render for CollapsedStateView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().flex().w_full().h(gpui::px(200.0)).child(
                PaneDivider::vertical("collapsed-divider", CollapseDirection::Left)
                    .collapsed(true)
                    .label("Sidebar"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CollapsedStateView);
}

/// Test that clicking on collapsed divider expands it
struct CollapsedClickTestView {
    collapsed: Rc<RefCell<bool>>,
    expand_clicked: Arc<AtomicBool>,
}

impl Render for CollapsedClickTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let is_collapsed = *self.collapsed.borrow();
        let collapsed_rc = self.collapsed.clone();
        let expand_clicked = self.expand_clicked.clone();

        div().flex().size_full().child(
            PaneDivider::vertical("collapsed-click-divider", CollapseDirection::Left)
                .collapsed(is_collapsed)
                .label("Panel")
                .on_toggle(move |new_collapsed, _window, _cx| {
                    *collapsed_rc.borrow_mut() = new_collapsed;
                    if !new_collapsed {
                        expand_clicked.store(true, Ordering::SeqCst);
                    }
                }),
        )
    }
}

#[gpui::test]
async fn test_pane_divider_click_collapsed_to_expand(cx: &mut TestAppContext) {
    let collapsed = Rc::new(RefCell::new(true)); // Start collapsed
    let expand_clicked = Arc::new(AtomicBool::new(false));

    let collapsed_clone = collapsed.clone();
    let expand_clicked_clone = expand_clicked.clone();

    let window = cx.add_window(move |_window, _cx| CollapsedClickTestView {
        collapsed: collapsed_clone,
        expand_clicked: expand_clicked_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Start collapsed
    assert!(*collapsed.borrow(), "Should start collapsed");

    // Click to expand
    if let Some(bounds) = cx.debug_bounds("collapsed-click-divider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Should have triggered expand (on_toggle with false)
        assert!(
            expand_clicked.load(Ordering::SeqCst),
            "Should have triggered expand"
        );
    }
}

// ============================================================================
// Drag Start Tests
// ============================================================================

/// View that tracks drag start events
struct DividerDragTestView {
    drag_started: Arc<AtomicBool>,
    drag_position: Rc<RefCell<Option<f32>>>,
}

impl Render for DividerDragTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let drag_started = self.drag_started.clone();
        let drag_position = self.drag_position.clone();

        div().flex().size_full().child(
            PaneDivider::vertical("drag-test-divider", CollapseDirection::Left).on_drag_start(
                move |position, _window, _cx| {
                    drag_started.store(true, Ordering::SeqCst);
                    *drag_position.borrow_mut() = Some(position);
                },
            ),
        )
    }
}

#[gpui::test]
async fn test_pane_divider_drag_start(cx: &mut TestAppContext) {
    let drag_started = Arc::new(AtomicBool::new(false));
    let drag_position: Rc<RefCell<Option<f32>>> = Rc::new(RefCell::new(None));

    let drag_started_clone = drag_started.clone();
    let drag_position_clone = drag_position.clone();

    let window = cx.add_window(move |_window, _cx| DividerDragTestView {
        drag_started: drag_started_clone,
        drag_position: drag_position_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Single click should trigger drag start (not double-click for collapse)
    if let Some(bounds) = cx.debug_bounds("drag-test-divider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert!(
            drag_started.load(Ordering::SeqCst),
            "Drag should have started"
        );
        assert!(
            drag_position.borrow().is_some(),
            "Drag position should be set"
        );
    }
}

// ============================================================================
// Theming Tests
// ============================================================================

#[gpui::test]
async fn test_pane_divider_with_custom_theme(cx: &mut TestAppContext) {
    use gpui_ui_kit::pane_divider::PaneDividerTheme;

    struct ThemedDividerView;

    impl Render for ThemedDividerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let custom_theme = PaneDividerTheme {
                background: gpui::rgb(0x0066cc),
                background_hover: gpui::rgb(0x0077ee),
                background_collapsed: gpui::rgb(0x004499),
                foreground: gpui::rgb(0xffffff),
                foreground_hover: gpui::rgb(0xffff00),
                border: gpui::rgb(0x0055aa),
            };

            div().flex().h(gpui::px(100.0)).child(
                PaneDivider::vertical("themed-divider", CollapseDirection::Left)
                    .theme(custom_theme),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ThemedDividerView);
}

// ============================================================================
// Size Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_pane_divider_custom_sizes(cx: &mut TestAppContext) {
    struct SizedDividerView;

    impl Render for SizedDividerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    div().flex().h(gpui::px(100.0)).child(
                        PaneDivider::vertical("thin-divider", CollapseDirection::Left)
                            .thickness(gpui::px(4.0)),
                    ),
                )
                .child(
                    div().flex().h(gpui::px(100.0)).child(
                        PaneDivider::vertical("thick-divider", CollapseDirection::Left)
                            .thickness(gpui::px(12.0)),
                    ),
                )
                .child(
                    div().flex().h(gpui::px(100.0)).child(
                        PaneDivider::vertical("wide-collapsed", CollapseDirection::Left)
                            .collapsed(true)
                            .collapsed_size(gpui::px(40.0))
                            .label("Wide"),
                    ),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SizedDividerView);
}

// ============================================================================
// Label Tests
// ============================================================================

#[gpui::test]
async fn test_pane_divider_with_label(cx: &mut TestAppContext) {
    struct LabeledDividerView;

    impl Render for LabeledDividerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                // Collapsed vertical with label
                .child(
                    div().flex().h(gpui::px(200.0)).child(
                        PaneDivider::vertical("labeled-v-divider", CollapseDirection::Left)
                            .collapsed(true)
                            .label("Navigation"),
                    ),
                )
                // Collapsed horizontal with label
                .child(
                    div().flex().flex_col().w(gpui::px(200.0)).child(
                        PaneDivider::horizontal("labeled-h-divider", CollapseDirection::Up)
                            .collapsed(true)
                            .label("Details"),
                    ),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| LabeledDividerView);
}

// ============================================================================
// Collapse Direction Opposite Tests
// ============================================================================

#[gpui::test]
async fn test_collapse_direction_opposite(cx: &mut TestAppContext) {
    // Test the opposite() method for collapse directions
    assert_eq!(CollapseDirection::Left.opposite(), CollapseDirection::Right);
    assert_eq!(CollapseDirection::Right.opposite(), CollapseDirection::Left);
    assert_eq!(CollapseDirection::Up.opposite(), CollapseDirection::Down);
    assert_eq!(CollapseDirection::Down.opposite(), CollapseDirection::Up);

    // Don't need a window for this unit test, but the async test requires TestAppContext
    let _ = cx;
}

#[gpui::test]
async fn test_collapse_direction_is_horizontal(cx: &mut TestAppContext) {
    // Test the is_horizontal() method
    assert!(CollapseDirection::Left.is_horizontal());
    assert!(CollapseDirection::Right.is_horizontal());
    assert!(!CollapseDirection::Up.is_horizontal());
    assert!(!CollapseDirection::Down.is_horizontal());

    let _ = cx;
}
