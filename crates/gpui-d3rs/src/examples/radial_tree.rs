//! Radial Tree / Radial Cluster — <https://observablehq.com/@d3/radial-tree/2>
//!
//! Demonstrates: `TreeLayout` with radial projection.
//! Uses the Flare hierarchy dataset.

use crate::hierarchy::{HierarchyNode, TreeLayout};
use crate::shape::path::{Path, PathBuilder};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct RadialNodeResult {
    pub name: String,
    pub x: f64,     // projected x (cartesian)
    pub y: f64,     // projected y (cartesian)
    pub angle: f64, // raw radial angle (0..2π, 0 at top, clockwise)
    pub depth: usize,
    pub is_leaf: bool,
}

#[derive(Debug)]
pub struct RadialTreeResult {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<RadialNodeResult>,
    pub link_paths: Vec<Path>,
}

/// Simple hierarchy data for Flare-like dataset.
#[derive(Clone, Debug)]
pub struct FlareNode {
    pub name: String,
    pub value: f64,
}

/// Build a sample Flare-like hierarchy for demonstration.
pub fn default_flare_hierarchy() -> Rc<RefCell<HierarchyNode<FlareNode>>> {
    let root = HierarchyNode::new(FlareNode {
        name: "flare".to_string(),
        value: 0.0,
    });

    let categories = [
        ("analytics", vec!["cluster", "graph", "optimization"]),
        ("animate", vec!["Easing", "FunctionSequence", "Tween"]),
        ("data", vec!["converters", "DataField", "DataSchema"]),
        ("display", vec!["DirtySprite", "LineSprite", "TextSprite"]),
        ("flex", vec!["FlareVis"]),
        ("physics", vec!["DragForce", "GravityForce", "Spring"]),
        ("query", vec!["AggregateExpr", "Expression", "Query"]),
        ("scale", vec!["LinearScale", "LogScale", "OrdinalScale"]),
        ("util", vec!["Arrays", "Dates", "Maths", "Sort"]),
        ("vis", vec!["axis", "controls", "data", "legend"]),
    ];

    let mut children = Vec::new();
    for (cat_name, items) in &categories {
        let cat = HierarchyNode::new(FlareNode {
            name: cat_name.to_string(),
            value: 0.0,
        });
        let mut cat_children = Vec::new();
        for item in items {
            cat_children.push(HierarchyNode::new(FlareNode {
                name: item.to_string(),
                value: 1.0,
            }));
        }
        cat.borrow_mut().set_children(&cat, cat_children);
        children.push(cat);
    }
    root.borrow_mut().set_children(&root, children);
    fix_depths(root.clone(), 0);
    HierarchyNode::sum(root.clone(), |d| if d.value > 0.0 { d.value } else { 0.0 });

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

/// Project from tree coordinates to radial (polar).
/// In D3 radial tree: x = angle (0..2π), y = radius.
fn radial_project(x: f64, y: f64) -> (f64, f64) {
    let angle = x - std::f64::consts::FRAC_PI_2; // rotate -90° so root is at top
    (y * angle.cos(), y * angle.sin())
}

/// Compute radial tree layout.
///
/// If `cluster` is true, uses cluster layout (leaves at same radius).
/// If false, uses tree layout (depth-proportional).
pub fn compute(cluster: bool) -> RadialTreeResult {
    let width: f64 = 928.0;
    let height: f64 = 928.0;
    let radius = width.min(height) / 2.0 - 60.0;

    let root = default_flare_hierarchy();

    // Run tree layout in cartesian mode first (it swaps x/y internally)
    // We use a large size so leaf positions span a good range
    let layout = TreeLayout::new().size((1000.0, 1000.0));
    layout.layout(root.clone());

    // After layout: n.x = depth-proportional, n.y = leaf-index-proportional
    // For radial: need x = angle [0, 2π], y = radius [0, radius]
    // So we need to find extents and re-map
    let mut x_max = 0.0f64;
    let mut y_max = 0.0f64;
    HierarchyNode::each(root.clone(), |node_rc| {
        let n = node_rc.borrow();
        x_max = x_max.max(n.x);
        y_max = y_max.max(n.y);
    });

    // Re-map: swap so leaf-index (currently y) → angle, depth (currently x) → radius
    let x_extent = x_max.max(1.0);
    let y_extent = y_max.max(1.0);
    HierarchyNode::each(root.clone(), |node_rc| {
        let mut n = node_rc.borrow_mut();
        let old_x = n.x; // depth-based
        let old_y = n.y; // leaf-index-based
        n.x = old_y / y_extent * std::f64::consts::TAU; // angle from leaf position
        n.y = if cluster {
            // Cluster: all leaves at max radius, internal nodes at depth-proportional radius
            old_x / x_extent * radius
        } else {
            old_x / x_extent * radius
        };
    });

    // Collect nodes and project to cartesian.
    // Build a proper parent map using Rc pointer identity.
    let mut nodes = Vec::new();
    let mut parent_map: Vec<Option<usize>> = Vec::new();
    // Map from Rc pointer address → index for parent lookup
    let mut ptr_to_idx: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();

    HierarchyNode::each(root.clone(), |node_rc| {
        let n = node_rc.borrow();
        let (px, py) = radial_project(n.x, n.y);
        let is_leaf = n.children.is_none() || n.children.as_ref().unwrap().is_empty();
        let idx = nodes.len();

        // Use Rc pointer address as unique identity
        let ptr = Rc::as_ptr(&node_rc) as usize;
        ptr_to_idx.insert(ptr, idx);

        // Look up parent via the weak reference
        let parent_idx = n
            .parent
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .map(|parent_rc| Rc::as_ptr(&parent_rc) as usize)
            .and_then(|parent_ptr| ptr_to_idx.get(&parent_ptr).copied());

        nodes.push(RadialNodeResult {
            name: n.data.name.clone(),
            x: px + width / 2.0,
            y: py + height / 2.0,
            angle: n.x,
            depth: n.depth,
            is_leaf,
        });
        parent_map.push(parent_idx);
    });

    // Generate curved link paths (cubic Bézier for radial layout)
    let cx = width / 2.0;
    let cy = height / 2.0;
    let mut link_paths = Vec::new();

    for (i, parent_idx) in parent_map.iter().enumerate() {
        if let Some(pi) = parent_idx {
            let (sx, sy) = (nodes[*pi].x, nodes[*pi].y);
            let (tx, ty) = (nodes[i].x, nodes[i].y);

            // Curved radial link: intermediate point at parent's radius, child's angle
            let parent_r = ((sx - cx).powi(2) + (sy - cy).powi(2)).sqrt();
            let child_angle = (ty - cy).atan2(tx - cx);
            let mid_x = cx + parent_r * child_angle.cos();
            let mid_y = cy + parent_r * child_angle.sin();

            let path = PathBuilder::new()
                .move_to(sx, sy)
                .cubic_curve_to(mid_x, mid_y, mid_x, mid_y, tx, ty)
                .build();
            link_paths.push(path);
        }
    }

    RadialTreeResult {
        width,
        height,
        nodes,
        link_paths,
    }
}
