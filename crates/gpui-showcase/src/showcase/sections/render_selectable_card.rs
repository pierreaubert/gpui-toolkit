use super::prelude::*;

impl Showcase {
    pub(crate) fn render_selectable_card_section(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionSelectableCard);
        let entity = self.weak_entity_handle();
        let selected = self.selected_card.clone();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Interactive selection
            .child(Text::new("Plan picker:").weight(TextWeight::Semibold))
            .child({
                let entity_clone = entity.clone();
                SelectableCard::new("plan-starter", "Starter")
                    .description("For side projects finding their pace.")
                    .selected(selected.as_ref() == "starter")
                    .on_click(move |_window, cx| {
                        entity_clone.update(cx, |this, cx| {
                            this.selected_card = "starter".into();
                            this.notify_content(cx);
                        });
                    })
            })
            .child({
                let entity_clone = entity.clone();
                SelectableCard::new("plan-pro", "Pro")
                    .description("For teams shipping every week.")
                    .selected(selected.as_ref() == "pro")
                    .on_click(move |_window, cx| {
                        entity_clone.update(cx, |this, cx| {
                            this.selected_card = "pro".into();
                            this.notify_content(cx);
                        });
                    })
            })
            // States
            .child(Text::new("States:").weight(TextWeight::Semibold))
            .child(
                SelectableCard::new("plan-team", "Team")
                    .description("Selected without a handler.")
                    .selected(true),
            )
            .child(
                SelectableCard::new("plan-legacy", "Legacy")
                    .description("No longer available.")
                    .disabled(true),
            )
    }
}
