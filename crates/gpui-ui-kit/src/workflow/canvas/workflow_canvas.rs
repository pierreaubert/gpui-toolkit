use super::super::bezier::ObstacleRect;
use super::super::history::{
    AddConnectionCommand, AddNodeCommand, ChangePortCountsCommand, CompositeCommand,
    HistoryManager, MoveNodesCommand, RemoveConnectionCommand, RemoveNodeCommand,
};
use super::super::hit_test::{HitTestResult, HitTester};
use super::super::node::WorkflowNode;
use super::super::state::{
    BoxSelection, BulkConnectDrag, CanvasState, Connection, ConnectionDrag, ContextMenuState,
    InteractionMode, LinkType, NodeDragState, NodeId, Position, SelectionState, ViewportState,
    WorkflowGraph, WorkflowNodeData,
};
use super::super::theme::WorkflowTheme;
use super::draw::draw_connection;
use super::draw::draw_connection_preview;
use super::misc::port_screen_position;
use super::types::ClipboardData;
use super::types::GraphChangeCallback;
use super::types::NodeDoubleClickCallback;
use super::types::NodeMenuItemsCallback;
use super::types::SharedNodeMenuSelectCallback;
use crate::menu::{Menu, MenuItem};
use crate::theme::ThemeExt;
use gpui::{
    App, Context, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render, ScrollDelta,
    ScrollWheelEvent, SharedString, StatefulInteractiveElement, Styled, Window, canvas, div, px,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

thread_local! {
    /// Reusable obstacle buffer for connection routing. Retains capacity across
    /// frames so we avoid allocating a fresh `Vec` for every connection.
    static CONNECTION_OBSTACLES: RefCell<Vec<ObstacleRect>> = const { RefCell::new(Vec::new()) };
}

/// Workflow canvas component
///
/// A ReactFlow-like canvas for editing node-based workflows.
pub struct WorkflowCanvas {
    pub(super) state: CanvasState,
    pub(super) history: HistoryManager,
    pub(super) hit_tester: HitTester,
    pub(super) theme: Option<WorkflowTheme>,
    /// Canvas element origin in window coordinates (updated during paint)
    pub(super) canvas_origin: Position,
    /// Focus handle for keyboard events
    pub(super) focus_handle: FocusHandle,
    /// Clipboard for copy/paste
    pub(super) clipboard: Option<String>,
    /// Custom context menu items (if None, uses default menu)
    pub(super) custom_menu_items: Option<Vec<MenuItem>>,
    /// Resolves which menu items to show when right-clicking on a node.
    /// Returning None for a given node falls through to the canvas-level
    /// menu, so different node types can opt out of the per-node menu.
    pub(super) node_menu_items: Option<NodeMenuItemsCallback>,
    /// Callback for node double-click
    pub(super) on_node_double_click: Option<NodeDoubleClickCallback>,
    /// Callback fired after the underlying graph mutates structurally
    pub(super) on_graph_change: Option<GraphChangeCallback>,
    /// Callback for node-context-menu item selection
    pub(super) on_node_menu_select: Option<SharedNodeMenuSelectCallback>,
}

impl WorkflowCanvas {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            state: CanvasState::new(),
            history: HistoryManager::new(),
            hit_tester: HitTester::new(),
            theme: None,
            canvas_origin: Position::new(0.0, 0.0),
            focus_handle: cx.focus_handle(),
            clipboard: None,
            custom_menu_items: None,
            node_menu_items: None,
            on_node_double_click: None,
            on_graph_change: None,
            on_node_menu_select: None,
        }
    }

    /// Create with an existing graph
    pub fn with_graph(graph: WorkflowGraph, cx: &mut Context<Self>) -> Self {
        Self {
            state: CanvasState::with_graph(graph),
            history: HistoryManager::new(),
            hit_tester: HitTester::new(),
            theme: None,
            canvas_origin: Position::new(0.0, 0.0),
            focus_handle: cx.focus_handle(),
            clipboard: None,
            custom_menu_items: None,
            node_menu_items: None,
            on_node_double_click: None,
            on_graph_change: None,
            on_node_menu_select: None,
        }
    }

    /// Set custom theme
    pub fn set_theme(&mut self, theme: WorkflowTheme) {
        self.theme = Some(theme);
    }

    /// Set custom context menu items
    /// These will replace the default menu items when right-clicking on the canvas
    pub fn set_menu_items(&mut self, items: Vec<MenuItem>) {
        self.custom_menu_items = Some(items);
    }

    /// Set the resolver that returns context-menu items for a right-clicked
    /// node, or `None` to fall back to the canvas-level menu for that node.
    /// This lets different kinds of nodes (e.g., plugin vs special I/O)
    /// surface different menus without the canvas needing domain knowledge.
    /// Combine with `set_on_node_menu_select` to handle the per-node action.
    pub fn set_node_menu_items(
        &mut self,
        callback: impl Fn(&super::super::state::WorkflowNodeData) -> Option<Vec<MenuItem>> + 'static,
    ) {
        self.node_menu_items = Some(Box::new(callback));
    }

    /// Set callback fired when a node-context-menu item is selected.
    /// The callback receives the menu item id and the right-clicked node id.
    pub fn set_on_node_menu_select(
        &mut self,
        callback: impl Fn(&SharedString, NodeId, &mut Window, &mut App) + 'static,
    ) {
        self.on_node_menu_select = Some(Rc::new(Box::new(callback)));
    }

    /// Set callback for node double-click events
    pub fn set_on_node_double_click(
        &mut self,
        callback: impl Fn(NodeId, &mut Window, &mut App) + 'static,
    ) {
        self.on_node_double_click = Some(Box::new(callback));
    }

    /// Set callback fired whenever the canvas graph mutates structurally
    /// (node added/removed, connection added/removed, paste, undo/redo,
    /// clear). The parent uses this to reconcile its own data model.
    pub fn set_on_graph_change(&mut self, callback: impl Fn(&mut App) + 'static) {
        self.on_graph_change = Some(Box::new(callback));
    }

    /// Internal helper: fire the graph-change observer if one is set.
    pub(super) fn fire_graph_change(&self, cx: &mut App) {
        if let Some(ref callback) = self.on_graph_change {
            callback(cx);
        }
    }

    // === Public API ===

    /// Get the current graph
    pub fn graph(&self) -> &WorkflowGraph {
        &self.state.graph
    }

    /// Get mutable access to the graph (for direct modifications)
    pub fn graph_mut(&mut self) -> &mut WorkflowGraph {
        &mut self.state.graph
    }

    /// Get the current selection
    pub fn selection(&self) -> &SelectionState {
        &self.state.selection
    }

    /// Get viewport state
    pub fn viewport(&self) -> &ViewportState {
        &self.state.viewport
    }

    /// Add a node at the given position
    pub fn add_node(&mut self, node: WorkflowNodeData) {
        self.history
            .execute(Box::new(AddNodeCommand { node }), &mut self.state.graph);
    }

    /// Remove selected nodes
    pub fn remove_selected(&mut self) {
        let selected: Vec<NodeId> = self
            .state
            .selection
            .selected_nodes
            .iter()
            .copied()
            .collect();

        for node_id in selected {
            if let Some(node) = self.state.graph.nodes.get(&node_id).cloned() {
                // Collect all connections to/from this node
                let connections: Vec<Connection> = self
                    .state
                    .graph
                    .connections
                    .iter()
                    .filter(|c| c.from_node == node_id || c.to_node == node_id)
                    .cloned()
                    .collect();

                self.history.execute(
                    Box::new(RemoveNodeCommand { node, connections }),
                    &mut self.state.graph,
                );
            }
        }

        // Also remove selected connections
        let selected_conns: Vec<_> = self
            .state
            .selection
            .selected_connections
            .iter()
            .copied()
            .collect();
        for conn_id in selected_conns {
            if let Some(conn) = self
                .state
                .graph
                .connections
                .iter()
                .find(|c| c.id == conn_id)
                .cloned()
            {
                self.history.execute(
                    Box::new(RemoveConnectionCommand { connection: conn }),
                    &mut self.state.graph,
                );
            }
        }

        self.state.selection.clear();
    }

    /// Undo last action (without notification)
    pub fn undo_internal(&mut self) -> bool {
        self.history.undo(&mut self.state.graph)
    }

    /// Redo last undone action (without notification)
    pub fn redo_internal(&mut self) -> bool {
        self.history.redo(&mut self.state.graph)
    }

    /// Undo last action (with notification)
    pub fn undo(&mut self, cx: &mut Context<Self>) -> bool {
        let result = self.history.undo(&mut self.state.graph);
        if result {
            self.fire_graph_change(cx);
            cx.notify();
        }
        result
    }

    /// Redo last undone action (with notification)
    pub fn redo(&mut self, cx: &mut Context<Self>) -> bool {
        let result = self.history.redo(&mut self.state.graph);
        if result {
            self.fire_graph_change(cx);
            cx.notify();
        }
        result
    }

    /// Select all nodes
    pub fn select_all(&mut self) {
        self.state.selection.selected_nodes = self.state.graph.nodes.keys().copied().collect();
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.state.selection.clear();
    }

    /// Copy selected nodes to clipboard (returns serialized data)
    pub fn copy_selection(&self) -> Option<String> {
        if self.state.selection.is_empty() {
            return None;
        }

        let selected_nodes: Vec<_> = self
            .state
            .selection
            .selected_nodes
            .iter()
            .filter_map(|id| self.state.graph.nodes.get(id).cloned())
            .collect();

        // Find connections between selected nodes only
        let internal_connections: Vec<_> = self
            .state
            .graph
            .connections
            .iter()
            .filter(|c| {
                self.state.selection.selected_nodes.contains(&c.from_node)
                    && self.state.selection.selected_nodes.contains(&c.to_node)
            })
            .cloned()
            .collect();

        let clipboard_data = ClipboardData {
            nodes: selected_nodes,
            connections: internal_connections,
        };

        serde_json::to_string(&clipboard_data).ok()
    }

    /// Paste nodes from clipboard at given position
    pub fn paste(&mut self, clipboard_json: &str, mouse_pos: Position) {
        let clipboard_data: ClipboardData = match serde_json::from_str(clipboard_json) {
            Ok(data) => data,
            Err(_) => return,
        };

        if clipboard_data.nodes.is_empty() {
            return;
        }

        // Calculate bounding box center
        let (min_x, min_y, max_x, max_y) = clipboard_data.nodes.iter().fold(
            (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
            |(min_x, min_y, max_x, max_y), node| {
                (
                    min_x.min(node.position.x),
                    min_y.min(node.position.y),
                    max_x.max(node.position.x + node.width),
                    max_y.max(node.position.y + node.height),
                )
            },
        );
        let center = Position::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);

        // Map old IDs to new IDs
        let mut id_map: HashMap<NodeId, NodeId> = HashMap::new();

        // Clear current selection
        self.state.selection.clear();

        // Add nodes with new IDs and offset positions
        for node in &clipboard_data.nodes {
            let mut new_node = node.clone();
            new_node.id = NodeId::new_v4();
            new_node.position.x += mouse_pos.x - center.x;
            new_node.position.y += mouse_pos.y - center.y;

            id_map.insert(node.id, new_node.id);
            self.state.selection.selected_nodes.insert(new_node.id);

            self.history.execute(
                Box::new(AddNodeCommand { node: new_node }),
                &mut self.state.graph,
            );
        }

        // Recreate connections with new IDs
        for conn in &clipboard_data.connections {
            if let (Some(&new_from), Some(&new_to)) =
                (id_map.get(&conn.from_node), id_map.get(&conn.to_node))
            {
                let new_conn = Connection::new(new_from, conn.from_port, new_to, conn.to_port);
                self.history.execute(
                    Box::new(AddConnectionCommand {
                        connection: new_conn,
                    }),
                    &mut self.state.graph,
                );
            }
        }
    }

    // === Methods with notification (for use with Entity) ===

    /// Add a node (with notification)
    pub fn add_node_notify(&mut self, node: WorkflowNodeData, cx: &mut Context<Self>) {
        self.add_node(node);
        self.fire_graph_change(cx);
        cx.notify();
    }

    /// Delete selected items (with notification)
    pub fn delete_selected(&mut self, cx: &mut Context<Self>) {
        self.remove_selected();
        self.fire_graph_change(cx);
        cx.notify();
    }

    /// Clear all nodes and connections
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.state.graph.nodes.clear();
        self.state.graph.connections.clear();
        self.state.selection.clear();
        self.history.clear();
        self.fire_graph_change(cx);
        cx.notify();
    }

    /// Reset viewport to origin with zoom 1.0
    pub fn reset_viewport(&mut self, cx: &mut Context<Self>) {
        self.state.viewport.offset = Position::new(0.0, 0.0);
        self.state.viewport.zoom = 1.0;
        cx.notify();
    }

    /// Get statistics: (node_count, connection_count, selected_count)
    pub fn stats(&self) -> (usize, usize, usize) {
        (
            self.state.graph.nodes.len(),
            self.state.graph.connections.len(),
            self.state.selection.selected_nodes.len()
                + self.state.selection.selected_connections.len(),
        )
    }

    // === Internal event handlers ===

    pub(super) fn handle_mouse_down(
        &mut self,
        position: Position,
        shift: bool,
        alt: bool,
        cx: &mut Context<Self>,
    ) {
        // Clear context menu on any click if visible
        if self.state.context_menu.is_some() {
            self.state.context_menu = None;
            cx.notify();
        }

        // position is in screen coordinates (relative to canvas element)
        let canvas_pos = self.state.viewport.screen_to_canvas(position.x, position.y);

        // Hit test uses screen coordinates for accurate port detection
        let hit = self.hit_tester.hit_test_with_viewport(
            position,
            &self.state.graph,
            &self.state.viewport,
        );

        match hit {
            HitTestResult::OutputPort(node_id, port_idx) => {
                self.state.mode = InteractionMode::CreatingConnection;
                self.state.connection_drag = Some(ConnectionDrag {
                    from_node: node_id,
                    from_port: port_idx,
                    is_output: true,
                    current_position: canvas_pos,
                });
            }
            HitTestResult::InputPort(node_id, port_idx) => {
                self.state.mode = InteractionMode::CreatingConnection;
                self.state.connection_drag = Some(ConnectionDrag {
                    from_node: node_id,
                    from_port: port_idx,
                    is_output: false,
                    current_position: canvas_pos,
                });
            }
            HitTestResult::Node(node_id) => {
                if alt {
                    // Alt+click on node body: start bulk-connect drag
                    self.state.mode = InteractionMode::BulkConnecting;
                    self.state.bulk_connect_drag = Some(BulkConnectDrag {
                        from_node: node_id,
                        current_position: canvas_pos,
                    });
                } else {
                    if shift {
                        self.state.selection.toggle_node(node_id);
                    } else if !self.state.selection.is_node_selected(node_id) {
                        self.state.selection.clear();
                        self.state.selection.select_node(node_id, false);
                    }

                    // Start dragging
                    let dragging_nodes: Vec<NodeId> = self
                        .state
                        .selection
                        .selected_nodes
                        .iter()
                        .copied()
                        .collect();
                    let original_positions: HashMap<NodeId, Position> = dragging_nodes
                        .iter()
                        .filter_map(|id| self.state.graph.nodes.get(id).map(|n| (*id, n.position)))
                        .collect();

                    self.state.mode = InteractionMode::DraggingNodes;
                    self.state.node_drag = Some(NodeDragState {
                        dragging_nodes,
                        start_mouse: canvas_pos,
                        original_positions,
                    });
                }
            }
            HitTestResult::Connection(conn_id) => {
                self.state.selection.select_connection(conn_id, shift);
            }
            HitTestResult::Canvas | HitTestResult::None => {
                if !shift {
                    self.state.selection.clear();
                }
                // Start box selection
                self.state.mode = InteractionMode::BoxSelecting;
                self.state.box_selection = Some(BoxSelection {
                    start: canvas_pos,
                    current: canvas_pos,
                });
            }
        }
        cx.notify();
    }

    pub(super) fn handle_mouse_move(&mut self, position: Position, cx: &mut Context<Self>) {
        let canvas_pos = self.state.viewport.screen_to_canvas(position.x, position.y);

        match self.state.mode {
            InteractionMode::DraggingNodes => {
                if let Some(ref drag) = self.state.node_drag {
                    let dx = canvas_pos.x - drag.start_mouse.x;
                    let dy = canvas_pos.y - drag.start_mouse.y;

                    for node_id in &drag.dragging_nodes {
                        if let (Some(node), Some(original)) = (
                            self.state.graph.nodes.get_mut(node_id),
                            drag.original_positions.get(node_id),
                        ) {
                            node.position.x = original.x + dx;
                            node.position.y = original.y + dy;
                        }
                    }
                    cx.notify();
                }
            }
            InteractionMode::CreatingConnection => {
                if let Some(ref mut drag) = self.state.connection_drag {
                    drag.current_position = canvas_pos;
                    cx.notify();
                }
            }
            InteractionMode::BoxSelecting => {
                if let Some(ref mut selection) = self.state.box_selection {
                    selection.current = canvas_pos;
                    cx.notify();
                }
            }
            InteractionMode::BulkConnecting => {
                if let Some(ref mut drag) = self.state.bulk_connect_drag {
                    drag.current_position = canvas_pos;
                    cx.notify();
                }
            }
            InteractionMode::Panning => {
                // Handled separately via middle mouse
            }
            InteractionMode::None => {}
        }
    }

    pub(super) fn handle_mouse_up(&mut self, position: Position, cx: &mut Context<Self>) {
        // Track whether the graph changed structurally during this mouse-up
        // (connection added, bulk-connect, or node moved). We notify the
        // observer once at the end so observers get a single signal even if
        // multiple commands ran.
        let mut graph_changed = false;
        // position is in screen coordinates (relative to canvas element)

        match self.state.mode {
            InteractionMode::DraggingNodes => {
                if let Some(drag) = self.state.node_drag.take() {
                    // Create move command for undo
                    let moves: Vec<_> = drag
                        .dragging_nodes
                        .iter()
                        .filter_map(|id| {
                            let node = self.state.graph.nodes.get(id)?;
                            let original = drag.original_positions.get(id)?;
                            Some((*id, *original, node.position))
                        })
                        .collect();

                    if !moves.is_empty() {
                        // Check if nodes actually moved
                        let moved = moves
                            .iter()
                            .any(|(_, old, new)| old.x != new.x || old.y != new.y);
                        if moved {
                            // Don't execute, just record (positions are already updated)
                            self.history.record(Box::new(MoveNodesCommand { moves }));
                            // Position changes are visual-only — no engine
                            // reconfiguration needed, so don't fire the
                            // graph-change observer.
                        }
                    }
                }
            }
            InteractionMode::CreatingConnection => {
                if let Some(drag) = self.state.connection_drag.take() {
                    // Hit test uses screen coordinates for accurate port detection
                    let hit = self.hit_tester.hit_test_with_viewport(
                        position,
                        &self.state.graph,
                        &self.state.viewport,
                    );

                    // Check if we dropped on a valid target port
                    let target = match (drag.is_output, hit) {
                        (true, HitTestResult::InputPort(node_id, port_idx)) => {
                            Some((node_id, port_idx))
                        }
                        (false, HitTestResult::OutputPort(node_id, port_idx)) => {
                            Some((node_id, port_idx))
                        }
                        _ => None,
                    };

                    if let Some((target_node, target_port)) = target {
                        let (from_node, from_port, to_node, to_port) = if drag.is_output {
                            (drag.from_node, drag.from_port, target_node, target_port)
                        } else {
                            (target_node, target_port, drag.from_node, drag.from_port)
                        };

                        // Try to create the connection
                        if self
                            .state
                            .graph
                            .add_connection(from_node, from_port, to_node, to_port)
                            .is_ok()
                        {
                            // Get the connection we just added
                            if let Some(conn) = self.state.graph.connections.last().cloned() {
                                // Record for undo
                                self.history
                                    .record(Box::new(AddConnectionCommand { connection: conn }));
                                graph_changed = true;
                            }
                        }
                    }
                }
            }
            InteractionMode::BulkConnecting => {
                if let Some(drag) = self.state.bulk_connect_drag.take() {
                    let hit = self.hit_tester.hit_test_with_viewport(
                        position,
                        &self.state.graph,
                        &self.state.viewport,
                    );
                    if let HitTestResult::Node(target_id) = hit
                        && target_id != drag.from_node
                        && self.execute_bulk_connect(drag.from_node, target_id)
                    {
                        graph_changed = true;
                    }
                }
            }
            InteractionMode::BoxSelecting => {
                if let Some(selection) = self.state.box_selection.take() {
                    // Box selection rect is in canvas coordinates
                    let (x, y, w, h) = selection.rect();
                    let nodes = self.hit_tester.nodes_in_rect(x, y, w, h, &self.state.graph);
                    for node_id in nodes {
                        self.state.selection.selected_nodes.insert(node_id);
                    }
                }
            }
            _ => {}
        }

        self.state.mode = InteractionMode::None;
        if graph_changed {
            self.fire_graph_change(cx);
        }
        cx.notify();
    }

    pub(super) fn handle_right_click(&mut self, position: Position, cx: &mut Context<Self>) {
        // Show context menu at click position.
        // `position` is in screen coordinates relative to the canvas element;
        // the menu is rendered as a child of the relative canvas div, so the
        // relative position can be used directly for menu placement.
        //
        // We hit-test using the same screen-coordinate path as the mouse-down
        // handler so that node body, ports, and connections all resolve
        // consistently. If a node is hit, store its id on the menu state so
        // the parent can dispatch per-node actions.
        let hit = self.hit_tester.hit_test_with_viewport(
            position,
            &self.state.graph,
            &self.state.viewport,
        );
        let target_node = match hit {
            HitTestResult::Node(id)
            | HitTestResult::InputPort(id, _)
            | HitTestResult::OutputPort(id, _) => Some(id),
            _ => None,
        };

        self.state.context_menu = Some(ContextMenuState {
            position,
            visible: true,
            target_node,
        });
        cx.notify();
    }

    pub(super) fn handle_double_click(
        &mut self,
        position: Position,
        window: &mut Window,
        cx: &mut App,
    ) {
        // position is in screen coordinates (relative to canvas element)
        // Hit test using the actual viewport so zoom and pan are accounted for
        let hit_result = self.hit_tester.hit_test_with_viewport(
            position,
            &self.state.graph,
            &self.state.viewport,
        );

        // If a node was double-clicked and we have a callback, call it
        if let HitTestResult::Node(node_id) = hit_result
            && let Some(ref callback) = self.on_node_double_click
        {
            callback(node_id, window, cx);
        }
    }

    /// Bulk-connect: wire source output[i] → target input[i] for all matching ports.
    /// Grows the target's input_count if allowed by max_input_count.
    /// Returns `true` if any commands were actually executed against the graph.
    pub(super) fn execute_bulk_connect(&mut self, source_id: NodeId, target_id: NodeId) -> bool {
        let source = match self.state.graph.nodes.get(&source_id) {
            Some(n) => n.clone(),
            None => return false,
        };
        let target = match self.state.graph.nodes.get(&target_id) {
            Some(n) => n.clone(),
            None => return false,
        };

        let source_outputs = source.output_count;
        if source_outputs == 0 {
            return false;
        }

        // Would adding any edge create a cycle?
        if self.state.graph.would_create_cycle(source_id, target_id) {
            return false;
        }

        // How many connections can we make?
        let max_target = target.max_input_count.unwrap_or(usize::MAX);
        let connect_count = source_outputs.min(max_target);
        if connect_count == 0 {
            return false;
        }

        let mut composite = CompositeCommand::new("Bulk connect");

        let output_grow_count = target
            .max_output_count
            .filter(|_| target.output_count > 0)
            .map(|max| connect_count.min(max))
            .filter(|&count| count > target.output_count);

        // Grow target input_count, and grow target output_count too when
        // the node explicitly allows variable outputs. This keeps adaptive
        // pass-through nodes wide after bulk-connecting from a multi-channel
        // source, while fixed-output nodes such as downmixers stay clamped.
        if connect_count > target.input_count || output_grow_count.is_some() {
            let old_height = target.height;
            let new_input_count = target.input_count.max(connect_count);
            let new_output_count = output_grow_count.unwrap_or(target.output_count);
            let ports = new_input_count.max(new_output_count);
            let new_height = old_height.max(48.0 + ports as f32 * 16.0);

            composite.add(Box::new(ChangePortCountsCommand {
                node_id: target_id,
                old_input_count: target.input_count,
                new_input_count,
                old_output_count: target.output_count,
                new_output_count,
                old_height,
                new_height,
            }));
        }

        // Create connections for each matching index
        for i in 0..connect_count {
            let already_exists = self.state.graph.connections.iter().any(|c| {
                c.from_node == source_id
                    && c.from_port == i
                    && c.to_node == target_id
                    && c.to_port == i
            });
            if already_exists {
                continue;
            }
            let conn = Connection::new(source_id, i, target_id, i);
            composite.add(Box::new(AddConnectionCommand { connection: conn }));
        }

        if composite.commands.is_empty() {
            return false;
        }

        self.history
            .execute(Box::new(composite), &mut self.state.graph);
        true
    }

    pub(super) fn handle_add_node_menu(
        &mut self,
        node_type: &SharedString,
        cx: &mut Context<Self>,
    ) {
        if let Some(menu_state) = &self.state.context_menu {
            // Position new node at the click location (converted to canvas coords)
            let click_pos = menu_state.position;
            let canvas_pos = self
                .state
                .viewport
                .screen_to_canvas(click_pos.x, click_pos.y);

            let node = match node_type.as_ref() {
                "input" => WorkflowNodeData::new("Input Source", canvas_pos).with_ports(0, 1),
                "filter" => WorkflowNodeData::new("Filter", canvas_pos).with_ports(1, 1),
                "transform" => WorkflowNodeData::new("Transform", canvas_pos).with_ports(1, 1),
                "mix" => WorkflowNodeData::new("Mix", canvas_pos).with_ports(2, 1),
                "output" => WorkflowNodeData::new("Output", canvas_pos).with_ports(1, 0),
                "process" => WorkflowNodeData::new("Process", canvas_pos),
                _ => WorkflowNodeData::new("Node", canvas_pos),
            };

            self.add_node(node);
            self.state.context_menu = None;
            self.fire_graph_change(cx);
            cx.notify();
        }
    }

    pub(super) fn handle_scroll(&mut self, delta: f32, position: Position, cx: &mut Context<Self>) {
        self.state.viewport.zoom_at(delta, position.x, position.y);
        cx.notify();
    }

    pub(super) fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;

        match &event.keystroke.key {
            // Delete selected
            key if (key == "backspace" || key == "delete") && !self.state.selection.is_empty() => {
                self.remove_selected();
                self.fire_graph_change(cx);
                cx.notify();
            }
            // Ctrl+Z or Cmd+Z: Undo
            key if key == "z" && modifiers.platform && !modifiers.shift => {
                self.undo(cx);
            }
            // Ctrl+Shift+Z or Cmd+Shift+Z: Redo
            key if key == "z" && modifiers.platform && modifiers.shift => {
                self.redo(cx);
            }
            // Ctrl+Y or Cmd+Y: Redo (alternative)
            key if key == "y" && modifiers.platform => {
                self.redo(cx);
            }
            // Ctrl+C or Cmd+C: Copy
            key if key == "c" && modifiers.platform => {
                if let Some(data) = self.copy_selection() {
                    self.clipboard = Some(data);
                }
            }
            // Ctrl+X or Cmd+X: Cut
            key if key == "x" && modifiers.platform => {
                if let Some(data) = self.copy_selection() {
                    self.clipboard = Some(data);
                    self.remove_selected();
                    self.fire_graph_change(cx);
                    cx.notify();
                }
            }
            // Ctrl+V or Cmd+V: Paste
            key if key == "v" && modifiers.platform => {
                if let Some(ref data) = self.clipboard.clone() {
                    // Paste at center of viewport
                    let center = Position::new(
                        self.state.viewport.size.0 / 2.0,
                        self.state.viewport.size.1 / 2.0,
                    );
                    let canvas_center = self.state.viewport.screen_to_canvas(center.x, center.y);
                    self.paste(data, canvas_center);
                    self.fire_graph_change(cx);
                    cx.notify();
                }
            }
            // Ctrl+A or Cmd+A: Select all
            key if key == "a" && modifiers.platform => {
                self.select_all();
                cx.notify();
            }
            // Escape: Clear selection or cancel operation
            key if key == "escape" => {
                if self.state.context_menu.is_some() {
                    self.state.context_menu = None;
                } else if self.state.mode != InteractionMode::None {
                    // Cancel current operation
                    self.state.mode = InteractionMode::None;
                    self.state.node_drag = None;
                    self.state.connection_drag = None;
                    self.state.box_selection = None;
                } else {
                    self.state.selection.clear();
                }
                cx.notify();
            }
            _ => {}
        }
    }
}

/// GPUI View implementation
impl Render for WorkflowCanvas {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| WorkflowTheme::from_theme(&cx.theme()));

        let viewport = self.state.viewport;
        let scaled_theme = theme.scale(viewport.zoom);

        // Build connection render data with screen-space port positions
        let connections: Arc<[_]> = Arc::from(
            self.state
                .graph
                .connections
                .iter()
                .filter_map(|conn| {
                    let from_node = self.state.graph.nodes.get(&conn.from_node)?;
                    let to_node = self.state.graph.nodes.get(&conn.to_node)?;
                    // Calculate port positions in screen coordinates (not canvas coordinates)
                    let from_pos =
                        port_screen_position(from_node, conn.from_port, false, &viewport);
                    let to_pos = port_screen_position(to_node, conn.to_port, true, &viewport);
                    let selected = self.state.selection.is_connection_selected(conn.id);
                    let link_type = conn.link_type;
                    Some((
                        from_pos,
                        to_pos,
                        selected,
                        link_type,
                        conn.from_node,
                        conn.to_node,
                    ))
                })
                .collect::<Vec<_>>(),
        );

        // Collect node bounding rects in screen coordinates for obstacle avoidance
        let node_screen_rects: Arc<[_]> = Arc::from(
            self.state
                .graph
                .nodes
                .values()
                .map(|node| {
                    let sp = viewport.canvas_to_screen(&node.position);
                    (
                        node.id,
                        ObstacleRect::new(
                            sp.x,
                            sp.y,
                            node.width * viewport.zoom,
                            node.height * viewport.zoom,
                        ),
                    )
                })
                .collect::<Vec<_>>(),
        );

        let connection_drag = self.state.connection_drag.clone();
        let bulk_connect_drag = self.state.bulk_connect_drag.clone();
        let graph = Arc::new(self.state.graph.clone());

        let conn_color = theme.connection_color;
        let conn_selected = theme.connection_selected;
        let conn_preview = theme.connection_preview;
        // Scale connection widths
        let conn_width_fat = scaled_theme.connection_width;
        let conn_width_thin = scaled_theme.connection_width_thin;
        // Port radius for shortening connection lines
        let port_radius = scaled_theme.port_radius;

        // Use entity to update canvas origin during prepaint
        let entity = cx.entity().clone();

        // Build connection lines using canvas
        let connections_element = canvas(
            move |bounds, _, cx| {
                // Update canvas origin for mouse event coordinate translation
                let origin_x: f32 = bounds.origin.x.into();
                let origin_y: f32 = bounds.origin.y.into();
                entity.update(cx, |this, _| {
                    this.canvas_origin = Position::new(origin_x, origin_y);
                });
                (
                    Arc::clone(&connections),
                    connection_drag.clone(),
                    bulk_connect_drag.clone(),
                    Arc::clone(&graph),
                    bounds,
                    Arc::clone(&node_screen_rects),
                )
            },
            move |_,
                  (
                connections,
                connection_drag,
                bulk_connect_drag,
                graph,
                bounds,
                node_screen_rects,
            ),
                  window,
                  _| {
                // Use fresh bounds from callback - bounds.origin gives us the canvas element position
                let origin_x: f32 = bounds.origin.x.into();
                let origin_y: f32 = bounds.origin.y.into();

                // Draw connections - positions are already in screen coordinates
                // Shorten lines by port_radius at each end so they don't overlap ports
                for (from_pos, to_pos, selected, link_type, from_node_id, to_node_id) in
                    connections.iter()
                {
                    let color = if *selected { conn_selected } else { conn_color };
                    let width = match link_type {
                        LinkType::Fat => conn_width_fat,
                        LinkType::Thin => conn_width_thin,
                    };

                    // Obstacles = all nodes except the connection's source and target.
                    // Reuse a thread-local buffer instead of allocating per connection.
                    CONNECTION_OBSTACLES.with(|obstacles| {
                        let mut obstacles = obstacles.borrow_mut();
                        obstacles.clear();
                        obstacles.extend(
                            node_screen_rects
                                .iter()
                                .filter(|(id, _)| id != from_node_id && id != to_node_id)
                                .map(|(_, r)| *r),
                        );

                        draw_connection(
                            window,
                            *from_pos,
                            *to_pos,
                            color,
                            width,
                            port_radius,
                            &obstacles,
                            origin_x,
                            origin_y,
                        );
                    });
                }

                // Draw connection preview
                if let Some(ref drag) = connection_drag
                    && let Some(from_node) = graph.nodes.get(&drag.from_node)
                {
                    // Calculate port position in screen coordinates
                    let port_screen_pos = port_screen_position(
                        from_node,
                        drag.from_port,
                        !drag.is_output, // is_input is opposite of is_output
                        &viewport,
                    );

                    // Convert drag position from canvas to screen coordinates
                    let drag_screen_pos = viewport.canvas_to_screen(&drag.current_position);

                    let (from, to) = if drag.is_output {
                        (port_screen_pos, drag_screen_pos)
                    } else {
                        (drag_screen_pos, port_screen_pos)
                    };

                    // For preview, only shorten the port end (not the mouse cursor end)
                    // Use fat width for preview (new connections default to fat)
                    draw_connection_preview(
                        window,
                        from,
                        to,
                        conn_preview,
                        conn_width_fat,
                        port_radius,
                        drag.is_output,
                        origin_x,
                        origin_y,
                    );
                }

                // Draw bulk-connect preview (line from source node center to cursor)
                if let Some(ref drag) = bulk_connect_drag
                    && let Some(from_node) = graph.nodes.get(&drag.from_node)
                {
                    let center = from_node.center();
                    let center_screen = viewport.canvas_to_screen(&center);
                    let cursor_screen = viewport.canvas_to_screen(&drag.current_position);

                    draw_connection_preview(
                        window,
                        center_screen,
                        cursor_screen,
                        conn_preview,
                        conn_width_fat,
                        0.0, // no port shortening
                        true,
                        origin_x,
                        origin_y,
                    );
                }
            },
        )
        .size_full()
        .absolute()
        .inset_0();

        // Build selection box element
        let selection_box_element = self.state.box_selection.as_ref().map(|sel| {
            let (x, y, w, h) = sel.rect();
            let screen_start = viewport.canvas_to_screen(&Position::new(x, y));

            div()
                .absolute()
                .left(px(screen_start.x))
                .top(px(screen_start.y))
                .w(px(w * viewport.zoom))
                .h(px(h * viewport.zoom))
                .bg(theme.selection_fill)
                .border_1()
                .border_color(theme.selection_border)
        });

        // Build node elements
        let node_elements: Vec<_> = self
            .state
            .graph
            .nodes
            .values()
            .map(|node| {
                let screen_pos = viewport.canvas_to_screen(&node.position);
                let selected = self.state.selection.is_node_selected(node.id);
                let dragging = self
                    .state
                    .node_drag
                    .as_ref()
                    .map(|d| d.dragging_nodes.contains(&node.id))
                    .unwrap_or(false);

                // Create a modified node data with screen position
                let mut screen_node = node.clone();
                screen_node.position = screen_pos;
                screen_node.width *= viewport.zoom;
                screen_node.height *= viewport.zoom;

                WorkflowNode::new(SharedString::from(format!("node-{}", node.id)), screen_node)
                    .selected(selected)
                    .dragging(dragging)
                    .theme(scaled_theme.clone())
            })
            .collect();

        // Build context menu.
        //
        // When the right-click hit a node and the parent has registered
        // node-specific menu items, show those instead of the canvas menu so
        // per-node actions (edit / solo / bypass / remove) can be dispatched.
        let context_menu = if let Some(menu_state) = &self.state.context_menu {
            let entity = cx.entity().clone();

            // Ask the resolver whether this specific node has a per-node
            // menu — falls through to canvas menu if the resolver returns
            // None or no resolver is registered.
            let node_menu = menu_state.target_node.and_then(|id| {
                let node_data = self.state.graph.nodes.get(&id)?;
                let items = self.node_menu_items.as_ref()?(node_data)?;
                if items.is_empty() {
                    None
                } else {
                    Some((id, items))
                }
            });

            let (menu_items, target_node, is_node_menu) = if let Some((id, items)) = node_menu {
                (items, Some(id), true)
            } else if let Some(custom_items) = &self.custom_menu_items {
                (custom_items.clone(), None, false)
            } else {
                (
                    vec![
                        MenuItem::new("process", "Process Node"),
                        MenuItem::new("input", "Input Node").with_icon("→"),
                        MenuItem::new("filter", "Filter Node").with_icon("⚡"),
                        MenuItem::new("transform", "Transform Node").with_icon("🔄"),
                        MenuItem::new("mix", "Mix Node").with_icon("🔀"),
                        MenuItem::separator(),
                        MenuItem::new("output", "Output Node").with_icon("🔊"),
                    ],
                    None,
                    false,
                )
            };

            let menu = Menu::new("workflow-context-menu", menu_items).on_select({
                let entity = entity.clone();
                move |id, window, cx| {
                    if is_node_menu {
                        if let Some(node_id) = target_node {
                            let cb = entity.update(cx, |this, _| {
                                this.state.context_menu = None;
                                this.on_node_menu_select.clone()
                            });
                            if let Some(cb) = cb {
                                (**cb)(id, node_id, window, cx);
                            }
                            entity.update(cx, |_, cx| cx.notify());
                        }
                    } else {
                        entity.update(cx, |this, cx| {
                            this.handle_add_node_menu(id, cx);
                        });
                    }
                }
            });

            Some(
                div()
                    .absolute()
                    .left(px(menu_state.position.x))
                    .top(px(menu_state.position.y))
                    // Stop propagation so clicking the menu doesn't trigger canvas click (which clears the menu)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(menu),
            )
        } else {
            None
        };

        let mut result = div()
            .id("workflow-canvas")
            .size_full()
            .relative()
            .bg(theme.canvas_background)
            .overflow_hidden()
            // Draw grid pattern (simplified)
            .child(div().absolute().inset_0().bg(theme.canvas_background))
            // Connections layer
            .child(connections_element)
            // Nodes layer
            .children(node_elements);

        // Add selection box if present
        if let Some(sel) = selection_box_element {
            result = result.child(sel);
        }

        // Add context menu if present
        if let Some(menu) = context_menu {
            result = result.child(menu);
        }

        // Add mouse event handlers
        // Note: event.position is in window coordinates, we subtract canvas_origin
        // to get coordinates relative to the canvas element
        result
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    let x: f32 = event.position.x.into();
                    let y: f32 = event.position.y.into();
                    // Convert from window coordinates to canvas-element-relative coordinates
                    let pos = Position::new(x - this.canvas_origin.x, y - this.canvas_origin.y);

                    // Handle double-click on nodes
                    if event.click_count == 2 {
                        this.handle_double_click(pos, window, cx);
                    } else {
                        let shift = event.modifiers.shift;
                        let alt = event.modifiers.alt;
                        this.handle_mouse_down(pos, shift, alt, cx);
                    }
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                    let x: f32 = event.position.x.into();
                    let y: f32 = event.position.y.into();
                    // Convert from window coordinates to canvas-element-relative coordinates
                    let pos = Position::new(x - this.canvas_origin.x, y - this.canvas_origin.y);
                    this.handle_right_click(pos, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                let x: f32 = event.position.x.into();
                let y: f32 = event.position.y.into();
                // Convert from window coordinates to canvas-element-relative coordinates
                let pos = Position::new(x - this.canvas_origin.x, y - this.canvas_origin.y);
                this.handle_mouse_move(pos, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _window, cx| {
                    let x: f32 = event.position.x.into();
                    let y: f32 = event.position.y.into();
                    // Convert from window coordinates to canvas-element-relative coordinates
                    let pos = Position::new(x - this.canvas_origin.x, y - this.canvas_origin.y);
                    this.handle_mouse_up(pos, cx);
                }),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _window, cx| {
                let delta = match event.delta {
                    ScrollDelta::Lines(lines) => lines.y,
                    ScrollDelta::Pixels(pixels) => {
                        let py: f32 = pixels.y.into();
                        py / 100.0
                    }
                };
                let x: f32 = event.position.x.into();
                let y: f32 = event.position.y.into();
                // Convert from window coordinates to canvas-element-relative coordinates
                let pos = Position::new(x - this.canvas_origin.x, y - this.canvas_origin.y);
                this.handle_scroll(delta, pos, cx);
            }))
            // Keyboard shortcuts
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                this.handle_key_down(event, cx);
            }))
            // Make focusable to receive keyboard events
            .focusable()
            .track_focus(&self.focus_handle)
    }
}
