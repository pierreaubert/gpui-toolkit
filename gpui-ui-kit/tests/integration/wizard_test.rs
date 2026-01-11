//! Integration tests for Wizard component
//!
//! Tests the wizard component including:
//! - Basic rendering with step indicators
//! - Step status display (Active, Completed, Error, Skipped)
//! - Navigation callbacks (back, next, finish, cancel)
//! - Busy/disabled states
//! - WizardHeader and WizardNavigation sub-components

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
};
use gpui_ui_kit::wizard::{StepStatus, Wizard, WizardHeader, WizardNavigation, WizardStep};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct WizardTestView;

impl Render for WizardTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            Wizard::new()
                .steps(vec![
                    WizardStep::new("step1", "Step 1"),
                    WizardStep::new("step2", "Step 2"),
                    WizardStep::new("step3", "Step 3"),
                ])
                .current_step(0),
        )
    }
}

#[gpui::test]
async fn test_wizard_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| WizardTestView);
}

/// Test wizard with various step statuses
#[gpui::test]
async fn test_wizard_step_statuses(cx: &mut TestAppContext) {
    struct StatusTestView;

    impl Render for StatusTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Wizard::new()
                    .steps(vec![
                        WizardStep::new("step1", "Completed"),
                        WizardStep::new("step2", "Active"),
                        WizardStep::new("step3", "Not Visited"),
                        WizardStep::new("step4", "Error"),
                        WizardStep::new("step5", "Skipped"),
                    ])
                    .step_statuses(vec![
                        StepStatus::Completed,
                        StepStatus::Active,
                        StepStatus::NotVisited,
                        StepStatus::Error,
                        StepStatus::Skipped,
                    ])
                    .current_step(1),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| StatusTestView);
}

// ============================================================================
// Navigation Callback Tests
// ============================================================================

/// View that tracks navigation events
struct WizardNavTestView {
    current_step: Rc<RefCell<usize>>,
    back_count: Arc<AtomicUsize>,
    next_count: Arc<AtomicUsize>,
    cancel_clicked: Arc<AtomicBool>,
    finish_clicked: Arc<AtomicBool>,
}

impl Render for WizardNavTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let step = *self.current_step.borrow();
        let current_step_rc = self.current_step.clone();
        let back_count = self.back_count.clone();
        let next_count = self.next_count.clone();
        let cancel_clicked = self.cancel_clicked.clone();
        let finish_clicked = self.finish_clicked.clone();

        div().child(
            Wizard::new()
                .steps(vec![
                    WizardStep::new("step1", "Step 1"),
                    WizardStep::new("step2", "Step 2"),
                    WizardStep::new("step3", "Step 3"),
                ])
                .current_step(step)
                .show_cancel(true)
                .on_back({
                    let current_step_rc = current_step_rc.clone();
                    move |step, _window, _cx| {
                        back_count.fetch_add(1, Ordering::SeqCst);
                        if step > 0 {
                            *current_step_rc.borrow_mut() = step - 1;
                        }
                    }
                })
                .on_next({
                    let current_step_rc = current_step_rc.clone();
                    move |step, _window, _cx| {
                        next_count.fetch_add(1, Ordering::SeqCst);
                        *current_step_rc.borrow_mut() = step + 1;
                    }
                })
                .on_cancel(move |_window, _cx| {
                    cancel_clicked.store(true, Ordering::SeqCst);
                })
                .on_finish(move |_window, _cx| {
                    finish_clicked.store(true, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_wizard_navigation_callbacks(cx: &mut TestAppContext) {
    let current_step = Rc::new(RefCell::new(0_usize));
    let back_count = Arc::new(AtomicUsize::new(0));
    let next_count = Arc::new(AtomicUsize::new(0));
    let cancel_clicked = Arc::new(AtomicBool::new(false));
    let finish_clicked = Arc::new(AtomicBool::new(false));

    let current_step_clone = current_step.clone();
    let back_count_clone = back_count.clone();
    let next_count_clone = next_count.clone();
    let cancel_clicked_clone = cancel_clicked.clone();
    let finish_clicked_clone = finish_clicked.clone();

    let window = cx.add_window(move |_window, _cx| WizardNavTestView {
        current_step: current_step_clone,
        back_count: back_count_clone,
        next_count: next_count_clone,
        cancel_clicked: cancel_clicked_clone,
        finish_clicked: finish_clicked_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Try clicking the Next button
    if let Some(bounds) = cx.debug_bounds("wizard-next") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert!(
            next_count.load(Ordering::SeqCst) >= 1,
            "on_next should have been called"
        );
    }
}

// ============================================================================
// WizardHeader Tests
// ============================================================================

struct WizardHeaderTestView;

impl Render for WizardHeaderTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            WizardHeader::new()
                .title("Test Wizard")
                .steps(vec![
                    WizardStep::new("step1", "First"),
                    WizardStep::new("step2", "Second"),
                    WizardStep::new("step3", "Third"),
                ])
                .step_statuses(vec![
                    StepStatus::Completed,
                    StepStatus::Active,
                    StepStatus::NotVisited,
                ])
                .current_step(1),
        )
    }
}

#[gpui::test]
async fn test_wizard_header_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| WizardHeaderTestView);
}

// ============================================================================
// WizardNavigation Tests
// ============================================================================

/// Test WizardNavigation with callbacks
struct WizardNavigationTestView {
    back_clicked: Arc<AtomicBool>,
    next_clicked: Arc<AtomicBool>,
}

impl Render for WizardNavigationTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let back_clicked = self.back_clicked.clone();
        let next_clicked = self.next_clicked.clone();

        div().size_full().child(
            WizardNavigation::new(1, 3) // Step 2 of 3
                .back_label("Previous")
                .next_label("Continue")
                .on_back(move |_step, _window, _cx| {
                    back_clicked.store(true, Ordering::SeqCst);
                })
                .on_next(move |_step, _window, _cx| {
                    next_clicked.store(true, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_wizard_navigation_buttons(cx: &mut TestAppContext) {
    let back_clicked = Arc::new(AtomicBool::new(false));
    let next_clicked = Arc::new(AtomicBool::new(false));

    let back_clone = back_clicked.clone();
    let next_clone = next_clicked.clone();

    let window = cx.add_window(move |_window, _cx| WizardNavigationTestView {
        back_clicked: back_clone,
        next_clicked: next_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Try clicking the back button
    if let Some(bounds) = cx.debug_bounds("wizard-nav-back") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert!(
            back_clicked.load(Ordering::SeqCst),
            "on_back should have been called"
        );
    }

    // Try clicking the next button
    if let Some(bounds) = cx.debug_bounds("wizard-nav-next") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert!(
            next_clicked.load(Ordering::SeqCst),
            "on_next should have been called"
        );
    }
}

/// Test WizardNavigation on first step (shows "Close" for back)
#[gpui::test]
async fn test_wizard_navigation_first_step(cx: &mut TestAppContext) {
    struct FirstStepView;

    impl Render for FirstStepView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(WizardNavigation::new(0, 3)) // First step
        }
    }

    let _window = cx.add_window(|_window, _cx| FirstStepView);
}

/// Test WizardNavigation on last step (shows "Finish" for next)
struct LastStepView {
    finish_clicked: Arc<AtomicBool>,
}

impl Render for LastStepView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let finish_clicked = self.finish_clicked.clone();

        div().size_full().child(
            WizardNavigation::new(2, 3) // Last step (0-indexed)
                .on_finish(move |_window, _cx| {
                    finish_clicked.store(true, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_wizard_navigation_last_step_finish(cx: &mut TestAppContext) {
    let finish_clicked = Arc::new(AtomicBool::new(false));
    let finish_clone = finish_clicked.clone();

    let window = cx.add_window(move |_window, _cx| LastStepView {
        finish_clicked: finish_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Click the finish button (which shows as "wizard-nav-next" with "Finish" label)
    if let Some(bounds) = cx.debug_bounds("wizard-nav-next") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert!(
            finish_clicked.load(Ordering::SeqCst),
            "on_finish should have been called"
        );
    }
}

/// Test WizardNavigation with busy state (buttons disabled)
#[gpui::test]
async fn test_wizard_navigation_busy_state(cx: &mut TestAppContext) {
    struct BusyStateView;

    impl Render for BusyStateView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                WizardNavigation::new(1, 3)
                    .is_busy(true)
                    .progress(0.5)
                    .status_message("Processing..."),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| BusyStateView);
}

/// Test wizard step with icons
#[gpui::test]
async fn test_wizard_step_with_icons(cx: &mut TestAppContext) {
    struct IconStepView;

    impl Render for IconStepView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Wizard::new()
                    .steps(vec![
                        WizardStep::new("step1", "Upload").icon("📤"),
                        WizardStep::new("step2", "Configure").icon("⚙️"),
                        WizardStep::new("step3", "Review").icon("👁"),
                    ])
                    .current_step(0),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| IconStepView);
}

/// Test wizard step with description and can_skip
#[gpui::test]
async fn test_wizard_step_options(cx: &mut TestAppContext) {
    struct StepOptionsView;

    impl Render for StepOptionsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Wizard::new()
                    .steps(vec![
                        WizardStep::new("step1", "Required").description("This step is required"),
                        WizardStep::new("step2", "Optional")
                            .description("This step can be skipped")
                            .can_skip(true),
                        WizardStep::new("step3", "Disabled").disabled(true),
                    ])
                    .current_step(0),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| StepOptionsView);
}
