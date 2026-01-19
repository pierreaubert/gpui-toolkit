//! Card component for content containers
//!
//! A flexible card component with optional header, content, and footer sections.
//!
//! # Composition Patterns
//!
//! Cards support two composition patterns:
//!
//! ## Static content (simple)
//! ```ignore
//! Card::new()
//!     .header(div().child("Title"))
//!     .content(div().child("Body"))
//! ```
//!
//! ## Dynamic content with theme access
//! ```ignore
//! Card::new()
//!     .header_with(|theme| {
//!         div().text_color(theme.accent).child("Themed Title")
//!     })
//!     .content_with(|theme| {
//!         div().bg(theme.muted).child("Themed Body")
//!     })
//! ```

use crate::theme::{Theme, ThemeExt};
use gpui::prelude::*;
use gpui::*;

/// Factory function type for creating elements with theme access
pub type SlotFactory = Box<dyn FnOnce(&Theme) -> AnyElement>;

/// A card container with optional sections
#[derive(IntoElement)]
pub struct Card {
    header: Option<AnyElement>,
    header_factory: Option<SlotFactory>,
    content: Option<AnyElement>,
    content_factory: Option<SlotFactory>,
    footer: Option<AnyElement>,
    footer_factory: Option<SlotFactory>,
    /// Custom background color (overrides theme)
    background: Option<Rgba>,
    /// Custom header background color (overrides theme)
    header_background: Option<Rgba>,
    /// Custom border color (overrides theme)
    border_color: Option<Rgba>,
    /// Additional styling
    extra_classes: Vec<Box<dyn FnOnce(Div) -> Div>>,
}

impl Card {
    /// Create a new empty card
    pub fn new() -> Self {
        Self {
            header: None,
            header_factory: None,
            content: None,
            content_factory: None,
            footer: None,
            footer_factory: None,
            background: None,
            header_background: None,
            border_color: None,
            extra_classes: Vec::new(),
        }
    }

    /// Set the card header with a static element
    pub fn header(mut self, element: impl IntoElement) -> Self {
        self.header = Some(element.into_any_element());
        self
    }

    /// Set the card header with a factory function that receives the theme
    ///
    /// This allows dynamic content creation with access to theme colors.
    ///
    /// # Example
    /// ```ignore
    /// Card::new().header_with(|theme| {
    ///     div()
    ///         .text_color(theme.accent)
    ///         .font_weight(FontWeight::BOLD)
    ///         .child("Themed Header")
    ///         .into_any_element()
    /// })
    /// ```
    pub fn header_with(mut self, factory: impl FnOnce(&Theme) -> AnyElement + 'static) -> Self {
        self.header_factory = Some(Box::new(factory));
        self
    }

    /// Set the card content with a static element
    pub fn content(mut self, element: impl IntoElement) -> Self {
        self.content = Some(element.into_any_element());
        self
    }

    /// Set the card content with a factory function that receives the theme
    ///
    /// # Example
    /// ```ignore
    /// Card::new().content_with(|theme| {
    ///     div()
    ///         .bg(theme.muted)
    ///         .p_4()
    ///         .child("Themed content with background")
    ///         .into_any_element()
    /// })
    /// ```
    pub fn content_with(mut self, factory: impl FnOnce(&Theme) -> AnyElement + 'static) -> Self {
        self.content_factory = Some(Box::new(factory));
        self
    }

    /// Set the card footer with a static element
    pub fn footer(mut self, element: impl IntoElement) -> Self {
        self.footer = Some(element.into_any_element());
        self
    }

    /// Set the card footer with a factory function that receives the theme
    ///
    /// # Example
    /// ```ignore
    /// Card::new().footer_with(|theme| {
    ///     div()
    ///         .text_color(theme.text_muted)
    ///         .text_sm()
    ///         .child("Footer with theme colors")
    ///         .into_any_element()
    /// })
    /// ```
    pub fn footer_with(mut self, factory: impl FnOnce(&Theme) -> AnyElement + 'static) -> Self {
        self.footer_factory = Some(Box::new(factory));
        self
    }

    /// Add custom styling to the card container
    pub fn style(mut self, f: impl FnOnce(Div) -> Div + 'static) -> Self {
        self.extra_classes.push(Box::new(f));
        self
    }

    /// Set custom background color (overrides theme)
    pub fn background(mut self, color: Rgba) -> Self {
        self.background = Some(color);
        self
    }

    /// Set custom header background color (overrides theme)
    pub fn header_background(mut self, color: Rgba) -> Self {
        self.header_background = Some(color);
        self
    }

    /// Set custom border color (overrides theme)
    pub fn border(mut self, color: Rgba) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Build the card into an element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let bg_color = self.background.unwrap_or(theme.surface);
        let border_color = self.border_color.unwrap_or(theme.border);
        let header_bg = self.header_background.unwrap_or(theme.muted);

        let mut card = div()
            .flex()
            .flex_col()
            .bg(bg_color)
            .text_color(theme.text_primary)
            .border_1()
            .border_color(border_color)
            .rounded_lg()
            .shadow_md()
            .overflow_hidden();

        // Apply extra classes
        for class_fn in self.extra_classes {
            card = class_fn(card);
        }

        // Header section - factory takes precedence over static element
        let header_element = self.header_factory.map(|f| f(theme)).or(self.header);
        if let Some(header) = header_element {
            card = card.child(
                div()
                    .px_4()
                    .py_3()
                    .bg(header_bg)
                    .text_color(theme.text_primary)
                    .border_b_1()
                    .border_color(border_color)
                    .child(header),
            );
        }

        // Content section - factory takes precedence over static element
        let content_element = self.content_factory.map(|f| f(theme)).or(self.content);
        if let Some(content) = content_element {
            card = card.child(
                div()
                    .px_4()
                    .py_4()
                    .text_color(theme.text_secondary)
                    .child(content),
            );
        }

        // Footer section - factory takes precedence over static element
        let footer_element = self.footer_factory.map(|f| f(theme)).or(self.footer);
        if let Some(footer) = footer_element {
            card = card.child(
                div()
                    .px_4()
                    .py_3()
                    .bg(header_bg)
                    .text_color(theme.text_muted)
                    .border_t_1()
                    .border_color(border_color)
                    .child(footer),
            );
        }

        card
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Card {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}
