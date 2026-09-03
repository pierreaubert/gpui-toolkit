//! ScrollArea component
//!
//! A bounded scrollable region (Radix ScrollArea parity): content larger than
//! the configured bounds scrolls on the enabled axes with native scrollbars.
//!
//! # Usage
//!
//! ```ignore
//! ScrollArea::new(list_element)
//!     .max_height(px(320.0))
//!     .axis(ScrollAxis::Vertical)
//! ```

use crate::theme::{Theme, ThemeExt};
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{AnyElement, App, Div, ElementId, Pixels, Stateful, Window, div};

/// Which axes a [`ScrollArea`] scrolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollAxis {
    /// Vertical scrolling only (default).
    #[default]
    Vertical,
    /// Horizontal scrolling only.
    Horizontal,
    /// Scroll on both axes.
    Both,
}

/// A bounded scrollable region.
#[derive(IntoElement)]
pub struct ScrollArea {
    id: ElementId,
    child: AnyElement,
    axis: ScrollAxis,
    max_height: Option<Pixels>,
    max_width: Option<Pixels>,
}

impl ScrollArea {
    /// Create a scroll area wrapping `element`.
    pub fn new(id: impl Into<ElementId>, element: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            child: element.into_any_element(),
            axis: ScrollAxis::default(),
            max_height: None,
            max_width: None,
        }
    }

    /// Set the scrollable axes.
    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;
        self
    }

    /// Cap the viewport height; taller content scrolls vertically.
    pub fn max_height(mut self, height: Pixels) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Cap the viewport width; wider content scrolls horizontally.
    pub fn max_width(mut self, width: Pixels) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Build into an element with the given theme.
    pub fn build_with_theme(self, _theme: &Theme) -> Stateful<Div> {
        let mut container = div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .overflow_hidden()
            .w_full();

        if let Some(max_height) = self.max_height {
            container = container.max_h(max_height);
        }
        if let Some(max_width) = self.max_width {
            container = container.max_w(max_width);
        }

        let mut viewport = div().id((self.id, "viewport")).w_full();
        match self.axis {
            ScrollAxis::Vertical => {
                viewport = viewport.overflow_y_scroll();
            }
            ScrollAxis::Horizontal => {
                viewport = viewport.overflow_x_scroll();
            }
            ScrollAxis::Both => {
                viewport = viewport.overflow_y_scroll().overflow_x_scroll();
            }
        }

        container.child(viewport.child(self.child))
    }
}

impl RenderOnce for ScrollArea {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        self.build_with_theme(&global_theme)
    }
}

#[cfg(test)]
mod tests {
    use super::{ScrollArea, ScrollAxis};
    use crate::theme::Theme;
    use gpui::prelude::ParentElement;
    use gpui::px;

    #[test]
    fn scroll_area_builders_record_axis_and_bounds() {
        let area = ScrollArea::new("scroll", gpui::div().child("content"))
            .axis(ScrollAxis::Both)
            .max_height(px(320.0))
            .max_width(px(480.0));

        assert_eq!(area.axis, ScrollAxis::Both);
        assert_eq!(area.max_height, Some(px(320.0)));
        assert_eq!(area.max_width, Some(px(480.0)));
    }

    #[test]
    fn scroll_area_builds_with_default_theme() {
        let _el = ScrollArea::new("scroll", gpui::div().child("content"))
            .max_height(px(200.0))
            .build_with_theme(&Theme::default());
    }
}
