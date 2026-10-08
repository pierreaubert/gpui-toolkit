use super::prelude::*;

impl Showcase {
    pub(crate) fn render_tokenizer_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionTokenizer);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Empty and filled states
            .child(Text::new("Empty and filled:").weight(TextWeight::Semibold))
            .child(Tokenizer::new("tokens-empty"))
            .child(Tokenizer::new("tokens-filled").tokens(["drums", "bass", "keys"]))
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                Tokenizer::new("tokens-sm")
                    .tokens(["verse", "chorus"])
                    .size(TokenizerSize::Sm),
            )
            .child(
                Tokenizer::new("tokens-lg")
                    .tokens(["intro", "outro"])
                    .size(TokenizerSize::Lg),
            )
            // Disabled
            .child(Text::new("Disabled:").weight(TextWeight::Semibold))
            .child(
                Tokenizer::new("tokens-disabled")
                    .tokens(["locked"])
                    .disabled(true),
            )
    }
}
