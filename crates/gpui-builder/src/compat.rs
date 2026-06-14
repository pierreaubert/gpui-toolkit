//! Compatibility bridge for the existing plugin layout system.
//!
//! Converts `ColumnConstraint` arrays from `sotf-host::plugin_layout` into
//! the generic `LayoutNode` tree, and derives plugin-specific adaptations
//! from the solver output.
//!
//! This module allows gradual migration: existing plugins keep their
//! `ColumnConstraint` definitions, while the generic solver resolves them.

use crate::solved::SolvedNode;
use crate::types::{Axis, ContainerNode, LayoutNode, Sizing, SlotNode};

// ============================================================================
// Plugin Column Role (mirrored from sotf-host to avoid dependency)
// ============================================================================

/// Column role, mirrored from `sotf_host::plugin_layout::ColumnRole`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnRole {
    Config,
    Main,
    Output,
    Diagnostic,
}

/// Column constraint, mirrored from `sotf_host::plugin_layout::ColumnConstraint`.
#[derive(Debug, Clone, Copy)]
pub struct PluginColumnConstraint {
    pub role: ColumnRole,
    pub min_width: f32,
    pub preferred_width: f32,
    pub max_width: f32,
    pub priority: f32,
    pub collapsible: bool,
}

impl PluginColumnConstraint {
    pub const fn config(min_width: f32, priority: f32) -> Self {
        Self {
            role: ColumnRole::Config,
            min_width,
            preferred_width: min_width,
            max_width: min_width,
            priority,
            collapsible: true,
        }
    }

    pub const fn main(min_width: f32) -> Self {
        Self {
            role: ColumnRole::Main,
            min_width,
            preferred_width: 500.0,
            max_width: f32::MAX,
            priority: 1.0,
            collapsible: false,
        }
    }

    pub const fn output(min_width: f32, priority: f32) -> Self {
        Self {
            role: ColumnRole::Output,
            min_width,
            preferred_width: min_width,
            max_width: min_width,
            priority,
            collapsible: true,
        }
    }

    pub const fn diagnostic(min_width: f32, priority: f32) -> Self {
        Self {
            role: ColumnRole::Diagnostic,
            min_width,
            preferred_width: min_width,
            max_width: min_width,
            priority,
            collapsible: true,
        }
    }
}

// ============================================================================
// Conversion: PluginColumnConstraint → LayoutNode
// ============================================================================

fn role_to_id(role: ColumnRole) -> &'static str {
    match role {
        ColumnRole::Config => "config",
        ColumnRole::Main => "main",
        ColumnRole::Output => "output",
        ColumnRole::Diagnostic => "diagnostic",
    }
}

fn role_to_label(role: ColumnRole) -> &'static str {
    match role {
        ColumnRole::Config => "Config",
        ColumnRole::Main => "Main",
        ColumnRole::Output => "Output",
        ColumnRole::Diagnostic => "Diagnostic",
    }
}

fn constraint_to_slot(c: &PluginColumnConstraint) -> LayoutNode<'static> {
    let sizing = if c.role == ColumnRole::Main {
        // Main is always flex
        Sizing::Flex {
            min: c.min_width,
            weight: 1.0,
        }
    } else if (c.max_width - c.min_width).abs() < 1.0 {
        // Fixed-width sidebar (preferred == min == max)
        Sizing::Fixed(c.preferred_width)
    } else {
        // Variable-width (rare for plugin columns, but supported)
        Sizing::Fractional {
            initial: c.preferred_width / 500.0, // rough ratio
            min: c.min_width,
            max: c.max_width,
        }
    };

    LayoutNode::Slot(SlotNode {
        id: role_to_id(c.role),
        sizing,
        priority: c.priority,
        collapsible: c.collapsible,
        display_tiers: &[],
        collapse_label: if c.collapsible {
            Some(role_to_label(c.role))
        } else {
            None
        },
    })
}

/// Convert an array of plugin column constraints into a flat list of
/// `LayoutNode` slot nodes. The caller wraps them in a Container.
///
/// Nodes are ordered: Config, Main, Diagnostic, Output (matching the
/// existing solver's column ordering).
pub fn plugin_constraints_to_slots(
    constraints: &[PluginColumnConstraint],
) -> Vec<LayoutNode<'static>> {
    let order = [
        ColumnRole::Config,
        ColumnRole::Main,
        ColumnRole::Diagnostic,
        ColumnRole::Output,
    ];

    let mut slots = Vec::new();
    for role in &order {
        if let Some(c) = constraints.iter().find(|c| c.role == *role) {
            slots.push(constraint_to_slot(c));
        }
    }

    // If no Main was specified, insert a default one
    if !constraints.iter().any(|c| c.role == ColumnRole::Main) {
        slots.insert(
            slots
                .iter()
                .position(|n| {
                    matches!(
                        n,
                        LayoutNode::Slot(s) if s.id == "diagnostic" || s.id == "output"
                    )
                })
                .unwrap_or(slots.len()),
            LayoutNode::Slot(SlotNode {
                id: "main",
                sizing: Sizing::Flex {
                    min: 300.0,
                    weight: 1.0,
                },
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
        );
    }

    slots
}

// ============================================================================
// Plugin-Specific Adaptations (post-solve)
// ============================================================================

/// Layout solver thresholds matching `sotf_host::design_system::LayoutThresholds`.
#[derive(Debug, Clone, Copy)]
pub struct PluginLayoutThresholds {
    pub vertical_threshold: f32,
    pub group_stack_threshold: f32,
    pub compact_slider_threshold: f32,
    pub hide_viz_threshold: f32,
    pub compact_knob_threshold: f32,
    pub large_knob_threshold: f32,
    pub slider_height_normal: f32,
    pub slider_height_compact: f32,
}

impl Default for PluginLayoutThresholds {
    fn default() -> Self {
        Self {
            vertical_threshold: 400.0,
            group_stack_threshold: 500.0,
            compact_slider_threshold: 700.0,
            hide_viz_threshold: 600.0,
            compact_knob_threshold: 400.0,
            large_knob_threshold: 800.0,
            slider_height_normal: 180.0,
            slider_height_compact: 120.0,
        }
    }
}

/// Orientation of the overall plugin layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// Direction for arranging control groups within the main area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupDirection {
    Row,
    Column,
}

/// Knob size tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnobSize {
    Xs,
    Sm,
    Md,
}

/// Plugin-specific UI adaptations derived from the solved layout.
#[derive(Debug, Clone, Copy)]
pub struct PluginAdaptations {
    pub orientation: Orientation,
    pub group_direction: GroupDirection,
    pub slider_height: f32,
    pub show_visualizations: bool,
    pub knob_size: KnobSize,
}

/// Derive plugin adaptations from a solved layout tree.
///
/// Finds the "main" slot's resolved width and applies threshold rules
/// (identical to the existing `layout_solver.rs` logic).
pub fn plugin_adaptations(
    solved: &SolvedNode<'_>,
    thresholds: &PluginLayoutThresholds,
) -> PluginAdaptations {
    // Determine orientation from the root container's resolved axis
    let orientation = match solved.resolved_axis {
        Some(Axis::Vertical) => Orientation::Vertical,
        _ => Orientation::Horizontal,
    };

    // Find the main slot's width
    let main_width = solved.find("main").map(|n| n.width).unwrap_or(solved.width);

    let group_direction = if main_width < thresholds.group_stack_threshold {
        GroupDirection::Column
    } else {
        GroupDirection::Row
    };

    let slider_height = if main_width < thresholds.compact_slider_threshold {
        thresholds.slider_height_compact
    } else {
        thresholds.slider_height_normal
    };

    let show_visualizations = main_width >= thresholds.hide_viz_threshold;

    let knob_size = if main_width < thresholds.compact_knob_threshold {
        KnobSize::Xs
    } else if main_width >= thresholds.large_knob_threshold {
        KnobSize::Md
    } else {
        KnobSize::Sm
    };

    PluginAdaptations {
        orientation,
        group_direction,
        slider_height,
        show_visualizations,
        knob_size,
    }
}

// ============================================================================
// Convenience: Build complete plugin layout tree
// ============================================================================

/// Build a complete plugin layout container node from column constraints.
///
/// Returns owned data structures that contain the layout tree. The returned
/// `PluginLayoutTree` holds the backing storage and provides a reference to
/// the root `LayoutNode` for passing to the solver.
pub struct PluginLayoutTree {
    slots: Vec<LayoutNode<'static>>,
}

impl PluginLayoutTree {
    pub fn from_constraints(constraints: &[PluginColumnConstraint]) -> Self {
        Self {
            slots: plugin_constraints_to_slots(constraints),
        }
    }

    /// Returns a `ContainerNode` referencing the slots. The returned value
    /// borrows `self`, so the tree must outlive any solver call.
    pub fn as_container(&self) -> ContainerNode<'_> {
        ContainerNode {
            id: "plugin_root",
            axis: Axis::Horizontal,
            auto_axis: None, // plugins use vertical_threshold differently
            sizing: Sizing::flex(0.0),
            children: &self.slots,
            divider_size: 0.0,
        }
    }

    pub fn as_layout_node(&self) -> LayoutNode<'_> {
        LayoutNode::Container(self.as_container())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::{
        GroupDirection, KnobSize, Orientation, PluginColumnConstraint, PluginLayoutThresholds,
        PluginLayoutTree, plugin_adaptations,
    };
    use crate::solver::solve;
    use crate::types::LayoutPreferences;

    fn compressor_constraints() -> Vec<PluginColumnConstraint> {
        vec![
            PluginColumnConstraint::config(100.0, 0.5),
            PluginColumnConstraint::main(300.0),
            PluginColumnConstraint::output(120.0, 0.6),
        ]
    }

    #[test]
    fn plugin_tree_wide_all_visible() {
        let constraints = compressor_constraints();
        let tree = PluginLayoutTree::from_constraints(&constraints);
        let root = tree.as_layout_node();
        let solved = solve(&root, 1200.0, 400.0, &LayoutPreferences::default());

        assert!(solved.find("config").unwrap().visible);
        assert!(solved.find("main").unwrap().visible);
        assert!(solved.find("output").unwrap().visible);

        let adaptations = plugin_adaptations(&solved, &PluginLayoutThresholds::default());
        assert_eq!(adaptations.orientation, Orientation::Horizontal);
    }

    #[test]
    fn plugin_tree_narrow_config_collapses() {
        let constraints = compressor_constraints();
        let tree = PluginLayoutTree::from_constraints(&constraints);
        let root = tree.as_layout_node();
        let solved = solve(&root, 450.0, 400.0, &LayoutPreferences::default());

        // Config (priority 0.5) should collapse before Output (priority 0.6)
        let config = solved.find("config").unwrap();
        assert!(!config.visible);
        assert_eq!(config.collapse_label.as_deref(), Some("Config"));

        let output = solved.find("output").unwrap();
        assert!(output.visible);
    }

    #[test]
    fn plugin_adaptations_from_solved() {
        let constraints = compressor_constraints();
        let tree = PluginLayoutTree::from_constraints(&constraints);
        let root = tree.as_layout_node();
        let thresholds = PluginLayoutThresholds::default();

        // Wide: main gets lots of space
        let solved = solve(&root, 1200.0, 400.0, &LayoutPreferences::default());
        let adapt = plugin_adaptations(&solved, &thresholds);
        assert_eq!(adapt.group_direction, GroupDirection::Row);
        assert_eq!(adapt.slider_height, 180.0);
        assert!(adapt.show_visualizations);
        assert_eq!(adapt.knob_size, KnobSize::Md);

        // Narrow: main is compressed
        let solved = solve(&root, 500.0, 400.0, &LayoutPreferences::default());
        let adapt = plugin_adaptations(&solved, &thresholds);
        assert_eq!(adapt.group_direction, GroupDirection::Column);
        assert_eq!(adapt.slider_height, 120.0);
        assert!(!adapt.show_visualizations);
    }

    #[test]
    fn four_column_diagnostic_collapses_first() {
        let constraints = vec![
            PluginColumnConstraint::config(100.0, 0.5),
            PluginColumnConstraint::main(300.0),
            PluginColumnConstraint::diagnostic(150.0, 0.3),
            PluginColumnConstraint::output(120.0, 0.6),
        ];
        let tree = PluginLayoutTree::from_constraints(&constraints);
        let root = tree.as_layout_node();
        let solved = solve(&root, 600.0, 400.0, &LayoutPreferences::default());

        // Diagnostic (priority 0.3) should collapse first
        assert!(!solved.find("diagnostic").unwrap().visible);
        assert!(solved.find("main").unwrap().visible);
        assert!(solved.find("output").unwrap().visible);
    }

    #[test]
    fn slot_ordering_matches_existing_solver() {
        let constraints = vec![
            PluginColumnConstraint::config(100.0, 0.5),
            PluginColumnConstraint::main(300.0),
            PluginColumnConstraint::diagnostic(150.0, 0.3),
            PluginColumnConstraint::output(120.0, 0.6),
        ];
        let tree = PluginLayoutTree::from_constraints(&constraints);
        let root = tree.as_layout_node();
        let solved = solve(&root, 1200.0, 400.0, &LayoutPreferences::default());

        // Order should be: Config, Main, Diagnostic, Output
        let ids: Vec<&str> = solved
            .children
            .iter()
            .filter(|c| c.visible)
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, vec!["config", "main", "diagnostic", "output"]);
    }

    #[test]
    fn total_width_never_exceeds_available() {
        let constraints = compressor_constraints();
        let tree = PluginLayoutTree::from_constraints(&constraints);
        let root = tree.as_layout_node();

        for width in [200.0, 450.0, 500.0, 600.0, 800.0, 1200.0] {
            let solved = solve(&root, width, 400.0, &LayoutPreferences::default());
            let total: f32 = solved.children.iter().map(|c| c.width).sum();
            assert!(total <= width + 0.01, "width={width}: total={total}");
        }
    }
}
