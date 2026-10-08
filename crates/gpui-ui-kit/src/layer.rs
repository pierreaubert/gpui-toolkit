//! `Layer` overlay component
//!
//! A minimal full-viewport overlay container with a backdrop and
//! Escape-to-dismiss behavior. The child owns its own visuals;
//! `Layer` only centers it, paints the backdrop, and forwards
//! dismissal. There is intentionally no z-index, portal, or
//! stacking management: paint order follows parent source order.
//!
//! # Usage
//!
//! ```ignore
//! Layer::new("session-expired")
//!     .open(true)
//!     .child(div().child("Session expired."))
//!     .on_close(|_window, _cx| {
//!         // Hide the layer.
//!     })
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::interaction::{OverlayKeyAction, overlay_key_action};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, Div, ElementId, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent, Rgba,
    ScrollWheelEvent, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// Swallow scroll events so the page behind the overlay stays still.
fn ignore_scroll_wheel(_event: &ScrollWheelEvent, _window: &mut Window, _cx: &mut App) {}

/// Stop clicks on the content from reaching the backdrop close handler.
fn stop_content_click(_event: &MouseDownEvent, _window: &mut Window, cx: &mut App) {
    cx.stop_propagation();
}

/// Theme colors for the overlay layer.
#[derive(Debug, Clone, ComponentTheme)]
pub struct LayerTheme {
    /// Dimmed backdrop behind the content.
    #[theme(default = 0x00000088, from = overlay_bg)]
    pub backdrop: Rgba,
}

/// Minimal overlay container without stacking management.
///
/// `Layer` centers one child over a backdrop while `open` is true
/// and renders nothing otherwise. Paint order follows normal parent
/// source order; there are no z-index, portal, or stacking props.
/// Backdrop clicks and `Escape` (with a focus handle) dismiss
/// through `on_close`.
///
/// # Examples
///
/// ```ignore
/// Layer::new("session-expired")
///     .open(show_layer)
///     .child(div().child("Session expired."))
/// ```
#[derive(IntoElement)]
pub struct Layer {
    id: ElementId,
    open: bool,
    child: Option<AnyElement>,
    show_backdrop: bool,
    dismiss_on_escape: bool,
    focus_handle: Option<FocusHandle>,
    restore_focus_to: Option<FocusHandle>,
    theme: Option<LayerTheme>,
    design: Option<Arc<DesignSystem>>,
    on_close: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Layer {
    /// Create an empty overlay layer.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: false,
            child: None,
            show_backdrop: true,
            dismiss_on_escape: true,
            focus_handle: None,
            restore_focus_to: None,
            theme: None,
            design: None,
            on_close: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set whether the overlay is visible.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set the content shown centered over the backdrop.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }

    /// Set whether the dimmed backdrop is painted.
    pub fn show_backdrop(mut self, show: bool) -> Self {
        self.show_backdrop = show;
        self
    }

    /// Set whether `Escape` dismisses the overlay.
    pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
        self.dismiss_on_escape = dismiss;
        self
    }

    /// Set the focus handle used for keyboard dismissal.
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set the focus handle to restore before running `on_close`.
    pub fn restore_focus_to(mut self, handle: FocusHandle) -> Self {
        self.restore_focus_to = Some(handle);
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: LayerTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set the handler run when the overlay dismisses.
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Box::new(handler));
        self
    }

    /// Set an explicit ARIA label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Dialog).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build the overlay with explicit theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &LayerTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        if !self.open {
            return div().id(self.id);
        }

        let on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>> =
            self.on_close.map(|handler| Rc::from(handler));

        let mut outer = div()
            .id(self.id)
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .p(px(design.spacing.control_padding_x))
            .on_scroll_wheel(ignore_scroll_wheel);
        if self.show_backdrop {
            outer = outer.bg(theme.backdrop);
            if let Some(handler) = on_close.clone() {
                let restore = self.restore_focus_to.clone();
                outer = outer.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                    if let Some(handle) = restore.as_ref() {
                        window.focus(handle, cx);
                    }
                    handler(window, cx);
                });
            }
        }

        if let Some(handle) = self.focus_handle.clone() {
            outer = outer.track_focus(&handle).focusable();
            if self.dismiss_on_escape && on_close.is_some() {
                let handler = on_close.clone();
                let restore = self.restore_focus_to.clone();
                outer = outer.on_key_down(
                    move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
                        if overlay_key_action(
                            event.keystroke.key.as_str(),
                            handle.is_focused(window),
                        ) != Some(OverlayKeyAction::Dismiss)
                        {
                            return;
                        }
                        let Some(handler) = handler.as_ref() else {
                            return;
                        };
                        cx.stop_propagation();
                        if let Some(target) = restore.as_ref() {
                            window.focus(target, cx);
                        }
                        handler(window, cx);
                    },
                );
            }
        }

        if let Some(child) = self.child {
            outer = outer.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_mouse_down(MouseButton::Left, stop_content_click)
                    .child(child),
            );
        }
        outer
    }
}

impl RenderOnce for Layer {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| SharedString::from("Overlay layer")),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Dialog)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| LayerTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{Layer, LayerTheme};
    use gpui::rgba;

    #[test]
    fn layer_defaults_to_closed_empty_overlay() {
        let layer = Layer::new("session-expired");
        assert!(!layer.open);
        assert!(layer.child.is_none());
        assert!(layer.show_backdrop);
        assert!(layer.dismiss_on_escape);
    }

    #[test]
    fn layer_builders_store_overlay_state() {
        use gpui::div;
        use gpui::prelude::ParentElement;
        let layer = Layer::new("session-expired")
            .open(true)
            .child(div().child("Session expired."))
            .show_backdrop(false)
            .dismiss_on_escape(false);
        assert!(layer.open);
        assert!(layer.child.is_some());
        assert!(!layer.show_backdrop);
        assert!(!layer.dismiss_on_escape);
    }

    #[test]
    fn layer_theme_defaults_match_compiled_tokens() {
        let theme = LayerTheme::default();
        assert_eq!(theme.backdrop, rgba(0x00000088));
    }
}
