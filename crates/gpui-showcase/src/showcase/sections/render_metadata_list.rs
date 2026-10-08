use super::prelude::*;

impl Showcase {
    pub(crate) fn render_metadata_list_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionMetadataList);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // File details
            .child(Text::new("File details:").weight(TextWeight::Semibold))
            .child(
                MetadataList::new("file-meta")
                    .entry(MetadataEntry::new("Author", "Ada"))
                    .entry(MetadataEntry::new("License", "MIT"))
                    .entry(MetadataEntry::new("Updated", "2026-10-07")),
            )
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                MetadataList::new("file-meta-sm")
                    .entry(MetadataEntry::new("Format", "FLAC"))
                    .entry(MetadataEntry::new("Channels", "2"))
                    .size(MetadataListSize::Sm),
            )
            .child(
                MetadataList::new("file-meta-lg")
                    .entry(MetadataEntry::new("Renderer", "Metal"))
                    .entry(MetadataEntry::new("Backend", "GPU"))
                    .size(MetadataListSize::Lg),
            )
    }
}
