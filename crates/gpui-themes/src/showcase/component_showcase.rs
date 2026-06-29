use crate::theme::EditorTheme;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::{
    Alert, AlertVariant, Badge, BadgeVariant, BreadcrumbItem, Breadcrumbs, Button, ButtonSize,
    ButtonTheme, ButtonVariant, Card, Code, EmptyState, HStack, Heading, KeyboardShortcutLabel,
    KeyboardShortcutSize, SearchBar, SearchBarSize, StackSpacing, StatusBar, StatusBarPosition,
    Text, TextSize, TextWeight, VStack,
};
use std::sync::Arc;

/// Component showcase that displays all UI kit components
pub struct ComponentShowcase {
    pub(super) theme: Arc<EditorTheme>,
    button_theme: ButtonTheme,
}

impl ComponentShowcase {
    pub fn new(theme: Arc<EditorTheme>) -> Self {
        let button_theme = theme.to_button_theme();
        Self {
            theme,
            button_theme,
        }
    }

    /// Update the theme
    pub fn set_theme(&mut self, theme: Arc<EditorTheme>) {
        self.button_theme = theme.to_button_theme();
        self.theme = theme;
    }

    /// Render section header
    pub(super) fn section_header(&self, title: &'static str) -> impl IntoElement {
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
    pub(super) fn render_buttons(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let button_theme = self.button_theme.clone();

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
    pub(super) fn render_text(&self, _cx: &mut Context<Self>) -> impl IntoElement {
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
    pub(super) fn render_badges(&self, _cx: &mut Context<Self>) -> impl IntoElement {
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
    pub(super) fn render_alerts(&self, _cx: &mut Context<Self>) -> impl IntoElement {
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
    pub(super) fn render_cards(&self, _cx: &mut Context<Self>) -> impl IntoElement {
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
    pub(super) fn render_breadcrumbs(&self, _cx: &mut Context<Self>) -> impl IntoElement {
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
    pub(super) fn render_color_swatches(&self, _cx: &mut Context<Self>) -> impl IntoElement {
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

    pub(super) fn color_swatch(
        &self,
        name: &'static str,
        color: crate::theme::Color,
    ) -> impl IntoElement {
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

    /// Render new Tier 1 components section
    pub(super) fn render_new_components(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let border = self.theme.border.to_rgba();
        VStack::new()
            .spacing(StackSpacing::Md)
            .child(self.section_header("New Components"))
            // Keyboard shortcut labels
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Keyboard Shortcuts").weight(TextWeight::Semibold))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(KeyboardShortcutLabel::new("⌘+K"))
                            .child(KeyboardShortcutLabel::new("Ctrl+Shift+P"))
                            .child(
                                KeyboardShortcutLabel::new("Alt+F4").size(KeyboardShortcutSize::Lg),
                            )
                            .child(KeyboardShortcutLabel::new("⌘+S").size(KeyboardShortcutSize::Sm))
                            .build(),
                    )
                    .build(),
            )
            // Empty state
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Empty State").weight(TextWeight::Semibold))
                    .child(
                        div()
                            .border_1()
                            .border_color(border)
                            .rounded_lg()
                            .p_4()
                            .child(
                                EmptyState::new("No albums found")
                                    .description("Try adjusting your search filters")
                                    .icon("♪"),
                            ),
                    )
                    .build(),
            )
            // Search bar
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Search Bar").weight(TextWeight::Semibold))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(div().w(px(250.0)).child(
                                SearchBar::new("search-demo").placeholder("Search albums..."),
                            ))
                            .child(
                                div().w(px(200.0)).child(
                                    SearchBar::new("search-filled")
                                        .value("Beethoven")
                                        .size(SearchBarSize::Sm),
                                ),
                            )
                            .build(),
                    )
                    .build(),
            )
            // Status bar
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Status Bar").weight(TextWeight::Semibold))
                    .child(
                        div()
                            .border_1()
                            .border_color(border)
                            .rounded_lg()
                            .overflow_hidden()
                            .child(
                                StatusBar::new("status-demo")
                                    .position(StatusBarPosition::Bottom)
                                    .left(Text::new("Playing: Track 1").size(TextSize::Xs))
                                    .center(Text::new("00:00 / 03:45").size(TextSize::Xs))
                                    .right(Text::new("Vol: 80%").size(TextSize::Xs)),
                            ),
                    )
                    .build(),
            )
            .build()
    }
}

impl Render for ComponentShowcase {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = self.theme.background.to_rgba();
        let surface = self.theme.surface.to_rgba();
        let border = self.theme.border.to_rgba();

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
                            .border_color(border)
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
                                                .border_color(border)
                                                .child(self.render_buttons(cx)),
                                        )
                                        .child(
                                            div()
                                                .p_4()
                                                .bg(surface)
                                                .rounded_lg()
                                                .border_1()
                                                .border_color(border)
                                                .child(self.render_text(cx)),
                                        )
                                        .child(
                                            div()
                                                .p_4()
                                                .bg(surface)
                                                .rounded_lg()
                                                .border_1()
                                                .border_color(border)
                                                .child(self.render_badges(cx)),
                                        )
                                        .child(
                                            div()
                                                .p_4()
                                                .bg(surface)
                                                .rounded_lg()
                                                .border_1()
                                                .border_color(border)
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
                                                .border_color(border)
                                                .child(self.render_alerts(cx)),
                                        )
                                        .child(
                                            div()
                                                .p_4()
                                                .bg(surface)
                                                .rounded_lg()
                                                .border_1()
                                                .border_color(border)
                                                .child(self.render_cards(cx)),
                                        )
                                        .child(
                                            div()
                                                .p_4()
                                                .bg(surface)
                                                .rounded_lg()
                                                .border_1()
                                                .border_color(border)
                                                .child(self.render_new_components(cx)),
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
