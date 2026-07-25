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
        // Assign breadth in leaf order using the configured separation, then
        // center each parent over its children. This is the tidy-tree invariant:
        // siblings retain order, subtrees do not overlap, and parents sit at
        // their descendants' midpoint.
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

    // Contour-apportioned tidy tree layout.
    fn layout_cluster(&self, root: Rc<RefCell<HierarchyNode<T>>>)
    where
        T: Clone + 'static,
    {
        HierarchyNode::count(root.clone());
        self.position_tidy(root.clone());

        let mut max_depth = 0usize;
        let mut min_breadth = f64::INFINITY;
        let mut max_breadth = f64::NEG_INFINITY;
        HierarchyNode::each(root.clone(), |node| {
            let n = node.borrow();
            max_depth = max_depth.max(n.depth);
            min_breadth = min_breadth.min(n.y);
            max_breadth = max_breadth.max(n.y);
        });

        if let Some((node_width, node_height)) = self.node_size {
            HierarchyNode::each(root, |node| {
                let mut node = node.borrow_mut();
                node.x = node.depth as f64 * node_width;
                node.y *= node_height;
            });
            return;
        }

        let (width, height) = self.size;
        let breadth_scale = height / (max_breadth - min_breadth).max(1.0);
        let depth_scale = width / (max_depth as f64).max(1.0);

        HierarchyNode::each(root, |node| {
            let mut n = node.borrow_mut();
            n.x = n.depth as f64 * depth_scale;
            n.y = (n.y - min_breadth) * breadth_scale;
        });
    }

    fn validate_separation(&self, root: Rc<RefCell<HierarchyNode<T>>>) -> Result<(), HierarchyError>
    where
        T: Clone + 'static,
    {
        let mut nodes = Vec::new();
        HierarchyNode::each(root, |node| nodes.push(node));
        for left_index in 0..nodes.len() {
            for right in &nodes[left_index + 1..] {
                let left = nodes[left_index].borrow();
                let right = right.borrow();
                validate_separation_value((self.separation)(&left, &right))?;
                validate_separation_value((self.separation)(&right, &left))?;
            }
        }
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn position_tidy(
        &self,
        node: Rc<RefCell<HierarchyNode<T>>>,
    ) -> Vec<(
        f64,
        Rc<RefCell<HierarchyNode<T>>>,
        f64,
        Rc<RefCell<HierarchyNode<T>>>,
    )> {
        let children = node.borrow().children.clone().unwrap_or_default();
        if children.is_empty() {
            node.borrow_mut().y = 0.0;
            return vec![(0.0, node.clone(), 0.0, node)];
        }

        let mut combined: Vec<(
            f64,
            Rc<RefCell<HierarchyNode<T>>>,
            f64,
            Rc<RefCell<HierarchyNode<T>>>,
        )> = Vec::new();
        for child in &children {
            let mut contour = self.position_tidy(child.clone());
            let shift = combined
                .iter()
                .zip(&contour)
                .map(|((_, _, right, right_node), (left, left_node, _, _))| {
                    let separation = {
                        let right_node = right_node.borrow();
                        let left_node = left_node.borrow();
                        (self.separation)(&right_node, &left_node)
                    };
                    right + separation - left
                })
                .fold(0.0_f64, f64::max);

            if shift > 0.0 {
                HierarchyNode::each(child.clone(), |descendant| {
                    descendant.borrow_mut().y += shift;
                });
                for (left, _, right, _) in &mut contour {
                    *left += shift;
                    *right += shift;
                }
            }

            if combined.is_empty() {
                combined = contour;
                continue;
            }
            for (depth, entry) in contour.into_iter().enumerate() {
                if let Some(existing) = combined.get_mut(depth) {
                    if entry.0 < existing.0 {
                        existing.0 = entry.0;
                        existing.1 = entry.1.clone();
                    }
                    if entry.2 > existing.2 {
                        existing.2 = entry.2;
                        existing.3 = entry.3;
                    }
                } else {
                    combined.push(entry);
                }
            }
        }

        let position =
            (children.first().unwrap().borrow().y + children.last().unwrap().borrow().y) / 2.0;
        node.borrow_mut().y = position;
        let mut contour = Vec::with_capacity(combined.len() + 1);
        contour.push((position, node.clone(), position, node));
        contour.extend(combined);
        contour
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
        assert_eq!(second_leaf.borrow().y, 80.0);
        assert_eq!(third_leaf.borrow().y, 120.0);
    }

    #[test]
    fn default_separation_keeps_non_siblings_farther_apart() {
        let root = HierarchyNode::new(());
        let left = HierarchyNode::new(());
        let right = HierarchyNode::new(());
        let first_leaf = HierarchyNode::new(());
        let second_leaf = HierarchyNode::new(());
        let third_leaf = HierarchyNode::new(());
        let fourth_leaf = HierarchyNode::new(());
        left.borrow_mut()
            .set_children(&left, vec![first_leaf.clone(), second_leaf.clone()]);
        right
            .borrow_mut()
            .set_children(&right, vec![third_leaf.clone(), fourth_leaf.clone()]);
        root.borrow_mut().set_children(&root, vec![left, right]);

        TreeLayout::new()
            .size((200.0, 120.0))
            .try_layout(root)
            .unwrap();

        assert_eq!(first_leaf.borrow().y, 0.0);
        assert_eq!(second_leaf.borrow().y, 30.0);
        assert_eq!(third_leaf.borrow().y, 90.0);
        assert_eq!(fourth_leaf.borrow().y, 120.0);
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
    fn node_size_is_applied_per_depth_and_per_separation_unit() {
        let (root, first_leaf, second_leaf, third_leaf) = sample_tree();
        let group = root.borrow().children.as_ref().unwrap()[0].clone();

        TreeLayout::new()
            .node_size((40.0, 12.0))
            .separation(|_, _| 1.0)
            .try_layout(root.clone())
            .unwrap();

        assert_eq!(root.borrow().x, 0.0);
        assert_eq!(group.borrow().x, 40.0);
        assert_eq!(first_leaf.borrow().x, 80.0);
        assert_eq!(second_leaf.borrow().x, 80.0);
        assert_eq!(third_leaf.borrow().x, 40.0);
        assert_eq!(second_leaf.borrow().y - first_leaf.borrow().y, 12.0);
        assert_eq!(third_leaf.borrow().y - group.borrow().y, 12.0);
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
