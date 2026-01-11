impl Showcase {
    fn render_icon_buttons_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionIconButtons);
        let variants_label = cx.t(TranslationKey::LabelVariants);
        let sizes_label = cx.t(TranslationKey::LabelSizes);
        let states_label = cx.t(TranslationKey::LabelStates);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Variants
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new(variants_label).weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .align(StackAlign::Center)
                            .child(
                                IconButton::new("ib-ghost", "?").variant(IconButtonVariant::Ghost),
                            )
                            .child(
                                IconButton::new("ib-filled", "S")
                                    .variant(IconButtonVariant::Filled),
                            )
                            .child(
                                IconButton::new("ib-outline", "E")
                                    .variant(IconButtonVariant::Outline),
                            ),
                    ),
            )
            // Sizes
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new(sizes_label).weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .align(StackAlign::End)
                            .child(IconButton::new("ib-xs", "*").size(IconButtonSize::Xs))
                            .child(IconButton::new("ib-sm", "*").size(IconButtonSize::Sm))
                            .child(IconButton::new("ib-md", "*").size(IconButtonSize::Md))
                            .child(IconButton::new("ib-lg", "*").size(IconButtonSize::Lg))
                            .child(IconButton::new("ib-xl", "*").size(IconButtonSize::Xl)),
                    ),
            )
            // States
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new(states_label).weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(
                                IconButton::new("ib-selected", "<3")
                                    .selected(true)
                                    .variant(IconButtonVariant::Filled),
                            )
                            .child(IconButton::new("ib-disabled", "X").disabled(true))
                            .child(
                                IconButton::new("ib-round", "!")
                                    .rounded_full()
                                    .variant(IconButtonVariant::Filled),
                            ),
                    ),
            )
    }
}
