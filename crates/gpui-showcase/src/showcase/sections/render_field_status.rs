use super::prelude::*;

impl Showcase {
    pub(crate) fn render_field_status_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionFieldStatus);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Severities
            .child(Text::new("Severities:").weight(TextWeight::Semibold))
            .child(FieldStatus::new(
                "status-info",
                "Stereo width is derived from the mid/side balance.",
            ))
            .child(
                FieldStatus::new("status-success", "Gain staging saved.")
                    .variant(FieldStatusVariant::Success),
            )
            .child(
                FieldStatus::new("status-warning", "Peak levels approach clipping.")
                    .variant(FieldStatusVariant::Warning),
            )
            .child(
                FieldStatus::new("status-error", "An output file is required.")
                    .variant(FieldStatusVariant::Error),
            )
    }
}
