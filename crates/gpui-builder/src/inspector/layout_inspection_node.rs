use super::misc::axis_name;
use super::option::option_number;
use super::option::option_text;
use super::sizing_inspection::SizingInspection;
use super::types::ContainerInspection;
use super::types::LayoutInspectionKind;
use super::types::SlotInspection;
use super::types::tiers_text;
use crate::util::format_number;

/// One node in an inspected layout declaration tree.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutInspectionNode {
    /// Slash-separated stable path from the inspected root.
    pub path: String,
    /// Node id from the declaration.
    pub id: String,
    /// Slot or container category.
    pub kind: LayoutInspectionKind,
    /// Owned sizing metadata.
    pub sizing: SizingInspection,
    /// Slot metadata when `kind == Slot`.
    pub slot: Option<SlotInspection>,
    /// Container metadata when `kind == Container`.
    pub container: Option<ContainerInspection>,
    /// Inspected children in declaration order.
    pub children: Vec<LayoutInspectionNode>,
}

impl LayoutInspectionNode {
    pub(super) fn collect_nodes<'a>(&'a self, nodes: &mut Vec<&'a LayoutInspectionNode>) {
        nodes.push(self);
        for child in &self.children {
            child.collect_nodes(nodes);
        }
    }

    pub(super) fn write_text(&self, output: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        match (&self.slot, &self.container) {
            (Some(slot), None) => {
                output.push_str(&format!(
                    "{pad}- {path} slot sizing={sizing} priority={priority} \
                     collapsible={collapsible} collapse_label={label} tiers={tiers}\n",
                    path = self.path,
                    sizing = self.sizing,
                    priority = format_number(slot.priority),
                    collapsible = slot.collapsible,
                    label = option_text(slot.collapse_label.as_deref()),
                    tiers = tiers_text(&slot.display_tiers),
                ));
            }
            (None, Some(container)) => {
                output.push_str(&format!(
                    "{pad}- {path} container sizing={sizing} axis={axis} auto_axis={auto_axis} \
                     divider={divider} children={children}\n",
                    path = self.path,
                    sizing = self.sizing,
                    axis = axis_name(container.axis),
                    auto_axis = option_number(container.auto_axis),
                    divider = format_number(container.divider_size),
                    children = self.children.len(),
                ));
            }
            _ => {
                output.push_str(&format!(
                    "{pad}- {path} unknown sizing={sizing} children={children}\n",
                    path = self.path,
                    sizing = self.sizing,
                    children = self.children.len(),
                ));
            }
        }

        for child in &self.children {
            child.write_text(output, indent + 1);
        }
    }
}
