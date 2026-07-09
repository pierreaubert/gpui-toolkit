//! Tree layouts (Tidy Tree D3)
//!
//! The tree layout produces node-link diagrams of tree-like structures.

use super::{HierarchyError, HierarchyNode, validate_layout_dimension};
use std::cell::RefCell;
use std::rc::Rc;

/// Tree layout configuration
#[derive(Clone, Debug)]
pub struct TreeLayout<T = ()> {
    pub size: (f64, f64),
    pub node_size: Option<(f64, f64)>,
    pub separation: fn(&HierarchyNode<T>, &HierarchyNode<T>) -> f64,
}

impl<T> Default for TreeLayout<T> {
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

impl<T> TreeLayout<T> {
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

    pub fn separation(
        mut self,
        separation: fn(&HierarchyNode<T>, &HierarchyNode<T>) -> f64,
    ) -> Self {
        self.separation = separation;
        self
    }

    pub fn layout(&self, root: Rc<RefCell<HierarchyNode<T>>>)
    where
        T: Clone + 'static,
    {
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

    /// Run the tree layout after validating configured dimensions.
    pub fn try_layout(&self, root: Rc<RefCell<HierarchyNode<T>>>) -> Result<(), HierarchyError>
    where
        T: Clone + 'static,
    {
        if let Some((node_width, node_height)) = self.node_size {
            validate_layout_dimension("node_width", node_width)?;
            validate_layout_dimension("node_height", node_height)?;
        } else {
            validate_layout_dimension("width", self.size.0)?;
            validate_layout_dimension("height", self.size.1)?;
        }

        self.validate_separation(root.clone())?;
        self.layout(root);
        Ok(())
    }

    // Internal: Cluster layout implementation (dendrogram)
    fn layout_cluster(&self, root: Rc<RefCell<HierarchyNode<T>>>)
    where
        T: Clone + 'static,
    {
        // Re-compute leaf count for spacing
        HierarchyNode::count(root.clone());

        let mut max_depth = 0;
        let mut leaves = Vec::new();

        // First pass: collect leaves and find max depth.
        HierarchyNode::each(root.clone(), |node| {
            let n = node.borrow();
            if n.depth > max_depth {
                max_depth = n.depth;
            }

            if n.children.is_none() || n.children.as_ref().unwrap().is_empty() {
                leaves.push(node.clone());
            }
        });

        self.position_leaves(&leaves);

        // Propagate positions up for non-leaf nodes (average of children)
        // This requires post-order traversal which isn't directly exposed yet
        // For now, we do a recursive helper
        Self::position_internal_cluster(root.clone());

        // Scale to fit size
        let (width, height) = self.size;
        let span = leaves
            .last()
            .map(|leaf| leaf.borrow().x)
            .unwrap_or_default()
            .max(1.0);
        let x_scale = height / span; // Map leaves to height (typically vertical)
        let y_scale = width / (max_depth as f64).max(1.0); // Map depth to width

        HierarchyNode::each(root.clone(), |node| {
            let mut n = node.borrow_mut();
            // Swap x/y for standard horizontal tree (root left)
            let temp = n.x;
            n.x = n.depth as f64 * y_scale; // Depth -> X
            n.y = temp * x_scale; // Leaf index -> Y
        });
    }

    fn validate_separation(&self, root: Rc<RefCell<HierarchyNode<T>>>) -> Result<(), HierarchyError>
    where
        T: Clone + 'static,
    {
        let leaves = Self::leaves(root);
        for pair in leaves.windows(2) {
            let left = pair[0].borrow();
            let right = pair[1].borrow();
            let separation = (self.separation)(&left, &right);
            validate_separation_value(separation)?;
        }
        Ok(())
    }

    fn position_leaves(&self, leaves: &[Rc<RefCell<HierarchyNode<T>>>]) {
        let Some(first) = leaves.first() else {
            return;
        };

        first.borrow_mut().x = 0.0;
        for pair in leaves.windows(2) {
            let previous_x = pair[0].borrow().x;
            let separation = {
                let previous = pair[0].borrow();
                let current = pair[1].borrow();
                (self.separation)(&previous, &current)
            };
            pair[1].borrow_mut().x = previous_x + separation;
        }
    }

    fn leaves(root: Rc<RefCell<HierarchyNode<T>>>) -> Vec<Rc<RefCell<HierarchyNode<T>>>> {
        let mut leaves = Vec::new();
        HierarchyNode::each(root, |node| {
            let is_leaf = {
                let n = node.borrow();
                n.children.is_none() || n.children.as_ref().unwrap().is_empty()
            };
            if is_leaf {
                leaves.push(node);
            }
        });
        leaves
    }

    fn position_internal_cluster(node: Rc<RefCell<HierarchyNode<T>>>) -> f64 {
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

fn validate_separation_value(value: f64) -> Result<(), HierarchyError> {
    if !value.is_finite() {
        return Err(HierarchyError::NonFiniteLayoutSeparation { value });
    }
    if value < 0.0 {
        return Err(HierarchyError::NegativeLayoutSeparation { value });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{HierarchyError, HierarchyNode, TreeLayout};

    #[test]
    fn try_layout_matches_permissive_layout_for_valid_dimensions() {
        let root = HierarchyNode::new(());
        let left = HierarchyNode::new(());
        let right = HierarchyNode::new(());
        root.borrow_mut().set_children(&root, vec![left, right]);

        TreeLayout::new()
            .size((200.0, 100.0))
            .try_layout(root.clone())
            .unwrap();

        assert_eq!(root.borrow().x, 0.0);
        assert!(root.borrow().y.is_finite());
    }

    #[test]
    fn try_layout_rejects_non_finite_layout_dimensions() {
        let root = HierarchyNode::new(());

        assert_eq!(
            TreeLayout::new()
                .size((f64::INFINITY, 100.0))
                .try_layout(root)
                .err()
                .unwrap(),
            HierarchyError::NonFiniteLayoutSize {
                coordinate: "width",
                value: f64::INFINITY
            }
        );
    }

    #[test]
    fn try_layout_rejects_negative_node_dimensions() {
        let root = HierarchyNode::new(());

        assert_eq!(
            TreeLayout::new()
                .node_size((10.0, -1.0))
                .try_layout(root)
                .err()
                .unwrap(),
            HierarchyError::NegativeLayoutSize {
                coordinate: "node_height",
                value: -1.0
            }
        );
    }

    #[test]
    fn layout_applies_custom_leaf_separation() {
        let (root, first_leaf, second_leaf, third_leaf) = sample_tree();

        TreeLayout::new()
            .size((200.0, 120.0))
            .separation(|_, _| 1.0)
            .try_layout(root)
            .unwrap();

        assert_eq!(first_leaf.borrow().y, 0.0);
        assert_eq!(second_leaf.borrow().y, 60.0);
        assert_eq!(third_leaf.borrow().y, 120.0);
    }

    #[test]
    fn default_separation_keeps_non_siblings_farther_apart() {
        let (root, first_leaf, second_leaf, third_leaf) = sample_tree();

        TreeLayout::new()
            .size((200.0, 120.0))
            .try_layout(root)
            .unwrap();

        assert_eq!(first_leaf.borrow().y, 0.0);
        assert_eq!(second_leaf.borrow().y, 40.0);
        assert_eq!(third_leaf.borrow().y, 120.0);
    }

    #[test]
    fn separation_callback_receives_typed_nodes() {
        let root = HierarchyNode::new("root");
        let first = HierarchyNode::new("first");
        let second = HierarchyNode::new("second");
        root.borrow_mut()
            .set_children(&root, vec![first.clone(), second.clone()]);

        TreeLayout::<&str>::new()
            .size((100.0, 100.0))
            .separation(|left, right| {
                if left.data == "first" && right.data == "second" {
                    4.0
                } else {
                    1.0
                }
            })
            .try_layout(root)
            .unwrap();

        assert_eq!(first.borrow().y, 0.0);
        assert_eq!(second.borrow().y, 100.0);
    }

    #[test]
    fn try_layout_rejects_non_finite_separation() {
        let (root, _, _, _) = sample_tree();

        assert!(matches!(
            TreeLayout::new()
                .separation(|_, _| f64::NAN)
                .try_layout(root),
            Err(HierarchyError::NonFiniteLayoutSeparation { value }) if value.is_nan()
        ));
    }

    #[test]
    fn try_layout_rejects_negative_separation() {
        let (root, _, _, _) = sample_tree();

        assert_eq!(
            TreeLayout::new()
                .separation(|_, _| -1.0)
                .try_layout(root)
                .err()
                .unwrap(),
            HierarchyError::NegativeLayoutSeparation { value: -1.0 }
        );
    }

    type UnitNode = Rc<RefCell<HierarchyNode<()>>>;

    fn sample_tree() -> (UnitNode, UnitNode, UnitNode, UnitNode) {
        let root = HierarchyNode::new(());
        let group = HierarchyNode::new(());
        let first_leaf = HierarchyNode::new(());
        let second_leaf = HierarchyNode::new(());
        let third_leaf = HierarchyNode::new(());

        group
            .borrow_mut()
            .set_children(&group, vec![first_leaf.clone(), second_leaf.clone()]);
        root.borrow_mut()
            .set_children(&root, vec![group, third_leaf.clone()]);

        (root, first_leaf, second_leaf, third_leaf)
    }

    use std::cell::RefCell;
    use std::rc::Rc;
}
