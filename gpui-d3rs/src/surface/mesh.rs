//! Surface mesh triangulation and depth sorting

use super::data::{SurfaceData, SurfacePoint3D};
use super::projection::Projection;
use crate::color::D3Color;

/// A triangle in the surface mesh
#[derive(Clone, Debug)]
pub struct Triangle {
    /// The three vertices of the triangle
    pub vertices: [SurfacePoint3D; 3],
    /// Average t value for color mapping
    pub avg_t: f64,
    /// Centroid Z value for depth sorting
    pub centroid_z: f64,
    /// Computed color (set during color mapping)
    pub color: D3Color,
}

impl Triangle {
    /// Create a new triangle from three vertices
    pub fn new(v0: SurfacePoint3D, v1: SurfacePoint3D, v2: SurfacePoint3D) -> Self {
        let avg_t = (v0.t + v1.t + v2.t) / 3.0;
        let centroid_z = (v0.z + v1.z + v2.z) / 3.0;

        Self {
            vertices: [v0, v1, v2],
            avg_t,
            centroid_z,
            color: D3Color::rgb(128, 128, 128), // Default gray
        }
    }

    /// Get the centroid of the triangle
    pub fn centroid(&self) -> SurfacePoint3D {
        let [v0, v1, v2] = &self.vertices;
        SurfacePoint3D {
            x: (v0.x + v1.x + v2.x) / 3.0,
            y: (v0.y + v1.y + v2.y) / 3.0,
            z: (v0.z + v1.z + v2.z) / 3.0,
            t: (v0.t + v1.t + v2.t) / 3.0,
        }
    }

    /// Compute the normal vector of the triangle (for lighting)
    pub fn normal(&self) -> (f64, f64, f64) {
        let [v0, v1, v2] = &self.vertices;

        // Edge vectors
        let e1 = (v1.x - v0.x, v1.y - v0.y, v1.z - v0.z);
        let e2 = (v2.x - v0.x, v2.y - v0.y, v2.z - v0.z);

        // Cross product
        let nx = e1.1 * e2.2 - e1.2 * e2.1;
        let ny = e1.2 * e2.0 - e1.0 * e2.2;
        let nz = e1.0 * e2.1 - e1.1 * e2.0;

        // Normalize
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        if len > 1e-10 {
            (nx / len, ny / len, nz / len)
        } else {
            (0.0, 0.0, 1.0) // Default to up if degenerate
        }
    }
}

/// A mesh of triangles representing a surface
#[derive(Clone, Debug)]
pub struct SurfaceMesh {
    /// All triangles in the mesh
    pub triangles: Vec<Triangle>,
}

impl SurfaceMesh {
    /// Create an empty mesh
    pub fn new() -> Self {
        Self {
            triangles: Vec::new(),
        }
    }

    /// Create a mesh from surface data by triangulating the grid
    ///
    /// Each grid cell is divided into two triangles.
    pub fn from_surface_data(data: &SurfaceData) -> Self {
        let mut triangles = Vec::new();
        let points = data.points();

        if points.is_empty() || points[0].is_empty() {
            return Self::new();
        }

        let rows = points.len();
        let cols = points[0].len();

        // For each cell in the grid, create two triangles
        for j in 0..rows - 1 {
            for i in 0..cols - 1 {
                let p00 = points[j][i];
                let p10 = points[j][i + 1];
                let p01 = points[j + 1][i];
                let p11 = points[j + 1][i + 1];

                // First triangle: p00, p10, p01
                triangles.push(Triangle::new(p00, p10, p01));

                // Second triangle: p10, p11, p01
                triangles.push(Triangle::new(p10, p11, p01));
            }
        }

        Self { triangles }
    }

    /// Sort triangles by depth using painter's algorithm
    ///
    /// Triangles are sorted back-to-front based on the projection's depth function.
    pub fn depth_sort<P: Projection>(&mut self, projection: &P) {
        self.triangles.sort_by(|a, b| {
            let centroid_a = a.centroid();
            let centroid_b = b.centroid();

            let depth_a = projection.point_depth(&centroid_a);
            let depth_b = projection.point_depth(&centroid_b);

            // Sort by depth: larger depth (further) should come first
            depth_b
                .partial_cmp(&depth_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Apply a color scale to all triangles based on their t values
    ///
    /// The color function takes a normalized t value in [0, 1] and returns a color.
    pub fn apply_color_scale<F>(&mut self, t_range: (f64, f64), color_fn: F)
    where
        F: Fn(f64) -> D3Color,
    {
        let t_min = t_range.0;
        let t_scale = t_range.1 - t_range.0;

        for triangle in &mut self.triangles {
            let normalized_t = if t_scale.abs() > 1e-10 {
                (triangle.avg_t - t_min) / t_scale
            } else {
                0.5
            };
            let clamped_t = normalized_t.clamp(0.0, 1.0);
            triangle.color = color_fn(clamped_t);
        }
    }

    /// Apply simple ambient + diffuse lighting to triangle colors
    ///
    /// Light direction should be normalized.
    pub fn apply_lighting(&mut self, light_dir: (f64, f64, f64), ambient: f64, diffuse: f64) {
        for triangle in &mut self.triangles {
            let normal = triangle.normal();

            // Dot product with light direction (ensure positive for surfaces facing light)
            let dot = (normal.0 * light_dir.0 + normal.1 * light_dir.1 + normal.2 * light_dir.2)
                .abs()
                .max(0.0);

            let light_factor = ambient + diffuse * dot;
            let light_factor = light_factor.clamp(0.0, 1.0);

            // Apply lighting to color
            triangle.color = D3Color {
                r: (triangle.color.r * light_factor as f32).clamp(0.0, 1.0),
                g: (triangle.color.g * light_factor as f32).clamp(0.0, 1.0),
                b: (triangle.color.b * light_factor as f32).clamp(0.0, 1.0),
                a: triangle.color.a,
            };
        }
    }

    /// Get the number of triangles
    pub fn len(&self) -> usize {
        self.triangles.len()
    }

    /// Check if mesh is empty
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }
}

impl Default for SurfaceMesh {
    fn default() -> Self {
        Self::new()
    }
}

/// Triangulate a grid of points into triangles
///
/// This is a utility function that can be used independently of SurfaceMesh.
pub fn _triangulate_grid(data: &SurfaceData) -> Vec<Triangle> {
    SurfaceMesh::from_surface_data(data).triangles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triangle_creation() {
        let v0 = SurfacePoint3D::new(0.0, 0.0, 0.0, 0.0);
        let v1 = SurfacePoint3D::new(1.0, 0.0, 0.0, 0.5);
        let v2 = SurfacePoint3D::new(0.0, 1.0, 0.0, 1.0);

        let tri = Triangle::new(v0, v1, v2);

        assert!((tri.avg_t - 0.5).abs() < 1e-10);
        assert!((tri.centroid_z - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_triangle_centroid() {
        let v0 = SurfacePoint3D::new(0.0, 0.0, 0.0, 0.0);
        let v1 = SurfacePoint3D::new(3.0, 0.0, 0.0, 0.0);
        let v2 = SurfacePoint3D::new(0.0, 3.0, 0.0, 0.0);

        let tri = Triangle::new(v0, v1, v2);
        let centroid = tri.centroid();

        assert!((centroid.x - 1.0).abs() < 1e-10);
        assert!((centroid.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_triangle_normal() {
        // Triangle in XY plane
        let v0 = SurfacePoint3D::new(0.0, 0.0, 0.0, 0.0);
        let v1 = SurfacePoint3D::new(1.0, 0.0, 0.0, 0.0);
        let v2 = SurfacePoint3D::new(0.0, 1.0, 0.0, 0.0);

        let tri = Triangle::new(v0, v1, v2);
        let normal = tri.normal();

        // Normal should point in Z direction
        assert!(normal.0.abs() < 1e-10);
        assert!(normal.1.abs() < 1e-10);
        assert!((normal.2.abs() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_mesh_from_surface_data() {
        let data = SurfaceData::from_z_function((0.0, 1.0), (0.0, 1.0), 3, |x, y| x + y);

        let mesh = SurfaceMesh::from_surface_data(&data);

        // 3x3 grid has 2x2 = 4 cells, each with 2 triangles = 8 triangles
        assert_eq!(mesh.len(), 8);
    }

    #[test]
    fn test_apply_color_scale() {
        let data = SurfaceData::from_function((0.0, 1.0), (0.0, 1.0), 3, |x, y| {
            let t = x + y; // Range 0 to 2
            (0.0, t)
        });

        let mut mesh = SurfaceMesh::from_surface_data(&data);
        mesh.apply_color_scale(data.t_range, |t| D3Color::rgb((t * 255.0) as u8, 0, 0));

        // All triangles should now have colors
        for tri in &mesh.triangles {
            assert!(tri.color.r >= 0.0 && tri.color.r <= 1.0);
        }
    }

    #[test]
    fn test_empty_mesh() {
        let mesh = SurfaceMesh::new();
        assert!(mesh.is_empty());
        assert_eq!(mesh.len(), 0);
    }
}
