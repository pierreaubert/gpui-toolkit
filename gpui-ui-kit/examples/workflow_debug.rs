//! Workflow Canvas Debug Example
//!
//! A comprehensive example demonstrating the workflow canvas component:
//! - Adding nodes with varying input/output ports
//! - Creating connections between nodes
//! - Selection (single, multi with shift, box selection)
//! - Dragging nodes
//! - Pan (middle mouse or space+drag) and zoom (scroll wheel)
//! - Undo/Redo
//! - Delete selected

use gpui::*;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::workflow::{Position, WorkflowCanvas, WorkflowGraph, WorkflowNodeData};
use gpui_ui_kit::*;

/// Main debug application
pub struct WorkflowDebug {
    canvas: Entity<WorkflowCanvas>,
    status_message: String,
    node_counter: usize,
    entity: Entity<Self>,
}

impl WorkflowDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        // Create initial graph with some example nodes
        let mut graph = WorkflowGraph::new();

        // Add some example nodes
        let node1 = WorkflowNodeData::new("Input Source", Position::new(50.0, 100.0))
            .with_ports(0, 2)
            .with_size(180.0, 80.0);
        let node1_id = node1.id;

        let node2 = WorkflowNodeData::new("Filter", Position::new(300.0, 50.0))
            .with_ports(1, 1)
            .with_size(160.0, 70.0);
        let node2_id = node2.id;

        let node3 = WorkflowNodeData::new("Transform", Position::new(300.0, 180.0))
            .with_ports(1, 1)
            .with_size(160.0, 70.0);
        let node3_id = node3.id;

        let node4 = WorkflowNodeData::new("Output", Position::new(550.0, 100.0))
            .with_ports(2, 0)
            .with_size(160.0, 80.0);

        graph.add_node(node1);
        graph.add_node(node2);
        graph.add_node(node3);
        graph.add_node(node4);

        // Add some connections
        let _ = graph.add_connection(node1_id, 0, node2_id, 0);
        let _ = graph.add_connection(node1_id, 1, node3_id, 0);

        let canvas = cx.new(|cx| WorkflowCanvas::with_graph(graph, cx));

        Self {
            canvas,
            status_message: "Ready. Try adding nodes, creating connections, or dragging!".into(),
            node_counter: 5,
            entity: cx.entity().clone(),
        }
    }

    fn add_node(&mut self, inputs: usize, outputs: usize, cx: &mut Context<Self>) {
        let name = format!("Node {}", self.node_counter);
        self.node_counter += 1;

        // Position new nodes at a varying location
        let x = 100.0 + (self.node_counter as f32 * 50.0) % 400.0;
        let y = 100.0 + (self.node_counter as f32 * 30.0) % 300.0;

        let node = WorkflowNodeData::new(&name, Position::new(x, y))
            .with_ports(inputs, outputs)
            .with_size(160.0, 70.0);

        self.canvas.update(cx, |canvas, cx| {
            canvas.add_node_notify(node, cx);
        });

        self.status_message = format!(
            "Added node '{}' with {} inputs, {} outputs",
            name, inputs, outputs
        );
        cx.notify();
    }

    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        self.canvas.update(cx, |canvas, cx| {
            canvas.delete_selected(cx);
        });
        self.status_message = "Deleted selected items".into();
        cx.notify();
    }

    fn undo(&mut self, cx: &mut Context<Self>) {
        let result = self.canvas.update(cx, |canvas, cx| canvas.undo(cx));
        self.status_message = if result {
            "Undo".into()
        } else {
            "Nothing to undo".into()
        };
        cx.notify();
    }

    fn redo(&mut self, cx: &mut Context<Self>) {
        let result = self.canvas.update(cx, |canvas, cx| canvas.redo(cx));
        self.status_message = if result {
            "Redo".into()
        } else {
            "Nothing to redo".into()
        };
        cx.notify();
    }

    fn clear_all(&mut self, cx: &mut Context<Self>) {
        self.canvas.update(cx, |canvas, cx| {
            canvas.clear(cx);
        });
        self.node_counter = 1;
        self.status_message = "Cleared all nodes and connections".into();
        cx.notify();
    }

    fn reset_viewport(&mut self, cx: &mut Context<Self>) {
        self.canvas.update(cx, |canvas, cx| {
            canvas.reset_viewport(cx);
        });
        self.status_message = "Reset viewport to origin".into();
        cx.notify();
    }

    fn get_stats(&self, cx: &Context<Self>) -> (usize, usize, usize) {
        self.canvas.read(cx).stats()
    }
}

impl Render for WorkflowDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (node_count, connection_count, selected_count) = self.get_stats(cx);
        let entity = self.entity.clone();

        div()
            .id("workflow-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .flex()
            .flex_col()
            // Header with controls
            .child(
                div()
                    .w_full()
                    .px_4()
                    .py_3()
                    .bg(theme.surface)
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_4()
                            .child(Heading::h2("Workflow Canvas Debug"))
                            .child(div().text_sm().text_color(theme.text_muted).child(format!(
                                "Nodes: {} | Connections: {} | Selected: {}",
                                node_count, connection_count, selected_count
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            // Add node buttons
                            .child(
                                Button::new("add-1-1", "Add 1-1")
                                    .size(ButtonSize::Sm)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, cx| this.add_node(1, 1, cx));
                                        }
                                    }),
                            )
                            .child(
                                Button::new("add-2-1", "Add 2-1")
                                    .size(ButtonSize::Sm)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, cx| this.add_node(2, 1, cx));
                                        }
                                    }),
                            )
                            .child(
                                Button::new("add-1-2", "Add 1-2")
                                    .size(ButtonSize::Sm)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, cx| this.add_node(1, 2, cx));
                                        }
                                    }),
                            )
                            .child(
                                Button::new("add-source", "Add Source")
                                    .size(ButtonSize::Sm)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, cx| this.add_node(0, 2, cx));
                                        }
                                    }),
                            )
                            .child(
                                Button::new("add-sink", "Add Sink")
                                    .size(ButtonSize::Sm)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, cx| this.add_node(2, 0, cx));
                                        }
                                    }),
                            )
                            .child(div().w(px(16.0)))
                            // Actions
                            .child(
                                Button::new("delete", "Delete")
                                    .size(ButtonSize::Sm)
                                    .variant(ButtonVariant::Secondary)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, cx| this.delete_selected(cx));
                                        }
                                    }),
                            )
                            .child(
                                Button::new("undo", "Undo")
                                    .size(ButtonSize::Sm)
                                    .variant(ButtonVariant::Secondary)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, cx| this.undo(cx));
                                        }
                                    }),
                            )
                            .child(
                                Button::new("redo", "Redo")
                                    .size(ButtonSize::Sm)
                                    .variant(ButtonVariant::Secondary)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, cx| this.redo(cx));
                                        }
                                    }),
                            )
                            .child(
                                Button::new("reset-view", "Reset View")
                                    .size(ButtonSize::Sm)
                                    .variant(ButtonVariant::Secondary)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, cx| this.reset_viewport(cx));
                                        }
                                    }),
                            )
                            .child(
                                Button::new("clear", "Clear All")
                                    .size(ButtonSize::Sm)
                                    .variant(ButtonVariant::Destructive)
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, cx| {
                                            entity.update(cx, |this, cx| this.clear_all(cx));
                                        }
                                    }),
                            ),
                    ),
            )
            // Instructions bar
            .child(
                div()
                    .w_full()
                    .px_4()
                    .py_2()
                    .bg(theme.surface_hover)
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .flex_row()
                    .gap_6()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .child("Click: Select")
                    .child("Shift+Click: Multi-select")
                    .child("Drag empty: Box select")
                    .child("Drag node: Move")
                    .child("Drag port to port: Connect")
                    .child("Scroll: Zoom")
                    .child("Middle drag: Pan"),
            )
            // Main canvas area
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .relative()
                    .child(self.canvas.clone()),
            )
            // Status bar
            .child(
                div()
                    .w_full()
                    .px_4()
                    .py_2()
                    .bg(theme.surface)
                    .border_t_1()
                    .border_color(theme.border)
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .child(self.status_message.clone()),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Workflow Canvas Debug")
            .size(1200.0, 800.0)
            .scrollable(false)
            .with_theme(true),
        |cx| cx.new(WorkflowDebug::new),
    );
}
