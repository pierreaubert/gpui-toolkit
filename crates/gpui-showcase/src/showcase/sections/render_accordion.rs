use super::prelude::*;

impl Showcase {
    pub(crate) fn render_accordion_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionAccordion);
        let entity = self.weak_entity_handle();

        let accordion_vertical_single = self.accordion_vertical_single.clone();
        let accordion_vertical_multiple = self.accordion_vertical_multiple.clone();
        let accordion_horizontal_single = self.accordion_horizontal_single.clone();
        let accordion_side_single = self.accordion_side_single.clone();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(Text::new("Expandable content sections with vertical and horizontal orientations:").muted(true))

            // VERTICAL ORIENTATION EXAMPLES
            .child(
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .child(Text::new("Vertical Orientation").weight(TextWeight::Bold))

                    // Single Mode - only one item can be open
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .child(Text::new("Single Mode").weight(TextWeight::Medium))
                                    .child(Badge::new("Only one open").variant(BadgeVariant::Primary))
                            )
                            .child(Text::new("Click any header - only one section stays open at a time").muted(true).size(TextSize::Xs))
                            .child(
                                div()
                                    .w(px(500.0))
                                    .child(
                                        Accordion::new()
                                            .mode(AccordionMode::Single)
                                            .orientation(AccordionOrientation::Vertical)
                                            .items(vec![
                                                AccordionItem::new("v-single-1", "Getting Started")
                                                    .content("Welcome! This is single mode - opening another section will close this one."),
                                                AccordionItem::new("v-single-2", "Features")
                                                    .content("• Multiple accordion modes\n• Vertical and horizontal layouts\n• Custom themes\n• Keyboard navigation"),
                                                AccordionItem::new("v-single-3", "Configuration")
                                                    .content("Use .mode(AccordionMode::Single) to ensure only one section is open at a time."),
                                            ])
                                            .expanded(accordion_vertical_single.clone())
                                            .on_change({
                                                let entity = entity.clone();
                                                move |id, is_expanded, _window, cx| {
                                                    entity.update(cx, |showcase, _cx| {
                                                        // Single mode: replace the expanded list with just this item
                                                        if is_expanded {
                                                            showcase.accordion_vertical_single = vec![id.clone()];
                                                        } else {
                                                            showcase.accordion_vertical_single.clear();
                                                        }
                                                    });
                                                }
                                            }),
                                    ),
                            )
                    )

                    // Multiple Mode - many items can be open
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .child(Text::new("Multiple Mode").weight(TextWeight::Medium))
                                    .child(Badge::new("Many open").variant(BadgeVariant::Success))
                            )
                            .child(Text::new("Open and close sections independently - multiple can be open simultaneously").muted(true).size(TextSize::Xs))
                            .child(
                                div()
                                    .w(px(500.0))
                                    .child(
                                        Accordion::new()
                                            .mode(AccordionMode::Multiple)
                                            .orientation(AccordionOrientation::Vertical)
                                            .items(vec![
                                                AccordionItem::new("v-multi-1", "Section 1")
                                                    .content("This section can stay open while you open others!"),
                                                AccordionItem::new("v-multi-2", "Section 2")
                                                    .content("Multiple mode lets you expand as many sections as you want."),
                                                AccordionItem::new("v-multi-3", "Section 3")
                                                    .content("Try opening all three sections at once - they'll all stay open!"),
                                            ])
                                            .expanded(accordion_vertical_multiple.clone())
                                            .on_change({
                                                let entity = entity.clone();
                                                move |id, is_expanded, _window, cx| {
                                                    entity.update(cx, |showcase, _cx| {
                                                        // Multiple mode: add/remove from the list
                                                        if is_expanded {
                                                            if !showcase.accordion_vertical_multiple.contains(id) {
                                                                showcase.accordion_vertical_multiple.push(id.clone());
                                                            }
                                                        } else {
                                                            showcase.accordion_vertical_multiple.retain(|i| i != id);
                                                        }
                                                    });
                                                }
                                            }),
                                    ),
                            )
                    )
            )

            // HORIZONTAL ORIENTATION EXAMPLES
            .child(
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .child(Text::new("Horizontal Orientation").weight(TextWeight::Bold))

                    // Single Mode Horizontal
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .child(Text::new("Single Mode Horizontal").weight(TextWeight::Medium))
                                    .child(Badge::new("Only one open").variant(BadgeVariant::Primary))
                            )
                            .child(Text::new("Headers arranged horizontally, content expands downward").muted(true).size(TextSize::Xs))
                            .child(
                                div()
                                    .w_full()
                                    .child(
                                        Accordion::new()
                                            .mode(AccordionMode::Single)
                                            .orientation(AccordionOrientation::Horizontal)
                                            .items(vec![
                                                AccordionItem::new("h-single-1", "Tab 1")
                                                    .content("Horizontal accordion with single mode behavior."),
                                                AccordionItem::new("h-single-2", "Tab 2")
                                                    .content("Content expands below the selected tab."),
                                                AccordionItem::new("h-single-3", "Tab 3")
                                                    .content("Great for tab-like interfaces with collapsible content!"),
                                            ])
                                            .expanded(accordion_horizontal_single.clone())
                                            .on_change({
                                                let entity = entity.clone();
                                                move |id, is_expanded, _window, cx| {
                                                    entity.update(cx, |showcase, _cx| {
                                                        if is_expanded {
                                                            showcase.accordion_horizontal_single = vec![id.clone()];
                                                        } else {
                                                            showcase.accordion_horizontal_single.clear();
                                                        }
                                                    });
                                                }
                                            }),
                                    ),
                            )
                    )
            )

            // SIDE ORIENTATION EXAMPLE
            .child(
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .child(Text::new("Side Orientation").weight(TextWeight::Bold))

                    // Multiple Mode Side
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .child(Text::new("Multiple Mode Side").weight(TextWeight::Medium))
                                    .child(Badge::new("Vertical tabs").variant(BadgeVariant::Success))
                            )
                            .child(Text::new("Vertical tabs before and including the first open item sit left of the content; later tabs sit right. Rotated labels and accent rails remain visible. Open multiple tabs to show their content in the center.").muted(true).size(TextSize::Xs))
                            .child(
                                div()
                                    .w_full()
                                    .h(px(300.0))
                                    .child(
                                        Accordion::new()
                                            .mode(AccordionMode::Multiple)
                                            .orientation(AccordionOrientation::Side)
                                            .items(vec![
                                                AccordionItem::new("side-single-1", "Tab1")
                                                    .content("The first open tab stays on the left side of the active content."),
                                                AccordionItem::new("side-single-2", "Tab2")
                                                    .content("Tabs after the first open tab move to the right of the content."),
                                                AccordionItem::new("side-single-3", "Tab3")
                                                    .content("Open multiple tabs to see their content together between the split tab bars."),
                                            ])
                                            .expanded(accordion_side_single.clone())
                                            .on_change({
                                                let entity = entity.clone();
                                                move |id, is_expanded, _window, cx| {
                                                    entity.update(cx, |showcase, _cx| {
                                                        // Multiple mode: add/remove from the list
                                                        if is_expanded {
                                                            if !showcase.accordion_side_single.contains(id) {
                                                                showcase.accordion_side_single.push(id.clone());
                                                            }
                                                        } else {
                                                            showcase.accordion_side_single.retain(|i| i != id);
                                                        }
                                                    });
                                                }
                                            }),
                                    ),
                            )
                    )
            )

            // Collapsible Explanation
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Collapsible Behavior").weight(TextWeight::Bold))
                            .child(Badge::new("Click to collapse").variant(BadgeVariant::Warning))
                    )
                    .child(
                        Text::new(
                            "All accordions are collapsible - click an expanded section's header to collapse it. \
                            In Single mode, clicking another header also collapses the current section."
                        )
                        .muted(true)
                        .size(TextSize::Sm)
                    )
            )
    }
}
