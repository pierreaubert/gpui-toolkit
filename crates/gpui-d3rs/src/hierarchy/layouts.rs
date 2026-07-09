//! First-class hierarchy layouts beyond tidy trees.
//!
//! These layouts operate on [`HierarchyNode`] trees. Rectangular layouts return
//! explicit rectangles, packed layouts return explicit circles, and cluster
//! layout writes coordinates to the existing node fields like [`TreeLayout`].

use super::{HierarchyError, HierarchyNode, validate_layout_dimension, validate_layout_padding};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

type NodeRef<T> = Rc<RefCell<HierarchyNode<T>>>;

/// A rectangular hierarchy layout result.
#[derive(Clone, Debug)]
pub struct HierarchyRect<T> {
    pub node: NodeRef<T>,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub depth: usize,
    pub value: f64,
}

/// A circular hierarchy layout result.
#[derive(Clone, Debug)]
pub struct HierarchyCircle<T> {
    pub node: NodeRef<T>,
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub depth: usize,
    pub value: f64,
}

/// Construct a hierarchy rectangle. Useful for tests and examples.
pub fn hierarchy_rect<T>(
    node: NodeRef<T>,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    depth: usize,
    value: f64,
) -> HierarchyRect<T> {
    HierarchyRect {
        node,
        x0,
        y0,
        x1,
        y1,
        depth,
        value,
    }
}

/// Construct a hierarchy circle. Useful for tests and examples.
pub fn hierarchy_circle<T>(
    node: NodeRef<T>,
    x: f64,
    y: f64,
    r: f64,
    depth: usize,
    value: f64,
) -> HierarchyCircle<T> {
    HierarchyCircle {
        node,
        x,
        y,
        r,
        depth,
        value,
    }
}

/// Slice-and-dice treemap layout.
#[derive(Clone, Debug)]
pub struct TreemapLayout<T = ()> {
    size: (f64, f64),
    padding: f64,
    _marker: PhantomData<T>,
}

/// Partition layout.
#[derive(Clone, Debug)]
pub struct PartitionLayout<T = ()> {
    size: (f64, f64),
    padding: f64,
    _marker: PhantomData<T>,
}

/// Circle packing layout.
#[derive(Clone, Debug)]
pub struct PackLayout<T = ()> {
    size: (f64, f64),
    padding: f64,
    _marker: PhantomData<T>,
}

/// Cluster layout configuration.
#[derive(Clone, Debug)]
pub struct ClusterLayout<T = ()> {
    size: (f64, f64),
    node_size: Option<(f64, f64)>,
    separation: fn(&HierarchyNode<T>, &HierarchyNode<T>) -> f64,
}

impl<T> Default for TreemapLayout<T> {
    fn default() -> Self {
        Self {
            size: (1.0, 1.0),
            padding: 0.0,
            _marker: PhantomData,
        }
    }
}

impl<T> Default for PartitionLayout<T> {
    fn default() -> Self {
        Self {
            size: (1.0, 1.0),
            padding: 0.0,
            _marker: PhantomData,
        }
    }
}

impl<T> Default for PackLayout<T> {
    fn default() -> Self {
        Self {
            size: (1.0, 1.0),
            padding: 0.0,
            _marker: PhantomData,
        }
    }
}

impl<T> Default for ClusterLayout<T> {
    fn default() -> Self {
        Self {
            size: (1.0, 1.0),
            node_size: None,
            separation: default_separation,
        }
    }
}

impl<T> TreemapLayout<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn size(mut self, size: (f64, f64)) -> Self {
        self.size = size;
        self
    }

    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    pub fn layout(&self, root: NodeRef<T>) -> Vec<HierarchyRect<T>> {
        let mut rects = Vec::new();
        self.layout_node(root, 0.0, 0.0, self.size.0, self.size.1, 0, &mut rects);
        rects
    }

    pub fn try_layout(&self, root: NodeRef<T>) -> Result<Vec<HierarchyRect<T>>, HierarchyError> {
        self.validate(root.clone())?;
        Ok(self.layout(root))
    }

    fn validate(&self, root: NodeRef<T>) -> Result<(), HierarchyError> {
        validate_common(self.size, self.padding, root)
    }

    fn layout_node(
        &self,
        node: NodeRef<T>,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        depth: usize,
        rects: &mut Vec<HierarchyRect<T>>,
    ) {
        let value = subtree_weight(&node);
        rects.push(hierarchy_rect(node.clone(), x0, y0, x1, y1, depth, value));

        let children = children(&node);
        if children.is_empty() {
            return;
        }

        let (ix0, iy0, ix1, iy1) = inset_rect(x0, y0, x1, y1, self.padding);
        let total = positive_total(&children);
        let count = children.len() as f64;
        let mut cursor = if depth % 2 == 0 { ix0 } else { iy0 };

        for (index, child) in children.iter().enumerate() {
            let fraction = if total > 0.0 {
                subtree_weight(child) / total
            } else {
                1.0 / count
            };
            if depth % 2 == 0 {
                let next = if index + 1 == children.len() {
                    ix1
                } else {
                    cursor + (ix1 - ix0) * fraction
                };
                self.layout_node(child.clone(), cursor, iy0, next, iy1, depth + 1, rects);
                cursor = next;
            } else {
                let next = if index + 1 == children.len() {
                    iy1
                } else {
                    cursor + (iy1 - iy0) * fraction
                };
                self.layout_node(child.clone(), ix0, cursor, ix1, next, depth + 1, rects);
                cursor = next;
            }
        }
    }
}

impl<T> PartitionLayout<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn size(mut self, size: (f64, f64)) -> Self {
        self.size = size;
        self
    }

    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    pub fn layout(&self, root: NodeRef<T>) -> Vec<HierarchyRect<T>> {
        let mut rects = Vec::new();
        let max_depth = max_depth(root.clone(), 0);
        let band_height = if max_depth == 0 {
            self.size.1
        } else {
            self.size.1 / (max_depth + 1) as f64
        };
        self.layout_node(root, 0.0, self.size.0, 0, band_height, &mut rects);
        rects
    }

    pub fn try_layout(&self, root: NodeRef<T>) -> Result<Vec<HierarchyRect<T>>, HierarchyError> {
        self.validate(root.clone())?;
        Ok(self.layout(root))
    }

    fn validate(&self, root: NodeRef<T>) -> Result<(), HierarchyError> {
        validate_common(self.size, self.padding, root)
    }

    fn layout_node(
        &self,
        node: NodeRef<T>,
        x0: f64,
        x1: f64,
        depth: usize,
        band_height: f64,
        rects: &mut Vec<HierarchyRect<T>>,
    ) {
        let y0 = depth as f64 * band_height;
        let y1 = y0 + band_height;
        let (rx0, ry0, rx1, ry1) = inset_rect(x0, y0, x1, y1, self.padding);
        let value = subtree_weight(&node);
        rects.push(hierarchy_rect(
            node.clone(),
            rx0,
            ry0,
            rx1,
            ry1,
            depth,
            value,
        ));

        let children = children(&node);
        if children.is_empty() {
            return;
        }

        let total = positive_total(&children);
        let count = children.len() as f64;
        let mut cursor = x0;
        for (index, child) in children.iter().enumerate() {
            let fraction = if total > 0.0 {
                subtree_weight(child) / total
            } else {
                1.0 / count
            };
            let next = if index + 1 == children.len() {
                x1
            } else {
                cursor + (x1 - x0) * fraction
            };
            self.layout_node(child.clone(), cursor, next, depth + 1, band_height, rects);
            cursor = next;
        }
    }
}

impl<T> PackLayout<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn size(mut self, size: (f64, f64)) -> Self {
        self.size = size;
        self
    }

    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    pub fn layout(&self, root: NodeRef<T>) -> Vec<HierarchyCircle<T>> {
        let mut circles = Vec::new();
        let r = self.size.0.min(self.size.1) / 2.0;
        self.layout_node(
            root,
            self.size.0 / 2.0,
            self.size.1 / 2.0,
            r,
            0,
            &mut circles,
        );
        circles
    }

    pub fn try_layout(&self, root: NodeRef<T>) -> Result<Vec<HierarchyCircle<T>>, HierarchyError> {
        self.validate(root.clone())?;
        Ok(self.layout(root))
    }

    fn validate(&self, root: NodeRef<T>) -> Result<(), HierarchyError> {
        validate_common(self.size, self.padding, root)
    }

    fn layout_node(
        &self,
        node: NodeRef<T>,
        x: f64,
        y: f64,
        r: f64,
        depth: usize,
        circles: &mut Vec<HierarchyCircle<T>>,
    ) {
        let value = subtree_weight(&node);
        circles.push(hierarchy_circle(node.clone(), x, y, r, depth, value));

        let children = children(&node);
        if children.is_empty() {
            return;
        }

        let available = (r - self.padding).max(0.0);
        let max_weight = children
            .iter()
            .map(subtree_weight)
            .fold(0.0f64, f64::max)
            .max(1.0);
        let count = children.len() as f64;
        let base_radius = if children.len() == 1 {
            available
        } else {
            available / (1.0 + count.sqrt())
        };

        for (index, child) in children.iter().enumerate() {
            let weight_scale = (subtree_weight(child) / max_weight).sqrt().clamp(0.2, 1.0);
            let child_r = if children.len() == 1 {
                available
            } else {
                base_radius * weight_scale
            };
            let ring = (available - child_r).max(0.0);
            let angle = std::f64::consts::TAU * index as f64 / count;
            let child_x = if children.len() == 1 {
                x
            } else {
                x + ring * angle.cos()
            };
            let child_y = if children.len() == 1 {
                y
            } else {
                y + ring * angle.sin()
            };
            self.layout_node(child.clone(), child_x, child_y, child_r, depth + 1, circles);
        }
    }
}

impl<T> ClusterLayout<T> {
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
        self.size = size;
        self
    }

    pub fn separation(
        mut self,
        separation: fn(&HierarchyNode<T>, &HierarchyNode<T>) -> f64,
    ) -> Self {
        self.separation = separation;
        self
    }

    pub fn layout(&self, root: NodeRef<T>)
    where
        T: Clone + 'static,
    {
        let leaves = leaves(root.clone());
        let max_depth = max_depth(root.clone(), 0);
        self.position_leaves(&leaves);
        position_internal_cluster(root.clone());

        if let Some((node_width, node_height)) = self.node_size {
            HierarchyNode::each(root, |node| {
                let mut n = node.borrow_mut();
                let breadth = n.x;
                n.x = n.depth as f64 * node_width;
                n.y = breadth * node_height;
            });
            return;
        }

        let span = leaves
            .last()
            .map(|leaf| leaf.borrow().x)
            .unwrap_or_default()
            .max(1.0);
        let x_scale = self.size.1 / span;
        let y_scale = self.size.0 / (max_depth as f64).max(1.0);

        HierarchyNode::each(root, |node| {
            let mut n = node.borrow_mut();
            let breadth = n.x;
            n.x = n.depth as f64 * y_scale;
            n.y = breadth * x_scale;
        });
    }

    pub fn try_layout(&self, root: NodeRef<T>) -> Result<(), HierarchyError>
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
        validate_separation(root.clone(), self.separation)?;
        self.layout(root);
        Ok(())
    }

    fn position_leaves(&self, leaves: &[NodeRef<T>]) {
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

fn validate_common<T>(
    size: (f64, f64),
    padding: f64,
    root: NodeRef<T>,
) -> Result<(), HierarchyError> {
    validate_layout_dimension("width", size.0)?;
    validate_layout_dimension("height", size.1)?;
    validate_layout_padding(padding)?;
    validate_node_values(root)
}

fn validate_node_values<T>(root: NodeRef<T>) -> Result<(), HierarchyError> {
    let mut index = 0;
    let mut error = None;
    HierarchyNode::each(root, |node| {
        if error.is_some() {
            return;
        }
        if let Some(value) = node.borrow().value {
            if !value.is_finite() {
                error = Some(HierarchyError::NonFiniteValue {
                    node_index: index,
                    value,
                });
            } else if value < 0.0 {
                error = Some(HierarchyError::NegativeValue {
                    node_index: index,
                    value,
                });
            }
        }
        index += 1;
    });
    error.map_or(Ok(()), Err)
}

fn validate_separation<T>(
    root: NodeRef<T>,
    separation: fn(&HierarchyNode<T>, &HierarchyNode<T>) -> f64,
) -> Result<(), HierarchyError>
where
    T: Clone + 'static,
{
    for pair in leaves(root).windows(2) {
        let left = pair[0].borrow();
        let right = pair[1].borrow();
        let value = separation(&left, &right);
        if !value.is_finite() {
            return Err(HierarchyError::NonFiniteLayoutSeparation { value });
        }
        if value < 0.0 {
            return Err(HierarchyError::NegativeLayoutSeparation { value });
        }
    }
    Ok(())
}

fn children<T>(node: &NodeRef<T>) -> Vec<NodeRef<T>> {
    node.borrow().children.clone().unwrap_or_default()
}

fn subtree_weight<T>(node: &NodeRef<T>) -> f64 {
    if let Some(value) = node.borrow().value {
        return sanitize_weight(value);
    }

    let children = children(node);
    if children.is_empty() {
        1.0
    } else {
        children.iter().map(subtree_weight).sum::<f64>().max(0.0)
    }
}

fn sanitize_weight(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

fn positive_total<T>(nodes: &[NodeRef<T>]) -> f64 {
    nodes.iter().map(subtree_weight).sum()
}

fn inset_rect(x0: f64, y0: f64, x1: f64, y1: f64, padding: f64) -> (f64, f64, f64, f64) {
    let half_width = ((x1 - x0) / 2.0).max(0.0);
    let half_height = ((y1 - y0) / 2.0).max(0.0);
    let inset_x = padding.min(half_width);
    let inset_y = padding.min(half_height);
    (x0 + inset_x, y0 + inset_y, x1 - inset_x, y1 - inset_y)
}

fn max_depth<T>(node: NodeRef<T>, depth: usize) -> usize {
    let children = children(&node);
    if children.is_empty() {
        depth
    } else {
        children
            .into_iter()
            .map(|child| max_depth(child, depth + 1))
            .max()
            .unwrap_or(depth)
    }
}

fn leaves<T>(root: NodeRef<T>) -> Vec<NodeRef<T>>
where
    T: Clone + 'static,
{
    let mut result = Vec::new();
    HierarchyNode::each(root, |node| {
        if children(&node).is_empty() {
            result.push(node);
        }
    });
    result
}

fn position_internal_cluster<T>(node: NodeRef<T>) -> f64 {
    let node_children = children(&node);
    if !node_children.is_empty() {
        let sum = node_children
            .iter()
            .map(|child| position_internal_cluster(child.clone()))
            .sum::<f64>();
        let x = sum / node_children.len() as f64;
        node.borrow_mut().x = x;
        return x;
    }

    node.borrow().x
}

#[cfg(test)]
mod tests {
    use super::{
        ClusterLayout, HierarchyError, HierarchyNode, PackLayout, PartitionLayout, TreemapLayout,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn treemap_returns_first_class_rectangles() {
        let (root, left, right) = valued_tree();

        let rects = TreemapLayout::new()
            .size((120.0, 80.0))
            .padding(2.0)
            .try_layout(root)
            .unwrap();

        assert_eq!(rects.len(), 3);
        assert_eq!(rects[0].x0, 0.0);
        assert_eq!(rects[0].y0, 0.0);
        assert_eq!(rects[0].x1, 120.0);
        assert_eq!(rects[0].y1, 80.0);

        let left_rect = rects
            .iter()
            .find(|rect| Rc::ptr_eq(&rect.node, &left))
            .unwrap();
        let right_rect = rects
            .iter()
            .find(|rect| Rc::ptr_eq(&rect.node, &right))
            .unwrap();
        assert_eq!(left_rect.depth, 1);
        assert!(left_rect.x0 >= 2.0);
        assert!(right_rect.x1 <= 118.0);
        assert!(left_rect.x1 < right_rect.x1);
    }

    #[test]
    fn partition_returns_depth_banded_rectangles() {
        let (root, left, right) = valued_tree();

        let rects = PartitionLayout::new()
            .size((120.0, 90.0))
            .try_layout(root.clone())
            .unwrap();

        let root_rect = rects
            .iter()
            .find(|rect| Rc::ptr_eq(&rect.node, &root))
            .unwrap();
        let left_rect = rects
            .iter()
            .find(|rect| Rc::ptr_eq(&rect.node, &left))
            .unwrap();
        let right_rect = rects
            .iter()
            .find(|rect| Rc::ptr_eq(&rect.node, &right))
            .unwrap();

        assert_eq!(root_rect.y0, 0.0);
        assert_eq!(root_rect.y1, 45.0);
        assert_eq!(left_rect.y0, 45.0);
        assert_eq!(left_rect.y1, 90.0);
        assert!(left_rect.x1 < right_rect.x1);
    }

    #[test]
    fn pack_returns_bounded_circles() {
        let (root, left, right) = valued_tree();

        let circles = PackLayout::new()
            .size((100.0, 80.0))
            .padding(2.0)
            .try_layout(root.clone())
            .unwrap();

        assert_eq!(circles.len(), 3);
        let root_circle = circles
            .iter()
            .find(|circle| Rc::ptr_eq(&circle.node, &root))
            .unwrap();
        assert_eq!(root_circle.x, 50.0);
        assert_eq!(root_circle.y, 40.0);
        assert_eq!(root_circle.r, 40.0);

        for node in [left, right] {
            let circle = circles
                .iter()
                .find(|circle| Rc::ptr_eq(&circle.node, &node))
                .unwrap();
            assert!(circle.x - circle.r >= 0.0);
            assert!(circle.y - circle.r >= 0.0);
            assert!(circle.x + circle.r <= 100.0);
            assert!(circle.y + circle.r <= 80.0);
        }
    }

    #[test]
    fn cluster_is_a_public_coordinate_layout() {
        let (root, first_leaf, second_leaf, third_leaf) = cluster_tree();

        ClusterLayout::<()>::new()
            .size((200.0, 120.0))
            .separation(|_, _| 1.0)
            .try_layout(root.clone())
            .unwrap();

        assert_eq!(root.borrow().x, 0.0);
        assert_eq!(first_leaf.borrow().y, 0.0);
        assert_eq!(second_leaf.borrow().y, 60.0);
        assert_eq!(third_leaf.borrow().y, 120.0);
    }

    #[test]
    fn checked_layouts_reject_invalid_padding_and_values() {
        let (root, _, _) = valued_tree();

        assert_eq!(
            TreemapLayout::new()
                .padding(-1.0)
                .try_layout(root.clone())
                .err()
                .unwrap(),
            HierarchyError::NegativeLayoutPadding { value: -1.0 }
        );

        root.borrow_mut().value = Some(f64::NAN);
        assert!(matches!(
            PackLayout::new().try_layout(root),
            Err(HierarchyError::NonFiniteValue {
                node_index: 0,
                value,
            }) if value.is_nan()
        ));
    }

    type NumberNode = Rc<RefCell<HierarchyNode<&'static str>>>;

    fn valued_tree() -> (NumberNode, NumberNode, NumberNode) {
        let root = HierarchyNode::new("root");
        let left = HierarchyNode::new("left");
        let right = HierarchyNode::new("right");
        root.borrow_mut()
            .set_children(&root, vec![left.clone(), right.clone()]);

        left.borrow_mut().value = Some(1.0);
        right.borrow_mut().value = Some(3.0);
        root.borrow_mut().value = Some(4.0);

        (root, left, right)
    }

    type UnitNode = Rc<RefCell<HierarchyNode<()>>>;

    fn cluster_tree() -> (UnitNode, UnitNode, UnitNode, UnitNode) {
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
}
