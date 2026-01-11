//! Workflow Canvas - A ReactFlow-like node graph editor
//!
//! Provides a GPU-accelerated canvas for building node-based workflows with:
//! - Draggable nodes with custom content
//! - Directional connections between input/output ports
//! - Selection (single, multi, box selection)
//! - Pan/zoom navigation
//! - Undo/redo history
//! - Copy/paste support
//! - State persistence with versioned JSON

mod bezier;
mod canvas;
mod history;
mod hit_test;
mod node;
mod port;
mod state;
mod theme;

#[cfg(test)]
mod tests;

// Re-export main types
pub use canvas::WorkflowCanvas;
pub use history::{Command, HistoryManager};
pub use hit_test::{HitTestResult, HitTester};
pub use node::{NodeContent, WorkflowNode};
pub use port::{Port, PortDirection};
pub use state::{
    BoxSelection, CanvasState, Connection, ConnectionDrag, ConnectionId, InteractionMode, LinkType,
    NodeDragState, NodeId, Position, SelectionState, ViewportState, WorkflowGraph,
    WorkflowNodeData,
};
pub use theme::WorkflowTheme;
