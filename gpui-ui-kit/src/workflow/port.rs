//! Port component for workflow nodes

use super::theme::WorkflowTheme;
use gpui::prelude::*;
use gpui::*;

/// Port direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
}

/// A connection port on a workflow node
#[derive(IntoElement)]
pub struct Port {
    id: ElementId,
    direction: PortDirection,
    index: usize,
    connected: bool,
    valid_target: Option<bool>,
    theme: Option<WorkflowTheme>,
    on_mouse_down: Option<Box<dyn Fn(PortDirection, usize, &mut Window, &mut App) + 'static>>,
    on_mouse_up: Option<Box<dyn Fn(PortDirection, usize, &mut Window, &mut App) + 'static>>,
}

impl Port {
    pub fn new(id: impl Into<ElementId>, direction: PortDirection, index: usize) -> Self {
        Self {
            id: id.into(),
            direction,
            index,
            connected: false,
            valid_target: None,
            theme: None,
            on_mouse_down: None,
            on_mouse_up: None,
        }
    }

    /// Set whether this port is connected
    pub fn connected(mut self, connected: bool) -> Self {
        self.connected = connected;
        self
    }

    /// Set whether this is a valid target for the current connection drag
    pub fn valid_target(mut self, valid: Option<bool>) -> Self {
        self.valid_target = valid;
        self
    }

    /// Set custom theme
    pub fn theme(mut self, theme: WorkflowTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set mouse down handler
    pub fn on_mouse_down(
        mut self,
        handler: impl Fn(PortDirection, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mouse_down = Some(Box::new(handler));
        self
    }

    /// Set mouse up handler
    pub fn on_mouse_up(
        mut self,
        handler: impl Fn(PortDirection, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mouse_up = Some(Box::new(handler));
        self
    }

    fn get_color(&self, theme: &WorkflowTheme) -> Rgba {
        // Priority: valid_target > connected > default
        if let Some(valid) = self.valid_target {
            if valid {
                return theme.port_valid;
            } else {
                return theme.port_invalid;
            }
        }

        match self.direction {
            PortDirection::Input => theme.port_input,
            PortDirection::Output => theme.port_output,
        }
    }
}

impl RenderOnce for Port {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let theme = self.theme.clone().unwrap_or_default();
        let color = self.get_color(&theme);
        let hover_color = theme.port_hover;
        let radius = theme.port_radius;
        let size = radius * 2.0;

        let direction = self.direction;
        let index = self.index;

        let on_mouse_down = self.on_mouse_down;
        let on_mouse_up = self.on_mouse_up;

        let border_color = if self.connected {
            gpui::white()
        } else {
            gpui::transparent_black()
        };

        let mut result = div()
            .id(self.id)
            .size(px(size))
            .rounded_full()
            .bg(color)
            .border_1()
            .border_color(border_color)
            .cursor_pointer()
            .hover(move |s| s.bg(hover_color).border_color(gpui::white()));

        if let Some(handler) = on_mouse_down {
            result = result.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                handler(direction, index, window, cx);
            });
        }

        if let Some(handler) = on_mouse_up {
            result = result.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                handler(direction, index, window, cx);
            });
        }

        result
    }
}
