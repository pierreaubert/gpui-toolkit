use super::box_selection::BoxSelection;
use super::selection_state::SelectionState;
use super::types::BulkConnectDrag;
use super::types::ConnectionDrag;
use super::types::ContextMenuState;
use super::types::InteractionMode;
use super::types::NodeDragState;
use super::viewport_state::ViewportState;
use super::workflow_graph::WorkflowGraph;

/// Complete canvas state
#[derive(Debug, Clone)]
pub struct CanvasState {
    pub graph: WorkflowGraph,
    pub viewport: ViewportState,
    pub selection: SelectionState,
    pub mode: InteractionMode,
    pub node_drag: Option<NodeDragState>,
    pub connection_drag: Option<ConnectionDrag>,
    pub bulk_connect_drag: Option<BulkConnectDrag>,
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
            bulk_connect_drag: None,
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
