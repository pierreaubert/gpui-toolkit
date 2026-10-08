use super::prelude::*;

impl Showcase {
    pub(crate) fn render_list_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionList);
        let entity = self.weak_entity_handle();
        let selected = self.selected_list_item.clone();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Interactive selection
            .child(Text::new("Servers:").weight(TextWeight::Semibold))
            .child({
                let entity_clone = entity.clone();
                List::new(
                    "showcase-list",
                    vec![
                        ListItem::new("servers-a", "Alpha").description("Primary region"),
                        ListItem::new("servers-b", "Beta").description("Staging region"),
                        ListItem::new("servers-c", "Gamma")
                            .description("Decommissioned")
                            .disabled(true),
                    ],
                )
                .selected(selected)
                .on_select(move |id, _window, cx| {
                    entity_clone.update(cx, |this, cx| {
                        this.selected_list_item = id;
                        this.notify_content(cx);
                    });
                })
            })
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                List::new(
                    "showcase-list-sizes",
                    vec![
                        ListItem::new("size-s", "Small"),
                        ListItem::new("size-m", "Medium"),
                    ],
                )
                .selected("size-m")
                .size(ListSize::Sm),
            )
    }
}
