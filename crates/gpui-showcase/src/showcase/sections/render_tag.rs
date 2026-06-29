use super::prelude::*;

impl Showcase {
    pub(crate) fn render_tag_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionTag);
        let _theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Variants
            .child(Text::new("Variants:").weight(TextWeight::Semibold))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Tag::new("tag-default", "Default"))
                    .child(Tag::new("tag-primary", "Primary").variant(TagVariant::Primary))
                    .child(Tag::new("tag-success", "Success").variant(TagVariant::Success))
                    .child(Tag::new("tag-warning", "Warning").variant(TagVariant::Warning))
                    .child(Tag::new("tag-error", "Error").variant(TagVariant::Error))
                    .child(Tag::new("tag-outlined", "Outlined").variant(TagVariant::Outlined)),
            )
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(
                        Tag::new("tag-sm", "Small")
                            .variant(TagVariant::Primary)
                            .size(TagSize::Sm),
                    )
                    .child(
                        Tag::new("tag-md", "Medium")
                            .variant(TagVariant::Primary)
                            .size(TagSize::Md),
                    )
                    .child(
                        Tag::new("tag-lg", "Large")
                            .variant(TagVariant::Primary)
                            .size(TagSize::Lg),
                    ),
            )
            // With icon
            .child(Text::new("With icon:").weight(TextWeight::Semibold))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(
                        Tag::new("tag-icon-music", "FLAC")
                            .variant(TagVariant::Success)
                            .icon("*"),
                    )
                    .child(
                        Tag::new("tag-icon-warn", "Lossy")
                            .variant(TagVariant::Warning)
                            .icon("!"),
                    ),
            )
            // Removable
            .child(Text::new("Removable:").weight(TextWeight::Semibold))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(
                        Tag::new("tag-rm-1", "Rock")
                            .variant(TagVariant::Primary)
                            .removable(true),
                    )
                    .child(
                        Tag::new("tag-rm-2", "Jazz")
                            .variant(TagVariant::Success)
                            .removable(true),
                    )
                    .child(
                        Tag::new("tag-rm-3", "Electronic")
                            .variant(TagVariant::Outlined)
                            .removable(true),
                    ),
            )
    }
}
