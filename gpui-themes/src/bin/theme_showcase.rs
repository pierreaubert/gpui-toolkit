//! Theme Showcase - Displays multiple theme variations
//!
//! This binary provides a visual showcase of different theme presets.

use gpui::prelude::*;
use gpui::*;
use gpui_themes::{ComponentShowcase, EditorTheme};
use gpui_ui_kit::{
    Button, ButtonSize, ButtonVariant, HStack, MiniApp, MiniAppConfig, StackSpacing, Text,
    TextSize, TextWeight, VStack,
};

/// Available theme presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThemePreset {
    Dark,
    Light,
    HighContrast,
    Nord,
    Dracula,
}

impl ThemePreset {
    fn all() -> &'static [ThemePreset] {
        &[
            ThemePreset::Dark,
            ThemePreset::Light,
            ThemePreset::HighContrast,
            ThemePreset::Nord,
            ThemePreset::Dracula,
        ]
    }

    fn name(&self) -> &'static str {
        match self {
            ThemePreset::Dark => "Dark",
            ThemePreset::Light => "Light",
            ThemePreset::HighContrast => "High Contrast",
            ThemePreset::Nord => "Nord",
            ThemePreset::Dracula => "Dracula",
        }
    }

    fn to_theme(&self) -> EditorTheme {
        match self {
            ThemePreset::Dark => EditorTheme::dark(),
            ThemePreset::Light => EditorTheme::light(),
            ThemePreset::HighContrast => EditorTheme::high_contrast(),
            ThemePreset::Nord => EditorTheme::nord(),
            ThemePreset::Dracula => EditorTheme::dracula(),
        }
    }
}

/// Theme showcase application
struct ThemeShowcase {
    current_theme: ThemePreset,
    showcase: Entity<ComponentShowcase>,
}

impl ThemeShowcase {
    fn new(cx: &mut Context<Self>) -> Self {
        let theme = ThemePreset::Dark.to_theme();
        let showcase = cx.new(|_| ComponentShowcase::new(theme));

        Self {
            current_theme: ThemePreset::Dark,
            showcase,
        }
    }

    fn set_theme(&mut self, preset: ThemePreset, cx: &mut Context<Self>) {
        self.current_theme = preset;
        let theme = preset.to_theme();
        self.showcase.update(cx, |showcase, _| {
            showcase.set_theme(theme);
        });
        cx.notify();
    }

    fn render_theme_selector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.current_theme;
        let theme = current.to_theme();

        div()
            .w_full()
            .px_6()
            .py_4()
            .bg(theme.background_secondary.to_rgba())
            .border_b_1()
            .border_color(theme.border.to_rgba())
            .child(
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        Text::new("Theme Showcase")
                            .size(TextSize::Xl)
                            .weight(TextWeight::Bold)
                            .color(theme.text_primary.to_rgba()),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(
                                Text::new("Select Theme:")
                                    .size(TextSize::Md)
                                    .color(theme.text_secondary.to_rgba()),
                            )
                            .children(ThemePreset::all().iter().map(|preset| {
                                let is_selected = *preset == current;
                                Button::new(
                                    SharedString::from(format!("theme-{:?}", preset)),
                                    preset.name(),
                                )
                                .variant(if is_selected {
                                    ButtonVariant::Primary
                                } else {
                                    ButtonVariant::Ghost
                                })
                                .size(ButtonSize::Md)
                                .theme(theme.to_button_theme())
                                .build()
                                .on_click(cx.listener({
                                    let preset = *preset;
                                    move |this, _: &ClickEvent, _window, cx| {
                                        this.set_theme(preset, cx);
                                    }
                                }))
                            }))
                            .build(),
                    )
                    .build(),
            )
    }
}

impl Render for ThemeShowcase {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.current_theme.to_theme();

        div()
            .size_full()
            .bg(theme.background.to_rgba())
            .flex()
            .flex_col()
            .child(self.render_theme_selector(cx))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.showcase.clone()),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("GPUI Theme Showcase")
            .size(1400.0, 900.0)
            .scrollable(true)
            .with_theme(false), // Showcase manages its own theme
        |cx| cx.new(ThemeShowcase::new),
    );
}
