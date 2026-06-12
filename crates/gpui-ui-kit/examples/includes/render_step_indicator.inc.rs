impl Showcase {
    fn render_step_indicator_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionStepIndicator);
        let theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Horizontal step indicator
            .child(Text::new("Horizontal (medium):").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .p_4()
                    .child(
                        StepIndicator::new(
                            "steps-h",
                            vec![
                                StepItem::new("Account").status(StepItemStatus::Completed),
                                StepItem::new("Profile").status(StepItemStatus::Active),
                                StepItem::new("Review").status(StepItemStatus::NotVisited),
                                StepItem::new("Confirm").status(StepItemStatus::NotVisited),
                            ],
                        )
                        .orientation(StepOrientation::Horizontal)
                        .size(StepIndicatorSize::Md),
                    ),
            )
            // Small size
            .child(Text::new("Small size:").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(500.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .p_4()
                    .child(
                        StepIndicator::new(
                            "steps-sm",
                            vec![
                                StepItem::new("Step 1").status(StepItemStatus::Completed),
                                StepItem::new("Step 2").status(StepItemStatus::Completed),
                                StepItem::new("Step 3").status(StepItemStatus::Active),
                            ],
                        )
                        .size(StepIndicatorSize::Sm),
                    ),
            )
            // With error
            .child(Text::new("With error state:").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(500.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .p_4()
                    .child(
                        StepIndicator::new(
                            "steps-err",
                            vec![
                                StepItem::new("Upload").status(StepItemStatus::Completed),
                                StepItem::new("Validate").status(StepItemStatus::Error),
                                StepItem::new("Process").status(StepItemStatus::NotVisited),
                            ],
                        )
                        .size(StepIndicatorSize::Md),
                    ),
            )
            // Vertical orientation
            .child(Text::new("Vertical:").weight(TextWeight::Semibold))
            .child(
                div()
                    .max_w(px(300.0))
                    .h(px(200.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .p_4()
                    .child(
                        StepIndicator::new(
                            "steps-v",
                            vec![
                                StepItem::new("Connect").status(StepItemStatus::Completed),
                                StepItem::new("Configure").status(StepItemStatus::Active),
                                StepItem::new("Deploy").status(StepItemStatus::NotVisited),
                            ],
                        )
                        .orientation(StepOrientation::Vertical)
                        .size(StepIndicatorSize::Lg),
                    ),
            )
    }
}
