impl Showcase {
    fn render_toasts_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionToasts);
        let info = cx.t(TranslationKey::AlertInfo);
        let success = cx.t(TranslationKey::AlertSuccess);
        let warning = cx.t(TranslationKey::AlertWarning);
        let error = cx.t(TranslationKey::AlertError);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(Text::new("Toast notifications with different variants:").muted(true))
            .child(
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        Toast::new("toast-info", "This is an informational toast notification.")
                            .title(info)
                            .variant(ToastVariant::Info)
                            .closeable(false),
                    )
                    .child(
                        Toast::new("toast-success", "Your operation completed successfully!")
                            .title(success)
                            .variant(ToastVariant::Success)
                            .closeable(false),
                    )
                    .child(
                        Toast::new("toast-warning", "Please be aware of potential issues.")
                            .title(warning)
                            .variant(ToastVariant::Warning)
                            .closeable(false),
                    )
                    .child(
                        Toast::new("toast-error", "An error occurred during the operation.")
                            .title(error)
                            .variant(ToastVariant::Error)
                            .closeable(false),
                    ),
            )
    }
}
