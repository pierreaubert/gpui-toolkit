//! Hierarchical data structures and algorithms
//!
//! This module provides tools for visualizing hierarchical data, such as trees,
//! treemaps, pack layouts, and partitions.

use std::cell::RefCell;
use std::fmt;
use std::rc::{Rc, Weak};

pub mod layouts;
pub mod tree;
pub use layouts::{
    ClusterLayout, PackLayout, PartitionLayout, TreemapLayout, hierarchy_circle, hierarchy_rect,
};
pub use tree::TreeLayout;

/// Recoverable errors for checked hierarchy operations.
#[derive(Debug, Clone, PartialEq)]
pub enum HierarchyError {
    /// Hierarchy value accessors must return finite values.
    NonFiniteValue { node_index: usize, value: f64 },
    /// Checked hierarchy value sums require non-negative values.
    NegativeValue { node_index: usize, value: f64 },
    /// Tree layout dimensions must be finite.
    NonFiniteLayoutSize {
        coordinate: &'static str,
        value: f64,
    },
    /// Tree layout dimensions cannot be negative.
    NegativeLayoutSize {
        coordinate: &'static str,
        value: f64,
    },
    /// Layout padding must be finite.
    NonFiniteLayoutPadding { value: f64 },
    /// Layout padding cannot be negative.
    NegativeLayoutPadding { value: f64 },
    /// Tree layout separation accessors must return finite values.
    NonFiniteLayoutSeparation { value: f64 },
    /// Tree layout separation accessors cannot return negative values.
    NegativeLayoutSeparation { value: f64 },
}

impl fmt::Display for HierarchyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue { node_index, value } => write!(
                f,
                "hierarchy value at traversal index {node_index} is not finite: {value}"
            ),
            Self::NegativeValue { node_index, value } => write!(
                f,
                "hierarchy value at traversal index {node_index} is negative: {value}"
            ),
            Self::NonFiniteLayoutSize { coordinate, value } => write!(
                f,
                "hierarchy layout dimension {coordinate} is not finite: {value}"
            ),
            Self::NegativeLayoutSize { coordinate, value } => write!(
                f,
                "hierarchy layout dimension {coordinate} is negative: {value}"
            ),
            Self::NonFiniteLayoutPadding { value } => {
                write!(f, "hierarchy layout padding is not finite: {value}")
            }
            Self::NegativeLayoutPadding { value } => {
                write!(f, "hierarchy layout padding is negative: {value}")
            }
            Self::NonFiniteLayoutSeparation { value } => {
                write!(f, "hierarchy tree separation is not finite: {value}")
            }
            Self::NegativeLayoutSeparation { value } => {
                write!(f, "hierarchy tree separation is negative: {value}")
            }
        }
    }
}

impl std::error::Error for HierarchyError {}

/// A node in a hierarchy
///
/// Wraps the generic data `T` and provides tree traversal/layout properties.
#[derive(Debug, Clone)]
pub struct HierarchyNode<T> {
    /// The associated data
    pub data: T,
    /// Parent node (weak reference to avoid cycles)
    pub parent: Option<Weak<RefCell<HierarchyNode<T>>>>,
    /// Children nodes
    pub children: Option<Vec<Rc<RefCell<HierarchyNode<T>>>>>,
    /// Accumulated value (e.g. sum of children values)
    pub value: Option<f64>,
    /// Depth of the node (root is 0)
    pub depth: usize,
    /// Height of the node (leaf is 0)
    pub height: usize,
    /// X coordinate (computed by layouts)
    pub x: f64,
    /// Y coordinate (computed by layouts)
    pub y: f64,
}

impl<T> HierarchyNode<T> {
    /// Create a new hierarchy node
    pub fn new(data: T) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            data,
            parent: None,
            children: None,
            value: None,
            depth: 0,
            height: 0,
            x: 0.0,
            y: 0.0,
        }))
    }

    /// Compute the node's value by summing a value accessor over descendants
    pub fn sum<F>(node: Rc<RefCell<Self>>, value_fn: F) -> Rc<RefCell<Self>>
    where
        F: Fn(&T) -> f64 + Copy,
    {
        let mut node_borrow = node.borrow_mut();
        let mut sum = value_fn(&node_borrow.data);

        if let Some(children) = &node_borrow.children {
            for child in children {
                Self::sum(child.clone(), value_fn);
                if let Some(child_val) = child.borrow().value {
                    sum += child_val;
                }
            }
        }

        node_borrow.value = Some(sum);
        drop(node_borrow);
        node
    }

    /// Compute the node's value after validating all accessor outputs.
    ///
    /// This checked variant leaves existing `value` fields unchanged when any
    /// node returns a non-finite or negative value.
    pub fn try_sum<F>(
        node: Rc<RefCell<Self>>,
        value_fn: F,
    ) -> Result<Rc<RefCell<Self>>, HierarchyError>
    where
        F: Fn(&T) -> f64 + Copy,
    {
        let mut node_index = 0;
        let mut values = Vec::new();
        Self::collect_checked_sum(node.clone(), value_fn, &mut node_index, &mut values)?;

        for (node, value) in values {
            node.borrow_mut().value = Some(value);
        }

        Ok(node)
    }

    /// Compute the node's count (number of leaves)
    pub fn count(node: Rc<RefCell<Self>>) -> Rc<RefCell<Self>> {
        Self::sum(node, |_| 1.0)
    }

    /// Sort children based on a comparator
    pub fn sort<F>(node: Rc<RefCell<Self>>, compare_fn: F) -> Rc<RefCell<Self>>
    where
        F: Fn(&HierarchyNode<T>, &HierarchyNode<T>) -> std::cmp::Ordering + Copy,
    {
        let mut node_borrow = node.borrow_mut();

        if let Some(children) = &mut node_borrow.children {
            for child in children.iter() {
                Self::sort(child.clone(), compare_fn);
            }
            children.sort_by(|a, b| compare_fn(&a.borrow(), &b.borrow()));
        }

        drop(node_borrow);
        node
    }

    /// Traverse the tree in pre-order
    pub fn each<F>(node: Rc<RefCell<Self>>, mut callback: F)
    where
        F: FnMut(Rc<RefCell<Self>>),
    {
        Self::each_recursive(node, &mut callback);
    }

    fn each_recursive<F>(node: Rc<RefCell<Self>>, callback: &mut F)
    where
        F: FnMut(Rc<RefCell<Self>>),
    {
        callback(node.clone());
        let node_borrow = node.borrow();
        if let Some(children) = &node_borrow.children {
            for child in children {
                Self::each_recursive(child.clone(), callback);
            }
        }
    }

    /// Helper to attach children to a parent
    pub fn set_children(&mut self, self_rc: &Rc<RefCell<Self>>, children: Vec<Rc<RefCell<Self>>>) {
        for child in &children {
            child.borrow_mut().parent = Some(Rc::downgrade(self_rc));
            Self::set_subtree_depth(child.clone(), self.depth + 1);
        }
        self.children = Some(children);
    }

    fn set_subtree_depth(node: Rc<RefCell<Self>>, depth: usize) {
        let children = {
            let mut node = node.borrow_mut();
            node.depth = depth;
            node.children.clone().unwrap_or_default()
        };
        for child in children {
            Self::set_subtree_depth(child, depth + 1);
        }
    }

    fn collect_checked_sum<F>(
        node: Rc<RefCell<Self>>,
        value_fn: F,
        node_index: &mut usize,
        values: &mut Vec<(Rc<RefCell<Self>>, f64)>,
    ) -> Result<f64, HierarchyError>
    where
        F: Fn(&T) -> f64 + Copy,
    {
        let current_index = *node_index;
        *node_index += 1;

        let (mut sum, children) = {
            let node_borrow = node.borrow();
            let value = value_fn(&node_borrow.data);
            if !value.is_finite() {
                return Err(HierarchyError::NonFiniteValue {
                    node_index: current_index,
                    value,
                });
            }
            if value < 0.0 {
                return Err(HierarchyError::NegativeValue {
                    node_index: current_index,
                    value,
                });
            }
            (value, node_borrow.children.clone())
        };

        if let Some(children) = children {
            for child in children {
                sum += Self::collect_checked_sum(child, value_fn, node_index, values)?;
            }
        }

        values.push((node, sum));
        Ok(sum)
    }
}

pub(crate) fn validate_layout_dimension(
    coordinate: &'static str,
    value: f64,
) -> Result<(), HierarchyError> {
    if !value.is_finite() {
        return Err(HierarchyError::NonFiniteLayoutSize { coordinate, value });
    }
    if value < 0.0 {
        return Err(HierarchyError::NegativeLayoutSize { coordinate, value });
    }
    Ok(())
}

pub(crate) fn validate_layout_padding(value: f64) -> Result<(), HierarchyError> {
    if !value.is_finite() {
        return Err(HierarchyError::NonFiniteLayoutPadding { value });
    }
    if value < 0.0 {
        return Err(HierarchyError::NegativeLayoutPadding { value });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{HierarchyError, HierarchyNode};

    #[test]
    fn try_sum_matches_permissive_sum_for_valid_values() {
        let root = HierarchyNode::new(1.0);
        let left = HierarchyNode::new(2.0);
        let right = HierarchyNode::new(3.0);
        root.borrow_mut()
            .set_children(&root, vec![left.clone(), right.clone()]);

        let checked = HierarchyNode::try_sum(root.clone(), |value| *value).unwrap();

        assert!(Rc::ptr_eq(&checked, &root));
        assert_eq!(root.borrow().value, Some(6.0));
        assert_eq!(left.borrow().value, Some(2.0));
        assert_eq!(right.borrow().value, Some(3.0));
    }

    #[test]
    fn try_sum_rejects_non_finite_values_without_mutating_existing_values() {
        let root = HierarchyNode::new(1.0);
        let child = HierarchyNode::new(f64::NAN);
        root.borrow_mut().value = Some(42.0);
        child.borrow_mut().value = Some(7.0);
        root.borrow_mut().set_children(&root, vec![child.clone()]);

        assert!(matches!(
            HierarchyNode::try_sum(root.clone(), |value| *value),
            Err(HierarchyError::NonFiniteValue {
                node_index: 1,
                value,
            }) if value.is_nan()
        ));
        assert_eq!(root.borrow().value, Some(42.0));
        assert_eq!(child.borrow().value, Some(7.0));
    }

    #[test]
    fn try_sum_rejects_negative_values_without_mutating_existing_values() {
        let root = HierarchyNode::new(1.0);
        let child = HierarchyNode::new(-1.0);
        root.borrow_mut().value = Some(42.0);
        child.borrow_mut().value = Some(7.0);
        root.borrow_mut().set_children(&root, vec![child.clone()]);

        assert_eq!(
            HierarchyNode::try_sum(root.clone(), |value| *value)
                .err()
                .unwrap(),
            HierarchyError::NegativeValue {
                node_index: 1,
                value: -1.0
            }
        );
        assert_eq!(root.borrow().value, Some(42.0));
        assert_eq!(child.borrow().value, Some(7.0));
    }

    use std::rc::Rc;
}
