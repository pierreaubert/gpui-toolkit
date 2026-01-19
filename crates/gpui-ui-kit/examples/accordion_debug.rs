//! Accordion Debug Example
//!
//! Interactive showcase for the Accordion component:
//! - Different orientations (Vertical, Horizontal, Side)
//! - Single vs Multiple mode
//! - Expanded/collapsed states
//! - Disabled items
//! - Custom themes

use gpui::*;
use gpui_ui_kit::accordion::{Accordion, AccordionItem, AccordionMode, AccordionOrientation};
use gpui_ui_kit::i18n::{I18nExt, TranslationKey};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

/// Demo state
pub struct AccordionDebug {
    /// Expanded items for vertical accordion (single mode)
    vertical_single_expanded: Vec<SharedString>,
    /// Expanded items for vertical accordion (multiple mode)
    vertical_multi_expanded: Vec<SharedString>,
    /// Expanded items for horizontal accordion
    horizontal_expanded: Vec<SharedString>,
    /// Expanded items for side accordion
    side_expanded: Vec<SharedString>,
    /// Entity reference
    entity: Entity<Self>,
}

impl AccordionDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            vertical_single_expanded: vec!["section-1".into()],
            vertical_multi_expanded: vec!["multi-1".into(), "multi-3".into()],
            horizontal_expanded: vec!["horiz-1".into()],
            side_expanded: vec!["side-1".into()],
            entity: cx.entity().clone(),
        }
    }

    /// Toggle item in single mode
    fn toggle_single(&mut self, id: &SharedString, expanded: bool) {
        if expanded {
            self.vertical_single_expanded = vec![id.clone()];
        } else {
            self.vertical_single_expanded.clear();
        }
    }

    /// Toggle item in multiple mode
    fn toggle_multi(&mut self, id: &SharedString, expanded: bool) {
        if expanded {
            if !self.vertical_multi_expanded.contains(id) {
                self.vertical_multi_expanded.push(id.clone());
            }
        } else {
            self.vertical_multi_expanded.retain(|x| x != id);
        }
    }

    /// Toggle horizontal item
    fn toggle_horizontal(&mut self, id: &SharedString, expanded: bool) {
        if expanded {
            self.horizontal_expanded = vec![id.clone()];
        } else {
            self.horizontal_expanded.clear();
        }
    }

    /// Toggle side item
    fn toggle_side(&mut self, id: &SharedString, expanded: bool) {
        if expanded {
            if !self.side_expanded.contains(id) {
                self.side_expanded.push(id.clone());
            }
        } else {
            self.side_expanded.retain(|x| x != id);
        }
    }

    /// Sample content for accordion items
    fn sample_content(text: impl Into<SharedString>, theme: &Theme) -> impl IntoElement {
        div().p_4().child(
            Text::new(text)
                .size(TextSize::Sm)
                .color(theme.text_secondary),
        )
    }
}

impl Render for AccordionDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = self.entity.clone();
        let theme = cx.theme();

        div()
            .id("accordion-debug-root")
            .w_full()
            .h_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_6()
            .flex()
            .flex_col()
            .gap_6()
            // Header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Heading::h1("Accordion Component Debug"))
                    .child(
                        Text::new("Click headers to expand/collapse sections. Double arrows indicate collapse direction.")
                            .muted(true),
                    ),
            )
            // i18n Status Bar - demonstrates language switching works
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_3()
                    .bg(theme.surface)
                    .rounded_lg()
                    .child(Text::new(format!("🌐 {}: ", cx.t(TranslationKey::MenuLanguage))).weight(TextWeight::Medium))
                    .child(Text::new(cx.language().native_name()).color(theme.accent))
                    .child(Text::new(" | "))
                    .child(Text::new(cx.t(TranslationKey::SectionAccordion)).color(theme.text_secondary)),
            )
            // Vertical Single Mode
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Vertical - Single Mode")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Only one section can be open at a time").size(TextSize::Sm).muted(true))
                    .child({
                        let expanded = self.vertical_single_expanded.clone();
                        Accordion::new()
                            .mode(AccordionMode::Single)
                            .orientation(AccordionOrientation::Vertical)
                            .expanded(expanded)
                            .item(
                                AccordionItem::new("section-1", "Section 1: Getting Started")
                                    .content(Self::sample_content(
                                        "Welcome to the accordion component! This section contains introductory content.",
                                        &theme,
                                    )),
                            )
                            .item(
                                AccordionItem::new("section-2", "Section 2: Configuration")
                                    .content(Self::sample_content(
                                        "Configure your settings here. Adjust parameters as needed.",
                                        &theme,
                                    )),
                            )
                            .item(
                                AccordionItem::new("section-3", "Section 3: Advanced Options")
                                    .content(Self::sample_content(
                                        "Advanced options for power users. Use with caution!",
                                        &theme,
                                    )),
                            )
                            .on_change({
                                let entity = entity.clone();
                                move |id, expanded, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.toggle_single(id, expanded);
                                    });
                                }
                            })
                    }),
            )
            // Vertical Multiple Mode
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Vertical - Multiple Mode")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Multiple sections can be open simultaneously").size(TextSize::Sm).muted(true))
                    .child({
                        let expanded = self.vertical_multi_expanded.clone();
                        Accordion::new()
                            .mode(AccordionMode::Multiple)
                            .orientation(AccordionOrientation::Vertical)
                            .expanded(expanded)
                            .item(
                                AccordionItem::new("multi-1", "FAQ: What is this?")
                                    .content(Self::sample_content(
                                        "This is an accordion component that supports multiple open sections.",
                                        &theme,
                                    )),
                            )
                            .item(
                                AccordionItem::new("multi-2", "FAQ: How do I use it?")
                                    .content(Self::sample_content(
                                        "Click on any header to toggle that section. Multiple can be open!",
                                        &theme,
                                    )),
                            )
                            .item(
                                AccordionItem::new("multi-3", "FAQ: Can I disable items?")
                                    .content(Self::sample_content(
                                        "Yes! See the disabled item example below.",
                                        &theme,
                                    )),
                            )
                            .on_change({
                                let entity = entity.clone();
                                move |id, expanded, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.toggle_multi(id, expanded);
                                    });
                                }
                            })
                    }),
            )
            // Horizontal Orientation
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Horizontal Orientation")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Headers arranged horizontally, content expands below").size(TextSize::Sm).muted(true))
                    .child({
                        let expanded = self.horizontal_expanded.clone();
                        Accordion::new()
                            .mode(AccordionMode::Single)
                            .orientation(AccordionOrientation::Horizontal)
                            .expanded(expanded)
                            .item(
                                AccordionItem::new("horiz-1", "Tab A")
                                    .content(Self::sample_content(
                                        "Content for Tab A. This horizontal accordion works like tabs!",
                                        &theme,
                                    )),
                            )
                            .item(
                                AccordionItem::new("horiz-2", "Tab B")
                                    .content(Self::sample_content(
                                        "Content for Tab B. Click the headers above to switch.",
                                        &theme,
                                    )),
                            )
                            .item(
                                AccordionItem::new("horiz-3", "Tab C")
                                    .content(Self::sample_content(
                                        "Content for Tab C. Great for tab-like interfaces!",
                                        &theme,
                                    )),
                            )
                            .on_change({
                                let entity = entity.clone();
                                move |id, expanded, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.toggle_horizontal(id, expanded);
                                    });
                                }
                            })
                    }),
            )
            // Side Orientation
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Side Orientation")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Vertical tabs on left, content on right - multiple can be open").size(TextSize::Sm).muted(true))
                    .child(
                        div().h(px(200.0)).child({
                            let expanded = self.side_expanded.clone();
                            Accordion::new()
                                .mode(AccordionMode::Multiple)
                                .orientation(AccordionOrientation::Side)
                                .expanded(expanded)
                                .item(
                                    AccordionItem::new("side-1", "Info")
                                        .content(Self::sample_content(
                                            "Information panel content. Side accordions are great for property panels!",
                                            &theme,
                                        )),
                                )
                                .item(
                                    AccordionItem::new("side-2", "Settings")
                                        .content(Self::sample_content(
                                            "Settings panel content. Multiple panels can be open at once.",
                                            &theme,
                                        )),
                                )
                                .item(
                                    AccordionItem::new("side-3", "Help")
                                        .content(Self::sample_content(
                                            "Help panel content. Click the vertical tabs on the left!",
                                            &theme,
                                        )),
                                )
                                .on_change({
                                    let entity = entity.clone();
                                    move |id, expanded, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.toggle_side(id, expanded);
                                        });
                                    }
                                })
                        }),
                    ),
            )
            // With Disabled Item
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("With Disabled Item")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("The middle item is disabled and cannot be clicked").size(TextSize::Sm).muted(true))
                    .child(
                        Accordion::new()
                            .mode(AccordionMode::Single)
                            .orientation(AccordionOrientation::Vertical)
                            .expanded(vec!["enabled-1".into()])
                            .item(
                                AccordionItem::new("enabled-1", "Enabled Section")
                                    .content(Self::sample_content(
                                        "This section is enabled and can be toggled.",
                                        &theme,
                                    )),
                            )
                            .item(
                                AccordionItem::new("disabled-1", "Disabled Section (Premium)")
                                    .disabled(true)
                                    .content(Self::sample_content(
                                        "This section is disabled. Upgrade to premium to access!",
                                        &theme,
                                    )),
                            )
                            .item(
                                AccordionItem::new("enabled-2", "Another Enabled Section")
                                    .content(Self::sample_content(
                                        "This section is also enabled and clickable.",
                                        &theme,
                                    )),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Accordion Debug")
            .size(900.0, 1000.0)
            .scrollable(true)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(AccordionDebug::new),
    );
}
