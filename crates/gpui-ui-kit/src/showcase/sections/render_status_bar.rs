use super::prelude::*;

impl Showcase {
    pub(crate) fn render_status_bar_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionStatusBar);
        let theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Top status bar
            .child(Text::new("Top position:").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .h(px(120.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        StatusBar::new("preview-top")
                            .position(StatusBarPosition::Top)
                            .left(Text::new("File  Edit  View").size(TextSize::Sm))
                            .center(Text::new("My Application").size(TextSize::Sm))
                            .right(Text::new("v1.0.0").size(TextSize::Sm)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Text::new("Content Area").muted(true)),
                    ),
            )
            // Bottom status bar
            .child(Text::new("Bottom position:").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .h(px(120.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Text::new("Content Area").muted(true)),
                    )
                    .child(
                        StatusBar::new("preview-bottom")
                            .position(StatusBarPosition::Bottom)
                            .left(Text::new("Ready").size(TextSize::Sm))
                            .center(Text::new("Ln 42, Col 8").size(TextSize::Sm))
                            .right(Text::new("UTF-8  LF  Rust").size(TextSize::Sm)),
                    ),
            )
    }
}
