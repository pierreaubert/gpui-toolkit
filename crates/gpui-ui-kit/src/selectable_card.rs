//! `SelectableCard` component
//!
//! A card with toggle selection: unlike `Card`, it owns a selected flag,
//! a disabled flag, and click activation (mouse plus Enter/Space). The
//! parent holds the selection state; the card only renders it and reports
//! clicks.
//!
//! # Usage
//!
//! ```ignore
//! SelectableCard::new("plan-pro", "Pro")
//!     .description("For teams shipping weekly.")
//!     .selected(current == "pro")
//!     .on_click(|_window, _cx| {
//!         // Store the new selection.
//!     })
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    App, ClickEvent, Div, ElementId, KeyDownEvent, KeyboardClickEvent, Rgba, SharedString,
    Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// Theme colors for the selectable card.
#[derive(Debug, Clone, ComponentTheme)]
pub struct SelectableCardTheme {
    /// Card background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub surface: Rgba,
    /// Card background when selected.
    #[theme(default = 0x0e2a3aff, from = accent_muted)]
    pub surface_selected: Rgba,
    /// Card border color.
    #[theme(default = 0x3e3e42ff, from = border)]
    pub border: Rgba,
    /// Card border color when selected.
    #[theme(default = 0x007accff, from = accent)]
    pub border_selected: Rgba,
    /// Title color.
    #[theme(default = 0xffffffff, from = text_primary)]
    pub title: Rgba,
    /// Description color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub description: Rgba,
}

/// Selectable card size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectableCardSize {
    /// Compact card.
    Sm,
    /// Standard card (default).
    #[default]
    Md,
    /// Large card.
    Lg,
}

impl From<crate::ComponentSize> for SelectableCardSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Card with selected state and click activation.
#[derive(IntoElement)]
pub struct SelectableCard {
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    selected: bool,
    disabled: bool,
    size: SelectableCardSize,
    theme: Option<SelectableCardTheme>,
    design: Option<Arc<DesignSystem>>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl SelectableCard {
    /// Create a new unselected card with `title`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let card = SelectableCard::new("plan-pro", "Pro");
    /// ```
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            selected: false,
            disabled: false,
            size: SelectableCardSize::default(),
            theme: None,
            design: None,
            on_click: None,
            aria_label: None,
            aria_role: None,
        }
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

    /// Set the selected state.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Disable the card.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the card size.
    pub fn size(mut self, size: SelectableCardSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: SelectableCardTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set the click handler that ignores the event payload.
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        let handler = Rc::new(handler);
        self.on_click = Some(Rc::new(move |_event: &ClickEvent, window, cx| {
            handler(window, cx);
        }));
        self
    }

    /// Set the click handler with the `ClickEvent` payload.
    pub fn on_click_event(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set an explicit ARIA label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Button).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build the card with theme.
    pub fn build_with_theme(self, theme: &SelectableCardTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the card with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &SelectableCardTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let mut card = div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(design.spacing.control_gap * 0.5))
            .px(px(design.spacing.control_padding_x))
            .py(px(design.spacing.control_padding_y))
            .rounded(px(design.corners.md))
            .border_1()
            .border_color(if self.selected {
                theme.border_selected
            } else {
                theme.border
            })
            .bg(if self.selected {
                theme.surface_selected
            } else {
                theme.surface
            });
        card = match self.size {
            SelectableCardSize::Sm => card.text_xs(),
            SelectableCardSize::Md => card.text_sm(),
            SelectableCardSize::Lg => card.text_lg(),
        };

        if self.disabled {
            card = card.opacity(0.5).cursor_not_allowed();
        } else {
            card = card.cursor_pointer();
            if let Some(handler) = self.on_click {
                let mouse_handler = handler.clone();
                card = card.on_click(move |event: &ClickEvent, window, cx| {
                    mouse_handler(event, window, cx);
                });
                let key_handler = handler;
                card = card.on_key_down(move |event: &KeyDownEvent, window, cx| {
                    let key = event.keystroke.key.as_str();
                    if key == "enter" || key == "space" {
                        let click = ClickEvent::Keyboard(KeyboardClickEvent::default());
                        key_handler(&click, window, cx);
                        cx.stop_propagation();
                    }
                });
            }
        }

        card = card.child(div().text_color(theme.title).child(self.title));
        if let Some(description) = self.description {
            card = card.child(div().text_color(theme.description).child(description));
        }
        card
    }
}

impl RenderOnce for SelectableCard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let access_props = AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Button))
            .maybe_state(self.disabled, AriaState::Disabled)
            .maybe_state(self.selected, AriaState::Pressed(true));
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| self.title.clone()),
            props: access_props,
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| SelectableCardTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{SelectableCard, SelectableCardSize, SelectableCardTheme};
    use gpui::rgba;

    #[test]
    fn selectable_card_defaults_to_unselected_enabled_card() {
        let card = SelectableCard::new("plan-pro", "Pro");
        assert_eq!(card.title.as_ref(), "Pro");
        assert!(card.description.is_none());
        assert!(!card.selected);
        assert!(!card.disabled);
        assert_eq!(card.size, SelectableCardSize::Md);
    }

    #[test]
    fn selectable_card_builders_store_state_and_size() {
        let card = SelectableCard::new("plan-pro", "Pro")
            .title("Pro Plus")
            .description("For teams shipping weekly.")
            .selected(true)
            .disabled(true)
            .size(SelectableCardSize::Sm);
        assert_eq!(card.title.as_ref(), "Pro Plus");
        assert_eq!(
            card.description.as_deref(),
            Some("For teams shipping weekly.")
        );
        assert!(card.selected);
        assert!(card.disabled);
        assert_eq!(card.size, SelectableCardSize::Sm);
    }

    #[test]
    fn selectable_card_theme_defaults_match_compiled_tokens() {
        let theme = SelectableCardTheme::default();
        assert_eq!(theme.surface, rgba(0x1e1e1eff));
        assert_eq!(theme.surface_selected, rgba(0x0e2a3aff));
        assert_eq!(theme.border, rgba(0x3e3e42ff));
        assert_eq!(theme.border_selected, rgba(0x007accff));
        assert_eq!(theme.title, rgba(0xffffffff));
        assert_eq!(theme.description, rgba(0x777777ff));
    }
}
