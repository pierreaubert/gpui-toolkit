//! Text component
//!
//! Typography and text styling utilities.

use crate::theme::{Theme, ThemeExt};
use gpui::prelude::*;
use gpui::{Component, *};

/// Text size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextSize {
    /// Extra small
    Xs,
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
    /// Extra large
    Xl,
    /// 2X large
    Xxl,
}

/// Text weight
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextWeight {
    /// Light
    Light,
    /// Normal (default)
    #[default]
    Normal,
    /// Medium
    Medium,
    /// Semibold
    Semibold,
    /// Bold
    Bold,
}

impl TextWeight {
    fn to_font_weight(&self) -> FontWeight {
        match self {
            TextWeight::Light => FontWeight::LIGHT,
            TextWeight::Normal => FontWeight::NORMAL,
            TextWeight::Medium => FontWeight::MEDIUM,
            TextWeight::Semibold => FontWeight::SEMIBOLD,
            TextWeight::Bold => FontWeight::BOLD,
        }
    }
}

/// A styled text component
#[derive(IntoElement)]
pub struct Text {
    content: SharedString,
    size: TextSize,
    weight: TextWeight,
    color: Option<Rgba>,
    muted: bool,
    truncate: bool,
    theme: Option<Theme>,
}

impl Text {
    /// Create new text
    pub fn new(content: impl Into<SharedString>) -> Self {
        Self {
            content: content.into(),
            size: TextSize::default(),
            weight: TextWeight::default(),
            color: None,
            muted: false,
            truncate: false,
            theme: None,
        }
    }

    /// Set theme
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set size
    pub fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }

    /// Set weight
    pub fn weight(mut self, weight: TextWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set custom color
    pub fn color(mut self, color: Rgba) -> Self {
        self.color = Some(color);
        self
    }

    /// Make text muted (secondary color)
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    /// Truncate with ellipsis
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    /// Build into element with theme from App context
    pub fn build_with_cx(self, cx: &App) -> Div {
        let theme = self.theme.clone().unwrap_or_else(|| cx.theme());
        self.build_with_theme(&theme)
    }

    /// Build into element with explicit theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let text_color = if let Some(color) = self.color {
            color
        } else if self.muted {
            theme.text_muted
        } else {
            theme.text_secondary
        };

        let mut text = div()
            .text_color(text_color)
            .font_weight(self.weight.to_font_weight());

        // Apply size
        text = match self.size {
            TextSize::Xs => text.text_xs(),
            TextSize::Sm => text.text_sm(),
            TextSize::Md => text.text_sm(),
            TextSize::Lg => text.text_lg(),
            TextSize::Xl => text.text_xl(),
            TextSize::Xxl => text.text_2xl(),
        };

        if self.truncate {
            text = text.overflow_hidden().whitespace_nowrap();
        }

        text.child(self.content)
    }

    /// Build into element (uses default dark theme colors for backwards compatibility)
    pub fn build(self) -> Div {
        let theme = self.theme.clone().unwrap_or_else(Theme::dark);
        self.build_with_theme(&theme)
    }
}

impl RenderOnce for Text {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.clone().unwrap_or_else(|| cx.theme());
        self.build_with_theme(&theme)
    }
}

/// A heading component
#[derive(IntoElement)]
pub struct Heading {
    content: SharedString,
    level: u8,
    theme: Option<Theme>,
}

impl Heading {
    /// Create a new heading
    pub fn new(content: impl Into<SharedString>) -> Self {
        Self {
            content: content.into(),
            level: 1,
            theme: None,
        }
    }

    /// Set theme
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set heading level (1-6)
    pub fn level(mut self, level: u8) -> Self {
        self.level = level.clamp(1, 6);
        self
    }

    /// Create h1
    pub fn h1(content: impl Into<SharedString>) -> Self {
        Self::new(content).level(1)
    }

    /// Create h2
    pub fn h2(content: impl Into<SharedString>) -> Self {
        Self::new(content).level(2)
    }

    /// Create h3
    pub fn h3(content: impl Into<SharedString>) -> Self {
        Self::new(content).level(3)
    }

    /// Create h4
    pub fn h4(content: impl Into<SharedString>) -> Self {
        Self::new(content).level(4)
    }

    /// Build into element with explicit theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let mut heading = div()
            .font_weight(FontWeight::BOLD)
            .text_color(theme.text_primary);

        heading = match self.level {
            1 => heading.text_2xl(),
            2 => heading.text_xl(),
            3 => heading.text_lg(),
            4 => heading,
            5 => heading.text_sm(),
            _ => heading.text_xs(),
        };

        heading.child(self.content)
    }

    /// Build into element (uses default dark theme colors for backwards compatibility)
    pub fn build(self) -> Div {
        let theme = self.theme.clone().unwrap_or_else(Theme::dark);
        self.build_with_theme(&theme)
    }
}

impl RenderOnce for Heading {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.clone().unwrap_or_else(|| cx.theme());
        self.build_with_theme(&theme)
    }
}

/// A code/monospace text component
#[derive(IntoElement)]
pub struct Code {
    content: SharedString,
    inline: bool,
    theme: Option<Theme>,
}

impl Code {
    /// Create inline code
    pub fn new(content: impl Into<SharedString>) -> Self {
        Self {
            content: content.into(),
            inline: true,
            theme: None,
        }
    }

    /// Create code block
    pub fn block(content: impl Into<SharedString>) -> Self {
        Self {
            content: content.into(),
            inline: false,
            theme: None,
        }
    }

    /// Set theme
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Build into element with explicit theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        // Code uses a slightly different color from accent
        let code_text = match theme.variant {
            crate::theme::ThemeVariant::Light => rgb(0xc7254e),
            // Dark, Midnight, Forest, BlackAndWhite all use dark-style colors
            crate::theme::ThemeVariant::Dark
            | crate::theme::ThemeVariant::Midnight
            | crate::theme::ThemeVariant::Forest
            | crate::theme::ThemeVariant::BlackAndWhite => rgb(0xe06c75),
        };

        if self.inline {
            div()
                .px_1()
                .py(px(1.0))
                .bg(theme.surface)
                .rounded(px(3.0))
                .text_xs()
                .text_color(code_text)
                .child(self.content)
        } else {
            div()
                .p_3()
                .bg(theme.muted)
                .rounded_md()
                .text_sm()
                .text_color(theme.text_secondary)
                .overflow_hidden()
                .child(self.content)
        }
    }

    /// Build into element (uses default dark theme colors for backwards compatibility)
    pub fn build(self) -> Div {
        let theme = self.theme.clone().unwrap_or_else(Theme::dark);
        self.build_with_theme(&theme)
    }
}

impl RenderOnce for Code {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.clone().unwrap_or_else(|| cx.theme());
        self.build_with_theme(&theme)
    }
}

/// A link component
pub struct Link {
    id: ElementId,
    content: SharedString,
    href: Option<SharedString>,
    external: bool,
    on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    theme: Option<Theme>,
}

impl Link {
    /// Create a new link
    pub fn new(id: impl Into<ElementId>, content: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            href: None,
            external: false,
            on_click: None,
            theme: None,
        }
    }

    /// Set theme
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set href
    pub fn href(mut self, href: impl Into<SharedString>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Mark as external link
    pub fn external(mut self, external: bool) -> Self {
        self.external = external;
        self
    }

    /// Set click handler
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Build into element with explicit theme
    pub fn build_with_theme(self, theme: &Theme) -> Stateful<Div> {
        let accent = theme.accent;
        let accent_hover = theme.accent_hover;

        let mut link = div()
            .id(self.id)
            .text_color(accent)
            .cursor_pointer()
            .hover(move |s| s.text_color(accent_hover));

        if let Some(handler) = self.on_click {
            link = link.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                handler(window, cx);
            });
        }

        link = link.child(self.content);

        if self.external {
            link = link.child(div().text_xs().ml_1().child("↗"));
        }

        link
    }

    /// Build into element (uses default dark theme colors for backwards compatibility)
    pub fn build(self) -> Stateful<Div> {
        let theme = self.theme.clone().unwrap_or_else(Theme::dark);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for Link {
    type Element = Component<Self>;

    fn into_element(self) -> Self::Element {
        Component::new(self)
    }
}

impl RenderOnce for Link {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.clone().unwrap_or_else(|| cx.theme());
        self.build_with_theme(&theme)
    }
}
