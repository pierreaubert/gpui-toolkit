use super::prelude::*;

impl Showcase {
    pub(crate) fn render_blockquote_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionBlockquote);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Citation states
            .child(Text::new("With and without citation:").weight(TextWeight::Semibold))
            .child(
                Blockquote::new("quote-cited", "Ship small, ship often.").cite("Release captain"),
            )
            .child(Blockquote::new(
                "quote-plain",
                "Make it work, make it right, make it fast.",
            ))
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(Blockquote::new("quote-sm", "Compact quotation.").size(BlockquoteSize::Sm))
            .child(
                Blockquote::new("quote-lg", "Large quotation.")
                    .cite("Design review")
                    .size(BlockquoteSize::Lg),
            )
    }
}
