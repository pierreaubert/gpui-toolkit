use gpui_ui_kit::{Position, WizardStep, WorkflowGraph, WorkflowNodeData};

pub(super) fn sample_wizard_steps() -> Vec<WizardStep> {
    vec![
        WizardStep::new("props", "Props").description("Edit story props"),
        WizardStep::new("preview", "Preview").description("Inspect responsive output"),
        WizardStep::new("qa", "QA").description("Run conformance"),
    ]
}

pub(super) fn sample_workflow_graph(label: impl Into<String>) -> WorkflowGraph {
    let mut graph = WorkflowGraph::new();
    let source = graph.add_node(
        WorkflowNodeData::new(label, Position::new(48.0, 72.0))
            .with_ports(0, 1)
            .with_size(150.0, 90.0),
    );
    let sink = graph.add_node(
        WorkflowNodeData::new("Preview", Position::new(250.0, 96.0))
            .with_ports(1, 0)
            .with_size(150.0, 90.0),
    );
    let _ = graph.add_connection(source, 0, sink, 0);
    graph
}
