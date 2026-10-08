//! `Lightbox` overlay component
//!
//! A display-only image overlay with a backdrop, a caption, and
//! Escape-to-dismiss behavior. The image is shown as-is; zoom,
//! fullscreen, and gallery navigation are out of scope.
//!
//! # Usage
//!
//! ```ignore
//! Lightbox::new("artwork", "assets/painting.png")
//!     .caption("Gallery preview")
//!     .open(true)
//!     .on_close(|_window, _cx| {
//!         // Hide the lightbox.
//!     })
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::interaction::{OverlayKeyAction, overlay_key_action};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
    StyledImage,
};
use gpui::{
    App, Div, ElementId, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent, ObjectFit, Rgba,
    ScrollWheelEvent, SharedString, Window, div, img, px,
};
use gpui_design::DesignSystem;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// Swallow scroll events so the page behind the overlay stays still.
fn ignore_scroll_wheel(_event: &ScrollWheelEvent, _window: &mut Window, _cx: &mut App) {}

/// Stop clicks on the panel from reaching the backdrop close handler.
fn stop_panel_click(_event: &MouseDownEvent, _window: &mut Window, cx: &mut App) {
    cx.stop_propagation();
}

/// Image preview area keeps a 16:9 frame (`9.0 / 16.0`).
const PREVIEW_ASPECT: f32 = 9.0 / 16.0;

/// Theme colors for the lightbox overlay.
#[derive(Debug, Clone, ComponentTheme)]
pub struct LightboxTheme {
    /// Dimmed backdrop behind the panel.
    #[theme(default = 0x00000088, from = overlay_bg)]
    pub backdrop: Rgba,
    /// Panel background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Panel border.
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Caption text color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub caption: Rgba,
    /// Placeholder text color when no image loads.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub placeholder: Rgba,
}

/// Lightbox panel size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LightboxSize {
    /// Compact panel.
    Sm,
    /// Standard panel (default).
    #[default]
    Md,
    /// Large panel.
    Lg,
}

impl LightboxSize {
    /// Panel width in pixels from the design system.
    ///
    /// Widths are touch-target multiples so the panel scales with
    /// density instead of using fixed pixel constants.
    fn width(&self, design: &DesignSystem) -> f32 {
        let unit = design.interaction.min_touch_target;
        match self {
            LightboxSize::Sm => unit * 8.0,
            LightboxSize::Md => unit * 12.0,
            LightboxSize::Lg => unit * 16.0,
        }
    }
}

impl From<crate::ComponentSize> for LightboxSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Display-only image overlay.
///
/// `Lightbox` shows one image with an optional caption while `open`
/// is true and renders nothing otherwise. The image always fits the
/// preview frame; there is no zoom, fullscreen, or gallery support.
/// Backdrop clicks and `Escape` (with a focus handle) dismiss through
/// `on_close`.
///
/// # Examples
///
/// ```ignore
/// Lightbox::new("artwork", "assets/painting.png")
///     .caption("Gallery preview")
///     .open(show_lightbox)
/// ```
#[derive(IntoElement)]
pub struct Lightbox {
    id: ElementId,
    open: bool,
    src: SharedString,
    alt: SharedString,
    caption: Option<SharedString>,
    size: LightboxSize,
    show_backdrop: bool,
    dismiss_on_escape: bool,
    focus_handle: Option<FocusHandle>,
    restore_focus_to: Option<FocusHandle>,
    theme: Option<LightboxTheme>,
    design: Option<Arc<DesignSystem>>,
    on_close: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Lightbox {
    /// Create a lightbox showing the image at `src`.
    pub fn new(id: impl Into<ElementId>, src: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            open: false,
            src: src.into(),
            alt: SharedString::from(""),
            caption: None,
            size: LightboxSize::default(),
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

    /// Set the image source path or URL.
    pub fn src(mut self, src: impl Into<SharedString>) -> Self {
        self.src = src.into();
        self
    }

    /// Set the image alternative text.
    pub fn alt(mut self, alt: impl Into<SharedString>) -> Self {
        self.alt = alt.into();
        self
    }

    /// Set the caption shown below the image.
    pub fn caption(mut self, caption: impl Into<SharedString>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// Set the panel size.
    pub fn size(mut self, size: LightboxSize) -> Self {
        self.size = size;
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
    pub fn theme(mut self, theme: LightboxTheme) -> Self {
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
    pub fn build_with_theme_and_design(self, theme: &LightboxTheme, design: &DesignSystem) -> Div {
        if !self.open {
            return div();
        }

        let on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>> =
            self.on_close.map(|handler| Rc::from(handler));

        let mut outer = div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
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

        let width = self.size.width(design);
        let mut panel = div()
            .id(self.id)
            .w(px(width))
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded(px(design.corners.md))
            .shadow_lg()
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, stop_panel_click);

        if let Some(handle) = self.focus_handle.clone() {
            panel = panel.track_focus(&handle).focusable();
            if self.dismiss_on_escape && on_close.is_some() {
                let handler = on_close.clone();
                let restore = self.restore_focus_to.clone();
                panel = panel.on_key_down(
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

        let source = self.src.clone();
        let mut image = if source.contains("://") || source.starts_with("data:") {
            img(source)
        } else {
            img(PathBuf::from(source.as_ref()))
        };
        let placeholder_color = theme.placeholder;
        image = image
            .size_full()
            .object_fit(ObjectFit::Contain)
            .with_fallback(move || {
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(placeholder_color)
                    .child("Image unavailable")
                    .into_any_element()
            });
        // Preview height follows the 16:9 frame constant above.
        let preview_height = px(width * PREVIEW_ASPECT);
        panel = panel.child(
            div()
                .w_full()
                .h(preview_height)
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.background)
                .child(image),
        );

        if !self.alt.is_empty() {
            panel = panel.aria_label(self.alt.clone());
        }
        if let Some(caption) = self.caption {
            panel = panel.child(
                div()
                    .w_full()
                    .flex()
                    .justify_center()
                    .px(px(design.spacing.control_padding_x))
                    .py(px(design.spacing.control_padding_y))
                    .text_size(px(design.typography.small_size))
                    .text_color(theme.caption)
                    .child(caption),
            );
        }

        outer.child(panel)
    }
}

impl RenderOnce for Lightbox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let fallback = self
            .caption
            .clone()
            .filter(|caption| !caption.is_empty())
            .or_else(|| {
                if self.alt.is_empty() {
                    None
                } else {
                    Some(self.alt.clone())
                }
            })
            .unwrap_or_else(|| SharedString::from("Image viewer"));
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or(fallback),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Dialog)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| LightboxTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{Lightbox, LightboxSize, LightboxTheme};
    use gpui::rgba;

    #[test]
    fn lightbox_defaults_to_closed_standard_panel() {
        let viewer = Lightbox::new("artwork", "assets/painting.png");
        assert!(!viewer.open);
        assert_eq!(viewer.src.as_ref(), "assets/painting.png");
        assert_eq!(viewer.size, LightboxSize::Md);
        assert!(viewer.show_backdrop);
        assert!(viewer.dismiss_on_escape);
    }

    #[test]
    fn lightbox_builders_store_viewer_state() {
        let viewer = Lightbox::new("artwork", "one.png")
            .open(true)
            .src("two.png")
            .alt("Second painting")
            .caption("Gallery preview")
            .size(LightboxSize::Lg)
            .show_backdrop(false);
        assert!(viewer.open);
        assert_eq!(viewer.src.as_ref(), "two.png");
        assert_eq!(viewer.alt.as_ref(), "Second painting");
        assert_eq!(viewer.caption.as_deref(), Some("Gallery preview"));
        assert_eq!(viewer.size, LightboxSize::Lg);
        assert!(!viewer.show_backdrop);
    }

    #[test]
    fn lightbox_theme_defaults_match_compiled_tokens() {
        let theme = LightboxTheme::default();
        assert_eq!(theme.backdrop, rgba(0x00000088));
        assert_eq!(theme.background, rgba(0x1e1e1eff));
        assert_eq!(theme.border, rgba(0x3a3a3aff));
        assert_eq!(theme.caption, rgba(0xccccccff));
        assert_eq!(theme.placeholder, rgba(0x777777ff));
    }
}
