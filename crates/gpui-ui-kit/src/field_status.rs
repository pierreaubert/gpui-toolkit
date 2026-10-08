//! `FieldStatus` component
//!
//! A validation message pairing a severity variant with text. Errors
//! announce assertively; success, warning, and info messages announce
//! politely. Use it under inputs or inside a [`crate::Field`] error slot.
//!
//! # Usage
//!
//! ```ignore
//! FieldStatus::new("gain-status", "Gain is required.")
//!     .variant(FieldStatusVariant::Error)
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaLive, AriaProps, AriaRole, apply_native_accessibility,
};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, ElementId, Rgba, SharedString, Stateful, Window, div, px};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Theme colors for field status severities.
#[derive(Debug, Clone, ComponentTheme)]
pub struct FieldStatusTheme {
    /// Info message color.
    #[theme(default = 0x4aa3f0ff, from = info)]
    pub info: Rgba,
    /// Success message color.
    #[theme(default = 0x4caf50ff, from = success)]
    pub success: Rgba,
    /// Warning message color.
    #[theme(default = 0xffb300ff, from = warning)]
    pub warning: Rgba,
    /// Error message color.
    #[theme(default = 0xcc3333ff, from = error)]
    pub error: Rgba,
}

/// Field status severity variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldStatusVariant {
    /// Neutral informational message (default).
    #[default]
    Info,
    /// Successful validation message.
    Success,
    /// Non-blocking warning message.
    Warning,
    /// Blocking error message.
    Error,
}

impl FieldStatusVariant {
    /// Glyph prefix rendered before the message.
    const fn glyph(self) -> &'static str {
        match self {
            Self::Info => "i",
            Self::Success => "✓",
            Self::Warning => "!",
            Self::Error => "✕",
        }
    }

    /// Whether the variant interrupts assistive announcements.
    const fn is_assertive(self) -> bool {
        matches!(self, Self::Error)
    }
}

/// Validation message with severity styling.
#[derive(IntoElement)]
pub struct FieldStatus {
    id: ElementId,
    variant: FieldStatusVariant,
    message: SharedString,
    theme: Option<FieldStatusTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl FieldStatus {
    /// Create a status message.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let status = FieldStatus::new("gain-status", "Gain is required.");
    /// ```
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            variant: FieldStatusVariant::default(),
            message: message.into(),
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the severity variant.
    pub fn variant(mut self, variant: FieldStatusVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the message text.
    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = message.into();
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: FieldStatusTheme) -> Self {
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

    /// Override the default ARIA role (Alert for errors, Status otherwise).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Default role for the current variant.
    fn default_role(&self) -> AriaRole {
        if self.variant.is_assertive() {
            AriaRole::Alert
        } else {
            AriaRole::Status
        }
    }

    /// Announcement politeness for the current variant.
    fn live(&self) -> AriaLive {
        if self.variant.is_assertive() {
            AriaLive::Assertive
        } else {
            AriaLive::Polite
        }
    }

    /// Color for the current variant.
    fn variant_color(theme: &FieldStatusTheme, variant: FieldStatusVariant) -> Rgba {
        match variant {
            FieldStatusVariant::Info => theme.info,
            FieldStatusVariant::Success => theme.success,
            FieldStatusVariant::Warning => theme.warning,
            FieldStatusVariant::Error => theme.error,
        }
    }

    /// Build the message with theme.
    pub fn build_with_theme(self, theme: &FieldStatusTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the message with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &FieldStatusTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let native_label = self
            .aria_label
            .clone()
            .unwrap_or_else(|| self.message.clone());
        let native_props =
            AriaProps::with_role(self.aria_role.unwrap_or_else(|| self.default_role()))
                .live(self.live());
        let color = Self::variant_color(theme, self.variant);

        let row = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(design.spacing.control_gap * 0.5))
            .text_size(px(design.typography.small_size))
            .text_color(color)
            .child(self.variant.glyph())
            .child(self.message);

        apply_native_accessibility(row, native_label, &native_props)
    }
}

impl RenderOnce for FieldStatus {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| self.message.clone()),
            props: AriaProps::with_role(self.aria_role.unwrap_or_else(|| self.default_role()))
                .live(self.live()),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| FieldStatusTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{FieldStatus, FieldStatusTheme, FieldStatusVariant};
    use crate::accessibility::{AriaLive, AriaRole};
    use gpui::rgba;

    #[test]
    fn field_status_defaults_to_info_message() {
        let status = FieldStatus::new("gain-status", "Gain is required.");
        assert_eq!(status.variant, FieldStatusVariant::Info);
        assert_eq!(status.message.as_ref(), "Gain is required.");
        assert_eq!(status.default_role(), AriaRole::Status);
        assert_eq!(status.live(), AriaLive::Polite);
    }

    #[test]
    fn field_status_error_uses_alert_role_and_assertive_live() {
        let status =
            FieldStatus::new("gain-status", "Gain is required.").variant(FieldStatusVariant::Error);
        assert_eq!(status.default_role(), AriaRole::Alert);
        assert_eq!(status.live(), AriaLive::Assertive);
    }

    #[test]
    fn field_status_builders_store_variant_and_message() {
        let status = FieldStatus::new("gain-status", "Gain is required.")
            .variant(FieldStatusVariant::Success)
            .message("Gain saved.");
        assert_eq!(status.variant, FieldStatusVariant::Success);
        assert_eq!(status.message.as_ref(), "Gain saved.");
    }

    #[test]
    fn field_status_theme_defaults_match_compiled_tokens() {
        let theme = FieldStatusTheme::default();
        assert_eq!(theme.info, rgba(0x4aa3f0ff));
        assert_eq!(theme.success, rgba(0x4caf50ff));
        assert_eq!(theme.warning, rgba(0xffb300ff));
        assert_eq!(theme.error, rgba(0xcc3333ff));
    }
}
