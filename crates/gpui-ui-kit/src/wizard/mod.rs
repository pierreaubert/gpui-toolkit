//! Wizard component for multi-step workflows
//!
//! Provides a step-by-step wizard with:
//! - Step indicators with status (not visited, active, completed, error, skipped)
//! - Navigation buttons (Back/Next/Finish/Cancel)
//! - Form validation support per step
//! - Step dependencies (can only advance if validation passes)
//! - Async operation support with progress tracking
//! - Cancelable operations

use crate::button::{Button, ButtonSize, ButtonVariant};
use crate::progress::{Progress, ProgressSize, ProgressVariant};
use crate::theme::ThemeExt;
use crate::validation::{Validate, ValidationError};
use gpui::prelude::{IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, ElementId, FontWeight, SharedString, Window, div, px};
use std::sync::atomic::{AtomicU64, Ordering};

mod types;
mod wizard_header;
mod wizard_navigation;
mod wizard_step;
mod wizard_step_indicator_density;

pub use types::{StepStatus, WizardTheme, WizardVariant};
pub use wizard_header::WizardHeader;
pub use wizard_navigation::WizardNavigation;
pub use wizard_step::WizardStep;
use wizard_step_indicator_density::WizardStepIndicatorDensity;

static NEXT_WIZARD_ID: AtomicU64 = AtomicU64::new(0);

/// A wizard component for multi-step workflows
pub struct Wizard {
    id: ElementId,
    steps: Vec<WizardStep>,
    step_statuses: Vec<StepStatus>,
    current_step: usize,
    variant: WizardVariant,
    theme: Option<WizardTheme>,
    /// Whether an operation is in progress (disables navigation)
    is_busy: bool,
    /// Progress value (0.0 - 1.0) for async operations
    progress: Option<f32>,
    /// Status message to display
    status_message: Option<SharedString>,
    /// Whether the cancel button is shown
    show_cancel: bool,
    /// Custom label for the back button
    back_label: Option<SharedString>,
    /// Custom label for the next button
    next_label: Option<SharedString>,
    /// Custom label for the finish button
    finish_label: Option<SharedString>,
    /// Custom label for the cancel button
    cancel_label: Option<SharedString>,
    /// Callback when step changes
    on_step_change: Option<std::rc::Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    /// Callback when validation is needed before advancing
    on_validate: Option<std::rc::Rc<dyn Fn(usize) -> bool + 'static>>,
    /// Callback when finish is clicked (last step)
    on_finish: Option<std::rc::Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
    /// Callback when cancel is clicked
    on_cancel: Option<std::rc::Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
    /// Callback when back is clicked
    on_back: Option<std::rc::Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    /// Callback when next is clicked
    on_next: Option<std::rc::Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
}

impl Validate for Wizard {
    /// Validate the wizard step config.
    ///
    /// Reports `steps` when no steps are configured, `current_step` when it
    /// points past the last step, `step_statuses` when the status list length
    /// diverges from the step list, and `progress` when an async progress
    /// value falls outside `0.0..=1.0`.
    fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        if self.steps.is_empty() {
            errors.push(ValidationError::new(
                "steps",
                "wizard requires at least one step",
            ));
        }
        if !self.steps.is_empty() && self.current_step >= self.steps.len() {
            errors.push(ValidationError::new(
                "current_step",
                format!(
                    "current_step ({}) must be < steps len ({})",
                    self.current_step,
                    self.steps.len()
                ),
            ));
        }
        if self.step_statuses.len() != self.steps.len() {
            errors.push(ValidationError::new(
                "step_statuses",
                format!(
                    "step_statuses len ({}) must match steps len ({})",
                    self.step_statuses.len(),
                    self.steps.len()
                ),
            ));
        }
        if let Some(progress) = self.progress
            && !(0.0..=1.0).contains(&progress)
        {
            errors.push(ValidationError::new(
                "progress",
                format!("progress ({progress}) must be within 0.0..=1.0"),
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Wizard {
    /// Create a new wizard
    pub fn new() -> Self {
        Self {
            id: ElementId::from(format!(
                "wizard-{}",
                NEXT_WIZARD_ID.fetch_add(1, Ordering::Relaxed)
            )),
            steps: Vec::new(),
            step_statuses: Vec::new(),
            current_step: 0,
            variant: WizardVariant::default(),
            theme: None,
            is_busy: false,
            progress: None,
            status_message: None,
            show_cancel: true,
            back_label: None,
            next_label: None,
            finish_label: None,
            cancel_label: None,
            on_step_change: None,
            on_validate: None,
            on_finish: None,
            on_cancel: None,
            on_back: None,
            on_next: None,
        }
    }

    /// Set the stable element-ID namespace used by this wizard's controls.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the wizard steps
    pub fn steps(mut self, steps: Vec<WizardStep>) -> Self {
        let count = steps.len();
        self.steps = steps;
        // Initialize step statuses - first step is active, rest are not visited
        self.step_statuses = vec![StepStatus::NotVisited; count];
        if count > 0 {
            self.step_statuses[0] = StepStatus::Active;
        }
        self
    }

    /// Set the step statuses (must match steps length)
    pub fn step_statuses(mut self, statuses: Vec<StepStatus>) -> Self {
        self.step_statuses = statuses;
        self
    }

    /// Set the current step index
    pub fn current_step(mut self, step: usize) -> Self {
        self.current_step = step;
        self
    }

    /// Set the wizard variant
    pub fn variant(mut self, variant: WizardVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the theme
    pub fn theme(mut self, theme: WizardTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set busy state
    pub fn is_busy(mut self, busy: bool) -> Self {
        self.is_busy = busy;
        self
    }

    /// Set progress value (0.0 - 1.0)
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Set status message
    pub fn status_message(mut self, message: impl Into<SharedString>) -> Self {
        self.status_message = Some(message.into());
        self
    }

    /// Show or hide cancel button
    pub fn show_cancel(mut self, show: bool) -> Self {
        self.show_cancel = show;
        self
    }

    /// Set custom back button label
    pub fn back_label(mut self, label: impl Into<SharedString>) -> Self {
        self.back_label = Some(label.into());
        self
    }

    /// Set custom next button label
    pub fn next_label(mut self, label: impl Into<SharedString>) -> Self {
        self.next_label = Some(label.into());
        self
    }

    /// Set custom finish button label
    pub fn finish_label(mut self, label: impl Into<SharedString>) -> Self {
        self.finish_label = Some(label.into());
        self
    }

    /// Set custom cancel button label
    pub fn cancel_label(mut self, label: impl Into<SharedString>) -> Self {
        self.cancel_label = Some(label.into());
        self
    }

    /// Set step change handler
    pub fn on_step_change(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_step_change = Some(std::rc::Rc::new(handler));
        self
    }

    /// Set validation handler (return true if step is valid)
    pub fn on_validate(mut self, handler: impl Fn(usize) -> bool + 'static) -> Self {
        self.on_validate = Some(std::rc::Rc::new(handler));
        self
    }

    /// Set finish handler
    pub fn on_finish(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_finish = Some(std::rc::Rc::new(handler));
        self
    }

    /// Set cancel handler
    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(std::rc::Rc::new(handler));
        self
    }

    /// Set back button handler
    pub fn on_back(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_back = Some(std::rc::Rc::new(handler));
        self
    }

    /// Set next button handler
    pub fn on_next(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_next = Some(std::rc::Rc::new(handler));
        self
    }

    /// Build the step indicators
    fn build_step_indicators(
        &self,
        theme: &WizardTheme,
        density: WizardStepIndicatorDensity,
    ) -> Div {
        let mut container = div().flex().items_center().gap_2().overflow_hidden();

        for (index, step) in self.steps.iter().enumerate() {
            if density == WizardStepIndicatorDensity::CurrentIcon && index != self.current_step {
                continue;
            }

            let status = self
                .step_statuses
                .get(index)
                .copied()
                .unwrap_or(StepStatus::NotVisited);
            let is_current = index == self.current_step;

            // Determine colors based on status
            let (bg_color, text_color, border_color) = match status {
                StepStatus::NotVisited => (theme.step_bg, theme.label_text, theme.step_border),
                StepStatus::Active => (theme.step_active_bg, theme.step_text, theme.step_active_bg),
                StepStatus::Completed => (
                    theme.step_completed_bg,
                    theme.step_text,
                    theme.step_completed_bg,
                ),
                StepStatus::Error => (theme.step_error_bg, theme.step_text, theme.step_error_bg),
                StepStatus::Skipped => (theme.step_bg, theme.label_text, theme.step_border),
            };

            // Step indicator circle
            let step_number = format!("{}", index + 1);
            let step_icon = if status == StepStatus::Completed {
                "✓".to_string()
            } else if status == StepStatus::Error {
                "✗".to_string()
            } else if let Some(icon) = &step.icon {
                icon.to_string()
            } else {
                step_number
            };

            let step_circle = div()
                .w(px(28.0))
                .h(px(28.0))
                .rounded_full()
                .bg(bg_color)
                .border_2()
                .border_color(border_color)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .font_weight(if is_current {
                            FontWeight::BOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(text_color)
                        .child(step_icon),
                );

            // Label
            let label_color = if is_current {
                theme.label_active_text
            } else {
                theme.label_text
            };

            let label = div()
                .text_sm()
                .font_weight(if is_current {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .text_color(label_color)
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(step.label.clone());

            // Step item (circle + label)
            let mut step_item = div().flex().items_center().gap_2().child(step_circle);

            let show_label = match density {
                WizardStepIndicatorDensity::Full => true,
                WizardStepIndicatorDensity::CurrentLabel => is_current,
                WizardStepIndicatorDensity::CurrentIcon => false,
            };
            if show_label {
                step_item = step_item.child(label);
            }

            container = container.child(step_item);

            // Connector line between steps (except after last step)
            if density != WizardStepIndicatorDensity::CurrentIcon && index < self.steps.len() - 1 {
                let connector_color = if status == StepStatus::Completed {
                    theme.connector_completed_color
                } else {
                    theme.connector_color
                };

                let connector_width = if density == WizardStepIndicatorDensity::CurrentLabel {
                    px(24.0)
                } else {
                    px(32.0)
                };
                let connector = div()
                    .flex_shrink_0()
                    .w(connector_width)
                    .h(px(2.0))
                    .bg(connector_color);

                container = container.child(connector);
            }
        }

        container
    }

    /// Build the navigation buttons
    fn build_navigation(&self, _theme: &WizardTheme) -> Div {
        let is_first_step = self.current_step == 0;
        let is_last_step = self.current_step >= self.steps.len().saturating_sub(1);

        let back_label = self.back_label.clone().unwrap_or_else(|| {
            if is_first_step {
                "Close".into()
            } else {
                "Back".into()
            }
        });

        let next_label = if is_last_step {
            self.finish_label.clone().unwrap_or_else(|| "Finish".into())
        } else {
            self.next_label.clone().unwrap_or_else(|| "Next".into())
        };

        let cancel_label = self.cancel_label.clone().unwrap_or_else(|| "Cancel".into());

        // Create button elements
        let mut buttons = div().flex().items_center().gap_3();

        // Cancel button (if shown and we have a handler)
        if self.show_cancel {
            let mut cancel_btn = Button::new((self.id.clone(), "cancel"), cancel_label)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Md)
                .disabled(self.is_busy);

            if let Some(handler) = self.on_cancel.clone() {
                cancel_btn = cancel_btn.on_click(move |window, cx| {
                    handler(window, cx);
                });
            }

            buttons = buttons.child(cancel_btn);
        }

        // Spacer
        buttons = buttons.child(div().flex_1());

        // Back button
        let current_step = self.current_step;

        let mut back_btn = Button::new((self.id.clone(), "back"), back_label)
            .variant(ButtonVariant::Secondary)
            .size(ButtonSize::Md)
            .disabled(self.is_busy);

        let on_back = self.on_back.clone();
        let on_step_change = self.on_step_change.clone();
        if on_back.is_some() || current_step > 0 && on_step_change.is_some() {
            back_btn = back_btn.on_click(move |window, cx| {
                if let Some(on_back) = &on_back {
                    on_back(current_step, window, cx);
                }
                if current_step > 0
                    && let Some(on_step_change) = &on_step_change
                {
                    on_step_change(current_step - 1, window, cx);
                }
            });
        }

        buttons = buttons.child(back_btn);

        // Next/Finish button
        let mut next_btn = Button::new((self.id.clone(), "next"), next_label)
            .variant(ButtonVariant::Primary)
            .size(ButtonSize::Md)
            .disabled(self.is_busy);

        if is_last_step {
            if let Some(handler) = self.on_finish.clone() {
                next_btn = next_btn.on_click(move |window, cx| {
                    handler(window, cx);
                });
            }
        } else {
            let on_next = self.on_next.clone();
            let on_step_change = self.on_step_change.clone();
            let on_validate = self.on_validate.clone();
            next_btn = next_btn.on_click(move |window, cx| {
                if on_validate
                    .as_ref()
                    .is_some_and(|validate| !validate(current_step))
                {
                    return;
                }
                if let Some(on_step_change) = &on_step_change {
                    on_step_change(current_step + 1, window, cx);
                }
                if let Some(on_next) = &on_next {
                    on_next(current_step, window, cx);
                }
            });
        }

        buttons = buttons.child(next_btn);

        buttons
    }

    /// Build into element with theme
    pub fn build_with_theme(self, global_theme: &WizardTheme) -> Div {
        self.build_with_theme_and_density(global_theme, WizardStepIndicatorDensity::Full)
    }

    fn build_with_theme_and_density(
        self,
        global_theme: &WizardTheme,
        density: WizardStepIndicatorDensity,
    ) -> Div {
        let theme = self.theme.as_ref().unwrap_or(global_theme);

        let mut container = div().flex().flex_col().gap_4().w_full();

        // Step indicators
        let indicators = self.build_step_indicators(theme, density);
        container = container.child(indicators);

        // Progress bar (if progress is set)
        if let Some(progress_value) = self.progress {
            let progress_bar = Progress::new(progress_value)
                .size(ProgressSize::Sm)
                .variant(ProgressVariant::Default);

            container = container.child(progress_bar);
        }

        // Status message (if set)
        if let Some(message) = &self.status_message {
            container = container.child(
                div()
                    .text_sm()
                    .text_color(theme.label_text)
                    .child(message.clone()),
            );
        }

        // Navigation buttons
        let navigation = self.build_navigation(theme);
        container = container.child(navigation);

        container
    }
}

impl Default for Wizard {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Wizard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let wizard_theme = WizardTheme::from(global_theme);
        self.build_with_theme_and_density(
            &wizard_theme,
            WizardStepIndicatorDensity::from_window(window),
        )
    }
}

impl IntoElement for Wizard {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Validate, Wizard, WizardStep};

    fn two_step_wizard() -> Wizard {
        Wizard::new().steps(vec![
            WizardStep::new("one", "One"),
            WizardStep::new("two", "Two"),
        ])
    }

    #[test]
    fn fully_configured_wizard_passes_schema_validation() {
        let wizard = two_step_wizard().current_step(1).progress(0.5);

        assert!(wizard.validate().is_ok());
        assert!(wizard.validate_first().is_ok());
        assert!(wizard.is_valid());
    }

    #[test]
    fn empty_wizard_reports_missing_steps() {
        let wizard = Wizard::new();

        let errors = wizard.validate().expect_err("wizard has no steps");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field.as_ref(), "steps");
        assert!(!wizard.is_valid());
    }

    #[test]
    fn out_of_range_step_and_progress_collect_every_failure() {
        let wizard = two_step_wizard().current_step(7).progress(1.5);

        let errors = wizard.validate().expect_err("wizard config is invalid");
        assert_eq!(errors.len(), 2);
        assert!(
            errors
                .iter()
                .any(|error| error.field.as_ref() == "current_step")
        );
        assert!(
            errors
                .iter()
                .any(|error| error.field.as_ref() == "progress")
        );

        let first = wizard
            .validate_first()
            .expect_err("first failure is an error");
        assert_eq!(first.field.as_ref(), "current_step");
    }

    #[test]
    fn mismatched_step_statuses_fail_schema_validation() {
        let wizard = two_step_wizard().step_statuses(vec![crate::wizard::StepStatus::Active]);

        let errors = wizard.validate().expect_err("statuses diverge from steps");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field.as_ref(), "step_statuses");
    }
}
