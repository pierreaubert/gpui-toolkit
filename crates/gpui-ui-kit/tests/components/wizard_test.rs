//! Wizard component tests

use gpui_ui_kit::wizard::{
    StepStatus, Wizard, WizardHeader, WizardNavigation, WizardStep, WizardVariant,
};

#[test]
fn test_wizard_step_creation() {
    let step = WizardStep::new("step-1", "First Step");
    drop(step);
}

#[test]
fn test_wizard_step_configuration() {
    let step = WizardStep::new("step-1", "Setup")
        .description("Configure your settings")
        .icon("gear")
        .can_skip(true)
        .disabled(false);
    drop(step);
}

#[test]
fn test_wizard_step_disabled() {
    let step = WizardStep::new("step-2", "Disabled Step").disabled(true);
    drop(step);
}

#[test]
fn test_step_status_variants() {
    let statuses = [
        StepStatus::NotVisited,
        StepStatus::Active,
        StepStatus::Completed,
        StepStatus::Error,
        StepStatus::Skipped,
    ];
    for status in &statuses {
        let _copy = *status;
    }
}

#[test]
fn test_step_status_default() {
    let status = StepStatus::default();
    assert_eq!(status, StepStatus::NotVisited);
}

#[test]
fn test_wizard_variant_variants() {
    let variants = [WizardVariant::Horizontal, WizardVariant::Vertical];
    for variant in &variants {
        let _copy = *variant;
    }
}

#[test]
fn test_wizard_variant_default() {
    let variant = WizardVariant::default();
    assert_eq!(variant, WizardVariant::Horizontal);
}

#[test]
fn test_wizard_creation() {
    let wizard = Wizard::new();
    drop(wizard);
}

#[test]
fn test_wizard_configuration() {
    let steps = vec![
        WizardStep::new("s1", "Step 1"),
        WizardStep::new("s2", "Step 2"),
        WizardStep::new("s3", "Step 3"),
    ];
    let statuses = vec![
        StepStatus::Completed,
        StepStatus::Active,
        StepStatus::NotVisited,
    ];

    let wizard = Wizard::new()
        .steps(steps)
        .step_statuses(statuses)
        .current_step(1)
        .variant(WizardVariant::Vertical)
        .is_busy(false)
        .progress(0.5)
        .status_message("In progress...")
        .show_cancel(true)
        .back_label("Previous")
        .next_label("Continue")
        .finish_label("Done")
        .cancel_label("Abort");

    drop(wizard);
}

#[test]
fn test_wizard_with_handlers() {
    let wizard = Wizard::new()
        .steps(vec![WizardStep::new("s1", "Step 1")])
        .on_step_change(|_step, _window, _cx| {})
        .on_validate(|_step| true)
        .on_finish(|_window, _cx| {})
        .on_cancel(|_window, _cx| {})
        .on_back(|_step, _window, _cx| {})
        .on_next(|_step, _window, _cx| {});

    drop(wizard);
}

#[test]
fn test_wizard_header_creation() {
    let header = WizardHeader::new()
        .steps(vec![
            WizardStep::new("s1", "Step 1"),
            WizardStep::new("s2", "Step 2"),
        ])
        .step_statuses(vec![StepStatus::Completed, StepStatus::Active])
        .current_step(1)
        .title("Setup Wizard");

    drop(header);
}

#[test]
fn test_wizard_navigation_creation() {
    let nav = WizardNavigation::new(0, 3);
    drop(nav);
}

#[test]
fn test_wizard_navigation_configuration() {
    let nav = WizardNavigation::new(1, 3)
        .is_busy(false)
        .progress(0.33)
        .status_message("Step 2 of 3")
        .show_cancel(true)
        .back_label("Back")
        .next_label("Next")
        .finish_label("Finish")
        .cancel_label("Cancel")
        .back_disabled(false)
        .next_disabled(false);

    drop(nav);
}

#[test]
fn test_wizard_navigation_handlers() {
    let nav = WizardNavigation::new(0, 2)
        .on_back(|_step, _window, _cx| {})
        .on_next(|_step, _window, _cx| {})
        .on_finish(|_window, _cx| {})
        .on_cancel(|_window, _cx| {});

    drop(nav);
}
