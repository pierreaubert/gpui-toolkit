//! Integration tests for PaneDivider drag-to-resize behavior
//!
//! Tests the full drag flow:
//! - Mouse down on divider triggers on_drag_start
//! - Mouse move on parent element updates panel size
//! - Mouse up ends drag
//! - Clamping at min/max bounds
//! - No-op when not dragging

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, point,
    prelude::*, px,
};
use gpui_ui_kit::pane_divider::{CollapseDirection, PaneDivider};
use std::cell::RefCell;
use std::rc::Rc;

const MIN_SIZE: f32 = 50.0;
const MAX_SIZE: f32 = 400.0;
const INITIAL_WIDTH: f32 = 150.0;

/// Shared drag state between test and view
#[derive(Clone)]
struct DragState {
    left_width: Rc<RefCell<f32>>,
    drag: Rc<RefCell<Option<(f32, f32)>>>, // (start_mouse_x, panel_width_at_start)
}

impl DragState {
    fn new() -> Self {
        Self {
            left_width: Rc::new(RefCell::new(INITIAL_WIDTH)),
            drag: Rc::new(RefCell::new(None)),
        }
    }
}

struct DragResizeTestView {
    state: DragState,
}

impl Render for DragResizeTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.clone();
        let state_move = self.state.clone();
        let state_up = self.state.clone();
        let left_width = *self.state.left_width.borrow();

        let mut root = div()
            .id("drag-resize-root")
            .w(px(600.0))
            .h(px(200.0))
            .flex();

        // Parent-level drag tracking
        root = root.on_mouse_move(move |event, _window, _cx| {
            let drag = *state_move.drag.borrow();
            if let Some((start_x, start_width)) = drag {
                let current_x: f32 = event.position.x.into();
                let delta = current_x - start_x;
                *state_move.left_width.borrow_mut() = (start_width + delta).clamp(MIN_SIZE, MAX_SIZE);
            }
        });

        root = root.on_mouse_up(MouseButton::Left, move |_event, _window, _cx| {
            *state_up.drag.borrow_mut() = None;
        });

        root.child(
            div()
                .id("left-panel")
                .w(px(left_width))
                .h_full()
                .bg(gpui::rgb(0x333333)),
        )
        .child(
            PaneDivider::vertical("resize-divider", CollapseDirection::Left)
                .on_drag_start(move |pos, _w, _cx| {
                    let width = *state.left_width.borrow();
                    *state.drag.borrow_mut() = Some((pos, width));
                }),
        )
        .child(div().id("right-panel").flex_1().h_full().bg(gpui::rgb(0x444444)))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[gpui::test]
async fn test_drag_start_sets_drag_state(cx: &mut TestAppContext) {
    let state = DragState::new();
    let state_check = state.clone();

    let window = cx.add_window(move |_window, _cx| DragResizeTestView { state });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("resize-divider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert!(
            state_check.drag.borrow().is_some(),
            "Drag state should be set after mouse down on divider"
        );
    }
}

#[gpui::test]
async fn test_drag_right_increases_left_panel_width(cx: &mut TestAppContext) {
    let state = DragState::new();
    let state_check = state.clone();

    let window = cx.add_window(move |_window, _cx| DragResizeTestView { state });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!((*state_check.left_width.borrow() - INITIAL_WIDTH).abs() < 0.01);

    if let Some(bounds) = cx.debug_bounds("resize-divider") {
        let center = bounds.center();

        // Mouse down on divider
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert!(state_check.drag.borrow().is_some(), "Drag should be active");

        // Mouse move 50px to the right
        let new_pos = point(center.x + px(50.0), center.y);
        cx.simulate_mouse_move(new_pos, Some(MouseButton::Left), Modifiers::default());
        cx.run_until_parked();

        let new_width = *state_check.left_width.borrow();
        assert!(
            (new_width - (INITIAL_WIDTH + 50.0)).abs() < 1.0,
            "Width should increase by ~50px, got {:.1} (delta={:.1})",
            new_width,
            new_width - INITIAL_WIDTH
        );
    }
}

#[gpui::test]
async fn test_drag_left_decreases_left_panel_width(cx: &mut TestAppContext) {
    let state = DragState::new();
    let state_check = state.clone();

    let window = cx.add_window(move |_window, _cx| DragResizeTestView { state });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("resize-divider") {
        let center = bounds.center();

        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Move 30px to the left
        let new_pos = point(center.x - px(30.0), center.y);
        cx.simulate_mouse_move(new_pos, Some(MouseButton::Left), Modifiers::default());
        cx.run_until_parked();

        let new_width = *state_check.left_width.borrow();
        assert!(
            (new_width - (INITIAL_WIDTH - 30.0)).abs() < 1.0,
            "Width should decrease by ~30px, got {:.1} (delta={:.1})",
            new_width,
            new_width - INITIAL_WIDTH
        );
    }
}

#[gpui::test]
async fn test_drag_clamps_at_min_size(cx: &mut TestAppContext) {
    let state = DragState::new();
    let state_check = state.clone();

    let window = cx.add_window(move |_window, _cx| DragResizeTestView { state });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("resize-divider") {
        let center = bounds.center();

        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Drag way past min
        let new_pos = point(center.x - px(500.0), center.y);
        cx.simulate_mouse_move(new_pos, Some(MouseButton::Left), Modifiers::default());
        cx.run_until_parked();

        let new_width = *state_check.left_width.borrow();
        assert!(
            (new_width - MIN_SIZE).abs() < 0.01,
            "Width should clamp at min={}, got {}",
            MIN_SIZE,
            new_width
        );
    }
}

#[gpui::test]
async fn test_drag_clamps_at_max_size(cx: &mut TestAppContext) {
    let state = DragState::new();
    let state_check = state.clone();

    let window = cx.add_window(move |_window, _cx| DragResizeTestView { state });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("resize-divider") {
        let center = bounds.center();

        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Drag way past max
        let new_pos = point(center.x + px(500.0), center.y);
        cx.simulate_mouse_move(new_pos, Some(MouseButton::Left), Modifiers::default());
        cx.run_until_parked();

        let new_width = *state_check.left_width.borrow();
        assert!(
            (new_width - MAX_SIZE).abs() < 0.01,
            "Width should clamp at max={}, got {}",
            MAX_SIZE,
            new_width
        );
    }
}

#[gpui::test]
async fn test_mouse_up_ends_drag(cx: &mut TestAppContext) {
    let state = DragState::new();
    let state_check = state.clone();

    let window = cx.add_window(move |_window, _cx| DragResizeTestView { state });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("resize-divider") {
        let center = bounds.center();

        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        assert!(state_check.drag.borrow().is_some());

        // Move 50px right
        let pos1 = point(center.x + px(50.0), center.y);
        cx.simulate_mouse_move(pos1, Some(MouseButton::Left), Modifiers::default());
        cx.run_until_parked();

        let width_after_drag = *state_check.left_width.borrow();

        // Mouse up
        cx.simulate_mouse_up(pos1, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        assert!(state_check.drag.borrow().is_none(), "Drag should end after mouse up");

        // Further mouse move should NOT change width
        let pos2 = point(center.x + px(100.0), center.y);
        cx.simulate_mouse_move(pos2, Some(MouseButton::Left), Modifiers::default());
        cx.run_until_parked();

        let width_after_up = *state_check.left_width.borrow();
        assert!(
            (width_after_up - width_after_drag).abs() < 0.01,
            "Width should not change after mouse up"
        );
    }
}

#[gpui::test]
async fn test_mouse_move_without_drag_start_does_nothing(cx: &mut TestAppContext) {
    let state = DragState::new();
    let state_check = state.clone();

    let window = cx.add_window(move |_window, _cx| DragResizeTestView { state });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Move mouse without clicking - should not change width
    let pos = point(px(300.0), px(100.0));
    cx.simulate_mouse_move(pos, None, Modifiers::default());
    cx.run_until_parked();

    let width = *state_check.left_width.borrow();
    assert!(
        (width - INITIAL_WIDTH).abs() < 0.01,
        "Width should not change without drag, got {}",
        width
    );
}
