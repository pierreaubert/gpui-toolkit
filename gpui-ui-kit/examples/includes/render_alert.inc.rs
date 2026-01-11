impl Showcase {
    fn render_alerts_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionAlerts);
        let info_title = cx.t(TranslationKey::AlertInfo);
        let success_title = cx.t(TranslationKey::AlertSuccess);
        let warning_title = cx.t(TranslationKey::AlertWarning);
        let error_title = cx.t(TranslationKey::AlertError);
        let info_msg = cx.t(TranslationKey::AlertInfoMessage);
        let success_msg = cx.t(TranslationKey::AlertSuccessMessage);
        let warning_msg = cx.t(TranslationKey::AlertWarningMessage);
        let error_msg = cx.t(TranslationKey::AlertErrorMessage);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        Alert::new("alert-info", info_msg)
                            .title(info_title)
                            .variant(AlertVariant::Info),
                    )
                    .child(
                        Alert::new("alert-success", success_msg)
                            .title(success_title)
                            .variant(AlertVariant::Success),
                    )
                    .child(
                        Alert::new("alert-warning", warning_msg)
                            .title(warning_title)
                            .variant(AlertVariant::Warning),
                    )
                    .child(
                        Alert::new("alert-error", error_msg)
                            .title(error_title)
                            .variant(AlertVariant::Error),
                    ),
            )
            // Inline Alerts
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Inline Alerts").weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Lg)
                            .child(InlineAlert::new("Info message").variant(AlertVariant::Info))
                            .child(
                                InlineAlert::new("Success message").variant(AlertVariant::Success),
                            )
                            .child(
                                InlineAlert::new("Warning message").variant(AlertVariant::Warning),
                            )
                            .child(InlineAlert::new("Error message").variant(AlertVariant::Error)),
                    ),
            )
    }
}
