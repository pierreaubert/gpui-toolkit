//! `HoverCard` overlay component
//!
//! A text-only card that appears next to a hovered or focused target.
//! Like `Tooltip` it carries short text (a title plus an optional
//! description) rather than rich slots; parents own hover tracking
//! and positioning context.
//!
//! # Usage
//!
//! ```ignore
//! HoverCard::new("user-ada", "Ada Lovelace")
//!     .description("First programmer, Analytical Engine.")
//!     .placement(HoverCardPlacement::Right)
//!     .open(true)
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
    App, Div, ElementId, FocusHandle, KeyDownEvent, Pixels, Rgba, SharedString, Stateful, Window,
    div, px,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// Theme colors for the hover card.
#[derive(Debug, Clone, ComponentTheme)]
pub struct HoverCardTheme {
    /// Card background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Card border.
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Title text color.
    #[theme(default = 0xffffffff, from = text_primary)]
    pub title: Rgba,
    /// Description text color.
    #[theme(default = 0x888888ff, from = text_muted)]
    pub description: Rgba,
}

/// Hover card placement relative to the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HoverCardPlacement {
    /// Above the target.
    Top,
    /// Below the target (default).
    #[default]
    Bottom,
    /// Left of the target.
    Left,
    /// Right of the target.
    Right,
}

/// Hover card size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HoverCardSize {
    /// Compact card.
    Sm,
    /// Standard card (default).
    #[default]
    Md,
    /// Large card.
    Lg,
}

impl HoverCardSize {
    /// Card width in pixels from the design system.
    ///
    /// Widths are touch-target multiples so the card scales with
    /// density instead of using fixed pixel constants.
    fn width(&self, design: &DesignSystem) -> f32 {
        let unit = design.interaction.min_touch_target;
        match self {
            HoverCardSize::Sm => unit * 5.0,
            HoverCardSize::Md => unit * 7.0,
            HoverCardSize::Lg => unit * 9.0,
        }
    }

    /// Title text size from the design system type scale.
    fn title_size(&self, design: &DesignSystem) -> Pixels {
        match self {
            HoverCardSize::Sm => px(design.typography.small_size),
            HoverCardSize::Md => px(design.typography.base_size),
            HoverCardSize::Lg => px(design.typography.large_size),
        }
    }
}

impl From<crate::ComponentSize> for HoverCardSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Text-only card shown next to a target.
///
/// `HoverCard` renders a title with an optional description while
/// `open` is true and renders nothing otherwise. Content stays
/// text-only (no child slots); parents own hover tracking and
/// dismissal through `on_close`.
///
/// # Examples
///
/// ```ignore
/// HoverCard::new("user-ada", "Ada Lovelace")
///     .description("First programmer, Analytical Engine.")
///     .open(show_card)
/// ```
#[derive(IntoElement)]
pub struct HoverCard {
    id: ElementId,
    open: bool,
    title: SharedString,
    description: Option<SharedString>,
    placement: HoverCardPlacement,
    size: HoverCardSize,
    dismiss_on_escape: bool,
    focus_handle: Option<FocusHandle>,
    restore_focus_to: Option<FocusHandle>,
    theme: Option<HoverCardTheme>,
    design: Option<Arc<DesignSystem>>,
    on_close: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl HoverCard {
    /// Create a hover card with a title.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            open: false,
            title: title.into(),
            description: None,
            placement: HoverCardPlacement::default(),
            size: HoverCardSize::default(),
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

    /// Set whether the card is visible.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set the card title.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the card description.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the card placement relative to the target.
    pub fn placement(mut self, placement: HoverCardPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Set the card size.
    pub fn size(mut self, size: HoverCardSize) -> Self {
        self.size = size;
        self
    }

    /// Set whether `Escape` dismisses the card.
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
    pub fn theme(mut self, theme: HoverCardTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set the handler run when the card dismisses.
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

    /// Build the card with explicit theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &HoverCardTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        if !self.open {
            return div().id(self.id);
        }

        let on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>> =
            self.on_close.map(|handler| Rc::from(handler));

        let mut card = div()
            .id(self.id)
            .absolute()
            .w(px(self.size.width(design)))
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded(px(design.corners.md))
            .shadow_lg()
            .px(px(design.spacing.control_padding_x))
            .py(px(design.spacing.control_padding_y))
            .flex()
            .flex_col()
            .gap(px(design.spacing.control_gap * 0.5));

        card = match self.placement {
            HoverCardPlacement::Top => card.bottom_full().left_0().mb_1(),
            HoverCardPlacement::Bottom => card.top_full().left_0().mt_1(),
            HoverCardPlacement::Left => card.right_full().top_0().mr_1(),
            HoverCardPlacement::Right => card.left_full().top_0().ml_1(),
        };

        if let Some(handle) = self.focus_handle.clone() {
            card = card.track_focus(&handle).focusable();
            if self.dismiss_on_escape && on_close.is_some() {
                let handler = on_close.clone();
                let restore = self.restore_focus_to.clone();
                card = card.on_key_down(
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

        card = card.child(
            div()
                .text_size(self.size.title_size(design))
                .text_color(theme.title)
                .child(self.title),
        );
        if let Some(description) = self.description {
            card = card.child(
                div()
                    .text_size(px(design.typography.small_size))
                    .text_color(theme.description)
                    .child(description),
            );
        }
        card
    }
}

impl RenderOnce for HoverCard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| self.title.clone()),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Dialog)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| HoverCardTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{HoverCard, HoverCardPlacement, HoverCardSize, HoverCardTheme};
    use gpui::rgba;

    #[test]
    fn hover_card_defaults_to_closed_standard_card() {
        let card = HoverCard::new("user-ada", "Ada Lovelace");
        assert!(!card.open);
        assert_eq!(card.title.as_ref(), "Ada Lovelace");
        assert!(card.description.is_none());
        assert_eq!(card.placement, HoverCardPlacement::Bottom);
        assert_eq!(card.size, HoverCardSize::Md);
    }

    #[test]
    fn hover_card_builders_store_card_state() {
        let card = HoverCard::new("user-ada", "Ada")
            .open(true)
            .title("Ada Lovelace")
            .description("First programmer.")
            .placement(HoverCardPlacement::Right)
            .size(HoverCardSize::Lg);
        assert!(card.open);
        assert_eq!(card.title.as_ref(), "Ada Lovelace");
        assert_eq!(card.description.as_deref(), Some("First programmer."));
        assert_eq!(card.placement, HoverCardPlacement::Right);
        assert_eq!(card.size, HoverCardSize::Lg);
    }

    #[test]
    fn hover_card_theme_defaults_match_compiled_tokens() {
        let theme = HoverCardTheme::default();
        assert_eq!(theme.background, rgba(0x1e1e1eff));
        assert_eq!(theme.border, rgba(0x3a3a3aff));
        assert_eq!(theme.title, rgba(0xffffffff));
        assert_eq!(theme.description, rgba(0x888888ff));
    }
}
