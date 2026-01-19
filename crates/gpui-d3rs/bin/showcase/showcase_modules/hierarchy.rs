use crate::ShowcaseApp;
use d3rs::hierarchy::{HierarchyNode, TreeLayout};
use gpui::*;

pub fn render(_app: &ShowcaseApp, _cx: &mut Context<ShowcaseApp>) -> Div {
    // Recreate hierarchy for stateless render
    // In a real app, this would be computed once and stored in app state

    use std::cell::RefCell;
    use std::rc::Rc;

    // Generate a Merkle Tree-like structure (Binary Tree)
    fn generate_merkle_node(
        depth: usize,
        max_depth: usize,
        index: usize,
    ) -> Rc<RefCell<HierarchyNode<String>>> {
        let hash = format!("0x{:x}", (depth * 1000 + index)); // Fake hash
        let label = if depth == 0 { "Root".to_string() } else { hash };
        let node = HierarchyNode::new(label);

        if depth < max_depth {
            let left = generate_merkle_node(depth + 1, max_depth, index * 2);
            let right = generate_merkle_node(depth + 1, max_depth, index * 2 + 1);

            let mut n = node.borrow_mut();
            n.set_children(&node, vec![left, right]);
        }

        node
    }

    fn assign_depths(node: Rc<RefCell<HierarchyNode<String>>>, depth: usize) {
        node.borrow_mut().depth = depth;
        if let Some(children) = &node.borrow().children {
            for child in children {
                assign_depths(child.clone(), depth + 1);
            }
        }
    }

    // Depth 4 gives 31 nodes (1+2+4+8+16)
    let root = generate_merkle_node(0, 4, 0);
    assign_depths(root.clone(), 0);

    HierarchyNode::count(root.clone());

    let width = 800.0;
    let height = 600.0;

    // Layout
    TreeLayout::new()
        .size((width - 100.0, height - 100.0)) // Reduce size slightly to ensure fit
        .layout(root.clone());

    // Compute Bounding Box
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    HierarchyNode::each(root.clone(), |node| {
        let n = node.borrow();
        if n.x < min_x {
            min_x = n.x;
        }
        if n.x > max_x {
            max_x = n.x;
        }
        if n.y < min_y {
            min_y = n.y;
        }
        if n.y > max_y {
            max_y = n.y;
        }
    });

    // Center the layout
    let layout_center_x = (min_x + max_x) / 2.0;
    let layout_center_y = (min_y + max_y) / 2.0;

    let container_center_x = width / 2.0;
    let container_center_y = height / 2.0;

    let offset_x = (container_center_x - layout_center_x) as f32;
    let offset_y = (container_center_y - layout_center_y) as f32;

    let mut nodes = Vec::new();
    let mut links = Vec::new();

    HierarchyNode::each(root.clone(), |node| {
        let n = node.borrow();

        if let Some(parent_weak) = &n.parent
            && let Some(parent_rc) = parent_weak.upgrade()
        {
            let p = parent_rc.borrow();

            let x1 = n.x as f32 + offset_x;
            let y1 = n.y as f32 + offset_y;
            let x2 = p.x as f32 + offset_x;
            let y2 = p.y as f32 + offset_y;

            let stroke_color = rgb(0x666666);
            let thickness = px(2.0);

            // Manhattan connector: Child (x1, y1) -> intermediate (x1, y2) -> Parent (x2, y2)
            // Or standard tree style: Child (x1, y1) -> (x1, mid_y) -> (x2, mid_y) -> (x2, y2)?
            // Let's do direct L-shape for simplicity:
            // From Child (x1, y1) go up to Parent Y (x1, y2), then across to Parent X (x2, y2).
            // Wait, hierarchy usually has parents above.
            // Child at y1, Parent at y2. y1 > y2 usually.

            // Segments:
            // 1. Vertical from Child (x1, y1) to (x1, y2)
            // 2. Horizontal from (x1, y2) to (x2, y2)
            // This implies a right angle at (x1, y2).

            // Segment 1 (Vertical)
            let v_top = y1.min(y2);
            let v_height = (y1 - y2).abs();

            links.push(
                div()
                    .absolute()
                    .left(px(x1))
                    .top(px(v_top))
                    .w(thickness)
                    .h(px(v_height))
                    .bg(stroke_color),
            );

            // Segment 2 (Horizontal)
            let h_left = x1.min(x2);
            let h_width = (x1 - x2).abs();

            links.push(
                div()
                    .absolute()
                    .left(px(h_left))
                    .top(px(y2))
                    .w(px(h_width))
                    .h(thickness)
                    .bg(stroke_color),
            );
        }

        nodes.push(
            div()
                .absolute()
                .left(px(n.x as f32 + offset_x - 20.0))
                .top(px(n.y as f32 + offset_y - 20.0))
                .size(px(40.0))
                .bg(rgb(0x4a90e2))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .text_xs()
                .text_color(rgb(0xffffff))
                .child(n.data.clone()),
        );
    });

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .text_xl()
                .font_weight(FontWeight::BOLD)
                .child("Hierarchy Layout (Tree)"),
        )
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(rgb(0xf0f0f0))
                .rounded_lg()
                .border_1()
                .border_color(rgb(0xcccccc))
                .relative()
                .children(links)
                .children(nodes),
        )
}
