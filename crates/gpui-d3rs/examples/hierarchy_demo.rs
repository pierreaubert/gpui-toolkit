//! Hierarchy Demo
//!
//! Visualizes a simple hierarchy using Tree and Cluster layouts.

use d3rs::hierarchy::{HierarchyNode, TreeLayout};
use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::rc::Rc;

struct HierarchyDemo {
    root: Rc<RefCell<HierarchyNode<String>>>,
    layout: TreeLayout,
    width: f64,
    height: f64,
}

impl HierarchyDemo {
    fn new(_cx: &mut Context<Self>) -> Self {
        // Create a simple hierarchy:
        // Root
        // ├── Child A
        // │   ├── Grandchild A1
        // │   └── Grandchild A2
        // └── Child B
        //     ├── Grandchild B1
        //     └── Grandchild B2

        let root = HierarchyNode::new("Root".to_string());

        let child_a = HierarchyNode::new("Child A".to_string());
        let child_b = HierarchyNode::new("Child B".to_string());

        let grandchild_a1 = HierarchyNode::new("GC A1".to_string());
        let grandchild_a2 = HierarchyNode::new("GC A2".to_string());

        let grandchild_b1 = HierarchyNode::new("GC B1".to_string());
        let grandchild_b2 = HierarchyNode::new("GC B2".to_string());

        {
            let mut a = child_a.borrow_mut();
            a.set_children(&child_a, vec![grandchild_a1, grandchild_a2]);
        }

        {
            let mut b = child_b.borrow_mut();
            b.set_children(&child_b, vec![grandchild_b1, grandchild_b2]);
        }

        {
            let mut r = root.borrow_mut();
            r.set_children(&root, vec![child_a, child_b]);
        }

        // Count leaves to setup layout
        HierarchyNode::count(root.clone());

        Self {
            root,
            layout: TreeLayout::new(),
            width: 800.0,
            height: 600.0,
        }
    }
}

impl Render for HierarchyDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Update layout dimensions
        // Note: TreeLayout in our impl is robust to repeated calls (it just updates coordinates)
        self.layout
            .clone()
            .size((self.width - 200.0, self.height - 200.0))
            .layout(self.root.clone());

        let mut nodes = Vec::new();
        let mut links = Vec::new();

        // Extract nodes and links
        HierarchyNode::each(self.root.clone(), |node| {
            let n = node.borrow();

            // Link to parent
            if let Some(parent_weak) = &n.parent {
                if let Some(parent_rc) = parent_weak.upgrade() {
                    let p = parent_rc.borrow();

                    // Draw link (simple SVG line)
                    let x1 = n.x as f32 + 100.0;
                    let y1 = n.y as f32 + 100.0;
                    let x2 = p.x as f32 + 100.0;
                    let y2 = p.y as f32 + 100.0;

                    links.push(
                        div().absolute().size_full().child(
                            svg()
                                .size_full()
                                .path(format!("M {},{} L {},{}", x2, y2, x1, y1))
                                .text_color(rgb(0x666666)),
                        ),
                    );
                }
            }

            // Node
            nodes.push(
                div()
                    .absolute()
                    .left(px(n.x as f32 + 100.0 - 20.0)) // Center node
                    .top(px(n.y as f32 + 100.0 - 20.0))
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
            .size_full()
            .bg(rgb(0x1e1e1e))
            .child(div().relative().size_full().children(links).children(nodes))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| HierarchyDemo::new(cx)),
        )
        .unwrap();

        cx.activate(true);
    });
}
