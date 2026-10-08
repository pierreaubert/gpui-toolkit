use super::prelude::*;

impl Showcase {
    pub(crate) fn render_pagination_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionPagination);
        let entity = self.weak_entity_handle();
        let page = self.pagination_page;

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Interactive window
            .child(Text::new("Twelve pages:").weight(TextWeight::Semibold))
            .child({
                let entity_clone = entity.clone();
                Pagination::new("showcase-pagination")
                    .page(page)
                    .total_pages(12)
                    .on_change(move |page, _window, cx| {
                        entity_clone.update(cx, |this, cx| {
                            this.pagination_page = page;
                            this.notify_content(cx);
                        });
                    })
            })
            // States
            .child(Text::new("States:").weight(TextWeight::Semibold))
            .child(
                Pagination::new("showcase-pagination-first")
                    .page(1)
                    .total_pages(12)
                    .size(PaginationSize::Sm),
            )
            .child(
                Pagination::new("showcase-pagination-short")
                    .page(2)
                    .total_pages(3),
            )
    }
}
