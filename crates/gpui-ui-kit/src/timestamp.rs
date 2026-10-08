//! `Timestamp` component
//!
//! A muted inline time label for "edited 2h ago" style metadata. The
//! caller owns formatting (absolute or relative); the component only
//! styles the label so timestamps look identical everywhere.
//!
//! # Usage
//!
//! ```ignore
//! Timestamp::new("edited-at", "Edited 2 hours ago")
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, ElementId, Rgba, SharedString, Stateful, Window, div};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Theme colors for the timestamp.
#[derive(Debug, Clone, ComponentTheme)]
pub struct TimestampTheme {
    /// Timestamp label color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub text: Rgba,
}

/// Timestamp size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimestampSize {
    /// Compact label.
    Sm,
    /// Standard label (default).
    #[default]
    Md,
    /// Large label.
    Lg,
}

impl From<crate::ComponentSize> for TimestampSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Muted inline time label.
#[derive(IntoElement)]
pub struct Timestamp {
    id: ElementId,
    text: SharedString,
    size: TimestampSize,
    theme: Option<TimestampTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Timestamp {
    /// Create a timestamp label with `text`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let stamp = Timestamp::new("edited-at", "Edited 2 hours ago");
    /// ```
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            size: TimestampSize::default(),
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the label text.
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.text = text.into();
        self
    }

    /// Set the label size.
    pub fn size(mut self, size: TimestampSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: TimestampTheme) -> Self {
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

    /// Override the default ARIA role (Group).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build the timestamp with theme.
    pub fn build_with_theme(self, theme: &TimestampTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the timestamp with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &TimestampTheme,
        _design: &DesignSystem,
    ) -> Stateful<Div> {
        let mut label = div().id(self.id).text_color(theme.text).child(self.text);
        label = match self.size {
            TimestampSize::Sm => label.text_xs(),
            TimestampSize::Md => label.text_sm(),
            TimestampSize::Lg => label.text_lg(),
        };
        label
    }
}

impl RenderOnce for Timestamp {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_else(|| self.text.clone()),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| TimestampTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{Timestamp, TimestampSize, TimestampTheme};
    use gpui::rgba;

    #[test]
    fn timestamp_defaults_to_standard_label() {
        let stamp = Timestamp::new("edited-at", "Edited 2 hours ago");
        assert_eq!(stamp.text.as_ref(), "Edited 2 hours ago");
        assert_eq!(stamp.size, TimestampSize::Md);
    }

    #[test]
    fn timestamp_builders_store_text_and_size() {
        let stamp = Timestamp::new("edited-at", "Edited 2 hours ago")
            .text("Just now")
            .size(TimestampSize::Sm);
        assert_eq!(stamp.text.as_ref(), "Just now");
        assert_eq!(stamp.size, TimestampSize::Sm);
    }

    #[test]
    fn timestamp_theme_defaults_match_compiled_tokens() {
        let theme = TimestampTheme::default();
        assert_eq!(theme.text, rgba(0x777777ff));
    }
}
