use super::misc::axis_name;
use super::option::option_text;
use crate::Axis;
use crate::util::format_number;

/// One node in an inspected solved layout tree.
#[derive(Debug, Clone, PartialEq)]
pub struct SolvedInspectionNode {
    /// Slash-separated stable path from the solved root.
    pub path: String,
    /// Node id from the solved tree.
    pub id: String,
    /// Resolved width in pixels.
    pub width: f32,
    /// Resolved height in pixels.
    pub height: f32,
    /// Whether this node is visible.
    pub visible: bool,
    /// Active display tier, if any.
    pub active_tier: Option<String>,
    /// Collapse label, if any.
    pub collapse_label: Option<String>,
    /// Resolved axis for containers.
    pub resolved_axis: Option<Axis>,
    /// Inspected children in solved order.
    pub children: Vec<SolvedInspectionNode>,
}

impl SolvedInspectionNode {
    pub(super) fn collect_nodes<'a>(&'a self, nodes: &mut Vec<&'a SolvedInspectionNode>) {
        nodes.push(self);
        for child in &self.children {
            child.collect_nodes(nodes);
        }
    }

    pub(super) fn write_text(&self, output: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        let visible = if self.visible { "visible" } else { "collapsed" };
        output.push_str(&format!(
            "{pad}- {path} size={width}x{height} {visible} axis={axis} tier={tier} \
             collapse_label={label} children={children}\n",
            path = self.path,
            width = format_number(self.width),
            height = format_number(self.height),
            axis = self.resolved_axis.map(axis_name).unwrap_or("-"),
            tier = option_text(self.active_tier.as_deref()),
            label = option_text(self.collapse_label.as_deref()),
            children = self.children.len(),
        ));

        for child in &self.children {
            child.write_text(output, indent + 1);
        }
    }
}
