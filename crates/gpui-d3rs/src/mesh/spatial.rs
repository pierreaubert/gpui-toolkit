//! Bounds, 2D projection, barycentric hit testing, and a uniform-grid
//! spatial index for triangle picking.

/// Which mesh coordinate feeds a 2D plot axis (spec §5, MeshPlotView).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoordinateAxis {
    X,
    Y,
    Z,
}

impl CoordinateAxis {
    pub fn component(self, p: [f64; 3]) -> f64 {
        match self {
            Self::X => p[0],
            Self::Y => p[1],
            Self::Z => p[2],
        }
    }
}

/// Project a 3D position to 2D plot coordinates.
pub fn project_2d(horizontal: CoordinateAxis, vertical: CoordinateAxis, p: [f64; 3]) -> [f64; 2] {
    [horizontal.component(p), vertical.component(p)]
}

/// Axis-aligned bounds in mesh coordinates, with the double-precision
/// origin used for f32 GPU rebasing (design §3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshBounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl MeshBounds {
    pub fn from_positions(positions: &[[f64; 3]]) -> Self {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for p in positions {
            for i in 0..3 {
                min[i] = min[i].min(p[i]);
                max[i] = max[i].max(p[i]);
            }
        }
        Self { min, max }
    }

    pub fn origin(&self) -> [f64; 3] {
        [
            0.5 * (self.min[0] + self.max[0]),
            0.5 * (self.min[1] + self.max[1]),
            0.5 * (self.min[2] + self.max[2]),
        ]
    }
}

/// Barycentric weights of `p` in triangle (a,b,c); None when outside.
/// Tolerance: points within 1e-12 of an edge count as inside (deterministic
/// tie-break, documented in module docs).
pub fn barycentric_2d(p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Option<[f64; 3]> {
    let v0 = [b[0] - a[0], b[1] - a[1]];
    let v1 = [c[0] - a[0], c[1] - a[1]];
    let v2 = [p[0] - a[0], p[1] - a[1]];
    let d00 = v0[0] * v0[0] + v0[1] * v0[1];
    let d01 = v0[0] * v1[0] + v0[1] * v1[1];
    let d11 = v1[0] * v1[0] + v1[1] * v1[1];
    let d20 = v2[0] * v0[0] + v2[1] * v0[1];
    let d21 = v2[0] * v1[0] + v2[1] * v1[1];
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < f64::EPSILON {
        return None;
    }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;
    const TOL: f64 = -1e-12;
    if u > TOL && v > TOL && w > TOL {
        Some([u, v, w])
    } else {
        None
    }
}

/// Uniform-grid spatial index over 2D-projected triangles for picking.
/// Cell size = max(domain extent) / 64, clamped; deterministic iteration
/// (triangle indices sorted).
#[derive(Debug, Clone)]
pub struct TriGridIndex {
    min: [f64; 2],
    inv_cell: [f64; 2],
    dims: [usize; 2],
    cells: Vec<Vec<u32>>,
}

impl TriGridIndex {
    pub fn build(positions: &[[f64; 2]], triangles: &[[u32; 3]]) -> Self {
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        for p in positions {
            min[0] = min[0].min(p[0]);
            min[1] = min[1].min(p[1]);
            max[0] = max[0].max(p[0]);
            max[1] = max[1].max(p[1]);
        }
        let extent = [(max[0] - min[0]).max(1e-12), (max[1] - min[1]).max(1e-12)];
        let n = (triangles.len() as f64).sqrt().clamp(8.0, 128.0);
        let dims = [n as usize, n as usize];
        let cell = [extent[0] / dims[0] as f64, extent[1] / dims[1] as f64];
        let inv_cell = [1.0 / cell[0], 1.0 / cell[1]];
        let mut cells = vec![Vec::new(); dims[0] * dims[1]];
        for (t, tri) in triangles.iter().enumerate() {
            // rasterize the triangle's AABB into overlapped cells
            let pts = [
                positions[tri[0] as usize],
                positions[tri[1] as usize],
                positions[tri[2] as usize],
            ];
            let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
            for p in pts {
                lo[0] = lo[0].min(p[0]);
                lo[1] = lo[1].min(p[1]);
                hi[0] = hi[0].max(p[0]);
                hi[1] = hi[1].max(p[1]);
            }
            let c0 = [
                ((lo[0] - min[0]) * inv_cell[0]) as usize,
                ((lo[1] - min[1]) * inv_cell[1]) as usize,
            ];
            let c1 = [
                (((hi[0] - min[0]) * inv_cell[0]) as usize).min(dims[0] - 1),
                (((hi[1] - min[1]) * inv_cell[1]) as usize).min(dims[1] - 1),
            ];
            for cy in c0[1]..=c1[1] {
                for cx in c0[0]..=c1[0] {
                    cells[cy * dims[0] + cx].push(t as u32);
                }
            }
        }
        Self { min, inv_cell, dims, cells }
    }

    /// Candidate triangles under `p`, sorted for determinism.
    pub fn query(&self, p: [f64; 2]) -> Vec<u32> {
        let cx = ((p[0] - self.min[0]) * self.inv_cell[0]) as isize;
        let cy = ((p[1] - self.min[1]) * self.inv_cell[1]) as isize;
        if cx < 0 || cy < 0 || cx >= self.dims[0] as isize || cy >= self.dims[1] as isize {
            return Vec::new();
        }
        let mut out = self.cells[cy as usize * self.dims[0] + cx as usize].clone();
        out.sort_unstable();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_and_origin() {
        let b = MeshBounds::from_positions(&[[-2.0, 0.0, 1.0], [4.0, 6.0, -3.0]]);
        assert_eq!(b.min, [-2.0, 0.0, -3.0]);
        assert_eq!(b.max, [4.0, 6.0, 1.0]);
        assert_eq!(b.origin(), [1.0, 3.0, -1.0]);
    }

    #[test]
    fn barycentric_inside_triangle() {
        let w = barycentric_2d([0.25, 0.25], [0.0, 0.0], [1.0, 0.0], [0.0, 1.0]).unwrap();
        assert!((w[0] - 0.5).abs() < 1e-12 && (w[1] - 0.25).abs() < 1e-12 && (w[2] - 0.25).abs() < 1e-12);
    }

    #[test]
    fn barycentric_outside_returns_none() {
        assert!(barycentric_2d([1.5, 1.5], [0.0, 0.0], [1.0, 0.0], [0.0, 1.0]).is_none());
    }

    #[test]
    fn grid_index_finds_known_triangle() {
        let positions: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [10.0, 10.0], [11.0, 10.0], [10.0, 11.0]];
        let triangles: Vec<[u32; 3]> = vec![[0, 1, 2], [3, 4, 5]];
        let idx = TriGridIndex::build(&positions, &triangles);
        let hit = idx.query([0.2, 0.2]);
        assert_eq!(hit, vec![0]);
        let far = idx.query([10.5, 10.5]);
        assert_eq!(far, vec![1]);
    }

    #[test]
    fn projection_picks_axes() {
        let p = [1.0, 2.0, 3.0];
        assert_eq!(project_2d(CoordinateAxis::X, CoordinateAxis::Y, p), [1.0, 2.0]);
        assert_eq!(project_2d(CoordinateAxis::Z, CoordinateAxis::X, p), [3.0, 1.0]);
    }
}
