use crate::theme::{Theme, ThemeExt};
use gpui::prelude::{IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, FontWeight, SharedString, Window, div};

/// A heading component
#[derive(IntoElement)]
pub struct Heading {
    pub(super) content: SharedString,
    pub(super) level: u8,
    pub(super) theme: Option<Theme>,
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
            .font_family(theme.font_family.clone())
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
        let mut this = self;
        let theme = this.theme.take().unwrap_or_else(Theme::dark);
        this.build_with_theme(&theme)
    }
}

impl RenderOnce for Heading {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut this = self;
        let global_theme = cx.theme();
        match this.theme.take() {
            Some(ref theme) => this.build_with_theme(theme),
            None => this.build_with_theme(global_theme.as_ref()),
        }
    }
}
