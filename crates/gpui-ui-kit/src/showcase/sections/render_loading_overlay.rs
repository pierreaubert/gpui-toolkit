use super::prelude::*;

impl Showcase {
    pub(crate) fn render_loading_overlay_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionLoadingOverlay);
        let theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Basic overlay
            .child(Text::new("With message:").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(400.0))
                    .h(px(200.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .relative()
                    .child(
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Text::new("Content behind overlay").muted(true)),
                    )
                    .child(
                        LoadingOverlay::new("overlay-msg")
                            .message("Loading library...")
                            .spinner_size(SpinnerSize::Lg),
                    ),
            )
            // With subtitle
            .child(Text::new("With subtitle:").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(400.0))
                    .h(px(200.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .relative()
                    .child(
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Text::new("Background content").muted(true)),
                    )
                    .child(
                        LoadingOverlay::new("overlay-sub")
                            .message("Scanning audio files")
                            .subtitle("This may take a moment...")
                            .spinner_size(SpinnerSize::Md),
                    ),
            )
            // Minimal (spinner only)
            .child(Text::new("Minimal (spinner only):").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(300.0))
                    .h(px(150.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .relative()
                    .child(
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Text::new("Loading...").muted(true)),
                    )
                    .child(LoadingOverlay::new("overlay-min")),
            )
    }
}
