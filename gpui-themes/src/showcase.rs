//! Component showcase for theme preview
//!
//! Displays all UI kit components with the current theme applied.

use crate::theme::EditorTheme;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::{
    Alert, AlertVariant, Badge, BadgeVariant, BreadcrumbItem, Breadcrumbs, Button, ButtonSize,
    ButtonVariant, Card, Code, HStack, Heading, StackSpacing, Text, TextSize, TextWeight, VStack,
};

/// Component showcase that displays all UI kit components
pub struct ComponentShowcase {
    theme: EditorTheme,
}

impl ComponentShowcase {
    pub fn new(theme: EditorTheme) -> Self {
        Self { theme }
    }

    /// Update the theme
    pub fn set_theme(&mut self, theme: EditorTheme) {
        self.theme = theme;
    }

    /// Render section header
    fn section_header(&self, title: &'static str) -> impl IntoElement {
        div()
            .w_full()
            .pb_2()
            .mb_3()
            .border_b_1()
            .border_color(self.theme.border.to_rgba())
            .child(
                Text::new(title)
                    .size(TextSize::Lg)
                    .weight(TextWeight::Bold)
                    .color(self.theme.text_primary.to_rgba()),
            )
    }

    /// Render buttons section
    fn render_buttons(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let button_theme = self.theme.to_button_theme();

        VStack::new()
            .spacing(StackSpacing::Md)
            .child(self.section_header("Buttons"))
            // Variants
            .child(
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(
                        Button::new("btn-primary", "Primary")
                            .variant(ButtonVariant::Primary)
                            .size(ButtonSize::Md)
                            .theme(button_theme.clone())
                            .build(),
                    )
                    .child(
                        Button::new("btn-secondary", "Secondary")
                            .variant(ButtonVariant::Secondary)
                            .size(ButtonSize::Md)
                            .theme(button_theme.clone())
                            .build(),
                    )
                    .child(
                        Button::new("btn-ghost", "Ghost")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Md)
                            .theme(button_theme.clone())
                            .build(),
                    )
                    .child(
                        Button::new("btn-destructive", "Destructive")
                            .variant(ButtonVariant::Destructive)
                            .size(ButtonSize::Md)
                            .theme(button_theme.clone())
                            .build(),
                    )
                    .build(),
            )
            // Sizes
            .child(
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(
                        Button::new("btn-xs", "XS")
                            .variant(ButtonVariant::Primary)
                            .size(ButtonSize::Xs)
                            .theme(button_theme.clone())
                            .build(),
                    )
                    .child(
                        Button::new("btn-sm", "Small")
                            .variant(ButtonVariant::Primary)
                            .size(ButtonSize::Sm)
                            .theme(button_theme.clone())
                            .build(),
                    )
                    .child(
                        Button::new("btn-md", "Medium")
                            .variant(ButtonVariant::Primary)
                            .size(ButtonSize::Md)
                            .theme(button_theme.clone())
                            .build(),
                    )
                    .child(
                        Button::new("btn-lg", "Large")
                            .variant(ButtonVariant::Primary)
                            .size(ButtonSize::Lg)
                            .theme(button_theme)
                            .build(),
                    )
                    .build(),
            )
            // Disabled
            .child(
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(
                        Button::new("btn-disabled", "Disabled")
                            .variant(ButtonVariant::Primary)
                            .disabled(true)
                            .build(),
                    )
                    .build(),
            )
            .build()
    }

    /// Render text section
    fn render_text(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        VStack::new()
            .spacing(StackSpacing::Md)
            .child(self.section_header("Typography"))
            .child(Heading::new("Heading 1").level(1))
            .child(Heading::new("Heading 2").level(2))
            .child(Heading::new("Heading 3").level(3))
            .child(
                Text::new("Primary text - The quick brown fox jumps over the lazy dog.")
                    .color(self.theme.text_primary.to_rgba()),
            )
            .child(
                Text::new("Secondary text - Lorem ipsum dolor sit amet.")
                    .color(self.theme.text_secondary.to_rgba()),
            )
            .child(
                Text::new("Muted text - Additional information.")
                    .color(self.theme.text_muted.to_rgba()),
            )
            .child(Text::new("Disabled text").color(self.theme.text_disabled.to_rgba()))
            .child(Code::new("let theme = EditorTheme::dark();"))
            .build()
    }

    /// Render badges section
    fn render_badges(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        VStack::new()
            .spacing(StackSpacing::Md)
            .child(self.section_header("Badges"))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Badge::new("Default"))
                    .child(Badge::new("Primary").variant(BadgeVariant::Primary))
                    .child(Badge::new("Success").variant(BadgeVariant::Success))
                    .child(Badge::new("Warning").variant(BadgeVariant::Warning))
                    .child(Badge::new("Error").variant(BadgeVariant::Error))
                    .build(),
            )
            .build()
    }

    /// Render alerts section
    fn render_alerts(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        VStack::new()
            .spacing(StackSpacing::Md)
            .child(self.section_header("Alerts"))
            .child(
                Alert::new("alert-info", "This is an informational message.")
                    .variant(AlertVariant::Info),
            )
            .child(
                Alert::new("alert-success", "Operation completed successfully!")
                    .variant(AlertVariant::Success),
            )
            .child(
                Alert::new("alert-warning", "Please review before proceeding.")
                    .variant(AlertVariant::Warning),
            )
            .child(
                Alert::new("alert-error", "An error occurred. Please try again.")
                    .variant(AlertVariant::Error),
            )
            .build()
    }

    /// Render cards section
    fn render_cards(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        VStack::new()
            .spacing(StackSpacing::Md)
            .child(self.section_header("Cards"))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        Card::new()
                            .content(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .child(Text::new("Card Title").weight(TextWeight::Bold))
                                    .child(Text::new(
                                        "Card content goes here. This is a sample card.",
                                    ))
                                    .build(),
                            )
                            .style(|card| card.w(px(200.0)).p_4()),
                    )
                    .child(
                        Card::new()
                            .content(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .child(Text::new("Another Card").weight(TextWeight::Bold))
                                    .child(Text::new("With different content."))
                                    .child(
                                        Button::new("card-btn", "Action")
                                            .variant(ButtonVariant::Primary)
                                            .size(ButtonSize::Sm)
                                            .build(),
                                    )
                                    .build(),
                            )
                            .style(|card| card.w(px(200.0)).p_4()),
                    )
                    .build(),
            )
            .build()
    }

    /// Render breadcrumbs section
    fn render_breadcrumbs(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        VStack::new()
            .spacing(StackSpacing::Md)
            .child(self.section_header("Breadcrumbs"))
            .child(Breadcrumbs::new().items(vec![
                BreadcrumbItem::new("home", "Home").href("#"),
                BreadcrumbItem::new("library", "Library").href("#"),
                BreadcrumbItem::new("settings", "Settings").href("#"),
                BreadcrumbItem::new("current", "Current Page"),
            ]))
            .build()
    }

    /// Render color swatches for theme colors
    fn render_color_swatches(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;

        VStack::new()
            .spacing(StackSpacing::Md)
            .child(self.section_header("Theme Colors"))
            // Base colors
            .child(
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(self.color_swatch("Background", theme.background))
                    .child(self.color_swatch("Surface", theme.surface))
                    .child(self.color_swatch("Surface Hover", theme.surface_hover))
                    .child(self.color_swatch("Surface Selected", theme.surface_selected))
                    .build(),
            )
            // Accent colors
            .child(
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(self.color_swatch("Accent", theme.accent))
                    .child(self.color_swatch("Accent Hover", theme.accent_hover))
                    .child(self.color_swatch("Accent Muted", theme.accent_muted))
                    .build(),
            )
            // Semantic colors
            .child(
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(self.color_swatch("Success", theme.success))
                    .child(self.color_swatch("Warning", theme.warning))
                    .child(self.color_swatch("Error", theme.error))
                    .child(self.color_swatch("Info", theme.info))
                    .build(),
            )
            // Text colors
            .child(
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(self.color_swatch("Text Primary", theme.text_primary))
                    .child(self.color_swatch("Text Secondary", theme.text_secondary))
                    .child(self.color_swatch("Text Muted", theme.text_muted))
                    .child(self.color_swatch("Text Disabled", theme.text_disabled))
                    .build(),
            )
            .build()
    }

    fn color_swatch(&self, name: &'static str, color: crate::theme::Color) -> impl IntoElement {
        VStack::new()
            .spacing(StackSpacing::Xs)
            .child(
                div()
                    .w(px(60.0))
                    .h(px(40.0))
                    .rounded_md()
                    .bg(color.to_rgba())
                    .border_1()
                    .border_color(self.theme.border.to_rgba()),
            )
            .child(
                Text::new(name)
                    .size(TextSize::Xs)
                    .color(self.theme.text_secondary.to_rgba()),
            )
            .build()
    }
}

impl Render for ComponentShowcase {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = self.theme.background.to_rgba();
        let surface = self.theme.surface.to_rgba();

        div().size_full().bg(bg).p_4().child(
            div().max_w(px(1200.0)).mx_auto().child(
                VStack::new()
                    .spacing(StackSpacing::Xl)
                    // Color swatches first
                    .child(
                        div()
                            .p_4()
                            .bg(surface)
                            .rounded_lg()
                            .border_1()
                            .border_color(self.theme.border.to_rgba())
                            .child(self.render_color_swatches(cx)),
                    )
                    // Two column layout for components
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Lg)
                            // Left column
                            .child(
                                div().flex_1().child(
                                    VStack::new()
                                        .spacing(StackSpacing::Lg)
                                        .child(
                                            div()
                                                .p_4()
                                                .bg(surface)
                                                .rounded_lg()
                                                .border_1()
                                                .border_color(self.theme.border.to_rgba())
                                                .child(self.render_buttons(cx)),
                                        )
                                        .child(
                                            div()
                                                .p_4()
                                                .bg(surface)
                                                .rounded_lg()
                                                .border_1()
                                                .border_color(self.theme.border.to_rgba())
                                                .child(self.render_text(cx)),
                                        )
                                        .child(
                                            div()
                                                .p_4()
                                                .bg(surface)
                                                .rounded_lg()
                                                .border_1()
                                                .border_color(self.theme.border.to_rgba())
                                                .child(self.render_badges(cx)),
                                        )
                                        .child(
                                            div()
                                                .p_4()
                                                .bg(surface)
                                                .rounded_lg()
                                                .border_1()
                                                .border_color(self.theme.border.to_rgba())
                                                .child(self.render_breadcrumbs(cx)),
                                        )
                                        .build(),
                                ),
                            )
                            // Right column
                            .child(
                                div().flex_1().child(
                                    VStack::new()
                                        .spacing(StackSpacing::Lg)
                                        .child(
                                            div()
                                                .p_4()
                                                .bg(surface)
                                                .rounded_lg()
                                                .border_1()
                                                .border_color(self.theme.border.to_rgba())
                                                .child(self.render_alerts(cx)),
                                        )
                                        .child(
                                            div()
                                                .p_4()
                                                .bg(surface)
                                                .rounded_lg()
                                                .border_1()
                                                .border_color(self.theme.border.to_rgba())
                                                .child(self.render_cards(cx)),
                                        )
                                        .build(),
                                ),
                            )
                            .build(),
                    )
                    .build(),
            ),
        )
    }
}
