//! Wizard Debug Example
//!
//! An interactive debug and showcase for the Wizard component:
//! - Click on step indicators to jump to any step
//! - Navigate with Back/Next buttons
//! - See step content change dynamically
//! - Test error and skip states

use gpui::*;
use gpui_ui_kit::i18n::{I18nExt, TranslationKey};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::wizard::{StepStatus, WizardHeader, WizardStep, WizardTheme};
use gpui_ui_kit::*;

/// Demo state
pub struct WizardDebug {
    /// Current step for main interactive wizard
    current_step: usize,
    /// Step statuses
    step_statuses: Vec<StepStatus>,
    /// Whether wizard is "busy" (simulating async operation)
    is_busy: bool,
    /// Progress value for busy state
    progress: f32,
    /// Second wizard state (3 steps)
    wizard2_step: usize,
    wizard2_statuses: Vec<StepStatus>,
    /// Third wizard state (with icons)
    wizard3_step: usize,
    wizard3_statuses: Vec<StepStatus>,
    /// Entity reference
    entity: Entity<Self>,
}

impl WizardDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            current_step: 0,
            step_statuses: vec![
                StepStatus::Active,
                StepStatus::NotVisited,
                StepStatus::NotVisited,
                StepStatus::NotVisited,
                StepStatus::NotVisited,
            ],
            is_busy: false,
            progress: 0.0,
            wizard2_step: 0,
            wizard2_statuses: vec![
                StepStatus::Active,
                StepStatus::NotVisited,
                StepStatus::NotVisited,
            ],
            wizard3_step: 0,
            wizard3_statuses: vec![
                StepStatus::Active,
                StepStatus::NotVisited,
                StepStatus::NotVisited,
                StepStatus::NotVisited,
            ],
            entity: cx.entity().clone(),
        }
    }

    /// Go to a specific step
    fn go_to_step(&mut self, step: usize) {
        let num_steps = self.step_statuses.len();
        if step >= num_steps {
            return;
        }

        // Mark all steps before target as completed
        for i in 0..step {
            if self.step_statuses[i] != StepStatus::Skipped
                && self.step_statuses[i] != StepStatus::Error
            {
                self.step_statuses[i] = StepStatus::Completed;
            }
        }

        // Mark target as active
        self.step_statuses[step] = StepStatus::Active;

        // Mark all steps after target as not visited (unless skipped/error)
        for i in (step + 1)..num_steps {
            if self.step_statuses[i] != StepStatus::Skipped
                && self.step_statuses[i] != StepStatus::Error
            {
                self.step_statuses[i] = StepStatus::NotVisited;
            }
        }

        self.current_step = step;
    }

    /// Move to next step
    fn next_step(&mut self) {
        if self.current_step < self.step_statuses.len() - 1 {
            self.step_statuses[self.current_step] = StepStatus::Completed;
            self.current_step += 1;
            self.step_statuses[self.current_step] = StepStatus::Active;
        }
    }

    /// Move to previous step
    fn prev_step(&mut self) {
        if self.current_step > 0 {
            self.step_statuses[self.current_step] = StepStatus::NotVisited;
            self.current_step -= 1;
            self.step_statuses[self.current_step] = StepStatus::Active;
        }
    }

    /// Toggle error on current step
    fn toggle_error(&mut self) {
        if self.step_statuses[self.current_step] == StepStatus::Error {
            self.step_statuses[self.current_step] = StepStatus::Active;
        } else {
            self.step_statuses[self.current_step] = StepStatus::Error;
        }
    }

    /// Skip current step
    fn skip_step(&mut self) {
        if self.current_step < self.step_statuses.len() - 1 {
            self.step_statuses[self.current_step] = StepStatus::Skipped;
            self.current_step += 1;
            self.step_statuses[self.current_step] = StepStatus::Active;
        }
    }

    /// Reset wizard to initial state
    fn reset(&mut self) {
        self.current_step = 0;
        self.step_statuses = vec![
            StepStatus::Active,
            StepStatus::NotVisited,
            StepStatus::NotVisited,
            StepStatus::NotVisited,
            StepStatus::NotVisited,
        ];
        self.is_busy = false;
        self.progress = 0.0;
    }

    /// Go to step for wizard 2
    fn wizard2_go_to(&mut self, step: usize) {
        if step >= 3 {
            return;
        }
        for i in 0..step {
            self.wizard2_statuses[i] = StepStatus::Completed;
        }
        self.wizard2_statuses[step] = StepStatus::Active;
        for i in (step + 1)..3 {
            self.wizard2_statuses[i] = StepStatus::NotVisited;
        }
        self.wizard2_step = step;
    }

    /// Go to step for wizard 3
    fn wizard3_go_to(&mut self, step: usize) {
        if step >= 4 {
            return;
        }
        for i in 0..step {
            self.wizard3_statuses[i] = StepStatus::Completed;
        }
        self.wizard3_statuses[step] = StepStatus::Active;
        for i in (step + 1)..4 {
            self.wizard3_statuses[i] = StepStatus::NotVisited;
        }
        self.wizard3_step = step;
    }

    /// Main wizard steps
    fn main_steps() -> Vec<WizardStep> {
        vec![
            WizardStep::new("load", "Load Data"),
            WizardStep::new("configure", "Configure"),
            WizardStep::new("process", "Process"),
            WizardStep::new("review", "Review"),
            WizardStep::new("export", "Export"),
        ]
    }

    /// Quick setup steps (3 steps)
    fn quick_steps() -> Vec<WizardStep> {
        vec![
            WizardStep::new("input", "Input"),
            WizardStep::new("process", "Process"),
            WizardStep::new("output", "Output"),
        ]
    }

    /// Icon steps
    fn icon_steps() -> Vec<WizardStep> {
        vec![
            WizardStep::new("upload", "Upload").icon("📤"),
            WizardStep::new("settings", "Settings").icon("⚙️"),
            WizardStep::new("run", "Run").icon("▶️"),
            WizardStep::new("download", "Download").icon("📥"),
        ]
    }

    /// Render step content based on current step
    fn render_step_content(&self, step: usize, theme: &Theme) -> impl IntoElement {
        let (title, description, icon) = match step {
            0 => (
                "Load Your Data",
                "Select a file or drag and drop to begin. Supported formats: CSV, JSON, XLSX",
                "📂",
            ),
            1 => (
                "Configure Settings",
                "Adjust parameters and options for processing your data.",
                "⚙️",
            ),
            2 => (
                "Processing",
                "Your data is being processed. This may take a few moments.",
                "🔄",
            ),
            3 => (
                "Review Results",
                "Check the processed output and make any necessary adjustments.",
                "🔍",
            ),
            4 => ("Export", "Choose your export format and destination.", "💾"),
            _ => ("Unknown Step", "Something went wrong.", "❓"),
        };

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .p_8()
            .bg(theme.background)
            .rounded_lg()
            .min_h(px(200.0))
            .child(div().text_3xl().child(icon))
            .child(Text::new(title).weight(TextWeight::Bold).size(TextSize::Xl))
            .child(Text::new(description).muted(true).size(TextSize::Md))
            .child(
                Text::new(format!("Step {} of 5", step + 1))
                    .size(TextSize::Sm)
                    .muted(true),
            )
    }

    /// Render clickable step indicator
    fn render_clickable_step(
        &self,
        index: usize,
        step: &WizardStep,
        status: StepStatus,
        is_current: bool,
        entity: Entity<Self>,
        theme: &Theme,
    ) -> impl IntoElement {
        let (bg_color, text_color, border_color) = match status {
            StepStatus::NotVisited => (theme.surface, theme.text_muted, theme.border),
            StepStatus::Active => (theme.accent, theme.text_primary, theme.accent),
            StepStatus::Completed => (theme.success, theme.text_primary, theme.success),
            StepStatus::Error => (theme.error, theme.text_primary, theme.error),
            StepStatus::Skipped => (theme.surface, theme.text_muted, theme.border),
        };

        let step_icon = if status == StepStatus::Completed {
            "✓".to_string()
        } else if status == StepStatus::Error {
            "✗".to_string()
        } else if status == StepStatus::Skipped {
            "—".to_string()
        } else {
            format!("{}", index + 1)
        };

        let label = step.label.clone();

        div()
            .id(ElementId::Name(format!("step-{}", index).into()))
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .cursor_pointer()
            .on_click(move |_event, _window, cx| {
                entity.update(cx, |this, _cx| {
                    this.go_to_step(index);
                });
            })
            .child(
                div()
                    .w(px(36.0))
                    .h(px(36.0))
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
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .font_weight(if is_current {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if is_current {
                        theme.text_primary
                    } else {
                        theme.text_muted
                    })
                    .child(label),
            )
    }

    /// Render connector line between steps
    fn render_connector(completed: bool, theme: &Theme) -> impl IntoElement {
        div()
            .h(px(2.0))
            .w(px(40.0))
            .bg(if completed {
                theme.success
            } else {
                theme.border
            })
            .mt(px(17.0)) // Align with center of step circle
    }
}

impl Render for WizardDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = self.entity.clone();
        let theme = cx.theme();

        let current_step = self.current_step;
        let statuses = self.step_statuses.clone();
        let steps = Self::main_steps();
        let is_first = current_step == 0;
        let is_last = current_step == steps.len() - 1;

        div()
            .id("wizard-debug-root")
            .w_full()
            .h_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_6()
            .flex()
            .flex_col()
            .gap_6()
            // Header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Heading::h1("Wizard Component Debug"))
                    .child(Text::new(
                        "Click on any step to jump directly. Use buttons to navigate or modify state.",
                    ).muted(true)),
            )
            // i18n Status Bar - demonstrates language switching works
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_3()
                    .bg(theme.surface)
                    .rounded_lg()
                    .child(Text::new(format!("🌐 {}: ", cx.t(TranslationKey::MenuLanguage))).weight(TextWeight::Medium))
                    .child(Text::new(cx.language().native_name()).color(theme.accent))
                    .child(Text::new(" | "))
                    .child(Text::new(cx.t(TranslationKey::SectionProgress)).color(theme.text_secondary)),
            )
            // Main Interactive Wizard
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_xl()
                    // Clickable step indicators
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .justify_center()
                            .gap_0()
                            .children(
                                steps.iter().enumerate().flat_map(|(i, step)| {
                                    let mut elements: Vec<gpui::AnyElement> = vec![];

                                    elements.push(
                                        self.render_clickable_step(
                                            i,
                                            step,
                                            statuses[i],
                                            i == current_step,
                                            entity.clone(),
                                            &theme,
                                        )
                                        .into_any_element(),
                                    );

                                    if i < steps.len() - 1 {
                                        let completed = statuses[i] == StepStatus::Completed;
                                        elements.push(
                                            Self::render_connector(completed, &theme).into_any_element(),
                                        );
                                    }

                                    elements
                                }),
                            ),
                    )
                    // Step content
                    .child(self.render_step_content(current_step, &theme))
                    // Status bar
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_4()
                            .py_2()
                            .bg(theme.background)
                            .rounded_md()
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Lg)
                                    .child(
                                        Text::new(format!("Current: Step {}", current_step + 1))
                                            .size(TextSize::Sm),
                                    )
                                    .child(
                                        Text::new(format!(
                                            "Status: {:?}",
                                            statuses[current_step]
                                        ))
                                        .size(TextSize::Sm)
                                        .muted(true),
                                    ),
                            )
                            .child(
                                Text::new("Click steps above to jump")
                                    .size(TextSize::Xs)
                                    .muted(true),
                            ),
                    )
                    // Control buttons
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Sm)
                            // Left side - state controls
                            .child(
                                Button::new("reset", "Reset")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Sm)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.reset();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Button::new("error", "Toggle Error")
                                    .variant(ButtonVariant::Destructive)
                                    .size(ButtonSize::Sm)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.toggle_error();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Button::new("skip", "Skip Step")
                                    .variant(ButtonVariant::Secondary)
                                    .size(ButtonSize::Sm)
                                    .disabled(is_last)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.skip_step();
                                            });
                                        }
                                    }),
                            )
                            .child(div().flex_1())
                            // Right side - navigation
                            .child(
                                Button::new("back", if is_first { "Close" } else { "Back" })
                                    .variant(ButtonVariant::Secondary)
                                    .size(ButtonSize::Md)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.prev_step();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Button::new("next", if is_last { "Finish" } else { "Next" })
                                    .variant(ButtonVariant::Primary)
                                    .size(ButtonSize::Md)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.next_step();
                                            });
                                        }
                                    }),
                            ),
                    ),
            )
            .child(Divider::new().build())
            // Additional Interactive Wizards Row
            .child(
                HStack::new()
                    .spacing(StackSpacing::Lg)
                    .align(StackAlign::Start)
                    // Quick Setup Wizard (3 steps)
                    .child({
                        let wizard2_step = self.wizard2_step;
                        let wizard2_statuses = self.wizard2_statuses.clone();

                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_4()
                            .bg(theme.surface)
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .child(
                                Text::new("Quick Setup (3 Steps)")
                                    .weight(TextWeight::Bold)
                                    .size(TextSize::Md),
                            )
                            .child(
                                WizardHeader::new()
                                    .steps(Self::quick_steps())
                                    .step_statuses(wizard2_statuses.clone())
                                    .current_step(wizard2_step),
                            )
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .children((0..3).map(|i| {
                                        let is_active = i == wizard2_step;
                                        let entity = entity.clone();
                                        Button::new(
                                            ("w2-step", i),
                                            format!("Step {}", i + 1),
                                        )
                                        .variant(if is_active {
                                            ButtonVariant::Primary
                                        } else {
                                            ButtonVariant::Ghost
                                        })
                                        .size(ButtonSize::Sm)
                                        .on_click(move |_, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.wizard2_go_to(i);
                                            });
                                        })
                                    })),
                            )
                    })
                    // Icon Wizard (4 steps)
                    .child({
                        let wizard3_step = self.wizard3_step;
                        let wizard3_statuses = self.wizard3_statuses.clone();

                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_4()
                            .bg(theme.surface)
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .child(
                                Text::new("File Transfer (With Icons)")
                                    .weight(TextWeight::Bold)
                                    .size(TextSize::Md),
                            )
                            .child(
                                WizardHeader::new()
                                    .steps(Self::icon_steps())
                                    .step_statuses(wizard3_statuses.clone())
                                    .current_step(wizard3_step),
                            )
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .children((0..4).map(|i| {
                                        let is_active = i == wizard3_step;
                                        let entity = entity.clone();
                                        let icon = match i {
                                            0 => "📤",
                                            1 => "⚙️",
                                            2 => "▶️",
                                            3 => "📥",
                                            _ => "?",
                                        };
                                        Button::new(("w3-step", i), icon)
                                            .variant(if is_active {
                                                ButtonVariant::Primary
                                            } else {
                                                ButtonVariant::Ghost
                                            })
                                            .size(ButtonSize::Sm)
                                            .on_click(move |_, cx| {
                                                entity.update(cx, |this, _cx| {
                                                    this.wizard3_go_to(i);
                                                });
                                            })
                                    })),
                            )
                    }),
            )
            // Static Examples Section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Static Examples - Step Status Showcase")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Md)
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .child(Text::new("All Completed:").size(TextSize::Sm).muted(true))
                                    .child(
                                        WizardHeader::new()
                                            .steps(Self::quick_steps())
                                            .step_statuses(vec![
                                                StepStatus::Completed,
                                                StepStatus::Completed,
                                                StepStatus::Completed,
                                            ])
                                            .current_step(2),
                                    ),
                            )
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .child(Text::new("With Error:").size(TextSize::Sm).muted(true))
                                    .child(
                                        WizardHeader::new()
                                            .steps(Self::quick_steps())
                                            .step_statuses(vec![
                                                StepStatus::Completed,
                                                StepStatus::Error,
                                                StepStatus::NotVisited,
                                            ])
                                            .current_step(1),
                                    ),
                            )
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .child(Text::new("With Skipped:").size(TextSize::Sm).muted(true))
                                    .child(
                                        WizardHeader::new()
                                            .steps(Self::quick_steps())
                                            .step_statuses(vec![
                                                StepStatus::Completed,
                                                StepStatus::Skipped,
                                                StepStatus::Active,
                                            ])
                                            .current_step(2),
                                    ),
                            ),
                    ),
            )
            // Custom theme example
            .child({
                let custom_theme = WizardTheme {
                    step_bg: rgba(0x1e1b4bff),        // Dark indigo
                    step_completed_bg: rgba(0x8b5cf6ff), // Violet
                    step_active_bg: rgba(0xfbbf24ff),    // Amber
                    step_error_bg: rgba(0xef4444ff),
                    step_text: rgba(0xffffffff),
                    label_text: rgba(0xa5b4fcff),     // Light indigo
                    label_active_text: rgba(0xfef3c7ff), // Amber light
                    connector_color: rgba(0x4338caff),
                    connector_completed_color: rgba(0x8b5cf6ff),
                    step_border: rgba(0x6366f1ff),
                };

                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(rgba(0x0f0d24ff))
                    .border_1()
                    .border_color(rgba(0x4338caff))
                    .rounded_lg()
                    .child(
                        Text::new("Custom Theme - Violet/Amber")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md)
                            .color(rgba(0xe0e7ffff)),
                    )
                    .child(
                        WizardHeader::new()
                            .title("Custom Styled")
                            .steps(Self::icon_steps())
                            .step_statuses(vec![
                                StepStatus::Completed,
                                StepStatus::Completed,
                                StepStatus::Active,
                                StepStatus::NotVisited,
                            ])
                            .current_step(2)
                            .theme(custom_theme),
                    )
            })
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Wizard Debug")
            .size(1100.0, 950.0)
            .scrollable(true)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(WizardDebug::new),
    );
}
