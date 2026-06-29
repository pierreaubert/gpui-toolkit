use super::prelude::*;

impl Showcase {
    pub(crate) fn render_search_bar_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionSearchBar);
        let entity = self.entity.clone();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Sizes - empty
            .child(Text::new("Sizes (empty):").weight(TextWeight::Semibold))
            .child(
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .align(StackAlign::Center)
                            .child(Badge::new("Sm").variant(BadgeVariant::Info))
                            .child(
                                div().w(px(250.0)).child(
                                    SearchBar::new("search-sm-empty")
                                        .size(SearchBarSize::Sm)
                                        .placeholder("Search..."),
                                ),
                            ),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .align(StackAlign::Center)
                            .child(Badge::new("Md").variant(BadgeVariant::Info))
                            .child(
                                div().w(px(300.0)).child(
                                    SearchBar::new("search-md-empty")
                                        .size(SearchBarSize::Md)
                                        .placeholder("Search albums..."),
                                ),
                            ),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .align(StackAlign::Center)
                            .child(Badge::new("Lg").variant(BadgeVariant::Info))
                            .child(
                                div().w(px(350.0)).child(
                                    SearchBar::new("search-lg-empty")
                                        .size(SearchBarSize::Lg)
                                        .placeholder("Search everything..."),
                                ),
                            ),
                    ),
            )
            // With value
            .child(Text::new("With value:").weight(TextWeight::Semibold))
            .child(
                VStack::new().spacing(StackSpacing::Md).child(
                    div().w(px(300.0)).child(
                        SearchBar::new("search-with-value")
                            .size(SearchBarSize::Md)
                            .value(self.search_bar_value.clone())
                            .on_change(move |query, _window, cx| {
                                let query = SharedString::from(query.to_string());
                                entity.update(cx, |this, _cx| {
                                    this.search_bar_value = query;
                                });
                            }),
                    ),
                ),
            )
    }
}
