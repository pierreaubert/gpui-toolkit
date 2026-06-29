use super::types::ConnectionId;
use super::types::LinkType;
use super::types::NodeId;
use serde::{Deserialize, Serialize};

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
