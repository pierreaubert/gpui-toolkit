

impl Showcase {
    fn render_wizard_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();
        let current_step = self.wizard_step;
        let statuses = self.wizard_statuses.clone();
        
        let steps = vec![
            WizardStep::new("load", "Load Data").icon("📂"),
            WizardStep::new("configure", "Configure").icon("⚙️"),
            WizardStep::new("process", "Process").icon("🔄"),
            WizardStep::new("review", "Review").icon("🔍"),
            WizardStep::new("export", "Export").icon("💾"),
        ];
        
        let is_first = current_step == 0;
        let is_last = current_step == steps.len() - 1;

        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(self.section_header("Wizard"))
            .child(
                Text::new("A multi-step wizard component with navigation, status tracking, and flexible styling.")
                    .color(theme.text_secondary)
            )
            
            // Interactive Demo
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
                    
                    // Header with steps
                    .child(
                        WizardHeader::new()
                            .steps(steps.clone())
                            .step_statuses(statuses.clone())
                            .current_step(current_step)
                    )
                    
                    // Content Area placeholder
                    .child(
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
                            .child(div().text_3xl().child(steps[current_step].icon.clone().unwrap_or_default()))
                            .child(Text::new(steps[current_step].label.clone()).weight(TextWeight::Bold).size(TextSize::Xl))
                            .child(Text::new(format!("Step {} Content", current_step + 1)).muted(true))
                    )
                    
                    // Controls
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(
                                Button::new("wizard-reset", "Reset")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Sm)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.wizard_reset();
                                            });
                                        }
                                    }),
                            )
                            .child(div().flex_1())
                            .child(
                                Button::new("wizard-back", if is_first { "Close" } else { "Back" })
                                    .variant(ButtonVariant::Secondary)
                                    .size(ButtonSize::Md)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.wizard_prev();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Button::new("wizard-next", if is_last { "Finish" } else { "Next" })
                                    .variant(ButtonVariant::Primary)
                                    .size(ButtonSize::Md)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.wizard_next();
                                            });
                                        }
                                    }),
                            ),
                    )
            )
            
            // Example Variants
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(Heading::h3("Variants"))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .p_4()
                            .bg(theme.surface)
                            .rounded_lg()
                            .child(Text::new("Simple Steps (No Icons)").weight(TextWeight::Semibold))
                            .child(
                                WizardHeader::new()
                                    .steps(vec![
                                        WizardStep::new("1", "Input"),
                                        WizardStep::new("2", "Process"),
                                        WizardStep::new("3", "Output"),
                                    ])
                                    .step_statuses(vec![
                                        StepStatus::Completed,
                                        StepStatus::Active,
                                        StepStatus::NotVisited,
                                    ])
                                    .current_step(1)
                            )
                    )
            )
    }

    fn wizard_reset(&mut self) {
        self.wizard_step = 0;
        self.wizard_statuses = vec![
            StepStatus::Active,
            StepStatus::NotVisited,
            StepStatus::NotVisited,
            StepStatus::NotVisited,
            StepStatus::NotVisited,
        ];
    }

    fn wizard_next(&mut self) {
        if self.wizard_step < self.wizard_statuses.len() - 1 {
            self.wizard_statuses[self.wizard_step] = StepStatus::Completed;
            self.wizard_step += 1;
            self.wizard_statuses[self.wizard_step] = StepStatus::Active;
        }
    }

    fn wizard_prev(&mut self) {
        if self.wizard_step > 0 {
            self.wizard_statuses[self.wizard_step] = StepStatus::NotVisited;
            self.wizard_step -= 1;
            self.wizard_statuses[self.wizard_step] = StepStatus::Active;
        }
    }
}
