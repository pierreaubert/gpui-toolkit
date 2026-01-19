//! Select Debug Example
//!
//! A minimal example to debug the select dropdown issues:
//! 1. Transparent dropdown background
//! 2. Dropdown going under other elements (z-index issue)
//!
//! Also demonstrates the new ButtonSet component.
//!
//! Solution: Use gpui::deferred() and gpui::anchored() for proper overlay rendering

use gpui::*;
use gpui_ui_kit::button_set::{ButtonSet, ButtonSetOption, ButtonSetSize};
use gpui_ui_kit::i18n::{I18nExt, TranslationKey};
use gpui_ui_kit::select::{Select, SelectOption};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct SelectDebug {
    select1_value: Option<SharedString>,
    select1_open: bool,
    select1_highlighted: Option<usize>,

    select2_value: Option<SharedString>,
    select2_open: bool,
    select2_highlighted: Option<usize>,

    // ButtonSet state
    view_mode: SharedString,
    alignment: SharedString,

    entity: Entity<Self>,
}

impl SelectDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            select1_value: Some("apple".into()),
            select1_open: false,
            select1_highlighted: None,

            select2_value: Some("red".into()),
            select2_open: false,
            select2_highlighted: None,

            view_mode: "grid".into(),
            alignment: "center".into(),

            entity: cx.entity().clone(),
        }
    }
}

impl Render for SelectDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = self.entity.clone();
        let theme = cx.theme();

        div()
            .id("select-debug-root")
            .w_full()
            .h_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            // Header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Heading::h1("Select Dropdown Debug"))
                    .child(Text::new(
                        "Testing: 1) Background transparency 2) Z-index/layering",
                    )),
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
                    .child(Text::new(cx.t(TranslationKey::SectionFormControls)).color(theme.text_secondary)),
            )
            .child(Divider::new().build())
            // Problem description
            .child(
                div()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .p_4()
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Issues to debug:").weight(TextWeight::Bold))
                            .child(Text::new("1. Dropdown background appears transparent"))
                            .child(Text::new("2. Dropdown goes under other elements"))
                            .child(Text::new("3. Try clicking Select 1 - dropdown should appear above the card below")),
                    ),
            )
            // First select - with content below it that might overlap
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(Text::new("Select 1 (dropdown should appear above card below):").weight(TextWeight::Medium))
                    .child({
                        let select1_value = self.select1_value.clone();
                        let select1_open = self.select1_open;
                        let select1_highlighted = self.select1_highlighted;

                        div().w(px(200.0)).child(
                            Select::new("select-1")
                                .options(vec![
                                    SelectOption::new("apple", "Apple"),
                                    SelectOption::new("banana", "Banana"),
                                    SelectOption::new("cherry", "Cherry"),
                                    SelectOption::new("grape", "Grape"),
                                    SelectOption::new("orange", "Orange"),
                                    SelectOption::new("mango", "Mango"),
                                    SelectOption::new("peach", "Peach"),
                                ])
                                .selected(select1_value.unwrap_or("apple".into()))
                                .placeholder("Choose a fruit...")
                                .label("Fruit Selection")
                                .is_open(select1_open)
                                .highlighted_index(select1_highlighted)
                                .on_change({
                                    let entity = entity.clone();
                                    move |value, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.select1_value = Some(value.clone());
                                            this.select1_open = false;
                                            this.select1_highlighted = None;
                                        });
                                    }
                                })
                                .on_toggle({
                                    let entity = entity.clone();
                                    move |open, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.select1_open = open;
                                            // Close other select if opening this one
                                            if open {
                                                this.select2_open = false;
                                            }
                                        });
                                    }
                                })
                                .on_highlight({
                                    let entity = entity.clone();
                                    move |idx, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.select1_highlighted = idx;
                                        });
                                    }
                                }),
                        )
                    }),
            )
            // Card that should be BELOW the dropdown
            .child(
                div()
                    .bg(rgba(0xff5555ff)) // Bright red to make it obvious
                    .rounded_md()
                    .p_4()
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("This is a card BELOW Select 1").weight(TextWeight::Bold))
                            .child(Text::new("The dropdown should appear ON TOP of this card"))
                            .child(Text::new("If you see this text THROUGH the dropdown, the background is transparent"))
                            .child(Text::new("If the dropdown appears UNDER this card, there's a z-index issue")),
                    ),
            )
            // Second select for comparison
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .mt_8()
                    .child(Text::new("Select 2 (another select for comparison):").weight(TextWeight::Medium))
                    .child({
                        let select2_value = self.select2_value.clone();
                        let select2_open = self.select2_open;
                        let select2_highlighted = self.select2_highlighted;

                        div().w(px(200.0)).child(
                            Select::new("select-2")
                                .options(vec![
                                    SelectOption::new("red", "Red"),
                                    SelectOption::new("green", "Green"),
                                    SelectOption::new("blue", "Blue"),
                                    SelectOption::new("yellow", "Yellow"),
                                ])
                                .selected(select2_value.unwrap_or("red".into()))
                                .placeholder("Choose a color...")
                                .label("Color Selection")
                                .is_open(select2_open)
                                .highlighted_index(select2_highlighted)
                                .on_change({
                                    let entity = entity.clone();
                                    move |value, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.select2_value = Some(value.clone());
                                            this.select2_open = false;
                                            this.select2_highlighted = None;
                                        });
                                    }
                                })
                                .on_toggle({
                                    let entity = entity.clone();
                                    move |open, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.select2_open = open;
                                            // Close other select if opening this one
                                            if open {
                                                this.select1_open = false;
                                            }
                                        });
                                    }
                                })
                                .on_highlight({
                                    let entity = entity.clone();
                                    move |idx, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.select2_highlighted = idx;
                                        });
                                    }
                                }),
                        )
                    }),
            )
            // ButtonSet demos
            .child(Divider::new().build())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(Heading::h2("ButtonSet Component"))
                    .child(Text::new("A group of mutually exclusive buttons (segmented control)"))
                    // View mode example
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("View Mode:").weight(TextWeight::Medium))
                            .child({
                                let view_mode = self.view_mode.clone();
                                ButtonSet::new("view-mode")
                                    .options(vec![
                                        ButtonSetOption::new("list", "List").icon("📋"),
                                        ButtonSetOption::new("grid", "Grid").icon("📱"),
                                        ButtonSetOption::new("table", "Table").icon("📊"),
                                    ])
                                    .selected(view_mode)
                                    .size(ButtonSetSize::Md)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |value, _window, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.view_mode = value.clone();
                                            });
                                        }
                                    })
                            })
                            .child(Text::new(format!("Selected: {}", self.view_mode)).size(TextSize::Sm).muted(true)),
                    )
                    // Alignment example with different sizes
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Alignment (Small size):").weight(TextWeight::Medium))
                            .child({
                                let alignment = self.alignment.clone();
                                ButtonSet::new("alignment")
                                    .options(vec![
                                        ButtonSetOption::new("left", "Left"),
                                        ButtonSetOption::new("center", "Center"),
                                        ButtonSetOption::new("right", "Right"),
                                        ButtonSetOption::new("justify", "Justify"),
                                    ])
                                    .selected(alignment)
                                    .size(ButtonSetSize::Sm)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |value, _window, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.alignment = value.clone();
                                            });
                                        }
                                    })
                            })
                            .child(Text::new(format!("Selected: {}", self.alignment)).size(TextSize::Sm).muted(true)),
                    )
                    // Large size example
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Large ButtonSet:").weight(TextWeight::Medium))
                            .child(
                                ButtonSet::new("large-demo")
                                    .options(vec![
                                        ButtonSetOption::new("on", "ON"),
                                        ButtonSetOption::new("off", "OFF"),
                                    ])
                                    .selected("on")
                                    .size(ButtonSetSize::Lg),
                            ),
                    )
                    // Disabled option example
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("With disabled option:").weight(TextWeight::Medium))
                            .child(
                                ButtonSet::new("disabled-demo")
                                    .options(vec![
                                        ButtonSetOption::new("available", "Available"),
                                        ButtonSetOption::new("soon", "Coming Soon").disabled(true),
                                        ButtonSetOption::new("premium", "Premium"),
                                    ])
                                    .selected("available")
                                    .size(ButtonSetSize::Md),
                            ),
                    ),
            )
            // Bottom content
            .child(
                div()
                    .mt_8()
                    .p_4()
                    .bg(theme.surface)
                    .rounded_md()
                    .child(Text::new("More content at the bottom to test scrolling interaction")),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Select Debug")
            .size(800.0, 700.0)
            .scrollable(true)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(SelectDebug::new),
    );
}
