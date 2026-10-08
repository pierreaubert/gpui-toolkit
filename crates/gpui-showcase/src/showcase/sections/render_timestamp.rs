use super::prelude::*;

impl Showcase {
    pub(crate) fn render_timestamp_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionTimestamp);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Relative and absolute labels
            .child(Text::new("Relative and absolute:").weight(TextWeight::Semibold))
            .child(Timestamp::new("edited-relative", "Edited 2 hours ago"))
            .child(Timestamp::new("edited-absolute", "2026-10-07 09:30"))
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(Timestamp::new("edited-sm", "Just now").size(TimestampSize::Sm))
            .child(Timestamp::new("edited-lg", "Yesterday at 18:04").size(TimestampSize::Lg))
    }
}
