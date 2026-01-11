//! Hierarchical data structures and algorithms
//!
//! This module provides tools for visualizing hierarchical data, such as trees,
//! treemaps, pack layouts, and partitions.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub mod tree;
pub use tree::TreeLayout;

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
            child.borrow_mut().depth = self.depth + 1;
        }
        self.children = Some(children);
    }
}
