//! Hit testing for workflow canvas elements

use super::bezier::{ObstacleRect, connection_path_into, horizontal_bezier};
use super::canvas::{
    CONNECTION_FLATTEN_TOLERANCE, CONNECTION_ROUTING_MARGIN, cached_connection_path_with_bounds,
};
use super::state::{
    Connection, ConnectionId, NodeId, Position, ViewportState, WorkflowGraph, WorkflowNodeData,
};
use std::cell::RefCell;

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
    /// Reused while checking connection segments; one hit test only needs one
    /// flattened path at a time.
    path_scratch: RefCell<Vec<Position>>,
}

impl HitTester {
    pub fn new() -> Self {
        Self {
            port_radius: 10.0,
            connection_tolerance: 5.0,
            path_scratch: RefCell::new(Vec::new()),
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
        self.hit_test_with_viewport_and_obstacles(screen_point, graph, viewport, &[])
    }

    /// Hit test against obstacle-routed connection curves.
    ///
    /// `obstacles` are node bounding rects in screen coordinates, matching
    /// what the canvas passes to connection routing at paint time. Detoured
    /// connections are then hittable along their routed (rather than direct)
    /// curves. Canvas mouse handlers can switch to this once they carry the
    /// paint-time obstacle set; until then the plain entry point above keeps
    /// behavior unchanged.
    pub(crate) fn hit_test_with_viewport_and_obstacles(
        &self,
        screen_point: Position,
        graph: &WorkflowGraph,
        viewport: &ViewportState,
        obstacles: &[ObstacleRect],
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
            if self.point_on_connection_screen_with_obstacles(
                screen_point,
                conn,
                graph,
                viewport,
                obstacles,
            ) {
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
        let content_height = (node_screen_height - header_height - 2.0 * border).max(0.0);
        let available = (content_height - 2.0 * padding).max(0.0);

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

        if !self.point_in_connection_bounds(point, from_pos, to_pos) {
            return false;
        }

        // Get the connection path points
        let mut path_points = self.path_scratch.borrow_mut();
        connection_path_into(from_pos, to_pos, 2.0, &mut path_points);

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

    /// Check if a point is near an obstacle-routed connection curve (screen
    /// coordinates).
    ///
    /// The flattened routed path and its exact bounding box come from the
    /// canvas connection-path cache, so repeated hit tests over unchanged
    /// geometry never re-flatten. A point outside the stored box (expanded
    /// by the connection tolerance) returns false without walking segments.
    fn point_on_connection_screen_with_obstacles(
        &self,
        point: Position,
        conn: &Connection,
        graph: &WorkflowGraph,
        viewport: &ViewportState,
        obstacles: &[ObstacleRect],
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

        // AABB pre-reject on the exact flattened-path bounds. With no
        // obstacles this matches the direct bezier within flattening
        // tolerance, so no separate control-hull check is needed here.
        let entry = cached_connection_path_with_bounds(
            from_pos,
            to_pos,
            obstacles,
            CONNECTION_ROUTING_MARGIN,
            CONNECTION_FLATTEN_TOLERANCE,
        );
        if !entry.contains_with_pad(point.x, point.y, self.connection_tolerance) {
            return false;
        }

        // Check if point is near any segment of the shared cached path
        let path_points = entry.path();
        for i in 0..path_points.len().saturating_sub(1) {
            let p1 = &path_points[i];
            let p2 = &path_points[i + 1];
            if point_to_segment_distance(&point, p1, p2) <= self.connection_tolerance {
                return true;
            }
        }

        false
    }

    /// Conservative control-point bounds for the horizontal cubic. A Bezier
    /// curve lies in the convex hull of its controls, so this eliminates the
    /// allocation-heavy flattening path for clicks nowhere near a connection.
    fn point_in_connection_bounds(&self, point: Position, from: Position, to: Position) -> bool {
        let (p0, p1, p2, p3) = horizontal_bezier(from, to);
        let min_x = p0.x.min(p1.x).min(p2.x).min(p3.x) - self.connection_tolerance;
        let max_x = p0.x.max(p1.x).max(p2.x).max(p3.x) + self.connection_tolerance;
        let min_y = p0.y.min(p1.y).min(p2.y).min(p3.y) - self.connection_tolerance;
        let max_y = p0.y.max(p1.y).max(p2.y).max(p3.y) + self.connection_tolerance;
        point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
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
    use super::super::{Position, ViewportState, WorkflowGraph, WorkflowNodeData};
    use super::{HitTestResult, HitTester};

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

    #[test]
    fn test_hit_test_with_viewport_zoom() {
        let graph = create_test_graph();
        let tester = HitTester::new();

        // With default viewport (zoom=1), node at (100,100) is hit at screen (150,130)
        let result = tester.hit_test_with_viewport(
            Position::new(150.0, 130.0),
            &graph,
            &ViewportState::default(),
        );
        assert!(matches!(result, HitTestResult::Node(_)));

        // With zoom=2, the same node occupies 2x the screen space.
        // Node screen position becomes (200, 200) with size (360, 200).
        // Screen point (150, 130) should now miss.
        let zoomed_viewport = ViewportState {
            zoom: 2.0,
            offset: Position::new(0.0, 0.0),
            size: (800.0, 600.0),
        };
        let result =
            tester.hit_test_with_viewport(Position::new(150.0, 130.0), &graph, &zoomed_viewport);
        assert_eq!(result, HitTestResult::Canvas);

        // Screen point (300, 250) should hit the zoomed node
        let result =
            tester.hit_test_with_viewport(Position::new(300.0, 250.0), &graph, &zoomed_viewport);
        assert!(matches!(result, HitTestResult::Node(_)));
    }

    #[test]
    fn obstacle_routed_connection_hit_follows_detour() {
        use super::super::bezier::{ObstacleRect, connection_path, connection_path_avoiding};
        use super::super::canvas::cached_connection_path_with_bounds;
        use std::sync::Arc;

        let mut graph = WorkflowGraph::new();
        let node1 = WorkflowNodeData::new("A", Position::new(0.0, 0.0))
            .with_ports(1, 1)
            .with_size(100.0, 60.0);
        let node2 = WorkflowNodeData::new("B", Position::new(300.0, 100.0))
            .with_ports(1, 1)
            .with_size(100.0, 60.0);
        let id1 = graph.add_node(node1);
        let id2 = graph.add_node(node2);
        graph.add_connection(id1, 0, id2, 0).unwrap();

        let tester = HitTester::new();
        let viewport = ViewportState::default();
        let from_node = graph.nodes.get(&id1).expect("node 1");
        let to_node = graph.nodes.get(&id2).expect("node 2");
        let from = tester.port_screen_position(from_node, 0, false, &viewport);
        let to = tester.port_screen_position(to_node, 0, true, &viewport);

        // Center an obstacle on a direct-curve sample so the direct path is
        // guaranteed to collide and the router is forced onto a detour.
        let direct = connection_path(from, to, 2.0);
        assert!(direct.len() >= 2);
        let anchor = direct[direct.len() / 2];
        let obstacle = ObstacleRect::new(anchor.x - 30.0, anchor.y - 30.0, 60.0, 60.0);

        // Pick the routed point farthest from the direct curve.
        let routed = connection_path_avoiding(from, to, &[obstacle], 15.0, 2.0);
        let clearance = |candidate: &Position| {
            direct
                .iter()
                .map(|sample| candidate.distance(sample))
                .fold(f32::INFINITY, f32::min)
        };
        let detour = routed
            .iter()
            .max_by(|a, b| {
                clearance(a)
                    .partial_cmp(&clearance(b))
                    .expect("distances are finite")
            })
            .expect("routed path is non-empty");
        assert!(
            clearance(detour) > 5.0,
            "test detour should clear the direct curve by more than hit tolerance"
        );

        let conn = graph.connections.first().expect("one connection").clone();
        assert!(tester.point_on_connection_screen_with_obstacles(
            *detour,
            &conn,
            &graph,
            &viewport,
            &[obstacle],
        ));
        // The same point misses the direct curve tested without obstacles.
        assert!(!tester.point_on_connection_screen_with_obstacles(
            *detour,
            &conn,
            &graph,
            &viewport,
            &[],
        ));

        // Shared cache: repeated lookups return the identical entry whose
        // stored box contains the detour and rejects far-away points.
        let first = cached_connection_path_with_bounds(from, to, &[obstacle], 15.0, 2.0);
        let second = cached_connection_path_with_bounds(from, to, &[obstacle], 15.0, 2.0);
        assert!(Arc::ptr_eq(&first, &second));
        assert!(first.contains_with_pad(detour.x, detour.y, 5.0));
        assert!(!first.contains_with_pad(-500.0, -500.0, 5.0));

        // A far-away point still resolves to canvas through the public API.
        assert_eq!(
            tester.hit_test_with_viewport_and_obstacles(
                Position::new(-500.0, -500.0),
                &graph,
                &viewport,
                &[obstacle],
            ),
            HitTestResult::Canvas
        );
    }
}
