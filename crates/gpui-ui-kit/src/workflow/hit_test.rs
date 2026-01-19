//! Hit testing for workflow canvas elements

use super::bezier::connection_path;
use super::state::{
    Connection, ConnectionId, NodeId, Position, ViewportState, WorkflowGraph, WorkflowNodeData,
};

/// Result of a hit test
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTestResult {
    /// Nothing was hit
    None,
    /// A node was hit
    Node(NodeId),
    /// An input port was hit (node_id, port_index)
    InputPort(NodeId, usize),
    /// An output port was hit (node_id, port_index)
    OutputPort(NodeId, usize),
    /// A connection was hit
    Connection(ConnectionId),
    /// The canvas background was hit
    Canvas,
}

/// Hit tester for efficient spatial queries
#[derive(Debug, Default)]
pub struct HitTester {
    /// Port hit radius in screen pixels
    port_radius: f32,
    /// Connection hit tolerance in screen pixels
    connection_tolerance: f32,
}

impl HitTester {
    pub fn new() -> Self {
        Self {
            port_radius: 10.0,
            connection_tolerance: 5.0,
        }
    }

    /// Set the port hit radius
    pub fn with_port_radius(mut self, radius: f32) -> Self {
        self.port_radius = radius;
        self
    }

    /// Set the connection hit tolerance
    pub fn with_connection_tolerance(mut self, tolerance: f32) -> Self {
        self.connection_tolerance = tolerance;
        self
    }

    /// Perform a hit test at the given screen coordinates (relative to canvas element)
    ///
    /// The viewport is needed because port positions include fixed pixel offsets
    /// (header, padding) that don't scale with zoom, and the viewport offset for panning.
    pub fn hit_test_with_viewport(
        &self,
        screen_point: Position,
        graph: &WorkflowGraph,
        viewport: &ViewportState,
    ) -> HitTestResult {
        // Test ports first (highest priority)
        // Port positions are calculated in screen coordinates for accurate hit testing
        for node in graph.nodes.values() {
            // Test output ports
            for i in 0..node.output_count {
                let port_pos = self.port_screen_position(node, i, false, viewport);
                if screen_point.distance(&port_pos) <= self.port_radius {
                    return HitTestResult::OutputPort(node.id, i);
                }
            }

            // Test input ports
            for i in 0..node.input_count {
                let port_pos = self.port_screen_position(node, i, true, viewport);
                if screen_point.distance(&port_pos) <= self.port_radius {
                    return HitTestResult::InputPort(node.id, i);
                }
            }
        }

        // Test nodes (second priority) - in screen coordinates
        for node in graph.nodes.values() {
            if self.point_in_node_screen(screen_point, node, viewport) {
                return HitTestResult::Node(node.id);
            }
        }

        // Test connections (third priority) - in screen coordinates
        for conn in &graph.connections {
            if self.point_on_connection_screen(screen_point, conn, graph, viewport) {
                return HitTestResult::Connection(conn.id);
            }
        }

        HitTestResult::Canvas
    }

    /// Legacy hit test without zoom (for tests) - assumes zoom = 1.0, no offset
    pub fn hit_test(&self, point: Position, graph: &WorkflowGraph) -> HitTestResult {
        let default_viewport = ViewportState::default();
        self.hit_test_with_viewport(point, graph, &default_viewport)
    }

    /// Calculate port position in screen coordinates
    ///
    /// This matches the visual layout where:
    /// - Node position is scaled by zoom and offset by viewport
    /// - Header, padding, and border are fixed screen pixels (matching WorkflowTheme defaults)
    /// - Content area is scaled node height minus fixed header
    /// - Ports are positioned at content edges (inside the border)
    fn port_screen_position(
        &self,
        node: &WorkflowNodeData,
        index: usize,
        is_input: bool,
        viewport: &ViewportState,
    ) -> Position {
        let count = if is_input {
            node.input_count
        } else {
            node.output_count
        };

        let zoom = viewport.zoom;

        // Screen position of node top-left (includes viewport offset)
        let node_screen_x = node.position.x * zoom + viewport.offset.x;
        let node_screen_y = node.position.y * zoom + viewport.offset.y;

        // Fixed pixel sizes (not scaled) - must match WorkflowTheme defaults and node.rs
        // node_header_height: 28.0 (py_1 + text_sm + py_1)
        // node_content_padding: 8.0 (py_2)
        // border: 2.0 (border_2)
        let header_height = 28.0_f32 * zoom;
        let padding = 8.0_f32 * zoom;
        let border = 2.0_f32; // Border stays fixed width

        // Scaled node dimensions
        let node_screen_width = node.width * zoom;
        let node_screen_height = node.height * zoom;

        // Content area height (scaled node height minus fixed header)
        let content_height = node_screen_height - header_height - 2.0 * border;
        let available = content_height - 2.0 * padding;

        let y = if count == 0 {
            node_screen_y + node_screen_height / 2.0
        } else {
            let spacing = available / count as f32;
            node_screen_y + border + header_height + padding + spacing * (index as f32 + 0.5)
        };
        // Ports are positioned at the content edge (inside the border)
        // Input ports: at left content edge (node_left + border)
        // Output ports: at right content edge (node_right - border)
        let x = if is_input {
            node_screen_x + border
        } else {
            node_screen_x + node_screen_width - border
        };

        Position::new(x, y)
    }

    /// Check if a point is inside a node's bounds (canvas coordinates)
    #[allow(dead_code)]
    fn point_in_node(&self, point: Position, node: &WorkflowNodeData) -> bool {
        point.x >= node.position.x
            && point.x <= node.position.x + node.width
            && point.y >= node.position.y
            && point.y <= node.position.y + node.height
    }

    /// Check if a point is inside a node's bounds (screen coordinates)
    fn point_in_node_screen(
        &self,
        point: Position,
        node: &WorkflowNodeData,
        viewport: &ViewportState,
    ) -> bool {
        let zoom = viewport.zoom;
        let screen_x = node.position.x * zoom + viewport.offset.x;
        let screen_y = node.position.y * zoom + viewport.offset.y;
        let screen_w = node.width * zoom;
        let screen_h = node.height * zoom;

        point.x >= screen_x
            && point.x <= screen_x + screen_w
            && point.y >= screen_y
            && point.y <= screen_y + screen_h
    }

    /// Check if a point is near a connection curve (canvas coordinates)
    #[allow(dead_code)]
    fn point_on_connection(
        &self,
        point: Position,
        conn: &Connection,
        graph: &WorkflowGraph,
    ) -> bool {
        let from_node = match graph.nodes.get(&conn.from_node) {
            Some(n) => n,
            None => return false,
        };
        let to_node = match graph.nodes.get(&conn.to_node) {
            Some(n) => n,
            None => return false,
        };

        let from_pos = from_node.output_port_position(conn.from_port);
        let to_pos = to_node.input_port_position(conn.to_port);

        // Get the connection path points
        let path_points = connection_path(from_pos, to_pos, 2.0);

        // Check if point is near any segment of the path
        for i in 0..path_points.len().saturating_sub(1) {
            let p1 = &path_points[i];
            let p2 = &path_points[i + 1];
            if point_to_segment_distance(&point, p1, p2) <= self.connection_tolerance {
                return true;
            }
        }

        false
    }

    /// Check if a point is near a connection curve (screen coordinates)
    fn point_on_connection_screen(
        &self,
        point: Position,
        conn: &Connection,
        graph: &WorkflowGraph,
        viewport: &ViewportState,
    ) -> bool {
        let from_node = match graph.nodes.get(&conn.from_node) {
            Some(n) => n,
            None => return false,
        };
        let to_node = match graph.nodes.get(&conn.to_node) {
            Some(n) => n,
            None => return false,
        };

        // Get port positions in screen coordinates
        let from_pos = self.port_screen_position(from_node, conn.from_port, false, viewport);
        let to_pos = self.port_screen_position(to_node, conn.to_port, true, viewport);

        // Get the connection path points (in screen coordinates)
        let path_points = connection_path(from_pos, to_pos, 2.0);

        // Check if point is near any segment of the path
        for i in 0..path_points.len().saturating_sub(1) {
            let p1 = &path_points[i];
            let p2 = &path_points[i + 1];
            if point_to_segment_distance(&point, p1, p2) <= self.connection_tolerance {
                return true;
            }
        }

        false
    }

    /// Find all nodes within a rectangle
    pub fn nodes_in_rect(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        graph: &WorkflowGraph,
    ) -> Vec<NodeId> {
        graph
            .nodes
            .values()
            .filter(|node| {
                // Check if node rect intersects with selection rect
                !(node.position.x + node.width < x
                    || node.position.x > x + width
                    || node.position.y + node.height < y
                    || node.position.y > y + height)
            })
            .map(|node| node.id)
            .collect()
    }
}

/// Calculate the minimum distance from a point to a line segment
fn point_to_segment_distance(point: &Position, seg_start: &Position, seg_end: &Position) -> f32 {
    let dx = seg_end.x - seg_start.x;
    let dy = seg_end.y - seg_start.y;
    let length_sq = dx * dx + dy * dy;

    if length_sq < 1e-10 {
        return point.distance(seg_start);
    }

    // Project point onto line segment
    let t = ((point.x - seg_start.x) * dx + (point.y - seg_start.y) * dy) / length_sq;
    let t = t.clamp(0.0, 1.0);

    let proj = Position::new(seg_start.x + t * dx, seg_start.y + t * dy);
    point.distance(&proj)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> WorkflowGraph {
        let mut graph = WorkflowGraph::new();

        let node1 = WorkflowNodeData::new("Node 1", Position::new(100.0, 100.0))
            .with_ports(1, 2)
            .with_size(180.0, 100.0);
        let node2 = WorkflowNodeData::new("Node 2", Position::new(400.0, 150.0))
            .with_ports(2, 1)
            .with_size(180.0, 100.0);

        let id1 = graph.add_node(node1);
        let id2 = graph.add_node(node2);
        graph.add_connection(id1, 0, id2, 0).unwrap();

        graph
    }

    #[test]
    fn test_hit_test_node() {
        let graph = create_test_graph();
        let tester = HitTester::new();

        // Hit the first node
        let result = tester.hit_test(Position::new(150.0, 130.0), &graph);
        match result {
            HitTestResult::Node(_) => (),
            _ => panic!("Expected Node hit, got {:?}", result),
        }
    }

    #[test]
    fn test_hit_test_canvas() {
        let graph = create_test_graph();
        let tester = HitTester::new();

        // Miss everything
        let result = tester.hit_test(Position::new(0.0, 0.0), &graph);
        assert_eq!(result, HitTestResult::Canvas);
    }

    #[test]
    fn test_nodes_in_rect() {
        let graph = create_test_graph();
        let tester = HitTester::new();

        // Select both nodes
        let nodes = tester.nodes_in_rect(50.0, 50.0, 600.0, 300.0, &graph);
        assert_eq!(nodes.len(), 2);

        // Select only the first node
        let nodes = tester.nodes_in_rect(50.0, 50.0, 200.0, 200.0, &graph);
        assert_eq!(nodes.len(), 1);
    }
}
