//! Single-child centering layout primitive.
//!
//! [`Center`] centers one child on both axes, optionally constraining its
//! width. It is a primitive for empty states, dialogs, and hero blocks.
//!
//! # Usage
//!
//! ```ignore
//! Center::new("empty")
//!     .max_width(px(480.0))
//!     .child(empty_state)
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{AnyElement, App, Div, ElementId, Pixels, Rgba, SharedString, Stateful, Window, div};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Theme colors for centering container styling.
#[derive(Debug, Clone, ComponentTheme)]
pub struct CenterTheme {
    /// Container background tint (transparent by default).
    #[theme(default = 0x00000000, from = transparent)]
    pub background: Rgba,
}

/// A single-child centering container.
///
/// Centers its child horizontally and vertically. An empty slot renders
/// nothing. Unlabeled containers register with an empty accessible label.
///
/// # Examples
///
/// ```ignore
/// Center::new("empty")
///     .max_width(px(480.0))
///     .child(empty_state)
/// ```
#[derive(IntoElement)]
pub struct Center {
    id: ElementId,
    child: Option<AnyElement>,
    max_width: Option<Pixels>,
    theme: Option<CenterTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Center {
    /// Create a new centering container.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            child: None,
            max_width: None,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the centered child element.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }

    /// Constrain the child width (`None`, the default, leaves it natural).
    pub fn max_width(mut self, max_width: Pixels) -> Self {
        self.max_width = Some(max_width);
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: CenterTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit accessible label for the centered group.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default accessible role (`Group`).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build the container into a `Stateful<Div>` for further composition.
    ///
    /// Note: This bypasses accessibility registration. Prefer using the
    /// component directly via `RenderOnce` for automatic accessibility
    /// tree integration.
    pub fn build(self) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        let mut this = self;
        let theme = this.theme.take().unwrap_or_default();
        this.build_with_theme_and_design(&theme, &design)
    }

    /// Build with explicit theme and design defaults.
    pub fn build_with_theme_and_design(
        self,
        theme: &CenterTheme,
        _design: &DesignSystem,
    ) -> Stateful<Div> {
        let mut el = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_center()
            .w_full()
            .h_full()
            .bg(theme.background);
        if let Some(child) = self.child {
            let mut slot = div().flex().items_center().justify_center();
            if let Some(max_width) = self.max_width {
                slot = slot.max_w(max_width).w_full();
            }
            el = el.child(slot.child(child));
        }
        el
    }
}

impl RenderOnce for Center {
    fn render(mut self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_default(),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
        });

        let design = crate::design::resolve_design(self.design.clone(), cx);
        let global_theme = cx.theme();
        let theme = self
            .theme
            .take()
            .unwrap_or_else(|| CenterTheme::from(global_theme.as_ref()));
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    #[test]
    fn defaults_have_empty_slot_and_no_max_width() {
        let center = Center::new("center");
        assert!(center.child.is_none());
        assert!(center.max_width.is_none());
        assert!(center.aria_label.is_none());
    }

    #[test]
    fn builders_set_fields() {
        let center = Center::new("center")
            .child(div())
            .max_width(px(480.0))
            .aria_label("Empty state")
            .aria_role(AriaRole::Region);
        assert!(center.child.is_some());
        assert_eq!(center.max_width, Some(px(480.0)));
        assert_eq!(center.aria_label, Some(SharedString::from("Empty state")));
        assert_eq!(center.aria_role, Some(AriaRole::Region));
    }

    #[test]
    fn build_does_not_panic() {
        let _el = Center::new("empty-center").build();
        let _el = Center::new("filled-center")
            .max_width(px(320.0))
            .child(div())
            .build();
    }
}
