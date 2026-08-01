//! Tooltip component
//!
//! Contextual information displayed on hover.

use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::{Theme, ThemeExt};
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, Render, RenderOnce, StatefulInteractiveElement,
    Styled,
};
use gpui::{AnyElement, App, AppContext, Div, ElementId, SharedString, Stateful, Window, div, px};
use std::hash::{Hash, Hasher};
use std::time::Duration;

/// Tooltip placement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TooltipPlacement {
    /// Above the target
    #[default]
    Top,
    /// Below the target
    Bottom,
    /// Left of the target
    Left,
    /// Right of the target
    Right,
}

/// A tooltip component.
///
/// `Tooltip` is the presentational popup. Use [`WithTooltip`] when the target
/// should own hover tracking and delayed show/hide behavior.
pub struct Tooltip {
    content: SharedString,
    placement: TooltipPlacement,
    delay_ms: u32,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Tooltip {
    /// Create a new tooltip
    pub fn new(content: impl Into<SharedString>) -> Self {
        Self {
            content: content.into(),
            placement: TooltipPlacement::default(),
            delay_ms: 200,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set placement
    pub fn placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Set an explicit ARIA label (overrides content as fallback)
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Tooltip)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Set delay in milliseconds
    pub fn delay(mut self, delay_ms: u32) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    /// Build the tooltip element with theme (to be positioned by parent)
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let mut tooltip = div()
            .absolute()
            .px_2()
            .py_1()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded(px(4.0))
            .shadow_lg()
            .text_xs()
            .text_color(theme.text_primary)
            .whitespace_nowrap();

        // Position based on placement
        match self.placement {
            TooltipPlacement::Top => {
                tooltip = tooltip.bottom_full().left_0().mb_1();
            }
            TooltipPlacement::Bottom => {
                tooltip = tooltip.top_full().left_0().mt_1();
            }
            TooltipPlacement::Left => {
                tooltip = tooltip.right_full().top_0().mr_1();
            }
            TooltipPlacement::Right => {
                tooltip = tooltip.left_full().top_0().ml_1();
            }
        }

        tooltip.child(self.content)
    }
}

impl RenderOnce for Tooltip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Register in accessibility tree
        cx.register_accessible(AccessibilityNode {
            element_id: ElementId::Name("tooltip".into()),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| self.content.clone()),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Tooltip)),
        });

        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}

impl IntoElement for Tooltip {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// A wrapper that shows a tooltip on hover.
///
/// By default the wrapper uses GPUI's native hoverable-tooltip handling, so
/// hover state and the configured delay are self-contained. [`Self::show`]
/// remains available for callers that intentionally control visibility (for
/// example, click-to-toggle help in a component showcase).
pub struct WithTooltip {
    child: AnyElement,
    id: ElementId,
    tooltip: SharedString,
    placement: TooltipPlacement,
    delay_ms: u32,
    show_tooltip: Option<bool>,
}

impl WithTooltip {
    /// Create a new tooltip wrapper
    pub fn new(child: impl IntoElement, tooltip: impl Into<SharedString>) -> Self {
        let tooltip = tooltip.into();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        tooltip.hash(&mut hasher);
        Self {
            child: child.into_any_element(),
            id: ElementId::from(("with-tooltip", hasher.finish())),
            tooltip,
            placement: TooltipPlacement::default(),
            delay_ms: 200,
            show_tooltip: None,
        }
    }

    /// Set a stable id when multiple targets use the same tooltip content.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set placement
    pub fn placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Set the delay before the automatically managed tooltip appears.
    pub fn delay(mut self, delay_ms: u32) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    /// Set whether tooltip is visible (controlled mode)
    pub fn show(mut self, show: bool) -> Self {
        self.show_tooltip = Some(show);
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Stateful<Div> {
        let mut container = div()
            // The wrapper needs a stable GPUI element id for native tooltip
            // state. The global element path still disambiguates repeated
            // wrappers in different parents.
            .id(self.id)
            .relative()
            .child(self.child);

        match self.show_tooltip {
            Some(true) => {
                container = container.child(
                    Tooltip::new(self.tooltip)
                        .placement(self.placement)
                        .build_with_theme(theme),
                );
            }
            Some(false) => {}
            None => {
                let content = self.tooltip.clone();
                let placement = self.placement;
                let delay = self.delay_ms;
                container = container
                    .tooltip_show_delay(Duration::from_millis(delay as u64))
                    .hoverable_tooltip(move |_window, cx| {
                        cx.new(|_| TooltipView {
                            content: content.clone(),
                            placement,
                        })
                        .into()
                    });
            }
        }

        container
    }
}

/// Entity-backed popup used by GPUI's native tooltip state machine.
struct TooltipView {
    content: SharedString,
    placement: TooltipPlacement,
}

impl Render for TooltipView {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: ElementId::Name("tooltip".into()),
            label: self.content.clone(),
            props: AriaProps::with_role(AriaRole::Tooltip),
        });

        let theme = cx.theme();
        Tooltip::new(self.content.clone())
            .placement(self.placement)
            .build_with_theme(&theme)
    }
}

impl RenderOnce for WithTooltip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}

impl IntoElement for WithTooltip {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Tooltip, TooltipPlacement, WithTooltip};
    use gpui::{ParentElement, div};

    #[test]
    fn with_tooltip_defaults_to_native_hover_management() {
        let tooltip = WithTooltip::new(div().child("Target"), "Help");
        assert_eq!(tooltip.delay_ms, 200);
        assert_eq!(tooltip.show_tooltip, None);
    }

    #[test]
    fn with_tooltip_keeps_explicit_control_mode_and_delay() {
        let tooltip = WithTooltip::new(div().child("Target"), "Help")
            .placement(TooltipPlacement::Bottom)
            .delay(450)
            .show(true);
        assert_eq!(tooltip.delay_ms, 450);
        assert_eq!(tooltip.show_tooltip, Some(true));
    }

    #[test]
    fn tooltip_delay_is_preserved_for_target_adapters() {
        let tooltip = Tooltip::new("Help").delay(700);
        assert_eq!(tooltip.delay_ms, 700);
    }
}
