use super::connection::Connection;
use super::types::ConnectionId;
use super::types::NodeId;
use super::workflow_node_data::WorkflowNodeData;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// The complete workflow graph
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowGraph {
    pub nodes: HashMap<NodeId, WorkflowNodeData>,
    pub connections: Vec<Connection>,
    /// ID counter for generating sequential IDs if needed
    #[serde(skip)]
    #[allow(dead_code)]
    pub(super) next_id: usize,
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
        let from_data = match self.nodes.get(&from_node) {
            Some(n) => n,
            None => return Err("Source node not found"),
        };
        let to_data = match self.nodes.get(&to_node) {
            Some(n) => n,
            None => return Err("Target node not found"),
        };

        // Validate port indices
        if from_port >= from_data.output_count {
            return Err("Output port index out of bounds");
        }
        if to_port >= to_data.input_count {
            return Err("Input port index out of bounds");
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
    pub fn would_create_cycle(&self, from_node: NodeId, to_node: NodeId) -> bool {
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
