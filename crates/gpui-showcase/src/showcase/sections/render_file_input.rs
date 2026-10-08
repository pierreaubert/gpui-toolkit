use super::prelude::*;

impl Showcase {
    pub(crate) fn render_file_input_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionFileInput);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Empty and chosen states
            .child(Text::new("Empty and chosen:").weight(TextWeight::Semibold))
            .child(FileInput::new("file-empty"))
            .child(
                FileInput::new("file-chosen")
                    .file_name("portrait.png")
                    .accept(".png,.jpg"),
            )
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                FileInput::new("file-sm")
                    .file_name("mix.wav")
                    .size(FileInputSize::Sm),
            )
            .child(
                FileInput::new("file-lg")
                    .file_name("session.zip")
                    .size(FileInputSize::Lg),
            )
            // Disabled
            .child(Text::new("Disabled:").weight(TextWeight::Semibold))
            .child(
                FileInput::new("file-disabled")
                    .file_name("readonly.pdf")
                    .disabled(true),
            )
    }
}
