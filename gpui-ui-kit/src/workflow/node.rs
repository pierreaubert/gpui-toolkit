//! Workflow node component

use super::port::{Port, PortDirection};
use super::state::{NodeId, Position, WorkflowNodeData};
use super::theme::WorkflowTheme;
use crate::theme::ThemeExt;
use gpui::prelude::*;
use gpui::*;

/// Trait for custom node content rendering
pub trait NodeContent: 'static {
    /// Render the interior content of the node
    fn render(&self, node: &WorkflowNodeData, cx: &mut App) -> AnyElement;

    /// Get preferred size for this content
    fn preferred_size(&self) -> (f32, f32) {
        (160.0, 60.0)
    }
}

/// Default node content that just shows the title
pub struct DefaultNodeContent;

impl NodeContent for DefaultNodeContent {
    fn render(&self, node: &WorkflowNodeData, cx: &mut App) -> AnyElement {
        let theme = cx.theme();
        div()
            .p_2()
            .text_sm()
            .text_color(theme.text_primary)
            .child(node.title.clone())
            .into_any_element()
    }
}

/// A workflow node component
#[derive(IntoElement)]
pub struct WorkflowNode {
    id: ElementId,
    node_id: NodeId,
    data: WorkflowNodeData,
    selected: bool,
    dragging: bool,
    theme: Option<WorkflowTheme>,
    content: Option<Box<dyn NodeContent>>,

    // Event handlers
    on_select: Option<Box<dyn Fn(NodeId, bool, &mut Window, &mut App) + 'static>>,
    on_drag_start: Option<Box<dyn Fn(NodeId, Position, &mut Window, &mut App) + 'static>>,
    on_port_mouse_down:
        Option<Box<dyn Fn(NodeId, PortDirection, usize, &mut Window, &mut App) + 'static>>,
    on_port_mouse_up:
        Option<Box<dyn Fn(NodeId, PortDirection, usize, &mut Window, &mut App) + 'static>>,
}

impl WorkflowNode {
    pub fn new(id: impl Into<ElementId>, data: WorkflowNodeData) -> Self {
        let node_id = data.id;
        Self {
            id: id.into(),
            node_id,
            data,
            selected: false,
            dragging: false,
            theme: None,
            content: None,
            on_select: None,
            on_drag_start: None,
            on_port_mouse_down: None,
            on_port_mouse_up: None,
        }
    }

    /// Set whether the node is selected
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set whether the node is being dragged
    pub fn dragging(mut self, dragging: bool) -> Self {
        self.dragging = dragging;
        self
    }

    /// Set custom theme
    pub fn theme(mut self, theme: WorkflowTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set custom content renderer
    pub fn content(mut self, content: impl NodeContent) -> Self {
        self.content = Some(Box::new(content));
        self
    }

    /// Set selection handler
    pub fn on_select(
        mut self,
        handler: impl Fn(NodeId, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }

    /// Set drag start handler
    pub fn on_drag_start(
        mut self,
        handler: impl Fn(NodeId, Position, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_drag_start = Some(Box::new(handler));
        self
    }

    /// Set port mouse down handler
    pub fn on_port_mouse_down(
        mut self,
        handler: impl Fn(NodeId, PortDirection, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_port_mouse_down = Some(Box::new(handler));
        self
    }

    /// Set port mouse up handler
    pub fn on_port_mouse_up(
        mut self,
        handler: impl Fn(NodeId, PortDirection, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_port_mouse_up = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for WorkflowNode {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| WorkflowTheme::from_theme(&cx.theme()));
        let node_id = self.node_id;

        // Calculate header height based on title length
        // Allow 2 lines for long titles
        let title_len = self.data.title.len();
        let chars_per_line = (self.data.width / 8.0) as usize; // Approximate chars that fit per line
        let needs_two_lines = title_len > chars_per_line;
        let header_height = if needs_two_lines {
            theme.node_header_height * 1.6 // Taller header for 2 lines
        } else {
            theme.node_header_height
        };

        // Adjust node height if title needs more space
        let adjusted_height = if needs_two_lines {
            self.data.height.max(header_height + 40.0) // Ensure minimum space for ports
        } else {
            self.data.height
        };

        // Build input ports with scaled theme
        let input_ports: Vec<_> = (0..self.data.input_count)
            .map(|i| {
                Port::new(
                    format!("port-in-{}-{}", node_id, i),
                    PortDirection::Input,
                    i,
                )
                .theme(theme.clone())
            })
            .collect();

        // Build output ports with scaled theme
        let output_ports: Vec<_> = (0..self.data.output_count)
            .map(|i| {
                Port::new(
                    format!("port-out-{}-{}", node_id, i),
                    PortDirection::Output,
                    i,
                )
                .theme(theme.clone())
            })
            .collect();

        let border_color = if self.selected {
            theme.node_border_selected
        } else {
            theme.node_border
        };

        let on_select = self.on_select;
        let on_drag_start = self.on_drag_start;

        div()
            .id(self.id)
            .absolute()
            .left(px(self.data.position.x))
            .top(px(self.data.position.y))
            .w(px(self.data.width))
            .min_h(px(adjusted_height))
            .bg(theme.node_background)
            .border_2()
            .border_color(border_color)
            .rounded(px(theme.node_border_radius))
            .shadow_md()
            .cursor_pointer()
            .when(self.dragging, |el| el.opacity(0.8))
            // Mouse events
            .when_some(on_select, |el, handler| {
                el.on_click(move |event, window, cx| {
                    let shift = event.modifiers().shift;
                    handler(node_id, shift, window, cx);
                })
            })
            .when_some(on_drag_start, |el, handler| {
                el.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    let x: f32 = event.position.x.into();
                    let y: f32 = event.position.y.into();
                    let pos = Position::new(x, y);
                    handler(node_id, pos, window, cx);
                })
            })
            // Node structure
            .child(
                // Header - height adjusts for long text (up to 2 lines)
                div()
                    .w_full()
                    .min_h(px(header_height))
                    .px_2()
                    .py_1()
                    .bg(theme.node_header)
                    .rounded_t(px(theme.node_border_radius - 2.0))
                    .text_size(px(theme.node_header_height * 0.45)) // Scale text with header
                    .line_height(px(theme.node_header_height * 0.55))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.node_text)
                    // Allow text to wrap, limit to 2 lines with ellipsis
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(self.data.title.clone()),
            )
            // Content area with ports
            .child({
                // Calculate content height using adjusted values
                let content_height = adjusted_height - header_height - 4.0;
                let padding = theme.node_content_padding;
                let available = (content_height - 2.0 * padding).max(0.0);

                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .min_h(px(content_height))
                    // Input ports column - use relative positioning to match hit_test.rs
                    .child({
                        let input_count = self.data.input_count;

                        div()
                            .relative()
                            .h(px(content_height))
                            .w(px(theme.port_radius))
                            .children(input_ports.into_iter().enumerate().map(|(i, port)| {
                                let y = if input_count == 0 {
                                    content_height / 2.0
                                } else {
                                    let spacing = available / input_count as f32;
                                    padding + spacing * (i as f32 + 0.5)
                                };
                                div()
                                    .absolute()
                                    .left(px(-theme.port_radius))
                                    .top(px(y - theme.port_radius))
                                    .child(port)
                            }))
                    })
                    // Main content
                    .child(
                        div()
                            .flex_1()
                            .p_2()
                            .child(if let Some(content) = self.content {
                                content.render(&self.data, cx)
                            } else {
                                DefaultNodeContent.render(&self.data, cx)
                            }),
                    )
                    // Output ports column - use relative positioning to match hit_test.rs
                    .child({
                        let output_count = self.data.output_count;

                        div()
                            .relative()
                            .h(px(content_height))
                            .w(px(theme.port_radius))
                            .children(output_ports.into_iter().enumerate().map(|(i, port)| {
                                let y = if output_count == 0 {
                                    content_height / 2.0
                                } else {
                                    let spacing = available / output_count as f32;
                                    padding + spacing * (i as f32 + 0.5)
                                };
                                div()
                                    .absolute()
                                    .right(px(-theme.port_radius))
                                    .top(px(y - theme.port_radius))
                                    .child(port)
                            }))
                    })
            })
    }
}
