//! Tooltip component
//!
//! Contextual information displayed on hover.

use crate::theme::{Theme, ThemeExt};
use gpui::prelude::*;
use gpui::*;

/// Tooltip placement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

/// A tooltip component
/// Note: Actual hover behavior requires state management in the parent
pub struct Tooltip {
    content: SharedString,
    placement: TooltipPlacement,
    delay_ms: u32,
}

impl Tooltip {
    /// Create a new tooltip
    pub fn new(content: impl Into<SharedString>) -> Self {
        Self {
            content: content.into(),
            placement: TooltipPlacement::default(),
            delay_ms: 200,
        }
    }

    /// Set placement
    pub fn placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
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

/// A wrapper that shows tooltip on hover
/// Note: Requires state management for hover tracking
pub struct WithTooltip {
    child: AnyElement,
    tooltip: SharedString,
    placement: TooltipPlacement,
    show_tooltip: bool,
}

impl WithTooltip {
    /// Create a new tooltip wrapper
    pub fn new(child: impl IntoElement, tooltip: impl Into<SharedString>) -> Self {
        Self {
            child: child.into_any_element(),
            tooltip: tooltip.into(),
            placement: TooltipPlacement::default(),
            show_tooltip: false,
        }
    }

    /// Set placement
    pub fn placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Set whether tooltip is visible (controlled mode)
    pub fn show(mut self, show: bool) -> Self {
        self.show_tooltip = show;
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let mut container = div().relative().child(self.child);

        if self.show_tooltip {
            container = container.child(
                Tooltip::new(self.tooltip)
                    .placement(self.placement)
                    .build_with_theme(theme),
            );
        }

        container
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
