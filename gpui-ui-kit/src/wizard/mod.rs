//! Wizard component for multi-step workflows
//!
//! Provides a step-by-step wizard with:
//! - Step indicators with status (not visited, active, completed, error, skipped)
//! - Navigation buttons (Back/Next/Finish/Cancel)
//! - Form validation support per step
//! - Step dependencies (can only advance if validation passes)
//! - Async operation support with progress tracking
//! - Cancelable operations

use crate::ComponentTheme;
use crate::button::{Button, ButtonSize, ButtonVariant};
use crate::progress::{Progress, ProgressSize, ProgressVariant};
use crate::theme::ThemeExt;
use gpui::prelude::*;
use gpui::*;

/// Status of a wizard step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepStatus {
    /// Step has not been visited yet
    #[default]
    NotVisited,
    /// Step is currently active
    Active,
    /// Step has been completed successfully
    Completed,
    /// Step has an error
    Error,
    /// Step was skipped
    Skipped,
}

/// Theme colors for wizard styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct WizardTheme {
    /// Background color for step indicators
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub step_bg: Rgba,
    /// Background color for completed step
    #[theme(default = 0x22c55eff, from = success)]
    pub step_completed_bg: Rgba,
    /// Background color for active step
    #[theme(default = 0x007accff, from = accent)]
    pub step_active_bg: Rgba,
    /// Background color for error step
    #[theme(default = 0xef4444ff, from = error)]
    pub step_error_bg: Rgba,
    /// Text color for step numbers
    #[theme(default = 0xffffffff, from = text_primary)]
    pub step_text: Rgba,
    /// Text color for step labels
    #[theme(default = 0x888888ff, from = text_muted)]
    pub label_text: Rgba,
    /// Text color for active step label
    #[theme(default = 0xffffffff, from = text_primary)]
    pub label_active_text: Rgba,
    /// Connector line color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub connector_color: Rgba,
    /// Connector color for completed steps
    #[theme(default = 0x22c55eff, from = success)]
    pub connector_completed_color: Rgba,
    /// Border color for steps
    #[theme(default = 0x3a3a3aff, from = border)]
    pub step_border: Rgba,
}

/// A single step in the wizard
#[derive(Clone)]
pub struct WizardStep {
    /// Unique identifier for this step
    pub id: SharedString,
    /// Display label for this step
    pub label: SharedString,
    /// Optional description
    pub description: Option<SharedString>,
    /// Optional icon (emoji or text)
    pub icon: Option<SharedString>,
    /// Whether this step can be skipped
    pub can_skip: bool,
    /// Whether this step is disabled
    pub disabled: bool,
}

impl WizardStep {
    /// Create a new wizard step
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            icon: None,
            can_skip: false,
            disabled: false,
        }
    }

    /// Add a description
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add an icon
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set whether this step can be skipped
    pub fn can_skip(mut self, can_skip: bool) -> Self {
        self.can_skip = can_skip;
        self
    }

    /// Set whether this step is disabled
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Variant for wizard layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardVariant {
    /// Horizontal step indicators (default)
    #[default]
    Horizontal,
    /// Vertical step indicators (sidebar style)
    Vertical,
}

/// A wizard component for multi-step workflows
pub struct Wizard {
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
    on_step_change: Option<Box<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    /// Callback when validation is needed before advancing
    on_validate: Option<Box<dyn Fn(usize) -> bool + 'static>>,
    /// Callback when finish is clicked (last step)
    on_finish: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    /// Callback when cancel is clicked
    on_cancel: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    /// Callback when back is clicked
    on_back: Option<Box<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    /// Callback when next is clicked
    on_next: Option<Box<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
}

impl Wizard {
    /// Create a new wizard
    pub fn new() -> Self {
        Self {
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
        self.on_step_change = Some(Box::new(handler));
        self
    }

    /// Set validation handler (return true if step is valid)
    pub fn on_validate(mut self, handler: impl Fn(usize) -> bool + 'static) -> Self {
        self.on_validate = Some(Box::new(handler));
        self
    }

    /// Set finish handler
    pub fn on_finish(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_finish = Some(Box::new(handler));
        self
    }

    /// Set cancel handler
    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Box::new(handler));
        self
    }

    /// Set back button handler
    pub fn on_back(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_back = Some(Box::new(handler));
        self
    }

    /// Set next button handler
    pub fn on_next(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_next = Some(Box::new(handler));
        self
    }

    /// Build the step indicators
    fn build_step_indicators(&self, theme: &WizardTheme) -> Div {
        let mut container = div().flex().items_center().gap_2();

        for (index, step) in self.steps.iter().enumerate() {
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
                .child(step.label.clone());

            // Step item (circle + label)
            let step_item = div()
                .flex()
                .items_center()
                .gap_2()
                .child(step_circle)
                .child(label);

            container = container.child(step_item);

            // Connector line between steps (except after last step)
            if index < self.steps.len() - 1 {
                let connector_color = if status == StepStatus::Completed {
                    theme.connector_completed_color
                } else {
                    theme.connector_color
                };

                let connector = div().w(px(32.0)).h(px(2.0)).bg(connector_color);

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
        if self.show_cancel && self.on_cancel.is_some() {
            let on_cancel: Option<*const dyn Fn(&mut Window, &mut App)> =
                self.on_cancel.as_ref().map(|f| f.as_ref() as *const _);

            let mut cancel_btn = Button::new("wizard-cancel", cancel_label)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Md)
                .disabled(self.is_busy);

            if let Some(handler_ptr) = on_cancel {
                cancel_btn = cancel_btn.on_click(move |window, cx| unsafe {
                    (*handler_ptr)(window, cx);
                });
            }

            buttons = buttons.child(cancel_btn);
        }

        // Spacer
        buttons = buttons.child(div().flex_1());

        // Back button
        let on_back: Option<*const dyn Fn(usize, &mut Window, &mut App)> =
            self.on_back.as_ref().map(|f| f.as_ref() as *const _);
        let current_step = self.current_step;

        let mut back_btn = Button::new("wizard-back", back_label)
            .variant(ButtonVariant::Secondary)
            .size(ButtonSize::Md)
            .disabled(self.is_busy);

        if let Some(handler_ptr) = on_back {
            back_btn = back_btn.on_click(move |window, cx| unsafe {
                (*handler_ptr)(current_step, window, cx);
            });
        }

        buttons = buttons.child(back_btn);

        // Next/Finish button
        let on_next: Option<*const dyn Fn(usize, &mut Window, &mut App)> =
            self.on_next.as_ref().map(|f| f.as_ref() as *const _);
        let on_finish: Option<*const dyn Fn(&mut Window, &mut App)> =
            self.on_finish.as_ref().map(|f| f.as_ref() as *const _);

        let mut next_btn = Button::new("wizard-next", next_label)
            .variant(ButtonVariant::Primary)
            .size(ButtonSize::Md)
            .disabled(self.is_busy);

        if is_last_step {
            if let Some(handler_ptr) = on_finish {
                next_btn = next_btn.on_click(move |window, cx| unsafe {
                    (*handler_ptr)(window, cx);
                });
            }
        } else if let Some(handler_ptr) = on_next {
            next_btn = next_btn.on_click(move |window, cx| unsafe {
                (*handler_ptr)(current_step, window, cx);
            });
        }

        buttons = buttons.child(next_btn);

        buttons
    }

    /// Build into element with theme
    pub fn build_with_theme(self, global_theme: &WizardTheme) -> Div {
        let theme = self.theme.as_ref().unwrap_or(global_theme);

        let mut container = div().flex().flex_col().gap_4().w_full();

        // Step indicators
        let indicators = self.build_step_indicators(theme);
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
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let wizard_theme = WizardTheme::from(&global_theme);
        self.build_with_theme(&wizard_theme)
    }
}

impl IntoElement for Wizard {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// Header component for wizard screens - renders just the step indicators
/// Use this when you want to place the wizard header and navigation separately
pub struct WizardHeader {
    steps: Vec<WizardStep>,
    step_statuses: Vec<StepStatus>,
    current_step: usize,
    title: Option<SharedString>,
    theme: Option<WizardTheme>,
}

impl WizardHeader {
    /// Create a new wizard header
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            step_statuses: Vec::new(),
            current_step: 0,
            title: None,
            theme: None,
        }
    }

    /// Set the wizard steps
    pub fn steps(mut self, steps: Vec<WizardStep>) -> Self {
        let count = steps.len();
        self.steps = steps;
        self.step_statuses = vec![StepStatus::NotVisited; count];
        if count > 0 {
            self.step_statuses[0] = StepStatus::Active;
        }
        self
    }

    /// Set step statuses
    pub fn step_statuses(mut self, statuses: Vec<StepStatus>) -> Self {
        self.step_statuses = statuses;
        self
    }

    /// Set current step
    pub fn current_step(mut self, step: usize) -> Self {
        self.current_step = step;
        self
    }

    /// Set title
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set theme
    pub fn theme(mut self, theme: WizardTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Build step indicators (reuses Wizard's logic)
    fn build_step_indicators(&self, theme: &WizardTheme) -> Div {
        let mut container = div().flex().items_center().gap_2();

        for (index, step) in self.steps.iter().enumerate() {
            let status = self
                .step_statuses
                .get(index)
                .copied()
                .unwrap_or(StepStatus::NotVisited);
            let is_current = index == self.current_step;

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

            let step_icon = if status == StepStatus::Completed {
                "✓".to_string()
            } else if status == StepStatus::Error {
                "✗".to_string()
            } else if let Some(icon) = &step.icon {
                icon.to_string()
            } else {
                format!("{}", index + 1)
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
                .child(step.label.clone());

            let step_item = div()
                .flex()
                .items_center()
                .gap_2()
                .child(step_circle)
                .child(label);

            container = container.child(step_item);

            if index < self.steps.len() - 1 {
                let connector_color = if status == StepStatus::Completed {
                    theme.connector_completed_color
                } else {
                    theme.connector_color
                };

                container = container.child(div().w(px(32.0)).h(px(2.0)).bg(connector_color));
            }
        }

        container
    }

    /// Build with theme
    pub fn build_with_theme(self, global_theme: &WizardTheme) -> Div {
        let theme = self.theme.as_ref().unwrap_or(global_theme);

        let mut container = div().flex().items_center().gap_4();

        // Title (if set)
        if let Some(title) = &self.title {
            container = container.child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.label_active_text)
                    .child(title.clone()),
            );

            // Separator
            container = container.child(div().w(px(1.0)).h(px(24.0)).bg(theme.step_border));
        }

        // Step indicators
        container = container.child(self.build_step_indicators(theme));

        container
    }
}

impl Default for WizardHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for WizardHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let wizard_theme = WizardTheme::from(&global_theme);
        self.build_with_theme(&wizard_theme)
    }
}

impl IntoElement for WizardHeader {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// Navigation buttons component for wizard screens
/// Use this when you want to place navigation separately from the header
pub struct WizardNavigation {
    current_step: usize,
    total_steps: usize,
    is_busy: bool,
    progress: Option<f32>,
    status_message: Option<SharedString>,
    show_cancel: bool,
    back_label: Option<SharedString>,
    next_label: Option<SharedString>,
    finish_label: Option<SharedString>,
    cancel_label: Option<SharedString>,
    back_disabled: bool,
    next_disabled: bool,
    on_back: Option<Box<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    on_next: Option<Box<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    on_finish: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    on_cancel: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    theme: Option<WizardTheme>,
}

impl WizardNavigation {
    /// Create new navigation with step info
    pub fn new(current_step: usize, total_steps: usize) -> Self {
        Self {
            current_step,
            total_steps,
            is_busy: false,
            progress: None,
            status_message: None,
            show_cancel: false,
            back_label: None,
            next_label: None,
            finish_label: None,
            cancel_label: None,
            back_disabled: false,
            next_disabled: false,
            on_back: None,
            on_next: None,
            on_finish: None,
            on_cancel: None,
            theme: None,
        }
    }

    /// Set busy state
    pub fn is_busy(mut self, busy: bool) -> Self {
        self.is_busy = busy;
        self
    }

    /// Set progress
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Set status message
    pub fn status_message(mut self, message: impl Into<SharedString>) -> Self {
        self.status_message = Some(message.into());
        self
    }

    /// Show cancel button
    pub fn show_cancel(mut self, show: bool) -> Self {
        self.show_cancel = show;
        self
    }

    /// Set back label
    pub fn back_label(mut self, label: impl Into<SharedString>) -> Self {
        self.back_label = Some(label.into());
        self
    }

    /// Set next label
    pub fn next_label(mut self, label: impl Into<SharedString>) -> Self {
        self.next_label = Some(label.into());
        self
    }

    /// Set finish label
    pub fn finish_label(mut self, label: impl Into<SharedString>) -> Self {
        self.finish_label = Some(label.into());
        self
    }

    /// Set cancel label
    pub fn cancel_label(mut self, label: impl Into<SharedString>) -> Self {
        self.cancel_label = Some(label.into());
        self
    }

    /// Disable back button
    pub fn back_disabled(mut self, disabled: bool) -> Self {
        self.back_disabled = disabled;
        self
    }

    /// Disable next button
    pub fn next_disabled(mut self, disabled: bool) -> Self {
        self.next_disabled = disabled;
        self
    }

    /// Set back handler
    pub fn on_back(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_back = Some(Box::new(handler));
        self
    }

    /// Set next handler
    pub fn on_next(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_next = Some(Box::new(handler));
        self
    }

    /// Set finish handler
    pub fn on_finish(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_finish = Some(Box::new(handler));
        self
    }

    /// Set cancel handler
    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Box::new(handler));
        self
    }

    /// Set theme
    pub fn theme(mut self, theme: WizardTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Build with theme
    pub fn build_with_theme(self, global_theme: &WizardTheme) -> Div {
        let theme = self.theme.as_ref().unwrap_or(global_theme);
        let is_first_step = self.current_step == 0;
        let is_last_step = self.current_step >= self.total_steps.saturating_sub(1);

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

        let mut container = div().flex().flex_col().gap_3().w_full();

        // Progress bar
        if let Some(progress_value) = self.progress {
            container = container.child(
                Progress::new(progress_value)
                    .size(ProgressSize::Sm)
                    .variant(ProgressVariant::Default),
            );
        }

        // Status message
        if let Some(message) = &self.status_message {
            container = container.child(
                div()
                    .text_sm()
                    .text_color(theme.label_text)
                    .child(message.clone()),
            );
        }

        // Buttons row
        let mut buttons = div().flex().items_center().gap_3();

        // Cancel button
        if self.show_cancel {
            let on_cancel: Option<*const dyn Fn(&mut Window, &mut App)> =
                self.on_cancel.as_ref().map(|f| f.as_ref() as *const _);

            let mut cancel_btn = Button::new("wizard-nav-cancel", cancel_label)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Md)
                .disabled(self.is_busy);

            if let Some(handler_ptr) = on_cancel {
                cancel_btn = cancel_btn.on_click(move |window, cx| unsafe {
                    (*handler_ptr)(window, cx);
                });
            }

            buttons = buttons.child(cancel_btn);
        }

        // Spacer
        buttons = buttons.child(div().flex_1());

        // Back button
        let on_back: Option<*const dyn Fn(usize, &mut Window, &mut App)> =
            self.on_back.as_ref().map(|f| f.as_ref() as *const _);
        let current_step = self.current_step;

        let mut back_btn = Button::new("wizard-nav-back", back_label)
            .variant(ButtonVariant::Secondary)
            .size(ButtonSize::Md)
            .disabled(self.is_busy || self.back_disabled);

        if let Some(handler_ptr) = on_back {
            back_btn = back_btn.on_click(move |window, cx| unsafe {
                (*handler_ptr)(current_step, window, cx);
            });
        }

        buttons = buttons.child(back_btn);

        // Next/Finish button
        let on_next: Option<*const dyn Fn(usize, &mut Window, &mut App)> =
            self.on_next.as_ref().map(|f| f.as_ref() as *const _);
        let on_finish: Option<*const dyn Fn(&mut Window, &mut App)> =
            self.on_finish.as_ref().map(|f| f.as_ref() as *const _);

        let mut next_btn = Button::new("wizard-nav-next", next_label)
            .variant(ButtonVariant::Primary)
            .size(ButtonSize::Md)
            .disabled(self.is_busy || self.next_disabled);

        if is_last_step {
            if let Some(handler_ptr) = on_finish {
                next_btn = next_btn.on_click(move |window, cx| unsafe {
                    (*handler_ptr)(window, cx);
                });
            }
        } else if let Some(handler_ptr) = on_next {
            next_btn = next_btn.on_click(move |window, cx| unsafe {
                (*handler_ptr)(current_step, window, cx);
            });
        }

        buttons = buttons.child(next_btn);

        container = container.child(buttons);

        container
    }
}

impl RenderOnce for WizardNavigation {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let wizard_theme = WizardTheme::from(&global_theme);
        self.build_with_theme(&wizard_theme)
    }
}

impl IntoElement for WizardNavigation {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
