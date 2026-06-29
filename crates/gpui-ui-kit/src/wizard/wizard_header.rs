use super::types::StepStatus;
use super::types::WizardTheme;
use super::wizard_step::WizardStep;
use super::wizard_step_indicator_density::WizardStepIndicatorDensity;
use crate::theme::ThemeExt;
use gpui::prelude::{IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, FontWeight, SharedString, Window, div, px};

/// Header component for wizard screens - renders just the step indicators
/// Use this when you want to place the wizard header and navigation separately
pub struct WizardHeader {
    pub(super) steps: Vec<WizardStep>,
    pub(super) step_statuses: Vec<StepStatus>,
    pub(super) current_step: usize,
    pub(super) title: Option<SharedString>,
    pub(super) theme: Option<WizardTheme>,
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
    pub(super) fn build_step_indicators(
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
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(step.label.clone());

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
                container = container.child(
                    div()
                        .flex_shrink_0()
                        .w(connector_width)
                        .h(px(2.0))
                        .bg(connector_color),
                );
            }
        }

        container
    }

    /// Build with theme
    pub fn build_with_theme(self, global_theme: &WizardTheme) -> Div {
        self.build_with_theme_and_density(global_theme, WizardStepIndicatorDensity::Full)
    }

    pub(super) fn build_with_theme_and_density(
        self,
        global_theme: &WizardTheme,
        density: WizardStepIndicatorDensity,
    ) -> Div {
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
        container = container.child(self.build_step_indicators(theme, density));

        container
    }
}

impl Default for WizardHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for WizardHeader {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let wizard_theme = WizardTheme::from(global_theme);
        self.build_with_theme_and_density(
            &wizard_theme,
            WizardStepIndicatorDensity::from_window(window),
        )
    }
}

impl IntoElement for WizardHeader {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
