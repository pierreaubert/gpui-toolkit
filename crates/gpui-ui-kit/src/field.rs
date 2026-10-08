//! `Field` component
//!
//! A shared form field wrapper pairing a label, an optional required
//! marker, a control slot, and help or error text. Individual inputs keep
//! their own rendering; `Field` supplies the consistent vertical
//! arrangement so forms do not rebuild it per control.
//!
//! # Usage
//!
//! ```ignore
//! Field::new("gain-field")
//!     .label("Gain")
//!     .required(true)
//!     .child(Slider::new("gain").min(0.0).max(1.0))
//!     .help("Applied before the limiter.")
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, apply_native_accessibility,
};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{AnyElement, App, Div, ElementId, Rgba, SharedString, Stateful, Window, div, px};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Theme colors for the field wrapper.
#[derive(Debug, Clone, ComponentTheme)]
pub struct FieldTheme {
    /// Label color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub label: Rgba,
    /// Required marker color.
    #[theme(default = 0xcc3333ff, from = error)]
    pub required: Rgba,
    /// Help text color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub help: Rgba,
    /// Error text color.
    #[theme(default = 0xcc3333ff, from = error)]
    pub error: Rgba,
}

/// Shared label, control slot, and help or error text.
#[derive(IntoElement)]
pub struct Field {
    id: ElementId,
    label: Option<SharedString>,
    required: bool,
    help: Option<SharedString>,
    error: Option<SharedString>,
    child: Option<AnyElement>,
    theme: Option<FieldTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Field {
    /// Create an empty field wrapper.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let field = Field::new("gain-field").label("Gain");
    /// ```
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            required: false,
            help: None,
            error: None,
            child: None,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the field label.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Mark the field as required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set the help text under the control.
    pub fn help(mut self, help: impl Into<SharedString>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Set the error text under the control.
    ///
    /// Error replaces help when both are set.
    pub fn error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Set the wrapped control element.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: FieldTheme) -> Self {
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

    /// Accessible name for the wrapper.
    fn accessible_label(&self) -> SharedString {
        self.aria_label
            .clone()
            .or_else(|| self.label.clone())
            .unwrap_or_default()
    }

    /// Build the wrapper with theme.
    pub fn build_with_theme(self, theme: &FieldTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the wrapper with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &FieldTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let native_label = self.accessible_label();
        let native_props = AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group));

        let mut column = div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(design.spacing.control_gap * 0.5));

        if let Some(label) = self.label {
            let mut label_row = div()
                .flex()
                .items_center()
                .gap(px(design.spacing.control_gap * 0.5))
                .text_size(px(design.typography.small_size))
                .text_color(theme.label)
                .child(label);
            if self.required {
                label_row = label_row.child(div().text_color(theme.required).child("*"));
            }
            column = column.child(label_row);
        }

        if let Some(child) = self.child {
            column = column.child(child);
        }

        if let Some(error) = self.error {
            column = column.child(
                div()
                    .text_size(px(design.typography.small_size))
                    .text_color(theme.error)
                    .child(error),
            );
        } else if let Some(help) = self.help {
            column = column.child(
                div()
                    .text_size(px(design.typography.small_size))
                    .text_color(theme.help)
                    .child(help),
            );
        }

        apply_native_accessibility(column, native_label, &native_props)
    }
}

impl RenderOnce for Field {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.accessible_label(),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| FieldTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{Field, FieldTheme};
    use gpui::rgba;

    #[test]
    fn field_defaults_to_optional_unlabeled_wrapper() {
        let field = Field::new("gain-field");
        assert!(field.label.is_none());
        assert!(!field.required);
        assert!(field.help.is_none());
        assert!(field.error.is_none());
    }

    #[test]
    fn field_builders_store_label_and_messages() {
        let field = Field::new("gain-field")
            .label("Gain")
            .required(true)
            .help("Applied before the limiter.")
            .error("Gain is required.");
        assert_eq!(field.label.as_deref(), Some("Gain"));
        assert!(field.required);
        assert!(field.error.is_some());
        assert_eq!(field.accessible_label().as_ref(), "Gain");
    }

    #[test]
    fn field_accessible_label_prefers_explicit_name() {
        let field = Field::new("gain-field")
            .label("Gain")
            .aria_label("Channel gain");
        assert_eq!(field.accessible_label().as_ref(), "Channel gain");
    }

    #[test]
    fn field_theme_defaults_match_compiled_tokens() {
        let theme = FieldTheme::default();
        assert_eq!(theme.label, rgba(0xccccccff));
        assert_eq!(theme.required, rgba(0xcc3333ff));
        assert_eq!(theme.help, rgba(0x777777ff));
        assert_eq!(theme.error, rgba(0xcc3333ff));
    }
}
