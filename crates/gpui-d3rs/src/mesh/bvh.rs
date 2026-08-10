//! Feature-independent acceleration structure for 3D mesh picking.
//!
//! The tree is built with a 16-bin centroid split and traversed iteratively.
//! All intersection arithmetic remains in `f64`, including the returned ray
//! parameter and barycentric coordinates.

use super::TriangleMesh;
use std::sync::Arc;

const BIN_COUNT: usize = 16;
const LEAF_SIZE: usize = 4;
const EPSILON: f64 = 1e-12;

#[derive(Debug, Clone, Copy)]
struct Aabb {
    min: [f64; 3],
    max: [f64; 3],
}

impl Aabb {
    fn empty() -> Self {
        Self {
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
        }
    }

    fn include_point(&mut self, point: [f64; 3]) {
        for axis in 0..3 {
            self.min[axis] = self.min[axis].min(point[axis]);
            self.max[axis] = self.max[axis].max(point[axis]);
        }
    }

    fn include_aabb(&mut self, other: Self) {
        self.include_point(other.min);
        self.include_point(other.max);
    }

    fn extent(&self) -> [f64; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    fn centroid(&self) -> [f64; 3] {
        [
            0.5 * (self.min[0] + self.max[0]),
            0.5 * (self.min[1] + self.max[1]),
            0.5 * (self.min[2] + self.max[2]),
        ]
    }

    fn hit(&self, origin: [f64; 3], direction: [f64; 3], max_t: f64) -> bool {
        let mut near: f64 = 0.0;
        let mut far: f64 = max_t;
        for axis in 0..3 {
            let d = direction[axis];
            if d.abs() <= f64::EPSILON {
                if origin[axis] < self.min[axis] || origin[axis] > self.max[axis] {
                    return false;
                }
                continue;
            }
            let inverse = 1.0 / d;
            let mut t0 = (self.min[axis] - origin[axis]) * inverse;
            let mut t1 = (self.max[axis] - origin[axis]) * inverse;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            near = near.max(t0);
            far = far.min(t1);
            if near > far {
                return false;
            }
        }
        far >= near.max(0.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct Node {
    bounds: Aabb,
    left: u32,
    right: u32,
    start: u32,
    count: u32,
}

impl Node {
    fn leaf(bounds: Aabb, start: usize, count: usize) -> Self {
        Self {
            bounds,
            left: u32::MAX,
            right: u32::MAX,
            start: start as u32,
            count: count as u32,
        }
    }

    fn branch(bounds: Aabb, left: usize, right: usize) -> Self {
        Self {
            bounds,
            left: left as u32,
            right: right as u32,
            start: 0,
            count: 0,
        }
    }

    fn is_leaf(self) -> bool {
        self.left == u32::MAX
    }
}

/// A binned bounding-volume hierarchy for a [`TriangleMesh`].
#[derive(Debug, Clone)]
pub struct MeshBvh {
    positions: Arc<[[f64; 3]]>,
    triangles: Arc<[[u32; 3]]>,
    triangle_indices: Vec<u32>,
    nodes: Vec<Node>,
}

impl MeshBvh {
    /// Build a deterministic 16-bin BVH. Triangle order is preserved inside
    /// leaves, so equal-distance hits have a stable tie-break.
    pub fn build(mesh: &TriangleMesh) -> Self {
        let mut bvh = Self {
            positions: mesh.positions.clone(),
            triangles: mesh.triangles.clone(),
            triangle_indices: (0..mesh.triangles.len() as u32).collect(),
            nodes: Vec::new(),
        };
        if !bvh.triangle_indices.is_empty() {
            let indices = bvh.triangle_indices.clone();
            bvh.build_node(&indices);
        }
        bvh
    }

    /// Intersect a ray and return the nearest triangle, ray parameter, and
    /// barycentric coordinates `[weight_a, weight_b, weight_c]`.
    pub fn ray_cast(&self, origin: [f64; 3], direction: [f64; 3]) -> Option<(u32, f64, [f64; 3])> {
        self.nodes.first()?;
        let mut stack = vec![0u32];
        let mut best: Option<(u32, f64, [f64; 3])> = None;

        while let Some(node_index) = stack.pop() {
            let node = self.nodes[node_index as usize];
            let max_t = best.map_or(f64::INFINITY, |(_, t, _)| t);
            if !node.bounds.hit(origin, direction, max_t) {
                continue;
            }
            if node.is_leaf() {
                let end = node.start as usize + node.count as usize;
                for &triangle_index in &self.triangle_indices[node.start as usize..end] {
                    let Some(triangle) = self.triangles.get(triangle_index as usize) else {
                        continue;
                    };
                    let Some(&a) = self.positions.get(triangle[0] as usize) else {
                        continue;
                    };
                    let Some(&b) = self.positions.get(triangle[1] as usize) else {
                        continue;
                    };
                    let Some(&c) = self.positions.get(triangle[2] as usize) else {
                        continue;
                    };
                    if let Some((t, barycentric)) = intersect_triangle(origin, direction, a, b, c)
                        && t < best.map_or(f64::INFINITY, |(_, best_t, _)| best_t)
                    {
                        best = Some((triangle_index, t, barycentric));
                    }
                }
            } else {
                // Push both children. Ordering by entry distance improves
                // pruning while keeping traversal iterative and deterministic.
                let children = [node.left, node.right];
                let mut hits = children
                    .into_iter()
                    .filter_map(|child| {
                        let child_node = self.nodes[child as usize];
                        ray_aabb_entry(&child_node.bounds, origin, direction)
                            .map(|entry| (entry, child))
                    })
                    .collect::<Vec<_>>();
                hits.sort_by(|a, b| b.0.total_cmp(&a.0));
                for (_, child) in hits {
                    stack.push(child);
                }
            }
        }

        best
    }

    fn build_node(&mut self, indices: &[u32]) -> usize {
        let node_index = self.nodes.len();
        self.nodes.push(Node::leaf(Aabb::empty(), 0, 0));

        let bounds = self.bounds_for(indices);
        if indices.len() <= LEAF_SIZE {
            let start = self.triangle_indices.len();
            self.triangle_indices.extend_from_slice(indices);
            self.nodes[node_index] = Node::leaf(bounds, start, indices.len());
            return node_index;
        }

        let centroid_bounds = self.centroid_bounds_for(indices);
        let extent = centroid_bounds.extent();
        let axis = (0..3)
            .max_by(|&a, &b| extent[a].total_cmp(&extent[b]))
            .unwrap_or(0);
        let split = partition_binned(
            indices,
            axis,
            centroid_bounds.min[axis],
            centroid_bounds.max[axis],
            &self.triangles,
            &self.positions,
        );
        let (left_indices, right_indices) = match split {
            Some((left, right)) if !left.is_empty() && !right.is_empty() => (left, right),
            _ => {
                let mut sorted = indices.to_vec();
                sorted.sort_by(|&a, &b| {
                    let ca = triangle_centroid(&self.positions, &self.triangles[a as usize])[axis];
                    let cb = triangle_centroid(&self.positions, &self.triangles[b as usize])[axis];
                    ca.total_cmp(&cb).then_with(|| a.cmp(&b))
                });
                let middle = sorted.len() / 2;
                (sorted[..middle].to_vec(), sorted[middle..].to_vec())
            }
        };

        let left = self.build_node(&left_indices);
        let right = self.build_node(&right_indices);
        self.nodes[node_index] = Node::branch(bounds, left, right);
        node_index
    }

    fn bounds_for(&self, indices: &[u32]) -> Aabb {
        let mut bounds = Aabb::empty();
        for &triangle_index in indices {
            let Some(triangle) = self.triangles.get(triangle_index as usize) else {
                continue;
            };
            for &vertex in triangle {
                if let Some(&point) = self.positions.get(vertex as usize) {
                    bounds.include_point(point);
                }
            }
        }
        bounds
    }

    fn centroid_bounds_for(&self, indices: &[u32]) -> Aabb {
        let mut bounds = Aabb::empty();
        for &triangle_index in indices {
            bounds.include_point(triangle_centroid(
                &self.positions,
                &self.triangles[triangle_index as usize],
            ));
        }
        bounds
    }
}

fn triangle_centroid(positions: &[[f64; 3]], triangle: &[u32; 3]) -> [f64; 3] {
    let a = positions
        .get(triangle[0] as usize)
        .copied()
        .unwrap_or([0.0; 3]);
    let b = positions.get(triangle[1] as usize).copied().unwrap_or(a);
    let c = positions.get(triangle[2] as usize).copied().unwrap_or(a);
    [
        (a[0] + b[0] + c[0]) / 3.0,
        (a[1] + b[1] + c[1]) / 3.0,
        (a[2] + b[2] + c[2]) / 3.0,
    ]
}

fn partition_binned(
    indices: &[u32],
    axis: usize,
    min: f64,
    max: f64,
    triangles: &[[u32; 3]],
    positions: &[[f64; 3]],
) -> Option<(Vec<u32>, Vec<u32>)> {
    let extent = max - min;
    if !extent.is_finite() || extent <= f64::EPSILON {
        return None;
    }
    let mut bins = [0usize; BIN_COUNT];
    for &index in indices {
        let value = triangle_centroid(positions, &triangles[index as usize])[axis];
        let bin = (((value - min) / extent) * BIN_COUNT as f64)
            .floor()
            .clamp(0.0, (BIN_COUNT - 1) as f64) as usize;
        bins[bin] += 1;
    }
    let half = indices.len().div_ceil(2);
    let mut cumulative = 0;
    let mut split_bin = None;
    for (bin, &count) in bins.iter().enumerate() {
        cumulative += count;
        if cumulative >= half {
            split_bin = Some(bin);
            break;
        }
    }
    let split_bin = split_bin?;
    let mut left = Vec::with_capacity(half);
    let mut right = Vec::with_capacity(indices.len() - half);
    for &index in indices {
        let value = triangle_centroid(positions, &triangles[index as usize])[axis];
        let bin = (((value - min) / extent) * BIN_COUNT as f64)
            .floor()
            .clamp(0.0, (BIN_COUNT - 1) as f64) as usize;
        if bin <= split_bin && left.len() < half {
            left.push(index);
        } else {
            right.push(index);
        }
    }
    Some((left, right))
}

fn ray_aabb_entry(bounds: &Aabb, origin: [f64; 3], direction: [f64; 3]) -> Option<f64> {
    if !bounds.hit(origin, direction, f64::INFINITY) {
        return None;
    }
    let mut near: f64 = 0.0;
    for axis in 0..3 {
        if direction[axis].abs() > f64::EPSILON {
            let mut t0 = (bounds.min[axis] - origin[axis]) / direction[axis];
            let mut t1 = (bounds.max[axis] - origin[axis]) / direction[axis];
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            near = near.max(t0);
        }
    }
    Some(near.max(0.0))
}

fn intersect_triangle(
    origin: [f64; 3],
    direction: [f64; 3],
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
) -> Option<(f64, [f64; 3])> {
    let edge1 = sub(b, a);
    let edge2 = sub(c, a);
    let pvec = cross(direction, edge2);
    let determinant = dot(edge1, pvec);
    if determinant.abs() <= EPSILON {
        return None;
    }
    let inverse = 1.0 / determinant;
    let tvec = sub(origin, a);
    let u = dot(tvec, pvec) * inverse;
    if !(-EPSILON..=1.0 + EPSILON).contains(&u) {
        return None;
    }
    let qvec = cross(tvec, edge1);
    let v = dot(direction, qvec) * inverse;
    if v < -EPSILON || u + v > 1.0 + EPSILON {
        return None;
    }
    let t = dot(edge2, qvec) * inverse;
    if t < EPSILON {
        return None;
    }
    Some((t, [1.0 - u - v, u, v]))
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(triangles: &[[u32; 3]], positions: &[[f64; 3]]) -> TriangleMesh {
        TriangleMesh {
            id: "bvh".into(),
            positions: Arc::from(positions),
            triangles: Arc::from(triangles),
            vertex_ids: None,
            cell_ids: None,
        }
    }

    #[test]
    fn known_hit_returns_triangle_and_barycentric_coordinates() {
        let mesh = mesh(
            &[[0, 1, 2]],
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        );
        let hit = MeshBvh::build(&mesh)
            .ray_cast([0.25, 0.25, 1.0], [0.0, 0.0, -1.0])
            .unwrap();
        assert_eq!(hit.0, 0);
        assert!((hit.1 - 1.0).abs() < 1e-12);
        assert!((hit.2.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert!(hit.2.iter().all(|value| *value >= -EPSILON));
    }

    #[test]
    fn miss_returns_none() {
        let mesh = mesh(
            &[[0, 1, 2]],
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        );
        assert!(
            MeshBvh::build(&mesh)
                .ray_cast([2.0, 2.0, 1.0], [0.0, 0.0, -1.0])
                .is_none()
        );
    }

    #[test]
    fn nearest_triangle_wins() {
        let mesh = mesh(
            &[[0, 1, 2], [3, 4, 5]],
            &[
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 2.0],
                [1.0, 0.0, 2.0],
                [0.0, 1.0, 2.0],
            ],
        );
        let hit = MeshBvh::build(&mesh)
            .ray_cast([0.2, 0.2, 3.0], [0.0, 0.0, -1.0])
            .unwrap();
        assert_eq!(hit.0, 1);
        assert!((hit.1 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn scale_sanity_preserves_hit_parameter() {
        let mesh = mesh(
            &[[0, 1, 2]],
            &[
                [0.0, 0.0, 0.0],
                [1_000_000.0, 0.0, 0.0],
                [0.0, 1_000_000.0, 0.0],
            ],
        );
        let hit = MeshBvh::build(&mesh)
            .ray_cast([250_000.0, 250_000.0, 10.0], [0.0, 0.0, -1.0])
            .unwrap();
        assert!((hit.1 - 10.0).abs() < 1e-10);
    }
}
