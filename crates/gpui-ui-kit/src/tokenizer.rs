//! `Tokenizer` component
//!
//! A multi-value token editor rendering parent-owned values as removable
//! chips. Unlike the single-select `Combobox` and `Select`, the tokenizer
//! keeps every chosen value visible and fires `on_remove` with the token
//! index when a chip is dismissed with mouse or keyboard.
//!
//! # Usage
//!
//! ```ignore
//! Tokenizer::new("tags")
//!     .tokens(["drums", "bass"])
//!     .on_remove(|index, _window, _cx| { /* drop token */ })
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState, apply_native_accessibility,
};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, KeyDownEvent, MouseButton, Rgba, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// Theme colors for the tokenizer field.
#[derive(Debug, Clone, ComponentTheme)]
pub struct TokenizerTheme {
    /// Field background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Field border.
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Placeholder color for the empty field.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub placeholder: Rgba,
    /// Token chip background.
    #[theme(default = 0x007acc33, from = accent_muted)]
    pub token_bg: Rgba,
    /// Token chip label color.
    #[theme(default = 0xe6e6e6ff, from = text_primary)]
    pub token_text: Rgba,
}

/// Tokenizer size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TokenizerSize {
    /// Compact field.
    Sm,
    /// Standard field (default).
    #[default]
    Md,
    /// Large field.
    Lg,
}

impl From<crate::ComponentSize> for TokenizerSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Multi-value token editor with removable chips.
///
/// Tokens are parent-owned; the component renders them and reports
/// removals. Text entry for new tokens stays with the parent control.
#[derive(IntoElement)]
pub struct Tokenizer {
    id: ElementId,
    tokens: Vec<SharedString>,
    placeholder: SharedString,
    size: TokenizerSize,
    disabled: bool,
    on_remove: Option<Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    theme: Option<TokenizerTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Tokenizer {
    /// Create an empty tokenizer.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let tokens = Tokenizer::new("tags");
    /// ```
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            tokens: Vec::new(),
            placeholder: SharedString::from("Add items..."),
            size: TokenizerSize::default(),
            disabled: false,
            on_remove: None,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Replace the token list.
    pub fn tokens(mut self, tokens: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.tokens = tokens.into_iter().map(Into::into).collect();
        self
    }

    /// Append one token.
    pub fn token(mut self, token: impl Into<SharedString>) -> Self {
        self.tokens.push(token.into());
        self
    }

    /// Set the placeholder shown when no tokens exist.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the field size.
    pub fn size(mut self, size: TokenizerSize) -> Self {
        self.size = size;
        self
    }

    /// Disable token removal.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the token removal handler receiving the token index.
    pub fn on_remove(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: TokenizerTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit ARIA label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Combobox).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Accessible name for the field.
    fn accessible_label(&self) -> SharedString {
        self.aria_label.clone().unwrap_or_else(|| {
            if self.tokens.is_empty() {
                self.placeholder.clone()
            } else {
                SharedString::from(format!("{} tokens", self.tokens.len()))
            }
        })
    }

    /// Build the field with theme.
    pub fn build_with_theme(self, theme: &TokenizerTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the field with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &TokenizerTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let native_label = self.accessible_label();
        let native_props = AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Combobox))
            .maybe_state(self.disabled, AriaState::Disabled);

        let mut field = div()
            .id(self.id.clone())
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(design.spacing.control_gap * 0.5))
            .px(px(design.spacing.control_padding_x))
            .py(px(design.spacing.control_padding_y * 0.5))
            .rounded(px(design.corners.md))
            .border_1()
            .border_color(theme.border)
            .bg(theme.background);
        field = match self.size {
            TokenizerSize::Sm => field.text_xs(),
            TokenizerSize::Md => field.text_sm(),
            TokenizerSize::Lg => field.text_lg(),
        };

        if self.tokens.is_empty() {
            field = field.child(div().text_color(theme.placeholder).child(self.placeholder));
        } else {
            let mut chips = Vec::with_capacity(self.tokens.len());
            for (index, token) in self.tokens.iter().enumerate() {
                let chip_id = (
                    self.id.clone(),
                    SharedString::from(format!("token-{index}")),
                );
                let mut chip = div()
                    .id(chip_id)
                    .flex()
                    .items_center()
                    .gap(px(design.spacing.control_gap * 0.5))
                    .px(px(design.spacing.control_padding_x * 0.5))
                    .rounded(px(design.corners.sm))
                    .bg(theme.token_bg)
                    .text_color(theme.token_text)
                    .child(token.clone());
                if self.disabled {
                    chip = chip.opacity(0.5);
                } else if let Some(handler) = &self.on_remove {
                    let mouse_handler = handler.clone();
                    let key_handler = handler.clone();
                    chip = chip
                        .cursor_pointer()
                        .child(div().child("x"))
                        .on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                            mouse_handler(index, window, cx);
                        })
                        .on_key_down(move |event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                                key_handler(index, window, cx);
                                cx.stop_propagation();
                            }
                        });
                }
                chips.push(chip);
            }
            field = field.children(chips);
        }

        if self.disabled {
            field = field.cursor_not_allowed();
        }

        apply_native_accessibility(field, native_label, &native_props)
    }
}

impl RenderOnce for Tokenizer {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.accessible_label(),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Combobox))
                .maybe_state(self.disabled, AriaState::Disabled),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| TokenizerTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{Tokenizer, TokenizerSize, TokenizerTheme};
    use gpui::rgba;

    #[test]
    fn tokenizer_defaults_to_empty_enabled_field() {
        let tokens = Tokenizer::new("tags");
        assert_eq!(tokens.tokens.len(), 0);
        assert_eq!(tokens.size, TokenizerSize::Md);
        assert!(!tokens.disabled);
        assert_eq!(tokens.placeholder.as_ref(), "Add items...");
    }

    #[test]
    fn tokenizer_builders_replace_and_append_tokens() {
        let tokens = Tokenizer::new("tags")
            .tokens(["drums", "bass"])
            .token("keys")
            .size(TokenizerSize::Sm);
        assert_eq!(tokens.tokens.len(), 3);
        assert_eq!(tokens.tokens[0].as_ref(), "drums");
        assert_eq!(tokens.tokens[2].as_ref(), "keys");
        assert_eq!(tokens.size, TokenizerSize::Sm);
    }

    #[test]
    fn tokenizer_accessible_label_summarizes_tokens() {
        let empty = Tokenizer::new("tags");
        assert_eq!(empty.accessible_label().as_ref(), "Add items...");
        let filled = Tokenizer::new("tags").tokens(["drums", "bass"]);
        assert_eq!(filled.accessible_label().as_ref(), "2 tokens");
    }

    #[test]
    fn tokenizer_theme_defaults_match_compiled_tokens() {
        let theme = TokenizerTheme::default();
        assert_eq!(theme.background, rgba(0x1e1e1eff));
        assert_eq!(theme.border, rgba(0x3a3a3aff));
        assert_eq!(theme.placeholder, rgba(0x777777ff));
        assert_eq!(theme.token_bg, rgba(0x007acc33));
        assert_eq!(theme.token_text, rgba(0xe6e6e6ff));
    }
}
