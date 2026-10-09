use super::prelude::*;

impl Showcase {
    pub(crate) fn render_tooltip_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionTooltips);
        let entity = self.weak_entity_handle();
        let hovered = self.tooltip_hovered;

        let placements: &[(&str, &str, TooltipPlacement, &str)] = &[
            (
                "top",
                "Top",
                TooltipPlacement::Top,
                "I appear above the trigger!",
            ),
            (
                "bottom",
                "Bottom",
                TooltipPlacement::Bottom,
                "I appear below the trigger!",
            ),
            (
                "left",
                "Left",
                TooltipPlacement::Left,
                "I appear to the left!",
            ),
            (
                "right",
                "Right",
                TooltipPlacement::Right,
                "I appear to the right!",
            ),
        ];

        let mut buttons = HStack::new().spacing(StackSpacing::Xl);

        for &(id, label, placement, tooltip_text) in placements {
            let is_shown = hovered == Some(id);
            let entity_clone = entity.clone();

            let trigger = Button::new(SharedString::from(format!("tooltip-trigger-{id}")), label)
                .variant(if is_shown {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                })
                .on_click(move |_window, cx| {
                    entity_clone.update(cx, |this, cx| {
                        if this.tooltip_hovered == Some(id) {
                            this.tooltip_hovered = None;
                        } else {
                            this.tooltip_hovered = Some(id);
                        }
                        this.notify_content(cx);
                    });
                });

            let wrapper = WithTooltip::new(trigger, tooltip_text)
                .placement(placement)
                .show(is_shown);

            buttons = buttons.child(wrapper);
        }

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                Text::new(
                    "Click a button to toggle its tooltip. Each shows a different placement:",
                )
                .muted(true),
            )
            .child(buttons)
    }
}
