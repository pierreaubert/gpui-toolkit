impl Showcase {
    fn render_accessibility_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionAccessibility);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                Text::new("Components support ARIA roles and labels for accessibility. \
                    These are stored in a runtime AccessibilityTree that can be queried by tests and future screen reader bridges.")
                    .size(TextSize::Sm)
            )
            // Buttons with aria-label
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Buttons with ARIA labels").weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(
                                Button::new("a11y-save", "Save")
                                    .aria_label("Save document")
                                    .variant(ButtonVariant::Primary),
                            )
                            .child(
                                Button::new("a11y-close", "Close")
                                    .aria_label("Close dialog")
                                    .variant(ButtonVariant::Ghost),
                            )
                            .child(
                                Button::new("a11y-delete", "Delete")
                                    .aria_label("Delete selected items")
                                    .variant(ButtonVariant::Destructive),
                            )
                            .child(
                                IconButton::new("a11y-settings", "S")
                                    .aria_label("Open settings"),
                            ),
                    ),
            )
            // Form controls with roles
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Form controls with default ARIA roles").weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Lg)
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("Checkbox (role=checkbox)").size(TextSize::Xs))
                                    .child(
                                        Checkbox::new("a11y-terms")
                                            .checked(true)
                                            .label("Accept terms")
                                            .aria_label("Accept terms and conditions"),
                                    ),
                            )
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("Toggle (role=switch)").size(TextSize::Xs))
                                    .child(
                                        Toggle::new("a11y-dark")
                                            .checked(true)
                                            .label("Dark mode"),
                                    ),
                            )
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(Text::new("Slider (role=slider)").size(TextSize::Xs))
                                    .child(
                                        Slider::new("a11y-volume")
                                            .value(75.0)
                                            .label("Volume")
                                            .aria_label("Master volume"),
                                    ),
                            ),
                    ),
            )
            // Custom role override
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Custom role overrides").weight(TextWeight::Medium))
                    .child(
                        Text::new("Components have sensible default roles. Override with .aria_role() when needed:")
                            .size(TextSize::Sm),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(
                                Button::new("a11y-link", "Visit website")
                                    .aria_role(AriaRole::Link)
                                    .variant(ButtonVariant::Ghost),
                            )
                            .child(
                                Button::new("a11y-tab", "Tab 1")
                                    .aria_role(AriaRole::Tab)
                                    .variant(ButtonVariant::Outline),
                            ),
                    ),
            )
    }
}
