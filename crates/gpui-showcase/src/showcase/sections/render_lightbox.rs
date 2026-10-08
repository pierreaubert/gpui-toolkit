use super::prelude::*;

impl Showcase {
    pub(crate) fn render_lightbox_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionLightbox);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Open lightbox with a caption
            .child(Text::new("Open lightbox with a caption:").weight(TextWeight::Semibold))
            .child(
                div()
                    .relative()
                    .h(px(360.0))
                    .w_full()
                    .max_w(px(640.0))
                    .overflow_hidden()
                    .rounded_md()
                    .child(
                        Lightbox::new(
                            "showcase-lightbox",
                            "assets/component-lab-gallery/contact-sheet-014.png",
                        )
                        .caption("Gallery preview")
                        .open(true),
                    ),
            )
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                div()
                    .relative()
                    .h(px(280.0))
                    .w_full()
                    .max_w(px(480.0))
                    .overflow_hidden()
                    .rounded_md()
                    .child(
                        Lightbox::new("showcase-lightbox-sm", "assets/missing.png")
                            .caption("Compact panel")
                            .size(LightboxSize::Sm)
                            .open(true),
                    ),
            )
            .child(Text::new("A closed lightbox renders nothing.").muted(true))
    }
}
