//! QuadTree module for spatial indexing (d3-quadtree inspired)
//!
//! This module provides a quadtree data structure for efficient 2D spatial queries.
//! Quadtrees recursively subdivide 2D space into four quadrants, enabling O(log n)
//! nearest neighbor searches and range queries.
//!
//! # Features
//!
//! - **Spatial Indexing**: Fast insertion and lookup of 2D points
//! - **Nearest Neighbor Search**: Find closest point to any location
//! - **Range Queries**: Find all points within a radius
//! - **Tree Traversal**: Visit nodes in pre-order or post-order
//!
//! # Example
//!
//! ```rust
//! use d3rs::quadtree::QuadTree;
//!
//! // Create a quadtree with some points
//! let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)];
//! let tree = QuadTree::from_data(&points, |p| p.0, |p| p.1);
//!
//! // Find nearest point to (1.5, 1.5)
//! if let Some(point) = tree.find(1.5, 1.5, None) {
//!     println!("Nearest point: {:?}", point);
//! }
//! ```

use std::f64;

/// A point stored in the quadtree with its original data
#[derive(Debug, Clone)]
pub struct QuadPoint<T> {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
    /// Original data
    pub data: T,
    /// Next point in the same leaf (for coincident points)
    pub next: Option<Box<QuadPoint<T>>>,
}

impl<T> QuadPoint<T> {
    /// Create a new quadtree point
    pub fn new(x: f64, y: f64, data: T) -> Self {
        Self {
            x,
            y,
            data,
            next: None,
        }
    }
}

/// A node in the quadtree - either a leaf with points or an internal node with children
#[derive(Debug, Clone)]
pub enum QuadNode<T> {
    /// Leaf node containing a point (and potentially coincident points via linked list)
    Leaf(QuadPoint<T>),
    /// Internal node with four children: [NE, NW, SE, SW]
    /// Children are stored as: [0]=NE (x>=mid, y<mid), [1]=NW (x<mid, y<mid),
    ///                        [2]=SE (x>=mid, y>=mid), [3]=SW (x<mid, y>=mid)
    Internal(Box<[Option<QuadNode<T>>; 4]>),
}

impl<T> QuadNode<T> {
    /// Create a new internal node with no children
    pub fn new_internal() -> Self {
        QuadNode::Internal(Box::new([None, None, None, None]))
    }
}

/// Extent (bounding box) of the quadtree
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    /// Minimum x coordinate
    pub x0: f64,
    /// Minimum y coordinate
    pub y0: f64,
    /// Maximum x coordinate
    pub x1: f64,
    /// Maximum y coordinate
    pub y1: f64,
}

impl Extent {
    /// Create a new extent
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self { x0, y0, x1, y1 }
    }

    /// Create an extent that covers both extents
    pub fn union(&self, other: &Extent) -> Extent {
        Extent {
            x0: self.x0.min(other.x0),
            y0: self.y0.min(other.y0),
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
        }
    }

    /// Width of the extent
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }

    /// Height of the extent
    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }

    /// Check if a point is within the extent
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
}

/// QuadTree for 2D spatial indexing
///
/// A quadtree recursively partitions 2D space into quadrants, enabling efficient
/// spatial queries like nearest neighbor search.
#[derive(Debug, Clone)]
pub struct QuadTree<T> {
    /// Root node of the tree
    root: Option<QuadNode<T>>,
    /// Bounding box of the tree
    extent: Option<Extent>,
    /// Number of points in the tree
    size: usize,
}

impl<T: Clone> Default for QuadTree<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> QuadTree<T> {
    /// Create a new empty quadtree
    pub fn new() -> Self {
        Self {
            root: None,
            extent: None,
            size: 0,
        }
    }

    /// Create a quadtree from data with accessor functions
    ///
    /// # Arguments
    /// * `data` - Slice of data items
    /// * `x` - Function to extract x coordinate
    /// * `y` - Function to extract y coordinate
    pub fn from_data<F, G>(data: &[T], x: F, y: G) -> Self
    where
        F: Fn(&T) -> f64,
        G: Fn(&T) -> f64,
    {
        let mut tree = Self::new();

        if data.is_empty() {
            return tree;
        }

        // Compute extent
        let mut x0 = f64::INFINITY;
        let mut y0 = f64::INFINITY;
        let mut x1 = f64::NEG_INFINITY;
        let mut y1 = f64::NEG_INFINITY;

        for item in data {
            let px = x(item);
            let py = y(item);
            if px.is_finite() && py.is_finite() {
                x0 = x0.min(px);
                y0 = y0.min(py);
                x1 = x1.max(px);
                y1 = y1.max(py);
            }
        }

        // Make extent square and slightly larger
        tree.cover(x0, y0);
        tree.cover(x1, y1);

        // Add all points
        for item in data {
            let px = x(item);
            let py = y(item);
            if px.is_finite() && py.is_finite() {
                tree.add(px, py, item.clone());
            }
        }

        tree
    }

    /// Expand the extent to cover the given point
    pub fn cover(&mut self, x: f64, y: f64) {
        if !x.is_finite() || !y.is_finite() {
            return;
        }

        match &mut self.extent {
            None => {
                // Initialize with a unit square centered on the point
                let x0 = (x).floor();
                let y0 = (y).floor();
                self.extent = Some(Extent::new(x0, y0, x0 + 1.0, y0 + 1.0));
            }
            Some(ext) => {
                // Expand to cover the point, doubling size as needed
                let mut x0 = ext.x0;
                let mut y0 = ext.y0;
                let mut x1 = ext.x1;
                let mut y1 = ext.y1;

                while x < x0 || x >= x1 || y < y0 || y >= y1 {
                    let i = ((y < y0) as usize) << 1 | (x < x0) as usize;
                    let z = x1 - x0;

                    match i {
                        0 => {
                            // Point is to SE - expand NW
                            x1 = x0 + z * 2.0;
                            y1 = y0 + z * 2.0;
                        }
                        1 => {
                            // Point is to SW - expand NE
                            x0 -= z;
                            x1 = x0 + z * 2.0;
                            y1 = y0 + z * 2.0;
                        }
                        2 => {
                            // Point is to NE - expand SW
                            x1 = x0 + z * 2.0;
                            y0 -= z;
                            y1 = y0 + z * 2.0;
                        }
                        3 => {
                            // Point is to NW - expand SE
                            x0 -= z;
                            y0 -= z;
                            x1 = x0 + z * 2.0;
                            y1 = y0 + z * 2.0;
                        }
                        _ => unreachable!(),
                    }

                    // Wrap existing root in new internal node
                    // D3.js places old node at parent[i] where i is the cover direction
                    if self.root.is_some() {
                        let old_root = self.root.take();
                        let mut new_children: [Option<QuadNode<T>>; 4] = [None, None, None, None];
                        new_children[i] = old_root;
                        self.root = Some(QuadNode::Internal(Box::new(new_children)));
                    }
                }

                self.extent = Some(Extent::new(x0, y0, x1, y1));
            }
        }
    }

    /// Add a point to the quadtree
    pub fn add(&mut self, x: f64, y: f64, data: T) -> &mut Self {
        if !x.is_finite() || !y.is_finite() {
            return self;
        }

        self.cover(x, y);
        self.size += 1;

        let ext = self.extent.unwrap();
        let point = QuadPoint::new(x, y, data);

        // If tree is empty, set root to the point
        if self.root.is_none() {
            self.root = Some(QuadNode::Leaf(point));
            return self;
        }

        // Navigate to the correct position and insert
        self.root = Some(Self::add_to_node(
            self.root.take().unwrap(),
            point,
            ext.x0,
            ext.y0,
            ext.x1,
            ext.y1,
        ));

        self
    }

    fn add_to_node(
        node: QuadNode<T>,
        point: QuadPoint<T>,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    ) -> QuadNode<T> {
        match node {
            QuadNode::Leaf(mut existing) => {
                // Check for coincident point
                if (existing.x - point.x).abs() < 1e-12 && (existing.y - point.y).abs() < 1e-12 {
                    // Add to linked list
                    let mut new_point = point;
                    new_point.next = existing.next.take();
                    existing.next = Some(Box::new(new_point));
                    QuadNode::Leaf(existing)
                } else {
                    // Split into internal node
                    let mut internal = QuadNode::new_internal();

                    // Insert existing point
                    internal = Self::insert_into_internal(internal, existing, x0, y0, x1, y1);
                    // Insert new point
                    internal = Self::insert_into_internal(internal, point, x0, y0, x1, y1);

                    internal
                }
            }
            QuadNode::Internal(mut children) => {
                let xm = (x0 + x1) / 2.0;
                let ym = (y0 + y1) / 2.0;
                let i = Self::quadrant(point.x, point.y, xm, ym);

                let (nx0, ny0, nx1, ny1) = Self::child_extent(i, x0, y0, x1, y1, xm, ym);

                children[i] = Some(match children[i].take() {
                    None => QuadNode::Leaf(point),
                    Some(child) => Self::add_to_node(child, point, nx0, ny0, nx1, ny1),
                });

                QuadNode::Internal(children)
            }
        }
    }

    fn insert_into_internal(
        node: QuadNode<T>,
        point: QuadPoint<T>,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    ) -> QuadNode<T> {
        match node {
            QuadNode::Internal(mut children) => {
                let xm = (x0 + x1) / 2.0;
                let ym = (y0 + y1) / 2.0;
                let i = Self::quadrant(point.x, point.y, xm, ym);

                let (nx0, ny0, nx1, ny1) = Self::child_extent(i, x0, y0, x1, y1, xm, ym);

                children[i] = Some(match children[i].take() {
                    None => QuadNode::Leaf(point),
                    Some(child) => Self::add_to_node(child, point, nx0, ny0, nx1, ny1),
                });

                QuadNode::Internal(children)
            }
            _ => panic!("Expected internal node"),
        }
    }

    /// Determine which quadrant a point belongs to
    /// D3.js convention: i = (bottom << 1) | right
    /// Returns: 0=NW (left, top), 1=NE (right, top), 2=SW (left, bottom), 3=SE (right, bottom)
    fn quadrant(x: f64, y: f64, xm: f64, ym: f64) -> usize {
        let right = x >= xm;
        let bottom = y >= ym;
        (bottom as usize) << 1 | (right as usize)
    }

    fn child_extent(
        i: usize,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        xm: f64,
        ym: f64,
    ) -> (f64, f64, f64, f64) {
        // D3.js convention: 0=NW, 1=NE, 2=SW, 3=SE
        match i {
            0 => (x0, y0, xm, ym), // NW (left, top)
            1 => (xm, y0, x1, ym), // NE (right, top)
            2 => (x0, ym, xm, y1), // SW (left, bottom)
            3 => (xm, ym, x1, y1), // SE (right, bottom)
            _ => unreachable!(),
        }
    }

    /// Add multiple points to the quadtree
    pub fn add_all<F, G>(&mut self, data: &[T], x: F, y: G) -> &mut Self
    where
        F: Fn(&T) -> f64,
        G: Fn(&T) -> f64,
    {
        // First, expand extent to cover all points
        for item in data {
            let px = x(item);
            let py = y(item);
            if px.is_finite() && py.is_finite() {
                self.cover(px, py);
            }
        }

        // Then add all points
        for item in data {
            let px = x(item);
            let py = y(item);
            if px.is_finite() && py.is_finite() {
                self.add(px, py, item.clone());
            }
        }

        self
    }

    /// Remove a point from the quadtree
    ///
    /// Returns true if a point was removed
    pub fn remove(&mut self, x: f64, y: f64) -> bool {
        if !x.is_finite() || !y.is_finite() || self.root.is_none() {
            return false;
        }

        let ext = match &self.extent {
            Some(e) => *e,
            None => return false,
        };

        if !ext.contains(x, y) {
            return false;
        }

        let (new_root, removed) = Self::remove_from_node(
            self.root.take().unwrap(),
            x,
            y,
            ext.x0,
            ext.y0,
            ext.x1,
            ext.y1,
        );

        self.root = new_root;

        if removed {
            self.size -= 1;
        }

        removed
    }

    fn remove_from_node(
        node: QuadNode<T>,
        x: f64,
        y: f64,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    ) -> (Option<QuadNode<T>>, bool) {
        match node {
            QuadNode::Leaf(mut point) => {
                if (point.x - x).abs() < 1e-12 && (point.y - y).abs() < 1e-12 {
                    // Found the point - check for linked list
                    if let Some(next) = point.next.take() {
                        (Some(QuadNode::Leaf(*next)), true)
                    } else {
                        (None, true)
                    }
                } else {
                    // Check linked list for coincident points
                    let mut removed = false;
                    let mut current = &mut point.next;

                    while let Some(next) = current {
                        if (next.x - x).abs() < 1e-12 && (next.y - y).abs() < 1e-12 {
                            *current = next.next.take();
                            removed = true;
                            break;
                        }
                        current = &mut current.as_mut().unwrap().next;
                    }

                    (Some(QuadNode::Leaf(point)), removed)
                }
            }
            QuadNode::Internal(mut children) => {
                let xm = (x0 + x1) / 2.0;
                let ym = (y0 + y1) / 2.0;
                let i = Self::quadrant(x, y, xm, ym);

                if let Some(child) = children[i].take() {
                    let (nx0, ny0, nx1, ny1) = Self::child_extent(i, x0, y0, x1, y1, xm, ym);
                    let (new_child, removed) =
                        Self::remove_from_node(child, x, y, nx0, ny0, nx1, ny1);
                    children[i] = new_child;

                    // Check if we can collapse the node
                    let mut non_empty_count = 0;
                    let mut single_leaf = None;

                    for (idx, child) in children.iter().enumerate() {
                        if child.is_some() {
                            non_empty_count += 1;
                            if matches!(child, Some(QuadNode::Leaf(_))) {
                                single_leaf = Some(idx);
                            }
                        }
                    }

                    if non_empty_count == 0 {
                        (None, removed)
                    } else if non_empty_count == 1 && single_leaf.is_some() {
                        // Collapse to single leaf
                        let leaf = children[single_leaf.unwrap()].take();
                        (leaf, removed)
                    } else {
                        (Some(QuadNode::Internal(children)), removed)
                    }
                } else {
                    (Some(QuadNode::Internal(children)), false)
                }
            }
        }
    }

    /// Remove all points matching the predicate
    pub fn remove_all<F>(&mut self, predicate: F) -> usize
    where
        F: Fn(&T, f64, f64) -> bool,
    {
        let mut removed = 0;
        let points_to_remove: Vec<(f64, f64)> = self
            .data()
            .iter()
            .filter(|(x, y, d)| predicate(d, *x, *y))
            .map(|(x, y, _)| (*x, *y))
            .collect();

        for (x, y) in points_to_remove {
            if self.remove(x, y) {
                removed += 1;
            }
        }

        removed
    }

    /// Find the closest point to (x, y) within optional radius
    ///
    /// # Arguments
    /// * `x` - X coordinate to search from
    /// * `y` - Y coordinate to search from
    /// * `radius` - Optional maximum search radius (None for unlimited)
    ///
    /// # Returns
    /// Reference to the closest point's data, or None if no point found
    pub fn find(&self, x: f64, y: f64, radius: Option<f64>) -> Option<&T> {
        self.root.as_ref()?;
        let ext = self.extent?;
        let mut best: Option<(&T, f64)> = None;
        let mut max_dist_sq = radius.map(|r| r * r).unwrap_or(f64::INFINITY);

        self.find_recursive(
            self.root.as_ref().unwrap(),
            x,
            y,
            ext.x0,
            ext.y0,
            ext.x1,
            ext.y1,
            &mut best,
            &mut max_dist_sq,
        );

        best.map(|(data, _)| data)
    }

    fn find_recursive<'a>(
        &'a self,
        node: &'a QuadNode<T>,
        x: f64,
        y: f64,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        best: &mut Option<(&'a T, f64)>,
        max_dist_sq: &mut f64,
    ) {
        match node {
            QuadNode::Leaf(point) => {
                // Check this point and all coincident points
                let mut current = Some(point);
                while let Some(p) = current {
                    let dx = p.x - x;
                    let dy = p.y - y;
                    let dist_sq = dx * dx + dy * dy;

                    if dist_sq < *max_dist_sq {
                        *max_dist_sq = dist_sq;
                        *best = Some((&p.data, dist_sq));
                    }

                    current = p.next.as_deref();
                }
            }
            QuadNode::Internal(children) => {
                let xm = (x0 + x1) / 2.0;
                let ym = (y0 + y1) / 2.0;

                // Visit children in order of distance to query point
                let mut order = [(0, 0.0), (1, 0.0), (2, 0.0), (3, 0.0)];

                for (i, item) in order.iter_mut().enumerate() {
                    let (cx0, cy0, cx1, cy1) = Self::child_extent(i, x0, y0, x1, y1, xm, ym);
                    item.0 = i;
                    item.1 = Self::box_distance_sq(x, y, cx0, cy0, cx1, cy1);
                }

                order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                for (i, dist_sq) in order {
                    // Skip if this quadrant is farther than current best
                    if dist_sq >= *max_dist_sq {
                        continue;
                    }

                    if let Some(child) = &children[i] {
                        let (cx0, cy0, cx1, cy1) = Self::child_extent(i, x0, y0, x1, y1, xm, ym);
                        self.find_recursive(child, x, y, cx0, cy0, cx1, cy1, best, max_dist_sq);
                    }
                }
            }
        }
    }

    /// Compute squared distance from point to box
    fn box_distance_sq(x: f64, y: f64, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
        let dx = if x < x0 {
            x0 - x
        } else if x > x1 {
            x - x1
        } else {
            0.0
        };

        let dy = if y < y0 {
            y0 - y
        } else if y > y1 {
            y - y1
        } else {
            0.0
        };

        dx * dx + dy * dy
    }

    /// Visit each node in pre-order (node before children)
    ///
    /// The callback receives the node extent and a reference to the node.
    /// Return false to skip visiting children of this node.
    pub fn visit<F>(&self, mut callback: F)
    where
        F: FnMut(f64, f64, f64, f64, &QuadNode<T>) -> bool,
    {
        if let (Some(root), Some(ext)) = (&self.root, &self.extent) {
            self.visit_recursive(root, ext.x0, ext.y0, ext.x1, ext.y1, &mut callback);
        }
    }

    fn visit_recursive<F>(
        &self,
        node: &QuadNode<T>,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        callback: &mut F,
    ) where
        F: FnMut(f64, f64, f64, f64, &QuadNode<T>) -> bool,
    {
        if !callback(x0, y0, x1, y1, node) {
            return;
        }

        if let QuadNode::Internal(children) = node {
            let xm = (x0 + x1) / 2.0;
            let ym = (y0 + y1) / 2.0;

            for (i, child) in children.iter().enumerate() {
                if let Some(c) = child {
                    let (cx0, cy0, cx1, cy1) = Self::child_extent(i, x0, y0, x1, y1, xm, ym);
                    self.visit_recursive(c, cx0, cy0, cx1, cy1, callback);
                }
            }
        }
    }

    /// Visit each node in post-order (children before node)
    pub fn visit_after<F>(&self, mut callback: F)
    where
        F: FnMut(f64, f64, f64, f64, &QuadNode<T>),
    {
        if let (Some(root), Some(ext)) = (&self.root, &self.extent) {
            self.visit_after_recursive(root, ext.x0, ext.y0, ext.x1, ext.y1, &mut callback);
        }
    }

    fn visit_after_recursive<F>(
        &self,
        node: &QuadNode<T>,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        callback: &mut F,
    ) where
        F: FnMut(f64, f64, f64, f64, &QuadNode<T>),
    {
        if let QuadNode::Internal(children) = node {
            let xm = (x0 + x1) / 2.0;
            let ym = (y0 + y1) / 2.0;

            for (i, child) in children.iter().enumerate() {
                if let Some(c) = child {
                    let (cx0, cy0, cx1, cy1) = Self::child_extent(i, x0, y0, x1, y1, xm, ym);
                    self.visit_after_recursive(c, cx0, cy0, cx1, cy1, callback);
                }
            }
        }

        callback(x0, y0, x1, y1, node);
    }

    /// Get all data points as (x, y, data) tuples
    pub fn data(&self) -> Vec<(f64, f64, T)> {
        let mut result = Vec::with_capacity(self.size);

        self.visit(|_x0, _y0, _x1, _y1, node| {
            if let QuadNode::Leaf(point) = node {
                let mut current = Some(point);
                while let Some(p) = current {
                    result.push((p.x, p.y, p.data.clone()));
                    current = p.next.as_deref();
                }
            }
            true
        });

        result
    }

    /// Get the number of points in the tree
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the extent (bounding box) of the tree
    pub fn extent(&self) -> Option<Extent> {
        self.extent
    }

    /// Create a deep copy of the tree
    pub fn copy(&self) -> Self {
        self.clone()
    }

    /// Find all points within a radius of (x, y)
    pub fn find_all(&self, x: f64, y: f64, radius: f64) -> Vec<&T> {
        let mut result = Vec::new();
        let radius_sq = radius * radius;

        self.find_all_recursive(
            self.root.as_ref(),
            x,
            y,
            radius_sq,
            self.extent,
            &mut result,
        );

        result
    }

    fn find_all_recursive<'a>(
        &'a self,
        node: Option<&'a QuadNode<T>>,
        x: f64,
        y: f64,
        radius_sq: f64,
        extent: Option<Extent>,
        result: &mut Vec<&'a T>,
    ) {
        let Some(node) = node else { return };
        let Some(ext) = extent else { return };

        // Check if this box could contain points within radius
        if Self::box_distance_sq(x, y, ext.x0, ext.y0, ext.x1, ext.y1) > radius_sq {
            return;
        }

        match node {
            QuadNode::Leaf(point) => {
                let mut current = Some(point);
                while let Some(p) = current {
                    let dx = p.x - x;
                    let dy = p.y - y;
                    if dx * dx + dy * dy <= radius_sq {
                        result.push(&p.data);
                    }
                    current = p.next.as_deref();
                }
            }
            QuadNode::Internal(children) => {
                let xm = (ext.x0 + ext.x1) / 2.0;
                let ym = (ext.y0 + ext.y1) / 2.0;

                for (i, child) in children.iter().enumerate() {
                    if child.is_some() {
                        let (cx0, cy0, cx1, cy1) =
                            Self::child_extent(i, ext.x0, ext.y0, ext.x1, ext.y1, xm, ym);
                        self.find_all_recursive(
                            child.as_ref(),
                            x,
                            y,
                            radius_sq,
                            Some(Extent::new(cx0, cy0, cx1, cy1)),
                            result,
                        );
                    }
                }
            }
        }
    }

    /// Check if the tree is empty
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let tree: QuadTree<i32> = QuadTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.size(), 0);
        assert!(tree.extent().is_none());
        assert!(tree.find(0.0, 0.0, None).is_none());
    }

    #[test]
    fn test_single_point() {
        let mut tree = QuadTree::new();
        tree.add(1.0, 2.0, 42);

        assert_eq!(tree.size(), 1);
        assert!(!tree.is_empty());
        assert!(tree.extent().is_some());
        assert_eq!(tree.find(1.0, 2.0, None), Some(&42));
    }

    #[test]
    fn test_multiple_points() {
        let mut tree = QuadTree::new();
        tree.add(0.0, 0.0, "origin");
        tree.add(1.0, 0.0, "right");
        tree.add(0.0, 1.0, "up");
        tree.add(1.0, 1.0, "diagonal");

        assert_eq!(tree.size(), 4);

        // Find nearest to each point
        assert_eq!(tree.find(0.0, 0.0, None), Some(&"origin"));
        assert_eq!(tree.find(1.0, 0.0, None), Some(&"right"));
        assert_eq!(tree.find(0.0, 1.0, None), Some(&"up"));
        assert_eq!(tree.find(1.0, 1.0, None), Some(&"diagonal"));

        // Find nearest to intermediate point
        let nearest = tree.find(0.4, 0.4, None);
        assert_eq!(nearest, Some(&"origin")); // Closest to origin
    }

    #[test]
    fn test_find_with_radius() {
        let mut tree = QuadTree::new();
        tree.add(0.0, 0.0, "a");
        tree.add(5.0, 0.0, "b");
        tree.add(10.0, 0.0, "c");

        // Within radius
        assert_eq!(tree.find(0.0, 0.0, Some(1.0)), Some(&"a"));

        // Outside radius
        assert!(tree.find(0.0, 0.0, Some(0.1)).is_some()); // Still finds "a" at origin

        // Find with larger radius from middle
        assert_eq!(tree.find(5.0, 0.0, Some(1.0)), Some(&"b"));
    }

    #[test]
    fn test_find_all() {
        let mut tree = QuadTree::new();
        tree.add(0.0, 0.0, 1);
        tree.add(1.0, 0.0, 2);
        tree.add(0.0, 1.0, 3);
        tree.add(1.0, 1.0, 4);
        tree.add(10.0, 10.0, 5);

        let within_2 = tree.find_all(0.5, 0.5, 2.0);
        assert_eq!(within_2.len(), 4); // All except (10, 10)

        let within_15 = tree.find_all(0.0, 0.0, 15.0);
        assert_eq!(within_15.len(), 5); // All points
    }

    #[test]
    fn test_remove() {
        let mut tree = QuadTree::new();
        tree.add(0.0, 0.0, "a");
        tree.add(1.0, 1.0, "b");
        tree.add(2.0, 2.0, "c");

        assert_eq!(tree.size(), 3);

        assert!(tree.remove(1.0, 1.0));
        assert_eq!(tree.size(), 2);

        // Should not find removed point
        assert_ne!(tree.find(1.0, 1.0, None), Some(&"b"));

        // Other points still exist
        assert_eq!(tree.find(0.0, 0.0, None), Some(&"a"));
        assert_eq!(tree.find(2.0, 2.0, None), Some(&"c"));

        // Removing non-existent point
        assert!(!tree.remove(100.0, 100.0));
    }

    #[test]
    fn test_coincident_points() {
        let mut tree = QuadTree::new();
        tree.add(1.0, 1.0, "first");
        tree.add(1.0, 1.0, "second");
        tree.add(1.0, 1.0, "third");

        assert_eq!(tree.size(), 3);

        // Remove one coincident point
        assert!(tree.remove(1.0, 1.0));
        assert_eq!(tree.size(), 2);

        // Remove another
        assert!(tree.remove(1.0, 1.0));
        assert_eq!(tree.size(), 1);

        // Remove last
        assert!(tree.remove(1.0, 1.0));
        assert_eq!(tree.size(), 0);
    }

    #[test]
    fn test_visit() {
        let points = vec![(0.0, 0.0, 1), (1.0, 0.0, 2), (0.0, 1.0, 3), (1.0, 1.0, 4)];

        let tree = QuadTree::from_data(&points, |p| p.0, |p| p.1);

        let mut visited = 0;
        tree.visit(|_x0, _y0, _x1, _y1, _node| {
            visited += 1;
            true
        });

        assert!(visited > 0);
    }

    #[test]
    fn test_data() {
        let points = vec![(0.0, 0.0, 'a'), (1.0, 1.0, 'b'), (2.0, 2.0, 'c')];

        let tree = QuadTree::from_data(&points, |p| p.0, |p| p.1);
        let data = tree.data();

        assert_eq!(data.len(), 3);
    }

    #[test]
    fn test_extent() {
        let points = vec![(-10.0, -5.0, 1), (20.0, 15.0, 2)];

        let tree = QuadTree::from_data(&points, |p| p.0, |p| p.1);
        let ext = tree.extent().unwrap();

        assert!(ext.x0 <= -10.0);
        assert!(ext.y0 <= -5.0);
        assert!(ext.x1 >= 20.0);
        assert!(ext.y1 >= 15.0);
    }

    #[test]
    fn test_copy() {
        let points = vec![(0.0, 0.0, 1), (1.0, 1.0, 2)];
        let tree = QuadTree::from_data(&points, |p| p.0, |p| p.1);
        let copy = tree.copy();

        assert_eq!(copy.size(), tree.size());
        assert_eq!(copy.extent(), tree.extent());
    }

    #[test]
    fn test_nan_handling() {
        let mut tree = QuadTree::new();
        tree.add(0.0, 0.0, "valid");
        tree.add(f64::NAN, 0.0, "nan_x");
        tree.add(0.0, f64::NAN, "nan_y");
        tree.add(f64::INFINITY, 0.0, "inf");

        // Only valid point should be added
        assert_eq!(tree.size(), 1);
    }

    #[test]
    fn test_large_dataset() {
        let points: Vec<(f64, f64, i32)> = (0..1000)
            .map(|i| {
                let x = (i as f64 * 0.618033988749895).fract() * 100.0;
                let y = (i as f64 * 0.381966011250105).fract() * 100.0;
                (x, y, i)
            })
            .collect();

        let tree = QuadTree::from_data(&points, |p| p.0, |p| p.1);

        assert_eq!(tree.size(), 1000);

        // Verify we can find points
        for (x, y, _) in points.iter().take(10) {
            assert!(tree.find(*x, *y, Some(0.1)).is_some());
        }
    }
}
