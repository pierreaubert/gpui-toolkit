use super::misc::code_text_color;
use crate::theme::{Theme, ThemeExt};
use gpui::prelude::{IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, SharedString, Window, div, px};

/// A code/monospace text component
#[derive(IntoElement)]
pub struct Code {
    pub(super) content: SharedString,
    pub(super) inline: bool,
    pub(super) theme: Option<Theme>,
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
        let code_text = code_text_color(theme);

        if self.inline {
            div()
                .font_family(theme.font_family.clone())
                .px_1()
                .py(px(1.0))
                .bg(theme.surface)
                .rounded(px(3.0))
                .text_xs()
                .text_color(code_text)
                .child(self.content)
        } else {
            div()
                .font_family(theme.font_family.clone())
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
        let mut this = self;
        let theme = this.theme.take().unwrap_or_else(Theme::dark);
        this.build_with_theme(&theme)
    }
}

impl RenderOnce for Code {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut this = self;
        let global_theme = cx.theme();
        match this.theme.take() {
            Some(ref theme) => this.build_with_theme(theme),
            None => this.build_with_theme(global_theme.as_ref()),
        }
    }
}
