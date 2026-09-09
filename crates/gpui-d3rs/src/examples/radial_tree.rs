//! Radial Tree / Radial Cluster — <https://observablehq.com/@d3/radial-tree/2>
//!
//! Demonstrates: [`TreeLayout`] and [`ClusterLayout`] with radial projection.
//!
//! The pipeline mirrors the official Observable examples
//! (`@d3/radial-tree/2` and `@d3/radial-cluster/2`):
//! sort ascending by name, lay out with `size([2π, radius])` and the radial
//! separation `(same parent ? 1 : 2) / depth`, project with the `-π/2`
//! rotation from d3-shape's `pointRadial`, and connect nodes with the exact
//! `linkRadial` (`bumpRadial`) cubic.
//!
//! Official dimensions: tree is 928² with center `(464, 0.59 * 928)` and
//! radius `464 - 30`; cluster is 1152² with center `(576, 0.54 * 1152)` and
//! radius `576 - 80`.

use crate::hierarchy::{ClusterLayout, HierarchyNode, TreeLayout};
use crate::shape::path::{Path, PathBuilder};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct RadialNodeResult {
    pub name: String,
    pub x: f64,      // projected x (cartesian, canvas space)
    pub y: f64,      // projected y (cartesian, canvas space)
    pub angle: f64,  // raw layout angle (0..2π, 0 at top after projection)
    pub radius: f64, // raw layout radius (0..layout radius)
    pub depth: usize,
    pub is_leaf: bool,
}

#[derive(Debug)]
pub struct RadialTreeResult {
    pub width: f64,
    pub height: f64,
    pub center_x: f64,
    pub center_y: f64,
    pub radius: f64,
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

/// Official radial separation from `@d3/radial-tree/2` and
/// `@d3/radial-cluster/2`:
/// `(a.parent == b.parent ? 1 : 2) / a.depth`.
///
/// The layouts only query this for nodes at depth >= 1 (leaf pairs in the
/// cluster first walk / extent padding, contour pairs in the tidy pass), so
/// the division is safe for any non-degenerate hierarchy.
fn radial_separation(a: &HierarchyNode<FlareNode>, b: &HierarchyNode<FlareNode>) -> f64 {
    let same_parent = a
        .parent
        .as_ref()
        .and_then(|p| p.upgrade())
        .map(|p| p.as_ptr())
        == b.parent
            .as_ref()
            .and_then(|p| p.upgrade())
            .map(|p| p.as_ptr());
    (if same_parent { 1.0 } else { 2.0 }) / a.depth as f64
}

/// Project from layout coordinates (angle, radius) to cartesian, matching
/// d3-shape's `pointRadial` (angle rotated by -π/2 so angle 0 is at top).
fn radial_project(angle: f64, radius: f64) -> (f64, f64) {
    let rotated = angle - std::f64::consts::FRAC_PI_2;
    (radius * rotated.cos(), radius * rotated.sin())
}

/// Build the exact d3 `linkRadial` (`bumpRadial`) cubic between a parent
/// `(source_angle, source_radius)` and a child `(target_angle, target_radius)`
/// around `(cx, cy)`:
///
/// ```text
/// M pointRadial(a0, r0)
/// C pointRadial(a0, rm) pointRadial(a1, rm) pointRadial(a1, r1)
/// ```
/// where `rm = (r0 + r1) / 2`.
fn radial_link_path(
    cx: f64,
    cy: f64,
    source_angle: f64,
    source_radius: f64,
    target_angle: f64,
    target_radius: f64,
) -> Path {
    let mid_radius = (source_radius + target_radius) / 2.0;
    let (sx, sy) = radial_project(source_angle, source_radius);
    let (c1x, c1y) = radial_project(source_angle, mid_radius);
    let (c2x, c2y) = radial_project(target_angle, mid_radius);
    let (tx, ty) = radial_project(target_angle, target_radius);
    PathBuilder::new()
        .move_to(cx + sx, cy + sy)
        .cubic_curve_to(cx + c1x, cy + c1y, cx + c2x, cy + c2y, cx + tx, cy + ty)
        .build()
}

/// Compute a radial tree (tidy) or radial cluster (dendrogram) layout.
///
/// If `cluster` is true, uses [`ClusterLayout`] (all leaves share the maximum
/// radius); otherwise uses [`TreeLayout`] (depth-proportional radius).
pub fn compute(cluster: bool) -> RadialTreeResult {
    compute_with_root(default_flare_hierarchy(), cluster)
}

/// Compute a radial layout for a caller-supplied hierarchy.
///
/// The hierarchy is sorted ascending by name (as in the official examples)
/// and re-rooted at depth 0 before layout.
pub fn compute_with_root(
    root: Rc<RefCell<HierarchyNode<FlareNode>>>,
    cluster: bool,
) -> RadialTreeResult {
    // Official dimensions per example.
    let (width, height, center_y_factor, margin): (f64, f64, f64, f64) = if cluster {
        (1152.0, 1152.0, 0.54, 80.0)
    } else {
        (928.0, 928.0, 0.59, 30.0)
    };
    let cx = width / 2.0;
    let cy = height * center_y_factor;
    let radius = width.min(height) / 2.0 - margin;

    HierarchyNode::sort(root.clone(), |a, b| a.data.name.cmp(&b.data.name));
    fix_depths(root.clone(), 0);

    // d3rs layouts write depth to `x` and breadth to `y`, so `size` is
    // (radius extent, angle extent): after layout, `n.x` is the radius in
    // [0, radius] and `n.y` is the angle in [0, 2π]. No remapping needed.
    if cluster {
        ClusterLayout::new()
            .size((radius, std::f64::consts::TAU))
            .separation(radial_separation)
            .layout(root.clone());
    } else {
        TreeLayout::new()
            .size((radius, std::f64::consts::TAU))
            .separation(radial_separation)
            .layout(root.clone());
    }

    // Collect nodes and project to cartesian, tracking parents by Rc identity.
    let mut nodes: Vec<RadialNodeResult> = Vec::new();
    let mut parent_map: Vec<Option<usize>> = Vec::new();
    let mut ptr_to_idx: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();

    HierarchyNode::each(root.clone(), |node_rc| {
        let n = node_rc.borrow();
        let layout_radius = n.x;
        let layout_angle = n.y;
        let (px, py) = radial_project(layout_angle, layout_radius);
        let is_leaf = n.children.is_none() || n.children.as_ref().unwrap().is_empty();
        let idx = nodes.len();

        ptr_to_idx.insert(Rc::as_ptr(&node_rc) as usize, idx);

        let parent_idx = n
            .parent
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .map(|parent_rc| Rc::as_ptr(&parent_rc) as usize)
            .and_then(|parent_ptr| ptr_to_idx.get(&parent_ptr).copied());

        nodes.push(RadialNodeResult {
            name: n.data.name.clone(),
            x: px + cx,
            y: py + cy,
            angle: layout_angle,
            radius: layout_radius,
            depth: n.depth,
            is_leaf,
        });
        parent_map.push(parent_idx);
    });

    // Links use the raw layout (angle, radius) pairs, exactly what d3's
    // linkRadial receives via its angle/radius accessors.
    let mut link_paths = Vec::new();
    for (i, parent_idx) in parent_map.iter().enumerate() {
        if let Some(pi) = parent_idx {
            link_paths.push(radial_link_path(
                cx,
                cy,
                nodes[*pi].angle,
                nodes[*pi].radius,
                nodes[i].angle,
                nodes[i].radius,
            ));
        }
    }

    RadialTreeResult {
        width,
        height,
        center_x: cx,
        center_y: cy,
        radius,
        nodes,
        link_paths,
    }
}

/// Label placement for one node, following the official examples.
///
/// The label baseline runs along the spoke: `rotation = angle − π/2`, plus π
/// on the left half (`angle ≥ π`) so text stays upright. Leaves sit outward
/// of their node, internal nodes inward (the official `x = ±6` rule); callers
/// offset the text center by half the measured width plus 6px along the
/// spoke in the indicated direction.
#[derive(Debug, Clone)]
pub struct RadialLabel {
    pub name: String,
    pub x: f64, // node position (canvas space)
    pub y: f64,
    pub angle: f64,    // raw layout angle
    pub rotation: f64, // baseline rotation, radians (y-down, clockwise)
    pub outward: bool, // leaves: true; internal nodes: false
    pub depth: usize,
    pub is_leaf: bool,
}

/// Compute label placements for every node in `result`.
pub fn labels(result: &RadialTreeResult) -> Vec<RadialLabel> {
    result
        .nodes
        .iter()
        .map(|n| RadialLabel {
            name: n.name.clone(),
            x: n.x,
            y: n.y,
            angle: n.angle,
            rotation: n.angle - std::f64::consts::FRAC_PI_2
                + if n.angle >= std::f64::consts::PI {
                    std::f64::consts::PI
                } else {
                    0.0
                },
            outward: n.is_leaf,
            depth: n.depth,
            is_leaf: n.is_leaf,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{compute, compute_with_root, default_flare_hierarchy};

    #[test]
    fn cluster_places_all_leaves_at_max_radius() {
        let result = compute(true);
        assert_eq!(result.nodes.len(), 41);
        assert_eq!(result.link_paths.len(), 40);

        for node in &result.nodes {
            assert!(
                (0.0..=std::f64::consts::TAU).contains(&node.angle),
                "angle out of range: {}",
                node.angle
            );
            if node.is_leaf {
                assert_eq!(node.radius, result.radius, "leaf {}", node.name);
            }
        }
        let root = result.nodes.iter().find(|n| n.depth == 0).unwrap();
        assert_eq!(root.radius, 0.0);

        // Half-separation padding: extreme leaves stay strictly inside [0, 2π].
        let mut leaf_angles: Vec<f64> = result
            .nodes
            .iter()
            .filter(|n| n.is_leaf)
            .map(|n| n.angle)
            .collect();
        leaf_angles.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(leaf_angles.first().unwrap() > &0.0);
        assert!(leaf_angles.last().unwrap() < &std::f64::consts::TAU);
    }

    #[test]
    fn tree_uses_depth_proportional_radius() {
        let result = compute(false);

        for node in &result.nodes {
            let expected = node.depth as f64 / 2.0 * result.radius;
            assert!(
                (node.radius - expected).abs() < 1e-9,
                "{}: {} != {}",
                node.name,
                node.radius,
                expected
            );
        }
    }

    #[test]
    fn custom_hierarchy_is_sorted_and_laid_out() {
        use crate::hierarchy::HierarchyNode;

        let root = HierarchyNode::new(super::FlareNode {
            name: "root".to_string(),
            value: 0.0,
        });
        // Inserted out of order; the official pipeline sorts ascending.
        let mut kids = Vec::new();
        for name in ["b", "a"] {
            let leaf = HierarchyNode::new(super::FlareNode {
                name: name.to_string(),
                value: 1.0,
            });
            kids.push(leaf);
        }
        root.borrow_mut().set_children(&root, kids);

        let result = compute_with_root(root, true);
        let leaves: Vec<&str> = result
            .nodes
            .iter()
            .filter(|n| n.is_leaf)
            .map(|n| n.name.as_str())
            .collect();
        assert_eq!(leaves, vec!["a", "b"]);
        for leaf in result.nodes.iter().filter(|n| n.is_leaf) {
            assert_eq!(leaf.radius, result.radius);
        }
    }

    #[test]
    fn radial_link_matches_d3_bump_radial() {
        // Hand-evaluated (axis angles, no trig): source at the center with
        // angle 0, target at angle π/2 and radius 100, mid radius 50.
        let path =
            super::radial_link_path(10.0, 20.0, 0.0, 0.0, std::f64::consts::FRAC_PI_2, 100.0);
        let cmds = path.commands();
        assert_eq!(cmds.len(), 2);
        // cos(-π/2) is ~6e-17 rather than exactly 0, so compare with tolerance.
        fn close(a: (f64, f64), b: (f64, f64)) -> bool {
            (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
        }
        assert!(close(move_to_xy(&cmds[0]), (10.0, 20.0)));
        // c1 = pointRadial(0, 50), c2 = pointRadial(π/2, 50).
        let (x1, y1, x2, y2, x, y) = cubic_xy(&cmds[1]);
        assert!(close((x1, y1), (10.0, -30.0)));
        assert!(close((x2, y2), (60.0, 20.0)));
        assert!(close((x, y), (110.0, 20.0)));
    }

    fn move_to_xy(cmd: &crate::shape::path::PathCommand) -> (f64, f64) {
        match cmd {
            crate::shape::path::PathCommand::MoveTo { x, y } => (*x, *y),
            other => panic!("expected MoveTo, got {other:?}"),
        }
    }

    #[allow(clippy::type_complexity)]
    fn cubic_xy(cmd: &crate::shape::path::PathCommand) -> (f64, f64, f64, f64, f64, f64) {
        match cmd {
            crate::shape::path::PathCommand::CubicCurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => (*x1, *y1, *x2, *y2, *x, *y),
            other => panic!("expected CubicCurveTo, got {other:?}"),
        }
    }

    #[test]
    fn link_endpoints_coincide_with_nodes() {
        for cluster in [false, true] {
            let result = compute(cluster);
            let dots: Vec<(f64, f64)> = result.nodes.iter().map(|n| (n.x, n.y)).collect();
            assert_eq!(result.link_paths.len(), result.nodes.len() - 1);
            for link in &result.link_paths {
                let cmds = link.commands();
                assert_eq!(cmds.len(), 2);
                let (sx, sy) = move_to_xy(&cmds[0]);
                assert!(
                    dots.contains(&(sx, sy)),
                    "link start {sx},{sy} matches no node"
                );
                let (_, _, _, _, ex, ey) = cubic_xy(&cmds[1]);
                assert!(
                    dots.contains(&(ex, ey)),
                    "link end {ex},{ey} matches no node"
                );
            }
        }
    }

    #[test]
    fn labels_follow_the_official_spoke_rule() {
        use super::{RadialNodeResult, RadialTreeResult, labels};
        use std::f64::consts::{FRAC_PI_2, PI};

        let result = RadialTreeResult {
            width: 100.0,
            height: 100.0,
            center_x: 50.0,
            center_y: 50.0,
            radius: 40.0,
            nodes: vec![
                RadialNodeResult {
                    name: "top-leaf".to_string(),
                    x: 50.0,
                    y: 10.0,
                    angle: 0.0,
                    radius: 40.0,
                    depth: 1,
                    is_leaf: true,
                },
                RadialNodeResult {
                    name: "left-branch".to_string(),
                    x: 30.0,
                    y: 50.0,
                    angle: 3.0 * FRAC_PI_2,
                    radius: 20.0,
                    depth: 1,
                    is_leaf: false,
                },
            ],
            link_paths: Vec::new(),
        };

        let placed = labels(&result);
        assert_eq!(placed.len(), 2);
        // Top leaf: baseline points up along the spoke, label outward.
        assert!((placed[0].rotation - -FRAC_PI_2).abs() < 1e-12);
        assert!(placed[0].outward);
        // Left-half internal node: flipped upright, label inward.
        assert!((placed[1].rotation - (3.0 * FRAC_PI_2 - FRAC_PI_2 + PI)).abs() < 1e-12);
        assert!(!placed[1].outward);
    }

    #[test]
    fn toy_hierarchy_node_count() {
        // 1 root + 10 categories + 30 leaves.
        let root = default_flare_hierarchy();
        let mut count = 0;
        crate::hierarchy::HierarchyNode::each(root, |_| count += 1);
        assert_eq!(count, 41);
    }

    /// Headless mirror of the showcase radial sections: every label shapes
    /// at 10px, anchors stay finite, and the assembled scene holds exactly
    /// one text run per label. Display QA (Retina eyeball) still belongs to
    /// `cargo run -p gpui-d3rs --example d3rs-showcase`.
    #[cfg(feature = "vello")]
    #[test]
    fn showcase_label_pipeline_shapes_every_label() {
        use crate::gputext::{FAMILY_SANS, FontEngine, TextWeight};
        use crate::vello2d::kurbo::Affine;
        use crate::vello2d::peniko::{Brush, Color};
        use crate::vello2d::{ChartCmd, ChartScene};

        for cluster in [false, true] {
            let result = compute(cluster);
            let placed = super::labels(&result);
            assert_eq!(placed.len(), result.nodes.len());
            let mut engine = FontEngine::new();
            let mut scene = ChartScene::new();
            for label in &placed {
                let size = 10.0f32;
                let width = FontEngine::line_width(&engine.shape(
                    &label.name,
                    size,
                    FAMILY_SANS,
                    TextWeight::NORMAL,
                ));
                assert!(width.is_finite() && width > 0.0, "shapable: {}", label.name);
                // Same anchor math as the showcase section.
                let spoke = label.angle - std::f64::consts::FRAC_PI_2;
                let (ux, uy) = (spoke.cos(), spoke.sin());
                let side = if label.outward { 1.0 } else { -1.0 };
                let dist = width as f64 / 2.0 + 6.0;
                let anchor =
                    Affine::translate((label.x + side * ux * dist, label.y + side * uy * dist))
                        * Affine::rotate(label.rotation)
                        * Affine::translate((-width as f64 / 2.0, 0.35 * size as f64));
                assert!(anchor.as_coeffs().iter().all(|c| c.is_finite()));
                scene.fill_text(
                    &mut engine,
                    &label.name,
                    size,
                    FAMILY_SANS,
                    TextWeight::NORMAL,
                    anchor,
                    Brush::Solid(Color::WHITE),
                );
            }
            assert_eq!(scene.len(), placed.len());
            assert!(matches!(scene.commands()[0], ChartCmd::Text { .. }));
        }
    }
}
