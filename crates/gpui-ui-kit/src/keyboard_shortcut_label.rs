//! KeyboardShortcutLabel component
//!
//! Renders keyboard shortcuts as styled key labels (e.g., ⌘+K displays as
//! individual styled key caps).
//!
//! # Usage
//!
//! ```ignore
//! KeyboardShortcutLabel::new("⌘+K")
//! KeyboardShortcutLabel::new("Ctrl+Shift+P")
//!     .size(KeyboardShortcutSize::Lg)
//! ```

use crate::theme::{Theme, ThemeExt};
use gpui::prelude::{IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, FontWeight, SharedString, Window, div, px};

/// Size of keyboard shortcut labels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyboardShortcutSize {
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
}

/// A keyboard shortcut label that renders each key as a styled cap
#[derive(IntoElement)]
pub struct KeyboardShortcutLabel {
    shortcut: SharedString,
    size: KeyboardShortcutSize,
    /// Separator to split keys (default: "+")
    separator: SharedString,
}

impl KeyboardShortcutLabel {
    /// Create a new keyboard shortcut label
    ///
    /// The shortcut string is split by "+" to render individual key caps.
    /// Example: "⌘+K", "Ctrl+Shift+P", "Alt+F4"
    pub fn new(shortcut: impl Into<SharedString>) -> Self {
        Self {
            shortcut: shortcut.into(),
            size: KeyboardShortcutSize::default(),
            separator: "+".into(),
        }
    }

    /// Set size
    pub fn size(mut self, size: KeyboardShortcutSize) -> Self {
        self.size = size;
        self
    }

    /// Set the key separator (default: "+")
    pub fn separator(mut self, sep: impl Into<SharedString>) -> Self {
        self.separator = sep.into();
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let (px_val, py_val, text_size_sm) = match self.size {
            KeyboardShortcutSize::Sm => (px(4.0), px(1.0), true),
            KeyboardShortcutSize::Md => (px(6.0), px(2.0), true),
            KeyboardShortcutSize::Lg => (px(8.0), px(3.0), false),
        };

        let keys: Vec<&str> = self.shortcut.split(self.separator.as_ref()).collect();

        let mut container = div().flex().items_center().gap(px(4.0));

        for (i, key) in keys.iter().enumerate() {
            if i > 0 {
                container = container.child(
                    div()
                        .text_color(theme.text_muted)
                        .text_xs()
                        .child(SharedString::from(self.separator.to_string())),
                );
            }

            let mut key_cap = div()
                .px(px_val)
                .py(py_val)
                .bg(theme.surface)
                .border_1()
                .border_color(theme.border)
                .rounded(px(4.0))
                .text_color(theme.text_secondary)
                .font_weight(FontWeight::MEDIUM)
                .child(SharedString::from(key.trim().to_string()));

            if text_size_sm {
                key_cap = key_cap.text_xs();
            } else {
                key_cap = key_cap.text_sm();
            }

            container = container.child(key_cap);
        }

        container
    }
}

impl RenderOnce for KeyboardShortcutLabel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}
