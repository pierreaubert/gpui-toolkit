use super::prelude::*;

impl Showcase {
    pub(crate) fn render_field_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionField);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Label with help text
            .child(Text::new("Label with help:").weight(TextWeight::Semibold))
            .child(
                Field::new("field-help")
                    .label("Gain")
                    .child(Tokenizer::new("field-help-control").tokens(["-6 dB"]))
                    .help("Applied before the limiter."),
            )
            // Required with error text
            .child(Text::new("Required with error:").weight(TextWeight::Semibold))
            .child(
                Field::new("field-error")
                    .label("Output file")
                    .required(true)
                    .child(FileInput::new("field-error-control"))
                    .error("An output file is required."),
            )
            // Unlabeled passthrough
            .child(Text::new("Unlabeled:").weight(TextWeight::Semibold))
            .child(Field::new("field-plain").child(Tokenizer::new("field-plain-control")))
    }
}
