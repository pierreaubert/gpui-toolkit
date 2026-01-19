//! Integration tests for the workflow canvas module

use super::bezier::{connection_path, flatten_cubic_bezier, horizontal_bezier};
use super::history::{
    AddConnectionCommand, AddNodeCommand, Command, HistoryManager, MoveNodesCommand,
    RemoveNodeCommand,
};
use super::hit_test::{HitTestResult, HitTester};
use super::state::{
    Connection, NodeId, Position, SelectionState, ViewportState, WorkflowGraph, WorkflowNodeData,
};

// ============================================================================
// Position Tests
// ============================================================================

#[test]
fn test_position_new() {
    let pos = Position::new(10.0, 20.0);
    assert_eq!(pos.x, 10.0);
    assert_eq!(pos.y, 20.0);
}

#[test]
fn test_position_distance() {
    let p1 = Position::new(0.0, 0.0);
    let p2 = Position::new(3.0, 4.0);
    assert_eq!(p1.distance(&p2), 5.0);
}

#[test]
fn test_position_default() {
    let pos = Position::default();
    assert_eq!(pos.x, 0.0);
    assert_eq!(pos.y, 0.0);
}

// ============================================================================
// WorkflowNodeData Tests
// ============================================================================

#[test]
fn test_node_creation() {
    let node = WorkflowNodeData::new("Test Node", Position::new(100.0, 200.0));
    assert_eq!(node.title, "Test Node");
    assert_eq!(node.position.x, 100.0);
    assert_eq!(node.position.y, 200.0);
    assert_eq!(node.input_count, 1);
    assert_eq!(node.output_count, 1);
}

#[test]
fn test_node_with_ports() {
    let node = WorkflowNodeData::new("Node", Position::new(0.0, 0.0)).with_ports(3, 2);
    assert_eq!(node.input_count, 3);
    assert_eq!(node.output_count, 2);
}

#[test]
fn test_node_with_size() {
    let node = WorkflowNodeData::new("Node", Position::new(0.0, 0.0)).with_size(200.0, 100.0);
    assert_eq!(node.width, 200.0);
    assert_eq!(node.height, 100.0);
}

#[test]
fn test_node_center() {
    let node = WorkflowNodeData::new("Node", Position::new(100.0, 100.0)).with_size(200.0, 100.0);
    let center = node.center();
    assert_eq!(center.x, 200.0); // 100 + 200/2
    assert_eq!(center.y, 150.0); // 100 + 100/2
}

#[test]
fn test_node_port_positions() {
    let node = WorkflowNodeData::new("Node", Position::new(0.0, 0.0))
        .with_ports(2, 2)
        .with_size(160.0, 90.0);

    // Input ports on left side
    let input0 = node.input_port_position(0);
    let input1 = node.input_port_position(1);
    assert_eq!(input0.x, 0.0);
    assert_eq!(input1.x, 0.0);
    assert!(input0.y < input1.y); // Port 0 is above port 1

    // Output ports on right side
    let output0 = node.output_port_position(0);
    let output1 = node.output_port_position(1);
    assert_eq!(output0.x, 160.0); // Right edge
    assert_eq!(output1.x, 160.0);
    assert!(output0.y < output1.y);
}

// ============================================================================
// Connection Tests
// ============================================================================

#[test]
fn test_connection_creation() {
    let from_node = NodeId::new_v4();
    let to_node = NodeId::new_v4();
    let conn = Connection::new(from_node, 0, to_node, 1);

    assert_eq!(conn.from_node, from_node);
    assert_eq!(conn.from_port, 0);
    assert_eq!(conn.to_node, to_node);
    assert_eq!(conn.to_port, 1);
}

// ============================================================================
// WorkflowGraph Tests
// ============================================================================

#[test]
fn test_graph_add_node() {
    let mut graph = WorkflowGraph::new();
    let node = WorkflowNodeData::new("Node 1", Position::new(0.0, 0.0));
    let id = node.id;

    let returned_id = graph.add_node(node);
    assert_eq!(id, returned_id);
    assert!(graph.nodes.contains_key(&id));
    assert_eq!(graph.nodes.len(), 1);
}

#[test]
fn test_graph_remove_node() {
    let mut graph = WorkflowGraph::new();
    let node = WorkflowNodeData::new("Node", Position::new(0.0, 0.0));
    let id = node.id;
    graph.add_node(node);

    let removed = graph.remove_node(id);
    assert!(removed.is_some());
    assert!(!graph.nodes.contains_key(&id));
}

#[test]
fn test_graph_remove_node_removes_connections() {
    let mut graph = WorkflowGraph::new();

    let node1 = WorkflowNodeData::new("Node 1", Position::new(0.0, 0.0));
    let node2 = WorkflowNodeData::new("Node 2", Position::new(200.0, 0.0));
    let id1 = node1.id;
    let id2 = node2.id;

    graph.add_node(node1);
    graph.add_node(node2);
    graph.add_connection(id1, 0, id2, 0).unwrap();

    assert_eq!(graph.connections.len(), 1);

    graph.remove_node(id1);

    // Connection should be removed
    assert_eq!(graph.connections.len(), 0);
}

#[test]
fn test_graph_add_connection() {
    let mut graph = WorkflowGraph::new();

    let node1 = WorkflowNodeData::new("Node 1", Position::new(0.0, 0.0));
    let node2 = WorkflowNodeData::new("Node 2", Position::new(200.0, 0.0));
    let id1 = node1.id;
    let id2 = node2.id;

    graph.add_node(node1);
    graph.add_node(node2);

    let result = graph.add_connection(id1, 0, id2, 0);
    assert!(result.is_ok());
    assert_eq!(graph.connections.len(), 1);
}

#[test]
fn test_graph_add_connection_invalid_node() {
    let mut graph = WorkflowGraph::new();

    let node1 = WorkflowNodeData::new("Node 1", Position::new(0.0, 0.0));
    let id1 = node1.id;
    let fake_id = NodeId::new_v4();

    graph.add_node(node1);

    let result = graph.add_connection(id1, 0, fake_id, 0);
    assert!(result.is_err());
}

#[test]
fn test_graph_add_connection_self_loop_prevented() {
    let mut graph = WorkflowGraph::new();

    let node = WorkflowNodeData::new("Node", Position::new(0.0, 0.0)).with_ports(1, 1);
    let id = node.id;
    graph.add_node(node);

    let result = graph.add_connection(id, 0, id, 0);
    assert!(result.is_err());
}

#[test]
fn test_graph_remove_connection() {
    let mut graph = WorkflowGraph::new();

    let node1 = WorkflowNodeData::new("Node 1", Position::new(0.0, 0.0));
    let node2 = WorkflowNodeData::new("Node 2", Position::new(200.0, 0.0));
    let id1 = node1.id;
    let id2 = node2.id;

    graph.add_node(node1);
    graph.add_node(node2);
    graph.add_connection(id1, 0, id2, 0).unwrap();
    assert_eq!(graph.connections.len(), 1);

    let conn_id = graph.connections[0].id;
    graph.remove_connection(conn_id);
    assert_eq!(graph.connections.len(), 0);
}

// ============================================================================
// SelectionState Tests
// ============================================================================

#[test]
fn test_selection_empty() {
    let selection = SelectionState::default();
    assert!(selection.is_empty());
}

#[test]
fn test_selection_select_node() {
    let mut selection = SelectionState::default();
    let node_id = NodeId::new_v4();

    selection.select_node(node_id, false);
    assert!(selection.is_node_selected(node_id));
    assert!(!selection.is_empty());
}

#[test]
fn test_selection_multi_select() {
    let mut selection = SelectionState::default();
    let id1 = NodeId::new_v4();
    let id2 = NodeId::new_v4();

    selection.select_node(id1, false);
    selection.select_node(id2, true); // Add to selection

    assert!(selection.is_node_selected(id1));
    assert!(selection.is_node_selected(id2));
    assert_eq!(selection.selected_nodes.len(), 2);
}

#[test]
fn test_selection_clear() {
    let mut selection = SelectionState::default();
    let node_id = NodeId::new_v4();

    selection.select_node(node_id, false);
    selection.clear();

    assert!(selection.is_empty());
}

#[test]
fn test_selection_toggle_node() {
    let mut selection = SelectionState::default();
    let node_id = NodeId::new_v4();

    selection.toggle_node(node_id);
    assert!(selection.is_node_selected(node_id));

    selection.toggle_node(node_id);
    assert!(!selection.is_node_selected(node_id));
}

// ============================================================================
// ViewportState Tests
// ============================================================================

#[test]
fn test_viewport_default() {
    let viewport = ViewportState::default();
    assert_eq!(viewport.zoom, 1.0);
    assert_eq!(viewport.offset.x, 0.0);
    assert_eq!(viewport.offset.y, 0.0);
}

#[test]
fn test_viewport_screen_to_canvas() {
    let viewport = ViewportState::default();
    let canvas_pos = viewport.screen_to_canvas(100.0, 100.0);
    assert_eq!(canvas_pos.x, 100.0);
    assert_eq!(canvas_pos.y, 100.0);
}

#[test]
fn test_viewport_screen_to_canvas_with_offset() {
    let mut viewport = ViewportState::default();
    viewport.offset = Position::new(50.0, 50.0);

    let canvas_pos = viewport.screen_to_canvas(100.0, 100.0);
    assert_eq!(canvas_pos.x, 50.0);
    assert_eq!(canvas_pos.y, 50.0);
}

#[test]
fn test_viewport_screen_to_canvas_with_zoom() {
    let mut viewport = ViewportState::default();
    viewport.zoom = 2.0;

    let canvas_pos = viewport.screen_to_canvas(200.0, 200.0);
    assert_eq!(canvas_pos.x, 100.0);
    assert_eq!(canvas_pos.y, 100.0);
}

#[test]
fn test_viewport_canvas_to_screen() {
    let viewport = ViewportState::default();
    let pos = Position::new(100.0, 100.0);
    let screen_pos = viewport.canvas_to_screen(&pos);
    assert_eq!(screen_pos.x, 100.0);
    assert_eq!(screen_pos.y, 100.0);
}

#[test]
fn test_viewport_roundtrip() {
    let mut viewport = ViewportState::default();
    viewport.offset = Position::new(30.0, 40.0);
    viewport.zoom = 1.5;

    let original = Position::new(100.0, 200.0);
    let screen = viewport.canvas_to_screen(&original);
    let back = viewport.screen_to_canvas(screen.x, screen.y);

    assert!((original.x - back.x).abs() < 0.001);
    assert!((original.y - back.y).abs() < 0.001);
}

// ============================================================================
// HitTester Tests
// ============================================================================

#[test]
fn test_hit_test_empty_graph() {
    let hit_tester = HitTester::new();
    let graph = WorkflowGraph::new();
    let result = hit_tester.hit_test(Position::new(0.0, 0.0), &graph);
    assert_eq!(result, HitTestResult::Canvas);
}

#[test]
fn test_hit_test_node() {
    let hit_tester = HitTester::new();
    let mut graph = WorkflowGraph::new();

    let node = WorkflowNodeData::new("Node", Position::new(100.0, 100.0)).with_size(160.0, 80.0);
    let id = node.id;
    graph.add_node(node);

    // Hit inside node
    let result = hit_tester.hit_test(Position::new(150.0, 140.0), &graph);
    assert_eq!(result, HitTestResult::Node(id));

    // Miss outside node - hits canvas background
    let result = hit_tester.hit_test(Position::new(0.0, 0.0), &graph);
    assert_eq!(result, HitTestResult::Canvas);
}

#[test]
fn test_hit_test_input_port() {
    let hit_tester = HitTester::new();
    let mut graph = WorkflowGraph::new();

    let node = WorkflowNodeData::new("Node", Position::new(100.0, 100.0))
        .with_size(160.0, 80.0)
        .with_ports(1, 1);
    let id = node.id;
    let port_pos = node.input_port_position(0);
    graph.add_node(node);

    let result = hit_tester.hit_test(port_pos, &graph);
    assert_eq!(result, HitTestResult::InputPort(id, 0));
}

#[test]
fn test_hit_test_output_port() {
    let hit_tester = HitTester::new();
    let mut graph = WorkflowGraph::new();

    let node = WorkflowNodeData::new("Node", Position::new(100.0, 100.0))
        .with_size(160.0, 80.0)
        .with_ports(1, 1);
    let id = node.id;
    let port_pos = node.output_port_position(0);
    graph.add_node(node);

    let result = hit_tester.hit_test(port_pos, &graph);
    assert_eq!(result, HitTestResult::OutputPort(id, 0));
}

// ============================================================================
// History (Command Pattern) Tests
// ============================================================================

#[test]
fn test_history_add_node_command() {
    let mut graph = WorkflowGraph::new();
    let node = WorkflowNodeData::new("Test", Position::new(0.0, 0.0));
    let id = node.id;

    let cmd = AddNodeCommand { node: node.clone() };

    // Execute
    cmd.execute(&mut graph);
    assert!(graph.nodes.contains_key(&id));

    // Undo
    cmd.undo(&mut graph);
    assert!(!graph.nodes.contains_key(&id));
}

#[test]
fn test_history_remove_node_command() {
    let mut graph = WorkflowGraph::new();
    let node = WorkflowNodeData::new("Test", Position::new(0.0, 0.0));
    let id = node.id;
    graph.add_node(node.clone());

    let cmd = RemoveNodeCommand {
        node: node.clone(),
        connections: vec![],
    };

    // Execute
    cmd.execute(&mut graph);
    assert!(!graph.nodes.contains_key(&id));

    // Undo
    cmd.undo(&mut graph);
    assert!(graph.nodes.contains_key(&id));
}

#[test]
fn test_history_move_nodes_command() {
    let mut graph = WorkflowGraph::new();
    let node = WorkflowNodeData::new("Test", Position::new(100.0, 100.0));
    let id = node.id;
    graph.add_node(node);

    let old_pos = Position::new(100.0, 100.0);
    let new_pos = Position::new(200.0, 200.0);

    let cmd = MoveNodesCommand {
        moves: vec![(id, old_pos, new_pos)],
    };

    // Execute
    cmd.execute(&mut graph);
    assert_eq!(graph.nodes.get(&id).unwrap().position.x, 200.0);
    assert_eq!(graph.nodes.get(&id).unwrap().position.y, 200.0);

    // Undo
    cmd.undo(&mut graph);
    assert_eq!(graph.nodes.get(&id).unwrap().position.x, 100.0);
    assert_eq!(graph.nodes.get(&id).unwrap().position.y, 100.0);
}

#[test]
fn test_history_add_connection_command() {
    let mut graph = WorkflowGraph::new();

    let node1 = WorkflowNodeData::new("N1", Position::new(0.0, 0.0));
    let node2 = WorkflowNodeData::new("N2", Position::new(200.0, 0.0));
    let id1 = node1.id;
    let id2 = node2.id;
    graph.add_node(node1);
    graph.add_node(node2);

    let conn = Connection::new(id1, 0, id2, 0);

    let cmd = AddConnectionCommand {
        connection: conn.clone(),
    };

    // Execute
    cmd.execute(&mut graph);
    assert_eq!(graph.connections.len(), 1);

    // Undo
    cmd.undo(&mut graph);
    assert_eq!(graph.connections.len(), 0);
}

#[test]
fn test_history_manager_undo_redo() {
    let mut graph = WorkflowGraph::new();
    let mut history = HistoryManager::new();

    let node = WorkflowNodeData::new("Test", Position::new(0.0, 0.0));
    let id = node.id;

    // Execute add command
    history.execute(Box::new(AddNodeCommand { node: node.clone() }), &mut graph);
    assert!(graph.nodes.contains_key(&id));
    assert!(history.can_undo());
    assert!(!history.can_redo());

    // Undo
    assert!(history.undo(&mut graph));
    assert!(!graph.nodes.contains_key(&id));
    assert!(!history.can_undo());
    assert!(history.can_redo());

    // Redo
    assert!(history.redo(&mut graph));
    assert!(graph.nodes.contains_key(&id));
    assert!(history.can_undo());
    assert!(!history.can_redo());
}

#[test]
fn test_history_manager_clear_redo_on_new_action() {
    let mut graph = WorkflowGraph::new();
    let mut history = HistoryManager::new();

    let node1 = WorkflowNodeData::new("N1", Position::new(0.0, 0.0));
    let node2 = WorkflowNodeData::new("N2", Position::new(100.0, 0.0));

    history.execute(
        Box::new(AddNodeCommand {
            node: node1.clone(),
        }),
        &mut graph,
    );
    history.undo(&mut graph);
    assert!(history.can_redo());

    // New action clears redo stack
    history.execute(
        Box::new(AddNodeCommand {
            node: node2.clone(),
        }),
        &mut graph,
    );
    assert!(!history.can_redo());
}

// ============================================================================
// Bezier Curve Tests
// ============================================================================

#[test]
fn test_horizontal_bezier() {
    let from = Position::new(0.0, 0.0);
    let to = Position::new(100.0, 0.0);

    let (p0, p1, p2, p3) = horizontal_bezier(from, to);

    assert_eq!(p0.x, 0.0);
    assert_eq!(p3.x, 100.0);
    // Control points should be between start and end
    assert!(p1.x > p0.x && p1.x < p3.x);
    assert!(p2.x > p0.x && p2.x < p3.x);
}

#[test]
fn test_flatten_cubic_bezier() {
    let p0 = Position::new(0.0, 0.0);
    let p1 = Position::new(33.0, 0.0);
    let p2 = Position::new(66.0, 0.0);
    let p3 = Position::new(100.0, 0.0);

    let points = flatten_cubic_bezier(p0, p1, p2, p3, 1.0);

    // Should have at least start and end points
    assert!(points.len() >= 2);
    assert_eq!(points[0].x, 0.0);
    assert_eq!(points[points.len() - 1].x, 100.0);
}

#[test]
fn test_connection_path() {
    let from = Position::new(0.0, 0.0);
    let to = Position::new(100.0, 50.0);

    let points = connection_path(from, to, 1.0);

    assert!(points.len() >= 2);
    assert_eq!(points[0].x, 0.0);
    assert_eq!(points[0].y, 0.0);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_workflow_scenario() {
    let mut graph = WorkflowGraph::new();
    let mut history = HistoryManager::new();

    // Add three nodes
    let node1 = WorkflowNodeData::new("Input", Position::new(0.0, 50.0)).with_ports(0, 2);
    let node2 = WorkflowNodeData::new("Process", Position::new(200.0, 0.0)).with_ports(1, 1);
    let node3 = WorkflowNodeData::new("Output", Position::new(400.0, 50.0)).with_ports(2, 0);

    let id1 = node1.id;
    let id2 = node2.id;
    let id3 = node3.id;

    history.execute(Box::new(AddNodeCommand { node: node1 }), &mut graph);
    history.execute(Box::new(AddNodeCommand { node: node2 }), &mut graph);
    history.execute(Box::new(AddNodeCommand { node: node3 }), &mut graph);

    assert_eq!(graph.nodes.len(), 3);

    // Add connections
    let conn1 = Connection::new(id1, 0, id2, 0);
    let conn2 = Connection::new(id2, 0, id3, 0);
    history.execute(
        Box::new(AddConnectionCommand { connection: conn1 }),
        &mut graph,
    );
    history.execute(
        Box::new(AddConnectionCommand { connection: conn2 }),
        &mut graph,
    );

    assert_eq!(graph.connections.len(), 2);

    // Move a node
    let old_pos = graph.nodes.get(&id2).unwrap().position;
    let new_pos = Position::new(250.0, 50.0);
    history.execute(
        Box::new(MoveNodesCommand {
            moves: vec![(id2, old_pos, new_pos)],
        }),
        &mut graph,
    );

    assert_eq!(graph.nodes.get(&id2).unwrap().position.x, 250.0);

    // Undo the move
    history.undo(&mut graph);
    assert_eq!(graph.nodes.get(&id2).unwrap().position.x, 200.0);

    // Undo a connection
    history.undo(&mut graph);
    assert_eq!(graph.connections.len(), 1);

    // Undo another connection
    history.undo(&mut graph);
    assert_eq!(graph.connections.len(), 0);

    // Redo connections
    history.redo(&mut graph);
    history.redo(&mut graph);
    assert_eq!(graph.connections.len(), 2);
}

#[test]
fn test_node_removal_cascade() {
    let mut graph = WorkflowGraph::new();

    // Create nodes
    let node1 = WorkflowNodeData::new("A", Position::new(0.0, 0.0));
    let node2 = WorkflowNodeData::new("B", Position::new(200.0, 0.0));
    let node3 = WorkflowNodeData::new("C", Position::new(400.0, 0.0));

    let id1 = node1.id;
    let id2 = node2.id;
    let id3 = node3.id;

    graph.add_node(node1);
    graph.add_node(node2);
    graph.add_node(node3);

    // Create chain: A -> B -> C
    graph.add_connection(id1, 0, id2, 0).unwrap();
    graph.add_connection(id2, 0, id3, 0).unwrap();

    assert_eq!(graph.connections.len(), 2);

    // Remove middle node - should remove both connections
    graph.remove_node(id2);

    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.connections.len(), 0);
}

#[test]
fn test_serialization_roundtrip() {
    let mut graph = WorkflowGraph::new();

    let node1 = WorkflowNodeData::new("Node 1", Position::new(100.0, 200.0))
        .with_ports(1, 2)
        .with_size(180.0, 90.0);
    let node2 = WorkflowNodeData::new("Node 2", Position::new(300.0, 200.0)).with_ports(2, 1);

    let id1 = node1.id;
    let id2 = node2.id;

    graph.add_node(node1);
    graph.add_node(node2);
    graph.add_connection(id1, 0, id2, 0).unwrap();

    // Serialize
    let json = serde_json::to_string(&graph).expect("Serialization failed");

    // Deserialize
    let restored: WorkflowGraph = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(restored.nodes.len(), 2);
    assert_eq!(restored.connections.len(), 1);
    assert!(restored.nodes.contains_key(&id1));
    assert!(restored.nodes.contains_key(&id2));

    let restored_node1 = restored.nodes.get(&id1).unwrap();
    assert_eq!(restored_node1.title, "Node 1");
    assert_eq!(restored_node1.position.x, 100.0);
}

#[test]
fn test_hit_test_priority() {
    // Ports should have higher priority than nodes
    let hit_tester = HitTester::new();
    let mut graph = WorkflowGraph::new();

    let node = WorkflowNodeData::new("Node", Position::new(100.0, 100.0))
        .with_size(160.0, 80.0)
        .with_ports(1, 1);
    let id = node.id;
    let input_port = node.input_port_position(0);
    graph.add_node(node);

    // Click exactly on input port should return port, not node
    let result = hit_tester.hit_test(input_port, &graph);
    match result {
        HitTestResult::InputPort(node_id, port) => {
            assert_eq!(node_id, id);
            assert_eq!(port, 0);
        }
        _ => panic!("Expected InputPort hit result"),
    }
}
