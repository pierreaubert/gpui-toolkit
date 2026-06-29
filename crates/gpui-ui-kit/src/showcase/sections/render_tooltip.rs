use super::prelude::*;

impl Showcase {
    pub(crate) fn render_tooltip_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionTooltips);
        let theme = cx.theme();
        let entity = self.entity.clone();
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

            let trigger = div()
                .id(SharedString::from(format!("tooltip-trigger-{}", id)))
                .px_4()
                .py_2()
                .bg(if is_shown {
                    theme.accent
                } else {
                    theme.surface
                })
                .border_1()
                .border_color(if is_shown { theme.accent } else { theme.border })
                .rounded_md()
                .cursor_pointer()
                .text_sm()
                .text_color(if is_shown {
                    rgba(0xffffffff)
                } else {
                    theme.text_primary
                })
                .hover(|s| s.bg(theme.surface_hover))
                .child(label)
                .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                    entity_clone.update(cx, |this, cx| {
                        if this.tooltip_hovered == Some(id) {
                            this.tooltip_hovered = None;
                        } else {
                            this.tooltip_hovered = Some(id);
                        }
                        cx.notify();
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
