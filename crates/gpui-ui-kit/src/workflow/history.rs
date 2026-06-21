//! Undo/Redo history management using command pattern

use super::state::{Connection, NodeId, Position, WorkflowGraph, WorkflowNodeData};

/// A command that can be executed and undone
pub trait Command: std::fmt::Debug + Send + Sync {
    /// Execute the command
    fn execute(&self, graph: &mut WorkflowGraph);

    /// Undo the command
    fn undo(&self, graph: &mut WorkflowGraph);

    /// Get a description of this command
    fn description(&self) -> &str;
}

/// Command to add a node
#[derive(Debug, Clone)]
pub struct AddNodeCommand {
    pub node: WorkflowNodeData,
}

impl Command for AddNodeCommand {
    fn execute(&self, graph: &mut WorkflowGraph) {
        graph.add_node(self.node.clone());
    }

    fn undo(&self, graph: &mut WorkflowGraph) {
        graph.remove_node(self.node.id);
    }

    fn description(&self) -> &str {
        "Add node"
    }
}

/// Command to remove a node (including its connections)
#[derive(Debug, Clone)]
pub struct RemoveNodeCommand {
    pub node: WorkflowNodeData,
    pub connections: Vec<Connection>,
}

impl Command for RemoveNodeCommand {
    fn execute(&self, graph: &mut WorkflowGraph) {
        graph.remove_node(self.node.id);
    }

    fn undo(&self, graph: &mut WorkflowGraph) {
        // Restore the node
        graph.add_node(self.node.clone());

        // Restore all connections
        for conn in &self.connections {
            // Add connection directly without validation since we're restoring state
            graph.connections.push(conn.clone());
        }
    }

    fn description(&self) -> &str {
        "Remove node"
    }
}

/// Command to move nodes
#[derive(Debug, Clone)]
pub struct MoveNodesCommand {
    /// (node_id, old_position, new_position)
    pub moves: Vec<(NodeId, Position, Position)>,
}

impl Command for MoveNodesCommand {
    fn execute(&self, graph: &mut WorkflowGraph) {
        for (node_id, _, new_pos) in &self.moves {
            if let Some(node) = graph.nodes.get_mut(node_id) {
                node.position = *new_pos;
            }
        }
    }

    fn undo(&self, graph: &mut WorkflowGraph) {
        for (node_id, old_pos, _) in &self.moves {
            if let Some(node) = graph.nodes.get_mut(node_id) {
                node.position = *old_pos;
            }
        }
    }

    fn description(&self) -> &str {
        "Move nodes"
    }
}

/// Command to add a connection
#[derive(Debug, Clone)]
pub struct AddConnectionCommand {
    pub connection: Connection,
}

impl Command for AddConnectionCommand {
    fn execute(&self, graph: &mut WorkflowGraph) {
        graph.connections.push(self.connection.clone());
    }

    fn undo(&self, graph: &mut WorkflowGraph) {
        graph.remove_connection(self.connection.id);
    }

    fn description(&self) -> &str {
        "Add connection"
    }
}

/// Command to remove a connection
#[derive(Debug, Clone)]
pub struct RemoveConnectionCommand {
    pub connection: Connection,
}

impl Command for RemoveConnectionCommand {
    fn execute(&self, graph: &mut WorkflowGraph) {
        graph.remove_connection(self.connection.id);
    }

    fn undo(&self, graph: &mut WorkflowGraph) {
        graph.connections.push(self.connection.clone());
    }

    fn description(&self) -> &str {
        "Remove connection"
    }
}

/// Command to change a node's port counts (for bulk-connect port growth)
#[derive(Debug, Clone)]
pub struct ChangePortCountsCommand {
    pub node_id: NodeId,
    pub old_input_count: usize,
    pub new_input_count: usize,
    pub old_output_count: usize,
    pub new_output_count: usize,
    pub old_height: f32,
    pub new_height: f32,
}

impl Command for ChangePortCountsCommand {
    fn execute(&self, graph: &mut WorkflowGraph) {
        if let Some(node) = graph.nodes.get_mut(&self.node_id) {
            node.input_count = self.new_input_count;
            node.output_count = self.new_output_count;
            node.height = self.new_height;
        }
    }

    fn undo(&self, graph: &mut WorkflowGraph) {
        if let Some(node) = graph.nodes.get_mut(&self.node_id) {
            node.input_count = self.old_input_count;
            node.output_count = self.old_output_count;
            node.height = self.old_height;
        }
    }

    fn description(&self) -> &str {
        "Change port counts"
    }
}

/// A composite command that groups multiple commands together
#[derive(Debug)]
#[allow(dead_code)]
pub struct CompositeCommand {
    pub commands: Vec<Box<dyn Command>>,
    pub description: String,
}

#[allow(dead_code)]
impl CompositeCommand {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            commands: Vec::new(),
            description: description.into(),
        }
    }

    pub fn add(&mut self, command: Box<dyn Command>) {
        self.commands.push(command);
    }

    pub fn with_command(mut self, command: Box<dyn Command>) -> Self {
        self.commands.push(command);
        self
    }
}

impl Command for CompositeCommand {
    fn execute(&self, graph: &mut WorkflowGraph) {
        for cmd in &self.commands {
            cmd.execute(graph);
        }
    }

    fn undo(&self, graph: &mut WorkflowGraph) {
        // Undo in reverse order
        for cmd in self.commands.iter().rev() {
            cmd.undo(graph);
        }
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// History manager for undo/redo
#[derive(Debug, Default)]
pub struct HistoryManager {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    max_history: usize,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self::with_max_history(100)
    }

    pub fn with_max_history(max: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: max,
        }
    }

    /// Execute a command and add it to the history
    pub fn execute(&mut self, command: Box<dyn Command>, graph: &mut WorkflowGraph) {
        command.execute(graph);
        self.undo_stack.push(command);
        self.redo_stack.clear();

        // Trim history if needed
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Record a command without executing (for when changes are already applied)
    pub fn record(&mut self, command: Box<dyn Command>) {
        self.undo_stack.push(command);
        self.redo_stack.clear();

        // Trim history if needed
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last command
    pub fn undo(&mut self, graph: &mut WorkflowGraph) -> bool {
        if let Some(command) = self.undo_stack.pop() {
            command.undo(graph);
            self.redo_stack.push(command);
            true
        } else {
            false
        }
    }

    /// Redo the last undone command
    pub fn redo(&mut self, graph: &mut WorkflowGraph) -> bool {
        if let Some(command) = self.redo_stack.pop() {
            command.execute(graph);
            self.undo_stack.push(command);
            true
        } else {
            false
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the description of the next undo command
    pub fn undo_description(&self) -> Option<&str> {
        self.undo_stack.last().map(|c| c.description())
    }

    /// Get the description of the next redo command
    pub fn redo_description(&self) -> Option<&str> {
        self.redo_stack.last().map(|c| c.description())
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Position, WorkflowGraph, WorkflowNodeData};
    use super::{AddNodeCommand, CompositeCommand, HistoryManager, MoveNodesCommand};

    #[test]
    fn test_add_node_undo_redo() {
        let mut graph = WorkflowGraph::new();
        let mut history = HistoryManager::new();

        let node = WorkflowNodeData::new("Test Node", Position::new(100.0, 100.0));
        let node_id = node.id;

        // Add node
        history.execute(Box::new(AddNodeCommand { node: node.clone() }), &mut graph);
        assert!(graph.nodes.contains_key(&node_id));

        // Undo
        assert!(history.undo(&mut graph));
        assert!(!graph.nodes.contains_key(&node_id));

        // Redo
        assert!(history.redo(&mut graph));
        assert!(graph.nodes.contains_key(&node_id));
    }

    #[test]
    fn test_move_nodes_undo() {
        let mut graph = WorkflowGraph::new();
        let mut history = HistoryManager::new();

        let node = WorkflowNodeData::new("Test Node", Position::new(100.0, 100.0));
        let node_id = node.id;
        graph.add_node(node);

        // Move node
        let old_pos = Position::new(100.0, 100.0);
        let new_pos = Position::new(200.0, 200.0);
        history.execute(
            Box::new(MoveNodesCommand {
                moves: vec![(node_id, old_pos, new_pos)],
            }),
            &mut graph,
        );

        assert_eq!(graph.nodes.get(&node_id).unwrap().position, new_pos);

        // Undo
        history.undo(&mut graph);
        assert_eq!(graph.nodes.get(&node_id).unwrap().position, old_pos);
    }

    #[test]
    fn test_composite_command() {
        let mut graph = WorkflowGraph::new();
        let mut history = HistoryManager::new();

        let node1 = WorkflowNodeData::new("Node 1", Position::new(100.0, 100.0));
        let node2 = WorkflowNodeData::new("Node 2", Position::new(200.0, 200.0));
        let id1 = node1.id;
        let id2 = node2.id;

        // Create composite command
        let composite = CompositeCommand::new("Add two nodes")
            .with_command(Box::new(AddNodeCommand { node: node1 }))
            .with_command(Box::new(AddNodeCommand { node: node2 }));

        history.execute(Box::new(composite), &mut graph);
        assert!(graph.nodes.contains_key(&id1));
        assert!(graph.nodes.contains_key(&id2));

        // Undo should remove both
        history.undo(&mut graph);
        assert!(!graph.nodes.contains_key(&id1));
        assert!(!graph.nodes.contains_key(&id2));
    }

    use super::super::Connection;
    use super::{ChangePortCountsCommand, RemoveNodeCommand};

    #[test]
    fn test_remove_node_undo_redo() {
        let mut graph = WorkflowGraph::new();
        let mut history = HistoryManager::new();

        let node = WorkflowNodeData::new("N", Position::new(0.0, 0.0));
        let id = node.id;
        graph.add_node(node.clone());
        let conn = Connection::new(id, 0, id, 0);
        let conn_id = conn.id;
        graph.connections.push(conn.clone());

        history.execute(Box::new(RemoveNodeCommand { node, connections: vec![conn] }), &mut graph);
        assert!(!graph.nodes.contains_key(&id));
        assert!(graph.connections.iter().all(|c| c.id != conn_id));

        history.undo(&mut graph);
        assert!(graph.nodes.contains_key(&id));
        assert!(graph.connections.iter().any(|c| c.id == conn_id));
    }

    #[test]
    fn test_change_port_counts_undo() {
        let mut graph = WorkflowGraph::new();
        let mut history = HistoryManager::new();

        let node = WorkflowNodeData::new("N", Position::new(0.0, 0.0));
        let id = node.id;
        graph.add_node(node);

        history.execute(
            Box::new(ChangePortCountsCommand {
                node_id: id,
                old_input_count: 1,
                new_input_count: 4,
                old_output_count: 1,
                new_output_count: 2,
                old_height: 100.0,
                new_height: 150.0,
            }),
            &mut graph,
        );
        assert_eq!(graph.nodes[&id].input_count, 4);
        assert_eq!(graph.nodes[&id].output_count, 2);
        assert_eq!(graph.nodes[&id].height, 150.0);

        history.undo(&mut graph);
        assert_eq!(graph.nodes[&id].input_count, 1);
        assert_eq!(graph.nodes[&id].output_count, 1);
        assert_eq!(graph.nodes[&id].height, 100.0);
    }

    #[test]
    fn test_history_record_and_descriptions() {
        let mut graph = WorkflowGraph::new();
        let mut history = HistoryManager::new();

        let node = WorkflowNodeData::new("N", Position::new(0.0, 0.0));
        let cmd = AddNodeCommand { node };
        history.record(Box::new(cmd));
        assert!(history.can_undo());
        assert_eq!(history.undo_description(), Some("Add node"));
        assert!(!history.can_redo());
        assert!(history.redo_description().is_none());

        history.undo(&mut graph);
        assert!(!history.can_undo());
        assert!(history.can_redo());
        assert_eq!(history.redo_description(), Some("Add node"));
    }

    #[test]
    fn test_composite_command_via_add() {
        let mut graph = WorkflowGraph::new();
        let mut history = HistoryManager::new();

        let mut composite = CompositeCommand::new("Add pair");
        let n1 = WorkflowNodeData::new("A", Position::new(0.0, 0.0));
        let n2 = WorkflowNodeData::new("B", Position::new(10.0, 10.0));
        composite.add(Box::new(AddNodeCommand { node: n1 }));
        composite.add(Box::new(AddNodeCommand { node: n2 }));

        history.execute(Box::new(composite), &mut graph);
        assert_eq!(graph.nodes.len(), 2);
        history.undo(&mut graph);
        assert!(graph.nodes.is_empty());
    }

    #[test]
    fn test_history_clear() {
        let mut graph = WorkflowGraph::new();
        let mut history = HistoryManager::new();
        history.execute(Box::new(AddNodeCommand { node: WorkflowNodeData::new("N", Position::new(0.0, 0.0)) }), &mut graph);
        history.clear();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

}
