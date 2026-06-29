use super::solved_inspection_node::SolvedInspectionNode;
use std::fmt;

/// Inspectable solved tree export.
#[derive(Debug, Clone, PartialEq)]
pub struct SolvedInspection {
    /// Inspected root node.
    pub root: SolvedInspectionNode,
}

impl SolvedInspection {
    /// Return all solved nodes in deterministic depth-first order.
    pub fn nodes(&self) -> Vec<&SolvedInspectionNode> {
        let mut nodes = Vec::new();
        self.root.collect_nodes(&mut nodes);
        nodes
    }

    /// Render a stable line-oriented report for tests and CLI tooling.
    pub fn to_text(&self) -> String {
        let mut output = String::from("solved inspection:\n");
        self.root.write_text(&mut output, 0);
        output
    }
}

impl fmt::Display for SolvedInspection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text())
    }
}
