use super::layout_inspection::LayoutInspection;
use super::layout_inspection_node::LayoutInspectionNode;
use super::misc::node_path;
use super::sizing_inspection::SizingInspection;
use super::solved_inspection::SolvedInspection;
use super::solved_inspection_node::SolvedInspectionNode;
use super::types::ContainerInspection;
use super::types::DisplayTierInspection;
use super::types::LayoutInspectionKind;
use super::types::SlotInspection;
use crate::{DisplayTier, LayoutNode, Sizing, SolvedNode};

/// Inspect a layout declaration tree.
pub fn inspect_layout(root: &LayoutNode<'_>) -> LayoutInspection {
    LayoutInspection {
        root: inspect_layout_node(root, None),
    }
}

/// Inspect a solved layout tree.
pub fn inspect_solved(root: &SolvedNode<'_>) -> SolvedInspection {
    SolvedInspection {
        root: inspect_solved_node(root, None),
    }
}

fn inspect_layout_node(node: &LayoutNode<'_>, parent_path: Option<&str>) -> LayoutInspectionNode {
    let id = node.id();
    let path = node_path(parent_path, id);
    match node {
        LayoutNode::Slot(slot) => LayoutInspectionNode {
            path,
            id: id.to_string(),
            kind: LayoutInspectionKind::Slot,
            sizing: inspect_sizing(slot.sizing),
            slot: Some(SlotInspection {
                priority: slot.priority,
                collapsible: slot.collapsible,
                collapse_label: slot.collapse_label.map(str::to_string),
                display_tiers: inspect_display_tiers(slot.display_tiers),
            }),
            container: None,
            children: Vec::new(),
        },
        LayoutNode::Container(container) => {
            let children = container
                .children
                .iter()
                .map(|child| inspect_layout_node(child, Some(&path)))
                .collect();
            LayoutInspectionNode {
                path,
                id: id.to_string(),
                kind: LayoutInspectionKind::Container,
                sizing: inspect_sizing(container.sizing),
                slot: None,
                container: Some(ContainerInspection {
                    axis: container.axis,
                    auto_axis: container.auto_axis,
                    divider_size: container.divider_size,
                }),
                children,
            }
        }
    }
}

fn inspect_solved_node(node: &SolvedNode<'_>, parent_path: Option<&str>) -> SolvedInspectionNode {
    let path = node_path(parent_path, node.id);
    let children = node
        .children
        .iter()
        .map(|child| inspect_solved_node(child, Some(&path)))
        .collect();
    SolvedInspectionNode {
        path,
        id: node.id.to_string(),
        width: node.width,
        height: node.height,
        visible: node.visible,
        active_tier: node.active_tier.map(str::to_string),
        collapse_label: node.collapse_label.map(str::to_string),
        resolved_axis: node.resolved_axis,
        children,
    }
}

fn inspect_sizing(sizing: Sizing<'_>) -> SizingInspection {
    match sizing {
        Sizing::Fixed(size) => SizingInspection::Fixed { size },
        Sizing::Fractional { initial, min, max } => {
            SizingInspection::Fractional { initial, min, max }
        }
        Sizing::Flex { min, weight } => SizingInspection::Flex { min, weight },
        Sizing::Text {
            text,
            line_height,
            min,
            ..
        } => SizingInspection::Text {
            text: text.to_string(),
            line_height,
            min,
        },
    }
}

fn inspect_display_tiers(tiers: &[DisplayTier<'_>]) -> Vec<DisplayTierInspection> {
    tiers
        .iter()
        .map(|tier| DisplayTierInspection {
            name: tier.name.to_string(),
            min_size: tier.min_size,
        })
        .collect()
}
