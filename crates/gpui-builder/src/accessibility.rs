//! Accessibility tree model for solved layouts.
//!
//! This module is platform-agnostic and converts a solved layout tree into an
//! accessibility-oriented tree that renderers or tests can consume.

use std::collections::HashMap;

use crate::SolvedNode;

/// Accessibility role assigned to a layout node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessibilityRole {
    None,
    Group,
    Region,
    Tab,
}

/// Optional per-node metadata used to enrich accessibility output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AccessibilityMetadata<'a> {
    /// Explicit role override.
    pub role: Option<AccessibilityRole>,
    /// Explicit accessible label.
    pub label: Option<&'a str>,
    /// Optional description.
    pub description: Option<&'a str>,
}

/// Accessibility node produced from a solved layout node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityNode {
    pub id: String,
    pub role: AccessibilityRole,
    pub label: Option<String>,
    pub description: Option<String>,
    pub visible: bool,
    pub collapsed: bool,
    pub active_tier: Option<String>,
    pub children: Vec<AccessibilityNode>,
}

impl AccessibilityNode {
    /// Depth-first lookup by node id.
    pub fn find(&self, id: &str) -> Option<&AccessibilityNode> {
        if self.id == id {
            return Some(self);
        }

        for child in &self.children {
            if let Some(found) = child.find(id) {
                return Some(found);
            }
        }

        None
    }
}

/// Accessibility tree rooted at the solved layout root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityTree {
    pub root: AccessibilityNode,
}

impl AccessibilityTree {
    /// Depth-first lookup by node id.
    pub fn find(&self, id: &str) -> Option<&AccessibilityNode> {
        self.root.find(id)
    }
}

/// Build an accessibility tree from solved layout output plus optional
/// per-node metadata keyed by solved node id.
pub fn accessibility_tree_from_solved(
    solved: &SolvedNode<'_>,
    metadata: &[(&str, AccessibilityMetadata<'_>)],
) -> AccessibilityTree {
    let metadata_map: HashMap<&str, AccessibilityMetadata<'_>> = metadata.iter().copied().collect();
    AccessibilityTree {
        root: build_node(solved, &metadata_map),
    }
}

fn build_node(
    node: &SolvedNode<'_>,
    metadata: &HashMap<&str, AccessibilityMetadata<'_>>,
) -> AccessibilityNode {
    let meta = metadata.get(node.id).copied().unwrap_or_default();

    let role = meta.role.unwrap_or_else(|| default_role(node));

    let label = meta.label.map(str::to_string).or_else(|| {
        if !node.visible {
            node.collapse_label.map(str::to_string)
        } else {
            None
        }
    });

    let description = meta.description.map(str::to_string);

    let children = node
        .children
        .iter()
        .map(|child| build_node(child, metadata))
        .collect();

    AccessibilityNode {
        id: node.id.to_string(),
        role,
        label,
        description,
        visible: node.visible,
        collapsed: !node.visible,
        active_tier: node.active_tier.map(str::to_string),
        children,
    }
}

fn default_role(node: &SolvedNode<'_>) -> AccessibilityRole {
    if node.resolved_axis.is_some() {
        return AccessibilityRole::Group;
    }

    if !node.visible && node.collapse_label.is_some() {
        return AccessibilityRole::Tab;
    }

    AccessibilityRole::Region
}

#[cfg(test)]
mod tests {
    use super::{
        AccessibilityMetadata, AccessibilityRole, SolvedNode, accessibility_tree_from_solved,
    };
    use crate::Axis;

    fn solved_tree() -> SolvedNode<'static> {
        SolvedNode {
            id: "root",
            width: 1200.0,
            height: 800.0,
            visible: true,
            active_tier: None,
            collapse_label: None,
            resolved_axis: Some(Axis::Horizontal),
            divider_size: 0.0,
            children: vec![
                SolvedNode {
                    id: "library",
                    width: 320.0,
                    height: 800.0,
                    visible: true,
                    active_tier: None,
                    collapse_label: Some("Library"),
                    resolved_axis: None,
                    divider_size: 0.0,
                    children: Vec::new(),
                },
                SolvedNode {
                    id: "rack",
                    width: 0.0,
                    height: 0.0,
                    visible: false,
                    active_tier: None,
                    collapse_label: Some("Rack"),
                    resolved_axis: None,
                    divider_size: 0.0,
                    children: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn builds_tree_with_default_roles() {
        let solved = solved_tree();
        let tree = accessibility_tree_from_solved(&solved, &[]);

        assert_eq!(tree.root.role, AccessibilityRole::Group);
        assert_eq!(
            tree.find("library").unwrap().role,
            AccessibilityRole::Region
        );

        let rack = tree.find("rack").unwrap();
        assert_eq!(rack.role, AccessibilityRole::Tab);
        assert_eq!(rack.label.as_deref(), Some("Rack"));
        assert!(rack.collapsed);
    }

    #[test]
    fn metadata_can_override_role_and_label() {
        let solved = solved_tree();
        let tree = accessibility_tree_from_solved(
            &solved,
            &[(
                "library",
                AccessibilityMetadata {
                    role: Some(AccessibilityRole::Tab),
                    label: Some("Media Library"),
                    description: Some("Primary browser panel"),
                },
            )],
        );

        let library = tree.find("library").unwrap();
        assert_eq!(library.role, AccessibilityRole::Tab);
        assert_eq!(library.label.as_deref(), Some("Media Library"));
        assert_eq!(
            library.description.as_deref(),
            Some("Primary browser panel")
        );
    }

    #[test]
    fn find_returns_none_for_unknown_id() {
        let solved = solved_tree();
        let tree = accessibility_tree_from_solved(&solved, &[]);
        assert!(tree.find("missing").is_none());
    }
}
