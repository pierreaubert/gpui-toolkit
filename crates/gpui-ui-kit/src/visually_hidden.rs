//! Visually hidden accessible content primitive.
//!
//! [`VisuallyHidden`] hides content from sighted users while keeping it
//! available to assistive technology, following the classic 1px clipping
//! technique. With `focusable`, it behaves like a skip link: hidden until
//! it receives keyboard focus.
//!
//! # Usage
//!
//! ```ignore
//! VisuallyHidden::new("chart-desc")
//!     .aria_label("Bar chart description")
//!     .child("Revenue grew quarter over quarter.")
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, Div, ElementId, FocusHandle, Rgba, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

thread_local! {
    static VISUALLY_HIDDEN_FOCUS_HANDLES: RefCell<HashMap<ElementId, FocusHandle>> =
        RefCell::new(HashMap::new());
}

/// Cap for retained focus handles; guards against unbounded growth.
const MAX_VISUALLY_HIDDEN_FOCUS_HANDLES: usize = 1024;

fn visually_hidden_focus_handle(id: &ElementId, cx: &mut App) -> FocusHandle {
    VISUALLY_HIDDEN_FOCUS_HANDLES.with(|handles| {
        let mut handles = handles.borrow_mut();
        while handles.len() > MAX_VISUALLY_HIDDEN_FOCUS_HANDLES {
            if let Some(key) = handles.keys().next().cloned() {
                handles.remove(&key);
            }
        }
        handles
            .entry(id.clone())
            .or_insert_with(|| cx.focus_handle())
            .clone()
    })
}

/// Theme colors for visually hidden content styling.
#[derive(Debug, Clone, ComponentTheme)]
pub struct VisuallyHiddenTheme {
    /// Wrapper background tint (transparent by default).
    #[theme(default = 0x00000000, from = transparent)]
    pub background: Rgba,
}

/// Content hidden visually but exposed to assistive technology.
///
/// Renders its child in a 1px clipped box (the standard visually-hidden
/// CSS technique) and registers an accessible node so screen-reader
/// bridges announce it. Set [`Self::aria_label`] to describe the content
/// for bridges that only read the tree. With [`Self::focusable`], the
/// content reveals itself on keyboard focus like a skip link. An empty
/// slot renders nothing.
///
/// # Examples
///
/// ```ignore
/// VisuallyHidden::new("skip")
///     .focusable(true)
///     .child("Skip to main content")
/// ```
#[derive(IntoElement)]
pub struct VisuallyHidden {
    id: ElementId,
    child: Option<AnyElement>,
    focusable: bool,
    theme: Option<VisuallyHiddenTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl VisuallyHidden {
    /// Create a new visually hidden container.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            child: None,
            focusable: false,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the hidden child element.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }

    /// Reveal the content on keyboard focus (skip-link behavior).
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: VisuallyHiddenTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit accessible label describing the hidden content.
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
        this.build_with_theme_and_design(&theme, &design, None)
    }

    /// Build with explicit theme and design defaults.
    ///
    /// `focus_handle` enables the focusable reveal path; `None` keeps the
    /// hidden box unfocusable even when [`Self::focusable`] is set.
    pub fn build_with_theme_and_design(
        self,
        theme: &VisuallyHiddenTheme,
        _design: &DesignSystem,
        focus_handle: Option<FocusHandle>,
    ) -> Stateful<Div> {
        // Standard visually-hidden technique: 1px absolute box with clipped
        // overflow. Opacity zero removes the last visible sliver without
        // affecting the accessibility data layer.
        let mut el = div()
            .id(self.id)
            .absolute()
            .w(px(1.0))
            .h(px(1.0))
            .overflow_hidden()
            .opacity(0.0)
            .bg(theme.background);
        if self.focusable
            && let Some(handle) = focus_handle
        {
            el = el
                .track_focus(&handle)
                .focusable()
                .focus_visible(|style| style.relative().w_auto().h_auto().opacity(1.0));
        }
        el.children(self.child)
    }
}

impl RenderOnce for VisuallyHidden {
    fn render(mut self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_default(),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
        });

        let design = crate::design::resolve_design(self.design.clone(), cx);
        let focus_handle = self
            .focusable
            .then(|| visually_hidden_focus_handle(&self.id, cx));
        let global_theme = cx.theme();
        let theme = self
            .theme
            .take()
            .unwrap_or_else(|| VisuallyHiddenTheme::from(global_theme.as_ref()));
        self.build_with_theme_and_design(&theme, &design, focus_handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_hide_without_focus() {
        let hidden = VisuallyHidden::new("hidden");
        assert!(hidden.child.is_none());
        assert!(!hidden.focusable);
        assert!(hidden.aria_label.is_none());
    }

    #[test]
    fn builders_set_fields() {
        let hidden = VisuallyHidden::new("hidden")
            .child("Skip to content")
            .focusable(true)
            .aria_label("Skip link")
            .aria_role(AriaRole::Link);
        assert!(hidden.child.is_some());
        assert!(hidden.focusable);
        assert_eq!(hidden.aria_label, Some(SharedString::from("Skip link")));
        assert_eq!(hidden.aria_role, Some(AriaRole::Link));
    }

    #[test]
    fn build_does_not_panic() {
        let _el = VisuallyHidden::new("empty-hidden").build();
        let _el = VisuallyHidden::new("skip-link")
            .focusable(true)
            .child("Skip to content")
            .build();
    }
}
