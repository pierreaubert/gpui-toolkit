impl Showcase {
    fn render_dialog_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionDialogs);
        let dialog_title = cx.t(TranslationKey::DialogConfirmTitle);
        let dialog_message = cx.t(TranslationKey::DialogConfirmMessage);
        let cancel = cx.t(TranslationKey::ButtonCancel);
        let confirm = cx.t(TranslationKey::ButtonConfirm);
        let theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                Text::new("Dialog component preview (backdrop disabled for showcase):").muted(true),
            )
            .child(
                div()
                    .relative()
                    .h(px(200.0))
                    .w_full()
                    .max_w(px(500.0))
                    .bg(theme.surface)
                    .rounded_lg()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        // Dialog preview without backdrop
                        div()
                            .w(px(350.0))
                            .bg(theme.background)
                            .border_1()
                            .border_color(theme.accent)
                            .rounded_lg()
                            .shadow_lg()
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            // Header
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px_4()
                                    .py_3()
                                    .border_b_1()
                                    .border_color(theme.border)
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.text_primary)
                                            .child(dialog_title),
                                    )
                                    .child(div().text_sm().text_color(theme.text_muted).child("x")),
                            )
                            // Content
                            .child(div().px_4().py_4().child(Text::new(dialog_message)))
                            // Footer
                            .child(
                                div()
                                    .px_4()
                                    .py_3()
                                    .border_t_1()
                                    .border_color(theme.border)
                                    .child(
                                        HStack::new()
                                            .justify(StackJustify::End)
                                            .spacing(StackSpacing::Sm)
                                            .child(
                                                Button::new("dlg-cancel", cancel)
                                                    .variant(ButtonVariant::Ghost),
                                            )
                                            .child(
                                                Button::new("dlg-confirm", confirm)
                                                    .variant(ButtonVariant::Primary),
                                            ),
                                    ),
                            ),
                    ),
            )
            // Dialog sizes info
            .child(
                HStack::new()
                    .spacing(StackSpacing::Lg)
                    .child(Badge::new("Sm: 320px").variant(BadgeVariant::Info))
                    .child(Badge::new("Md: 480px").variant(BadgeVariant::Info))
                    .child(Badge::new("Lg: 640px").variant(BadgeVariant::Info))
                    .child(Badge::new("Xl: 800px").variant(BadgeVariant::Info)),
            )
    }
}
