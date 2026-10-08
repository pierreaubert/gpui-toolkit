use super::prelude::*;

impl Showcase {
    pub(crate) fn render_layer_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionLayer);
        let theme = cx.theme();

        let panel = div()
            .px(px(24.0))
            .py(px(16.0))
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .shadow_lg()
            .text_sm()
            .text_color(theme.text_primary)
            .child("Session expired. Sign in again to continue.");

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Open layer with backdrop
            .child(Text::new("Open layer with backdrop:").weight(TextWeight::Semibold))
            .child(
                div()
                    .relative()
                    .h(px(220.0))
                    .w_full()
                    .max_w(px(520.0))
                    .overflow_hidden()
                    .rounded_md()
                    .bg(theme.background)
                    .child(Layer::new("showcase-layer").open(true).child(panel)),
            )
            // Without backdrop
            .child(Text::new("Without backdrop:").weight(TextWeight::Semibold))
            .child(
                div()
                    .relative()
                    .h(px(160.0))
                    .w_full()
                    .max_w(px(520.0))
                    .overflow_hidden()
                    .rounded_md()
                    .bg(theme.background)
                    .child(
                        Layer::new("showcase-layer-plain")
                            .open(true)
                            .show_backdrop(false)
                            .child(
                                div()
                                    .px(px(24.0))
                                    .py(px(16.0))
                                    .bg(theme.surface)
                                    .border_1()
                                    .border_color(theme.border)
                                    .rounded_lg()
                                    .text_sm()
                                    .text_color(theme.text_primary)
                                    .child("Bare overlay content."),
                            ),
                    ),
            )
            .child(Text::new("A closed layer renders nothing.").muted(true))
    }
}
