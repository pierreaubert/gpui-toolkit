//! Sunburst — <https://observablehq.com/@d3/sunburst/2>
//!
//! Demonstrates: Partition layout with Arc rendering for hierarchical data.
//! Uses `d3.partition().size([2π, radius])` to assign angular and radial extents,
//! then renders each node as an arc slice.

use crate::hierarchy::HierarchyNode;
use crate::shape::arc::{Arc, ArcDatum};
use crate::shape::path::Path;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct SunburstSlice {
    pub name: String,
    pub depth: usize,
    /// Top-level (depth 1) ancestor name, for official rainbow coloring.
    pub group: String,
    pub x0: f64, // start angle
    pub x1: f64, // end angle
    pub y0: f64, // inner radius
    pub y1: f64, // outer radius
    pub value: f64,
    pub arc_path: Path,
}

#[derive(Debug)]
pub struct SunburstResult {
    pub width: f64,
    pub height: f64,
    pub slices: Vec<SunburstSlice>,
}

#[derive(Clone, Debug)]
pub struct SunNode {
    pub name: String,
    pub value: f64,
}

/// Build the same hierarchy used by the golden generator.
/// 3-level deep: root → categories → items (some categories have sub-categories).
pub fn default_hierarchy() -> Rc<RefCell<HierarchyNode<SunNode>>> {
    let root = HierarchyNode::new(SunNode {
        name: "root".to_string(),
        value: 0.0,
    });

    // Exact data from golden/generate_observable_examples.js sunburst generator
    type SubItem<'a> = (&'a str, Vec<(&'a str, f64)>);
    type Category<'a> = (&'a str, Vec<SubItem<'a>>);
    let categories: Vec<Category<'_>> = vec![
        (
            "analytics",
            vec![
                ("cluster", vec![]),
                ("graph", vec![]),
                ("optimization", vec![]),
            ],
        ),
        (
            "animate",
            vec![
                ("Easing", vec![]),
                ("Parallel", vec![]),
                (
                    "interpolate",
                    vec![
                        ("ArrayInterp", 2000.0),
                        ("ColorInterp", 3000.0),
                        ("NumberInterp", 1800.0),
                    ],
                ),
            ],
        ),
        (
            "data",
            vec![
                ("DataField", vec![]),
                ("DataSchema", vec![]),
                ("DataUtil", vec![]),
            ],
        ),
        (
            "display",
            vec![
                ("DirtySprite", vec![]),
                ("LineSprite", vec![]),
                ("TextSprite", vec![]),
            ],
        ),
    ];

    // Leaf values matching golden JS exactly
    let leaf_values: std::collections::HashMap<&str, f64> = [
        ("cluster", 10000.0),
        ("graph", 8000.0),
        ("optimization", 5000.0),
        ("Easing", 17000.0),
        ("Parallel", 5000.0),
        ("DirtySprite", 8800.0),
        ("LineSprite", 1700.0),
        ("TextSprite", 10000.0),
        ("DataField", 1800.0),
        ("DataSchema", 2200.0),
        ("DataUtil", 3300.0),
    ]
    .into_iter()
    .collect();

    let mut top_children = Vec::new();
    for (cat_name, items) in &categories {
        let cat = HierarchyNode::new(SunNode {
            name: cat_name.to_string(),
            value: 0.0,
        });
        let mut cat_children = Vec::new();

        for (item_name, sub_items) in items {
            if sub_items.is_empty() {
                let val = leaf_values.get(item_name).copied().unwrap_or(1000.0);
                cat_children.push(HierarchyNode::new(SunNode {
                    name: item_name.to_string(),
                    value: val,
                }));
            } else {
                // Sub-category with children
                let sub = HierarchyNode::new(SunNode {
                    name: item_name.to_string(),
                    value: 0.0,
                });
                let mut sub_children = Vec::new();
                for (sub_name, sub_val) in sub_items {
                    sub_children.push(HierarchyNode::new(SunNode {
                        name: sub_name.to_string(),
                        value: *sub_val,
                    }));
                }
                sub.borrow_mut().set_children(&sub, sub_children);
                cat_children.push(sub);
            }
        }
        cat.borrow_mut().set_children(&cat, cat_children);
        top_children.push(cat);
    }
    root.borrow_mut().set_children(&root, top_children);
    fix_depths(root.clone(), 0);
    HierarchyNode::sum(root.clone(), |d| if d.value > 0.0 { d.value } else { 0.0 });
    // Sort descending by value (matching D3's .sort((a, b) => b.value - a.value))
    HierarchyNode::sort(root.clone(), |a, b| {
        let av = b.value.unwrap_or(0.0);
        let bv = a.value.unwrap_or(0.0);
        av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
    });

    root
}

fn fix_depths<T>(node: Rc<RefCell<HierarchyNode<T>>>, depth: usize) {
    node.borrow_mut().depth = depth;
    let children = node.borrow().children.clone();
    if let Some(children) = children {
        for child in children {
            fix_depths(child, depth + 1);
        }
    }
}

/// Compute sunburst layout using partition (angular x, radial y).
/// Matches D3's `d3.partition().size([2π, radius])`.
pub fn compute() -> SunburstResult {
    let width: f64 = 928.0;
    let height: f64 = 928.0;
    let radius = width.min(height) / 2.0;
    let padding = 1.0;

    let root = default_hierarchy();
    let root_value = root.borrow().value.unwrap_or(1.0);

    // Compute max depth
    let mut max_depth = 0usize;
    HierarchyNode::each(root.clone(), |n| {
        let d = n.borrow().depth;
        if d > max_depth {
            max_depth = d;
        }
    });

    // Partition layout: evenly divide radius by depth, angles by value proportion.
    // This matches d3.partition().size([2π, radius]).
    let mut slices = Vec::new();
    let arc_gen = Arc::default();
    partition_node(
        &root,
        0.0,
        std::f64::consts::TAU,
        root_value,
        radius,
        max_depth,
        padding,
        &arc_gen,
        String::new(),
        &mut slices,
    );

    SunburstResult {
        width,
        height,
        slices,
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "recursive partition helper carries angular, radial, and output state explicitly"
)]
fn partition_node(
    node: &Rc<RefCell<HierarchyNode<SunNode>>>,
    start_angle: f64,
    end_angle: f64,
    parent_value: f64,
    radius: f64,
    max_depth: usize,
    padding: f64,
    arc_gen: &Arc,
    ancestor: String,
    result: &mut Vec<SunburstSlice>,
) {
    let (depth, value, name, children_data) = {
        let n = node.borrow();
        let children_data: Vec<(Rc<RefCell<HierarchyNode<SunNode>>>, f64)> = n
            .children
            .as_ref()
            .map(|cs| {
                cs.iter()
                    .map(|c| (c.clone(), c.borrow().value.unwrap_or(0.0)))
                    .collect()
            })
            .unwrap_or_default();
        (
            n.depth,
            n.value.unwrap_or(0.0),
            n.data.name.clone(),
            children_data,
        )
    };

    // d3.partition: y0 = depth * (radius / (max_depth + 1)), y1 = (depth + 1) * ...
    let ring = radius / (max_depth as f64 + 1.0);
    let y0 = depth as f64 * ring;
    let y1 = (depth as f64 + 1.0) * ring;

    if depth > 0 && end_angle - start_angle > 0.001 {
        // Arc with padAngle matching Observable: min((x1-x0)/2, 2*padding/radius)
        let angular_width = end_angle - start_angle;
        let pad = (angular_width / 2.0).min(2.0 * padding / radius);

        let datum = ArcDatum {
            inner_radius: y0,
            outer_radius: y1 - padding,
            start_angle: start_angle + pad / 2.0,
            end_angle: end_angle - pad / 2.0,
            corner_radius: 0.0,
            pad_angle: 0.0,
        };
        let arc_path = arc_gen.generate(&datum);

        // Depth-1 nodes found their group; deeper nodes inherit it.
        let group = if depth == 1 { name.clone() } else { ancestor.clone() };
        result.push(SunburstSlice {
            name,
            depth,
            group,
            x0: start_angle,
            x1: end_angle,
            y0,
            y1,
            value,
            arc_path,
        });
    }

    // Recurse into children. The root itself is not a group; each depth-1
    // child seeds the group with its own name, deeper nodes inherit it.
    let mut angle = start_angle;
    for (child, child_val) in &children_data {
        let child_extent = if parent_value > 0.0 {
            (end_angle - start_angle) * (child_val / parent_value)
        } else {
            0.0
        };
        let next_ancestor = if depth == 0 {
            child.borrow().data.name.clone()
        } else {
            ancestor.clone()
        };
        partition_node(
            child,
            angle,
            angle + child_extent,
            *child_val,
            radius,
            max_depth,
            padding,
            arc_gen,
            next_ancestor,
            result,
        );
        angle += child_extent;
    }
}

#[cfg(test)]
mod tests {
    use super::compute;
    use std::collections::BTreeSet;

    #[test]
    fn slices_carry_top_level_group() {
        let result = compute();
        assert!(!result.slices.is_empty());
        // Every slice names a depth-1 ancestor; depth-1 slices name themselves.
        let groups: BTreeSet<&str> = result.slices.iter().map(|s| s.group.as_str()).collect();
        assert!(!groups.contains(""));
        assert!(!groups.contains("root"));
        for s in &result.slices {
            assert!(groups.contains(s.group.as_str()));
            if s.depth == 1 {
                assert_eq!(s.group, s.name);
            }
        }
    }
}
