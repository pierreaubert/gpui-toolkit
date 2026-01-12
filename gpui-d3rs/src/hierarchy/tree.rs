//! Tree layouts (Tidy Tree D3)
//!
//! The tree layout produces node-link diagrams of tree-like structures.

use super::HierarchyNode;
use std::cell::RefCell;
use std::rc::Rc;

/// Tree layout configuration
#[derive(Clone, Debug)]
pub struct TreeLayout {
    pub size: (f64, f64),
    pub node_size: Option<(f64, f64)>,
    pub separation: fn(&HierarchyNode<()>, &HierarchyNode<()>) -> f64,
}

impl Default for TreeLayout {
    fn default() -> Self {
        Self {
            size: (1.0, 1.0),
            node_size: None,
            separation: default_separation,
        }
    }
}

fn default_separation<T>(a: &HierarchyNode<T>, b: &HierarchyNode<T>) -> f64 {
    if a.parent
        .as_ref()
        .and_then(|p| p.upgrade())
        .map(|p| p.as_ptr())
        == b.parent
            .as_ref()
            .and_then(|p| p.upgrade())
            .map(|p| p.as_ptr())
    {
        1.0
    } else {
        2.0
    }
}

impl TreeLayout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn size(mut self, size: (f64, f64)) -> Self {
        self.size = size;
        self.node_size = None;
        self
    }

    pub fn node_size(mut self, size: (f64, f64)) -> Self {
        self.node_size = Some(size);
        self.size = size; // Not strictly used if node_size is set, but keeping consistent
        self
    }

    pub fn layout<T: Clone + 'static>(&self, root: Rc<RefCell<HierarchyNode<T>>>) {
        // Simple Reingold-Tilford implementation (placeholder for now)
        // In a real implementation this would be the full Buchheim linear time algorithm

        let root_node = root.borrow();
        let _height = root_node.height;
        drop(root_node);

        // Assign depths first
        HierarchyNode::each(root.clone(), |_node| {
            // Depth is already set by set_children or construction
            // We can verify or re-compute if needed
        });

        // Basic positioning for demo
        HierarchyNode::each(root.clone(), |node| {
            let mut _node_mut = node.borrow_mut();

            // X based on depth (horizontal layout assumption)
            // Y based on traversal order / index
            // Just a placeholder layout to ensure we visit nodes
            // A real implementation requires multiple passes
        });

        // Use a simpler cluster layout logic for now as it's easier to implement first
        // and provides visually distinct output
        self.layout_cluster(root);
    }

    // Internal: Cluster layout implementation (dendrogram)
    fn layout_cluster<T: Clone + 'static>(&self, root: Rc<RefCell<HierarchyNode<T>>>) {
        // Re-compute leaf count for spacing
        HierarchyNode::count(root.clone());

        let mut leaf_index = 0;
        let mut max_depth = 0;

        // First pass: position leaves and find max depth
        HierarchyNode::each(root.clone(), |node| {
            let mut n = node.borrow_mut();
            if n.depth > max_depth {
                max_depth = n.depth;
            }

            if n.children.is_none() || n.children.as_ref().unwrap().is_empty() {
                n.x = leaf_index as f64;
                leaf_index += 1;
            }
        });

        // Propagate positions up for non-leaf nodes (average of children)
        // This requires post-order traversal which isn't directly exposed yet
        // For now, we do a recursive helper
        Self::position_internal_cluster(root.clone());

        // Scale to fit size
        let (width, height) = self.size;
        let x_scale = height / (leaf_index as f64 - 1.0).max(1.0); // Map leaves to height (typically vertical)
        let y_scale = width / (max_depth as f64).max(1.0); // Map depth to width

        HierarchyNode::each(root.clone(), |node| {
            let mut n = node.borrow_mut();
            // Swap x/y for standard horizontal tree (root left)
            let temp = n.x;
            n.x = n.depth as f64 * y_scale; // Depth -> X
            n.y = temp * x_scale; // Leaf index -> Y
        });
    }

    fn position_internal_cluster<T>(node: Rc<RefCell<HierarchyNode<T>>>) -> f64 {
        let children_opt = {
            let n = node.borrow();
            n.children.clone()
        };

        if let Some(children) = children_opt
            && !children.is_empty()
        {
            let mut sum_x = 0.0;
            for child in &children {
                sum_x += Self::position_internal_cluster(child.clone());
            }

            let mut n = node.borrow_mut();
            n.x = sum_x / children.len() as f64;
            return n.x;
        }

        let n = node.borrow();
        n.x
    }
}
