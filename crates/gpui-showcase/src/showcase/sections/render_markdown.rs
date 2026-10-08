use super::prelude::*;

impl Showcase {
    pub(crate) fn render_markdown_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionMarkdown);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Block coverage
            .child(Text::new("Headings, code, lists, quotes:").weight(TextWeight::Semibold))
            .child(Markdown::new(
                "markdown-blocks",
                "# Release notes\n\nShipped today with fixes.\n\n```rust\nlet ready = true;\n```\n\n- Fast startup\n- Small bundle\n\n> Thanks to every contributor.",
            ))
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                Markdown::new("markdown-sm", "## Compact\n\nSmall prose.")
                    .size(MarkdownSize::Sm),
            )
            .child(
                Markdown::new("markdown-lg", "## Large\n\nLarge prose.")
                    .size(MarkdownSize::Lg),
            )
    }
}
