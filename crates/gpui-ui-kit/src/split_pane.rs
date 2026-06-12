//! SplitPane component
//!
//! A resizable split view with a draggable divider between two panes.
//!
//! # Usage
//!
//! ```ignore
//! SplitPane::new("main-split")
//!     .direction(SplitDirection::Horizontal)
//!     .first(sidebar_element)
//!     .second(content_element)
//!     .initial_ratio(0.3)
//! ```

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, Div, ElementId, MouseButton, Pixels, Rgba, Stateful, Window, div, px, relative,
};
use gpui_design::DesignSystem;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Split direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitDirection {
    /// Side by side (default)
    #[default]
    Horizontal,
    /// Stacked top/bottom
    Vertical,
}

/// Theme colors for split pane styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct SplitPaneTheme {
    /// Divider gutter background
    #[theme(default = 0x2d2d2dff, from = surface)]
    pub divider_gutter: Rgba,
    /// Divider gutter background when hovered
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub divider_gutter_hover: Rgba,
    /// Divider color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub divider: Rgba,
    /// Divider hover color
    #[theme(default = 0x007accff, from = accent)]
    pub divider_hover: Rgba,
    /// Divider active/dragging color
    #[theme(default = 0x007accff, from = accent)]
    pub divider_active: Rgba,
}

/// Drag state stored in thread-local storage so it survives re-renders.
#[derive(Clone, Copy, Debug)]
struct SplitPaneDragState {
    start_pos: f32,
    start_ratio: f32,
    is_vertical: bool,
}

thread_local! {
    static SPLIT_PANE_DRAG_STATES: RefCell<HashMap<ElementId, SplitPaneDragState>> = RefCell::new(HashMap::new());
}

/// A split pane component with a draggable divider
pub struct SplitPane {
    id: ElementId,
    direction: SplitDirection,
    first: Option<AnyElement>,
    second: Option<AnyElement>,
    ratio: f32,
    min_first: Pixels,
    min_second: Pixels,
    divider_width: Pixels,
    on_resize: Option<Rc<dyn Fn(f32, &mut Window, &mut App) + 'static>>,
    design: Option<Arc<DesignSystem>>,
}

impl SplitPane {
    /// Create a new split pane
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            direction: SplitDirection::default(),
            first: None,
            second: None,
            ratio: 0.5,
            min_first: px(100.0),
            min_second: px(100.0),
            divider_width: px(10.0),
            on_resize: None,
            design: None,
        }
    }

    /// Set split direction
    pub fn direction(mut self, direction: SplitDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set the first (left/top) pane content
    pub fn first(mut self, element: impl IntoElement) -> Self {
        self.first = Some(element.into_any_element());
        self
    }

    /// Set the second (right/bottom) pane content
    pub fn second(mut self, element: impl IntoElement) -> Self {
        self.second = Some(element.into_any_element());
        self
    }

    /// Set initial split ratio (0.0 to 1.0, default 0.5)
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio.clamp(0.0, 1.0);
        self
    }

    /// Set minimum size for the first pane
    pub fn min_first(mut self, min: Pixels) -> Self {
        self.min_first = min;
        self
    }

    /// Set minimum size for the second pane
    pub fn min_second(mut self, min: Pixels) -> Self {
        self.min_second = min;
        self
    }

    /// Set divider width
    pub fn divider_width(mut self, width: Pixels) -> Self {
        self.divider_width = width;
        self
    }

    /// Called when the user drags the divider (receives new ratio)
    pub fn on_resize(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_resize = Some(Rc::new(handler));
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Build the split pane with theme
    pub fn build_with_theme(self, theme: &SplitPaneTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the split pane with explicit theme and design defaults.
    pub fn build_with_theme_and_design(
        self,
        theme: &SplitPaneTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let divider_gutter = theme.divider_gutter;
        let divider_gutter_hover = theme.divider_gutter_hover;
        let divider_color = theme.divider;
        let divider_hover = theme.divider_hover;
        let divider_active = theme.divider_active;
        let handle_long = px(design.interaction.min_touch_target.max(24.0));
        let handle_short = px(design.interaction.border_width.max(2.0));
        let divider_width = self.divider_width.max(px(design.spacing.grid_unit));

        let mut container = div()
            .id(self.id.clone())
            .size_full()
            .flex()
            .overflow_hidden();

        container = match self.direction {
            SplitDirection::Horizontal => container.flex_row(),
            SplitDirection::Vertical => container.flex_col(),
        };

        // First pane
        let first_pane = div().flex_shrink_0().overflow_hidden().children(self.first);

        let first_pane = match self.direction {
            SplitDirection::Horizontal => first_pane
                .h_full()
                .w(relative(self.ratio))
                .min_w(self.min_first),
            SplitDirection::Vertical => first_pane
                .w_full()
                .h(relative(self.ratio))
                .min_h(self.min_first),
        };

        let is_vertical = self.direction == SplitDirection::Horizontal;
        let on_resize = self.on_resize;
        let id = self.id.clone();

        // Divider
        let mut divider = match self.direction {
            SplitDirection::Horizontal => div()
                .id("split-divider")
                .w(divider_width)
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .bg(divider_gutter)
                .border_x_1()
                .border_color(divider_color)
                .cursor_col_resize()
                .child(
                    div()
                        .w(handle_short)
                        .h(handle_long)
                        .rounded(px(1.0))
                        .bg(divider_color),
                )
                .hover(move |s| s.bg(divider_gutter_hover).border_color(divider_hover))
                .active(move |s| s.bg(divider_gutter_hover).border_color(divider_active)),
            SplitDirection::Vertical => div()
                .id("split-divider")
                .h(divider_width)
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .bg(divider_gutter)
                .border_y_1()
                .border_color(divider_color)
                .cursor_row_resize()
                .child(
                    div()
                        .w(handle_long)
                        .h(handle_short)
                        .rounded(px(1.0))
                        .bg(divider_color),
                )
                .hover(move |s| s.bg(divider_gutter_hover).border_color(divider_hover))
                .active(move |s| s.bg(divider_gutter_hover).border_color(divider_active)),
        };

        // Drag start
        if on_resize.is_some() {
            let start_ratio = self.ratio;
            divider = divider.on_mouse_down(MouseButton::Left, move |event, _window, _cx| {
                let pos: f32 = if is_vertical {
                    event.position.x.into()
                } else {
                    event.position.y.into()
                };
                SPLIT_PANE_DRAG_STATES.with(|states| {
                    states.borrow_mut().insert(
                        id.clone(),
                        SplitPaneDragState {
                            start_pos: pos,
                            start_ratio,
                            is_vertical,
                        },
                    );
                });
            });
        }

        // Second pane
        let second_pane = div().flex_1().overflow_hidden().children(self.second);

        let second_pane = match self.direction {
            SplitDirection::Horizontal => second_pane.h_full().min_w(self.min_second),
            SplitDirection::Vertical => second_pane.w_full().min_h(self.min_second),
        };

        container = container
            .child(first_pane)
            .child(divider)
            .child(second_pane);

        // Drag move / end on the container so tracking continues when the
        // cursor leaves the thin divider.
        if let Some(resize_cb) = on_resize {
            let id_move = self.id.clone();
            container = container.on_mouse_move(move |event, window, cx| {
                SPLIT_PANE_DRAG_STATES.with(|states| {
                    if let Some(state) = states.borrow().get(&id_move).copied() {
                        let pos: f32 = if state.is_vertical {
                            event.position.x.into()
                        } else {
                            event.position.y.into()
                        };
                        let viewport: f32 = if state.is_vertical {
                            window.viewport_size().width.into()
                        } else {
                            window.viewport_size().height.into()
                        };
                        let delta = pos - state.start_pos;
                        let new_ratio = (state.start_ratio + delta / viewport).clamp(0.0, 1.0);
                        resize_cb(new_ratio, window, cx);
                    }
                });
            });

            let id_up = self.id.clone();
            container = container.on_mouse_up(MouseButton::Left, move |_event, _window, _cx| {
                SPLIT_PANE_DRAG_STATES.with(|states| {
                    states.borrow_mut().remove(&id_up);
                });
            });
        }

        container
    }
}

impl RenderOnce for SplitPane {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = SplitPaneTheme::from(&global_theme);
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

impl IntoElement for SplitPane {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::SplitPane;

    #[test]
    fn test_split_pane_drag_handlers_do_not_panic() {
        let _el = SplitPane::new("test-split")
            .on_resize(|_ratio, _window, _cx| {})
            .build_with_theme(&super::SplitPaneTheme::default());
    }
}
