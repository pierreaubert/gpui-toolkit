use super::prelude::*;

impl Showcase {
    pub(crate) fn render_citation_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionCitation);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Layouts
            .child(Text::new("Inline and block:").weight(TextWeight::Semibold))
            .child(
                Citation::new("ref-inline", "Move slowly and fix things.")
                    .source("Maintenance handbook"),
            )
            .child(
                Citation::new("ref-block", "Make it work, make it right, make it fast.")
                    .source("Engineering notes")
                    .variant(CitationVariant::Block),
            )
            .child(Citation::new(
                "ref-plain",
                "An attribution can be omitted entirely.",
            ))
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                Citation::new("ref-sm", "Compact reference.")
                    .source("Style guide")
                    .size(CitationSize::Sm),
            )
            .child(
                Citation::new("ref-lg", "Large reference.")
                    .source("Style guide")
                    .variant(CitationVariant::Block)
                    .size(CitationSize::Lg),
            )
    }
}
