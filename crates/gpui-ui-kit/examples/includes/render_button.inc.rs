impl Showcase {
    fn render_buttons_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let variants_label = cx.t(TranslationKey::LabelVariants);
        let sizes_label = cx.t(TranslationKey::LabelSizes);
        let states_label = cx.t(TranslationKey::LabelStates);
        let section_title = cx.t(TranslationKey::SectionButtons);

        let primary = cx.t(TranslationKey::ButtonPrimary);
        let secondary = cx.t(TranslationKey::ButtonSecondary);
        let destructive = cx.t(TranslationKey::ButtonDestructive);
        let ghost = cx.t(TranslationKey::ButtonGhost);
        let outline = cx.t(TranslationKey::ButtonOutline);

        let disabled = cx.t(TranslationKey::LabelDisabled);
        let selected = cx.t(TranslationKey::LabelSelected);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Button Variants
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new(variants_label).weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(
                                Button::new("btn-primary", primary).variant(ButtonVariant::Primary),
                            )
                            .child(
                                Button::new("btn-secondary", secondary)
                                    .variant(ButtonVariant::Secondary),
                            )
                            .child(
                                Button::new("btn-destructive", destructive)
                                    .variant(ButtonVariant::Destructive),
                            )
                            .child(Button::new("btn-ghost", ghost).variant(ButtonVariant::Ghost))
                            .child(
                                Button::new("btn-outline", outline).variant(ButtonVariant::Outline),
                            ),
                    ),
            )
            // Button Sizes
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new(sizes_label).weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .align(StackAlign::End)
                            .child(Button::new("btn-xs", "Extra Small").size(ButtonSize::Xs))
                            .child(Button::new("btn-sm", "Small").size(ButtonSize::Sm))
                            .child(Button::new("btn-md", "Medium").size(ButtonSize::Md))
                            .child(Button::new("btn-lg", "Large").size(ButtonSize::Lg)),
                    ),
            )
            // Button States
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new(states_label).weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(Button::new("btn-disabled", disabled).disabled(true))
                            .child(Button::new("btn-selected", selected).selected(true))
                            .child(Button::new("btn-icon", "With Icon").icon_left("*")),
                    ),
            )
    }
}
