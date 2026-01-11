//! Workflow canvas state management

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Unique identifier for workflow nodes
pub type NodeId = uuid::Uuid;

/// Unique identifier for connections
pub type ConnectionId = uuid::Uuid;

/// Position on the 2D canvas
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Distance to another position
    pub fn distance(&self, other: &Position) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// A workflow node with position and port configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeData {
    pub id: NodeId,
    pub position: Position,
    pub width: f32,
    pub height: f32,
    pub title: String,
    pub input_count: usize,
    pub output_count: usize,
    /// Custom data associated with this node (application-specific)
    #[serde(default)]
    pub user_data: serde_json::Value,
}

impl WorkflowNodeData {
    pub fn new(title: impl Into<String>, position: Position) -> Self {
        Self {
            id: NodeId::new_v4(),
            position,
            width: 180.0,
            height: 100.0,
            title: title.into(),
            input_count: 1,
            output_count: 1,
            user_data: serde_json::Value::Null,
        }
    }

    /// Create with specific port counts
    pub fn with_ports(mut self, inputs: usize, outputs: usize) -> Self {
        self.input_count = inputs;
        self.output_count = outputs;
        self
    }

    /// Create with specific size
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Create with user data
    pub fn with_user_data(mut self, data: serde_json::Value) -> Self {
        self.user_data = data;
        self
    }

    /// Get the center position of this node
    pub fn center(&self) -> Position {
        Position::new(
            self.position.x + self.width / 2.0,
            self.position.y + self.height / 2.0,
        )
    }

    /// Get port position for an input port (left side)
    ///
    /// This matches the layout in node.rs where:
    /// - Header takes ~28px (py_1 + text_sm + py_1)
    /// - Content area has py_2 (8px) padding
    /// - Ports are distributed with justify_around
    pub fn input_port_position(&self, index: usize) -> Position {
        let header_height = 28.0;
        let padding = 8.0;
        let border = 2.0;
        let content_height = self.height - header_height - 2.0 * border;
        let available = content_height - 2.0 * padding;

        let y = if self.input_count == 0 {
            self.position.y + self.height / 2.0
        } else {
            let spacing = available / self.input_count as f32;
            self.position.y + border + header_height + padding + spacing * (index as f32 + 0.5)
        };

        Position::new(self.position.x, y)
    }

    /// Get port position for an output port (right side)
    ///
    /// This matches the layout in node.rs where:
    /// - Header takes ~28px (py_1 + text_sm + py_1)
    /// - Content area has py_2 (8px) padding
    /// - Ports are distributed with justify_around
    pub fn output_port_position(&self, index: usize) -> Position {
        let header_height = 28.0;
        let padding = 8.0;
        let border = 2.0;
        let content_height = self.height - header_height - 2.0 * border;
        let available = content_height - 2.0 * padding;

        let y = if self.output_count == 0 {
            self.position.y + self.height / 2.0
        } else {
            let spacing = available / self.output_count as f32;
            self.position.y + border + header_height + padding + spacing * (index as f32 + 0.5)
        };

        Position::new(self.position.x + self.width, y)
    }
}

/// Link type for connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LinkType {
    /// Fat link - carries all channels bundled together
    #[default]
    Fat,
    /// Thin link - carries a single channel
    Thin,
}

/// A connection between two ports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: ConnectionId,
    pub from_node: NodeId,
    pub from_port: usize,
    pub to_node: NodeId,
    pub to_port: usize,
    /// Type of link (fat = all channels, thin = single channel)
    #[serde(default)]
    pub link_type: LinkType,
}

impl Connection {
    pub fn new(from_node: NodeId, from_port: usize, to_node: NodeId, to_port: usize) -> Self {
        Self {
            id: ConnectionId::new_v4(),
            from_node,
            from_port,
            to_node,
            to_port,
            link_type: LinkType::Fat, // Default to fat links
        }
    }

    /// Create a thin (single-channel) connection
    pub fn new_thin(from_node: NodeId, from_port: usize, to_node: NodeId, to_port: usize) -> Self {
        Self {
            id: ConnectionId::new_v4(),
            from_node,
            from_port,
            to_node,
            to_port,
            link_type: LinkType::Thin,
        }
    }

    /// Set the link type
    pub fn with_link_type(mut self, link_type: LinkType) -> Self {
        self.link_type = link_type;
        self
    }
}

/// The complete workflow graph
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowGraph {
    pub nodes: HashMap<NodeId, WorkflowNodeData>,
    pub connections: Vec<Connection>,
    /// ID counter for generating sequential IDs if needed
    #[serde(skip)]
    #[allow(dead_code)]
    next_id: usize,
}

impl WorkflowGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: WorkflowNodeData) -> NodeId {
        let id = node.id;
        self.nodes.insert(id, node);
        id
    }

    /// Remove a node and all its connections
    pub fn remove_node(&mut self, node_id: NodeId) -> Option<WorkflowNodeData> {
        let node = self.nodes.remove(&node_id);
        self.connections
            .retain(|c| c.from_node != node_id && c.to_node != node_id);
        node
    }

    /// Add a connection between two ports
    pub fn add_connection(
        &mut self,
        from_node: NodeId,
        from_port: usize,
        to_node: NodeId,
        to_port: usize,
    ) -> Result<ConnectionId, &'static str> {
        // Validate nodes exist
        if !self.nodes.contains_key(&from_node) {
            return Err("Source node not found");
        }
        if !self.nodes.contains_key(&to_node) {
            return Err("Target node not found");
        }

        // Check for self-loops
        if from_node == to_node {
            return Err("Self-loops are not allowed");
        }

        // Check for duplicate connections
        if self.connections.iter().any(|c| {
            c.from_node == from_node
                && c.from_port == from_port
                && c.to_node == to_node
                && c.to_port == to_port
        }) {
            return Err("Connection already exists");
        }

        // Check for cycles
        if self.would_create_cycle(from_node, to_node) {
            return Err("Connection would create a cycle");
        }

        let conn = Connection::new(from_node, from_port, to_node, to_port);
        let id = conn.id;
        self.connections.push(conn);
        Ok(id)
    }

    /// Remove a connection by ID
    pub fn remove_connection(&mut self, connection_id: ConnectionId) {
        self.connections.retain(|c| c.id != connection_id);
    }

    /// Check if adding an edge from -> to would create a cycle
    fn would_create_cycle(&self, from_node: NodeId, to_node: NodeId) -> bool {
        use std::collections::VecDeque;

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(to_node);

        while let Some(current) = queue.pop_front() {
            if current == from_node {
                return true;
            }
            if visited.insert(current) {
                for conn in &self.connections {
                    if conn.from_node == current {
                        queue.push_back(conn.to_node);
                    }
                }
            }
        }
        false
    }

    /// Get connections originating from a node
    pub fn connections_from(&self, node_id: NodeId) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|c| c.from_node == node_id)
            .collect()
    }

    /// Get connections going to a node
    pub fn connections_to(&self, node_id: NodeId) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|c| c.to_node == node_id)
            .collect()
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Viewport state (pan/zoom)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ViewportState {
    /// Pan offset in canvas coordinates
    pub offset: Position,
    /// Zoom level (0.25 to 4.0)
    pub zoom: f32,
    /// Canvas size in pixels
    pub size: (f32, f32),
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            offset: Position::new(0.0, 0.0),
            zoom: 1.0,
            size: (800.0, 600.0),
        }
    }
}

impl ViewportState {
    /// Convert screen coordinates to canvas coordinates
    pub fn screen_to_canvas(&self, screen_x: f32, screen_y: f32) -> Position {
        Position::new(
            (screen_x - self.offset.x) / self.zoom,
            (screen_y - self.offset.y) / self.zoom,
        )
    }

    /// Convert canvas coordinates to screen coordinates
    pub fn canvas_to_screen(&self, canvas_pos: &Position) -> Position {
        Position::new(
            canvas_pos.x * self.zoom + self.offset.x,
            canvas_pos.y * self.zoom + self.offset.y,
        )
    }

    /// Apply zoom centered on a point
    pub fn zoom_at(&mut self, delta: f32, screen_x: f32, screen_y: f32) {
        let old_zoom = self.zoom;
        self.zoom = (self.zoom * (1.0 + delta * 0.1)).clamp(0.25, 4.0);

        // Adjust offset to keep the point under the cursor fixed
        let scale_change = self.zoom / old_zoom;
        self.offset.x = screen_x - (screen_x - self.offset.x) * scale_change;
        self.offset.y = screen_y - (screen_y - self.offset.y) * scale_change;
    }

    /// Pan by a delta in screen coordinates
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.offset.x += dx;
        self.offset.y += dy;
    }
}

/// Selection state
#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    pub selected_nodes: HashSet<NodeId>,
    pub selected_connections: HashSet<ConnectionId>,
}

impl SelectionState {
    pub fn clear(&mut self) {
        self.selected_nodes.clear();
        self.selected_connections.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.selected_nodes.is_empty() && self.selected_connections.is_empty()
    }

    pub fn select_node(&mut self, node_id: NodeId, add_to_selection: bool) {
        if !add_to_selection {
            self.clear();
        }
        self.selected_nodes.insert(node_id);
    }

    pub fn select_connection(&mut self, conn_id: ConnectionId, add_to_selection: bool) {
        if !add_to_selection {
            self.clear();
        }
        self.selected_connections.insert(conn_id);
    }

    pub fn toggle_node(&mut self, node_id: NodeId) {
        if self.selected_nodes.contains(&node_id) {
            self.selected_nodes.remove(&node_id);
        } else {
            self.selected_nodes.insert(node_id);
        }
    }

    pub fn is_node_selected(&self, node_id: NodeId) -> bool {
        self.selected_nodes.contains(&node_id)
    }

    pub fn is_connection_selected(&self, conn_id: ConnectionId) -> bool {
        self.selected_connections.contains(&conn_id)
    }
}

/// Current interaction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InteractionMode {
    #[default]
    None,
    /// Panning the canvas
    Panning,
    /// Dragging nodes
    DraggingNodes,
    /// Creating a connection
    CreatingConnection,
    /// Box selection
    BoxSelecting,
}

/// State for dragging nodes
#[derive(Debug, Clone)]
pub struct NodeDragState {
    /// IDs of nodes being dragged
    pub dragging_nodes: Vec<NodeId>,
    /// Starting mouse position
    pub start_mouse: Position,
    /// Original positions (for undo)
    pub original_positions: HashMap<NodeId, Position>,
}

/// State for dragging a new connection
#[derive(Debug, Clone)]
pub struct ConnectionDrag {
    pub from_node: NodeId,
    pub from_port: usize,
    pub is_output: bool,
    pub current_position: Position,
}

/// State for box selection
#[derive(Debug, Clone)]
pub struct BoxSelection {
    pub start: Position,
    pub current: Position,
}

impl BoxSelection {
    /// Get the selection rectangle in canvas coordinates
    pub fn rect(&self) -> (f32, f32, f32, f32) {
        let min_x = self.start.x.min(self.current.x);
        let min_y = self.start.y.min(self.current.y);
        let max_x = self.start.x.max(self.current.x);
        let max_y = self.start.y.max(self.current.y);
        (min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Check if a rectangle intersects with the selection box
    pub fn intersects(&self, x: f32, y: f32, width: f32, height: f32) -> bool {
        let (sx, sy, sw, sh) = self.rect();
        !(x + width < sx || x > sx + sw || y + height < sy || y > sy + sh)
    }
}

/// State for context menu
#[derive(Debug, Clone)]
pub struct ContextMenuState {
    pub position: Position,
    pub visible: bool,
}

/// Complete canvas state
#[derive(Debug, Clone)]
pub struct CanvasState {
    pub graph: WorkflowGraph,
    pub viewport: ViewportState,
    pub selection: SelectionState,
    pub mode: InteractionMode,
    pub node_drag: Option<NodeDragState>,
    pub connection_drag: Option<ConnectionDrag>,
    pub box_selection: Option<BoxSelection>,
    pub context_menu: Option<ContextMenuState>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            graph: WorkflowGraph::new(),
            viewport: ViewportState::default(),
            selection: SelectionState::default(),
            mode: InteractionMode::None,
            node_drag: None,
            connection_drag: None,
            box_selection: None,
            context_menu: None,
        }
    }
}

impl CanvasState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with an existing graph
    pub fn with_graph(graph: WorkflowGraph) -> Self {
        Self {
            graph,
            ..Default::default()
        }
    }
}
