use std::cell::RefCell;

/// A node in the treemap hierarchy.
///
/// Nodes can be leaf nodes (with a value) or internal nodes (with children).
/// The total value of a node is the sum of its value plus all descendant values.
#[derive(Debug, Clone)]
pub struct TreemapNode {
    /// Display name for this node
    pub name: String,
    /// Direct value (for leaf nodes)
    pub value: f64,
    /// Child nodes
    pub children: Vec<TreemapNode>,
    /// Cached total value including all descendants.
    cached_total: RefCell<Option<f64>>,
}

impl TreemapNode {
    /// Create a new leaf node with a value.
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
            children: Vec::new(),
            cached_total: RefCell::new(None),
        }
    }

    /// Create a new internal node with children.
    pub fn with_children(name: impl Into<String>, children: Vec<TreemapNode>) -> Self {
        Self {
            name: name.into(),
            value: 0.0,
            children,
            cached_total: RefCell::new(None),
        }
    }

    /// Add a child node (builder pattern).
    pub fn add_child(mut self, child: TreemapNode) -> Self {
        self.children.push(child);
        self.cached_total.take();
        self
    }

    /// Get the total value including all descendants.
    pub fn total_value(&self) -> f64 {
        if let Some(cached) = *self.cached_total.borrow() {
            return cached;
        }

        let total = if self.children.is_empty() {
            self.value
        } else {
            self.value + self.children.iter().map(|c| c.total_value()).sum::<f64>()
        };

        *self.cached_total.borrow_mut() = Some(total);
        total
    }

    /// Check if this is a leaf node (has no children).
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}
