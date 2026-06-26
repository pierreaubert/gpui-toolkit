use super::prelude::*;

impl Showcase {
    pub(crate) fn render_confirm_dialog_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionConfirmDialog);
        let theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                Text::new("Confirm dialog variant previews (rendered inline without backdrop):")
                    .muted(true),
            )
            // Default variant
            .child(Text::new("Default variant:").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(400.0))
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .shadow_md()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px_4()
                            .pt_4()
                            .pb_2()
                            .child(
                                Text::new("Save Changes")
                                    .weight(TextWeight::Semibold)
                                    .size(TextSize::Lg),
                            ),
                    )
                    .child(
                        div()
                            .px_4()
                            .pb_3()
                            .child(Text::new("Do you want to save your changes before closing?")),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .px_4()
                            .py_3()
                            .border_t_1()
                            .border_color(theme.border)
                            .child(
                                Button::new("cd-default-cancel", "Cancel")
                                    .variant(ButtonVariant::Ghost),
                            )
                            .child(
                                Button::new("cd-default-confirm", "Confirm")
                                    .variant(ButtonVariant::Primary),
                            ),
                    ),
            )
            // Destructive variant
            .child(Text::new("Destructive variant:").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(400.0))
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.error)
                    .rounded_lg()
                    .shadow_md()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px_4()
                            .pt_4()
                            .pb_2()
                            .child(
                                Text::new("Delete Album")
                                    .weight(TextWeight::Semibold)
                                    .size(TextSize::Lg),
                            ),
                    )
                    .child(
                        div().px_4().pb_3().child(Text::new(
                            "Are you sure you want to delete this album? This action cannot be undone.",
                        )),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .px_4()
                            .py_3()
                            .border_t_1()
                            .border_color(theme.border)
                            .child(
                                Button::new("cd-dest-cancel", "Cancel")
                                    .variant(ButtonVariant::Ghost),
                            )
                            .child(
                                Button::new("cd-dest-confirm", "Delete")
                                    .variant(ButtonVariant::Destructive),
                            ),
                    ),
            )
            // Warning variant
            .child(Text::new("Warning variant:").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(400.0))
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.warning)
                    .rounded_lg()
                    .shadow_md()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px_4()
                            .pt_4()
                            .pb_2()
                            .child(
                                Text::new("Reset Settings")
                                    .weight(TextWeight::Semibold)
                                    .size(TextSize::Lg),
                            ),
                    )
                    .child(
                        div().px_4().pb_3().child(Text::new(
                            "This will reset all settings to their defaults. You may lose custom configurations.",
                        )),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .px_4()
                            .py_3()
                            .border_t_1()
                            .border_color(theme.border)
                            .child(
                                Button::new("cd-warn-cancel", "Cancel")
                                    .variant(ButtonVariant::Ghost),
                            )
                            .child(
                                Button::new("cd-warn-confirm", "Reset")
                                    .variant(ButtonVariant::Outline),
                            ),
                    ),
            )
            // Variant badges
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(Badge::new("Default").variant(BadgeVariant::Primary))
                    .child(Badge::new("Destructive").variant(BadgeVariant::Error))
                    .child(Badge::new("Warning").variant(BadgeVariant::Warning)),
            )
    }
}
