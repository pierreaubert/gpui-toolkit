use super::position::Position;
use super::types::NodeId;
use serde::{Deserialize, Serialize};

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
    /// Maximum number of input ports this node can have (None = no limit)
    #[serde(default)]
    pub max_input_count: Option<usize>,
    /// Maximum number of output ports this node can have (None = no limit)
    #[serde(default)]
    pub max_output_count: Option<usize>,
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
            max_input_count: None,
            max_output_count: None,
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

    /// Set maximum port counts (None = no limit)
    pub fn with_max_ports(mut self, max_inputs: Option<usize>, max_outputs: Option<usize>) -> Self {
        self.max_input_count = max_inputs;
        self.max_output_count = max_outputs;
        self
    }

    /// Create with user data
    pub fn with_user_data(mut self, data: serde_json::Value) -> Self {
        self.user_data = data;
        self
    }

    /// Grow input_count up to `target`, clamped by max_input_count.
    /// Returns the resulting input_count.
    pub fn grow_inputs_to(&mut self, target: usize) -> usize {
        let max = self.max_input_count.unwrap_or(usize::MAX);
        let new_count = target.min(max);
        if new_count > self.input_count {
            self.input_count = new_count;
            self.ensure_min_height();
        }
        self.input_count
    }

    /// Ensure the node is tall enough for the current port count.
    pub fn ensure_min_height(&mut self) {
        let ports = self.input_count.max(self.output_count);
        let min_height = 48.0 + ports as f32 * 16.0;
        self.height = self.height.max(min_height);
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
        let content_height = (self.height - header_height - 2.0 * border).max(0.0);
        let available = (content_height - 2.0 * padding).max(0.0);

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
        let content_height = (self.height - header_height - 2.0 * border).max(0.0);
        let available = (content_height - 2.0 * padding).max(0.0);

        let y = if self.output_count == 0 {
            self.position.y + self.height / 2.0
        } else {
            let spacing = available / self.output_count as f32;
            self.position.y + border + header_height + padding + spacing * (index as f32 + 0.5)
        };

        Position::new(self.position.x + self.width, y)
    }
}
