//! Sphere mesh generation for gallery grid
//!
//! Maps a 2D grid of cells onto a curved 3D surface using configurable map projections.
//! The center of the grid is the apex (highest point), and edges slope down,
//! creating a "halo" or dome effect.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// GPU vertex for sphere gallery (must match shader layout)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GalleryVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub cell_index: f32,
    pub _padding: [f32; 3], // Align to 48 bytes (12 floats)
}

impl GalleryVertex {
    pub fn new(position: Vec3, normal: Vec3, uv: [f32; 2], cell_index: f32) -> Self {
        Self {
            position: position.to_array(),
            normal: normal.to_array(),
            uv,
            cell_index,
            _padding: [0.0; 3],
        }
    }
}

/// Map projection type for the gallery dome surface.
///
/// Each projection maps the flat grid differently onto the curved surface,
/// trading off between area preservation, angle preservation, and visual aesthetics.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Projection {
    /// Linear mapping from distance to polar angle. Uniform angular spacing.
    /// Simple and predictable — cells stretch slightly at edges.
    Equirectangular,

    /// Conformal (angle-preserving) projection. Cells at the center appear
    /// slightly larger; edges are compressed but shapes are preserved.
    /// Good default for a natural-looking dome.
    #[default]
    Stereographic,

    /// Equal-area projection. All cells occupy roughly the same area on the
    /// surface, regardless of position. Good when uniform visual weight matters.
    LambertEqualArea,

    /// Parallel-ray projection. Heavy compression at edges — cells near the
    /// rim appear very thin. Creates a dramatic "looking down at a globe" effect.
    Orthographic,

    /// Inverse Mercator. Stretches cells near the center vertically, compresses
    /// edges. Recognizable "Mercator" distortion pattern.
    Mercator,

    /// Not a cartographic projection. Simple cosine height displacement on a
    /// flat grid: `y = apex_height * cos(π/2 * r)`. Cells remain rectangular
    /// (no horizontal distortion), only the surface bulges upward.
    Cosine,

    /// Cylinder wrap — horizontal curvature only (around the X axis).
    /// Rows curve; columns stay straight. Like wrapping the grid around a drum.
    Cylindrical,
}

impl Projection {
    /// All available projections, for cycling through them in demos.
    pub const ALL: &[Projection] = &[
        Projection::Equirectangular,
        Projection::Stereographic,
        Projection::LambertEqualArea,
        Projection::Orthographic,
        Projection::Mercator,
        Projection::Cosine,
        Projection::Cylindrical,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Equirectangular => "Equirectangular",
            Self::Stereographic => "Stereographic",
            Self::LambertEqualArea => "Lambert Equal-Area",
            Self::Orthographic => "Orthographic",
            Self::Mercator => "Mercator",
            Self::Cosine => "Cosine Dome",
            Self::Cylindrical => "Cylindrical",
        }
    }
}

/// Configuration for sphere mesh generation
#[derive(Debug, Clone)]
pub struct SphereMeshConfig {
    /// Sphere radius (for sphere-based projections)
    pub radius: f32,
    /// Height of the center above the edge plane.
    ///
    /// - `0.0` = completely flat
    /// - `0.5` = moderate dome (~60° cap) — **default**
    /// - `1.0` = hemisphere (90°)
    /// - `2.0` = full sphere (180°, not recommended)
    ///
    /// Internally converted to `max_angle = acos(1 - apex_height)` for sphere
    /// projections, or used directly as the height multiplier for `Cosine`.
    pub apex_height: f32,
    /// Map projection to use
    pub projection: Projection,
    /// Number of subdivisions per cell edge (higher = smoother curvature per cell)
    pub subdivisions: u32,
}

impl Default for SphereMeshConfig {
    fn default() -> Self {
        Self {
            radius: 1.0,
            apex_height: 0.5,
            projection: Projection::default(),
            subdivisions: 4,
        }
    }
}

impl SphereMeshConfig {
    /// Compute the max angle from apex_height (for sphere-based projections).
    ///
    /// `apex_height = R * (1 - cos(max_angle))` when R = radius.
    /// Solving: `max_angle = acos(1 - apex_height / radius)`.
    fn max_angle(&self) -> f32 {
        let h = (self.apex_height / self.radius).clamp(0.0, 2.0);
        (1.0 - h).acos()
    }
}

/// Generated sphere gallery mesh
pub struct SphereGalleryMesh {
    pub vertices: Vec<GalleryVertex>,
    pub indices: Vec<u32>,
}

impl SphereGalleryMesh {
    pub fn vertex_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.vertices)
    }

    pub fn index_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.indices)
    }
}

// ---------------------------------------------------------------------------
// Projection functions
//
// Each takes (r, theta) where r ∈ [0, 1] is normalized distance from center
// and theta is the azimuth, plus the config. Returns a 3D position.
// ---------------------------------------------------------------------------

/// Map normalized radius `r` to the polar angle `phi` based on the projection.
fn project_r_to_phi(r: f32, max_angle: f32, projection: Projection) -> f32 {
    match projection {
        // Linear: phi = r * max_angle
        Projection::Equirectangular => r * max_angle,

        // Stereographic: phi = 2 * atan(r * tan(max_angle / 2))
        Projection::Stereographic => 2.0 * (r * (max_angle / 2.0).tan()).atan(),

        // Lambert equal-area: phi = 2 * asin(r * sin(max_angle / 2))
        Projection::LambertEqualArea => {
            let arg = (r * (max_angle / 2.0).sin()).clamp(-1.0, 1.0);
            2.0 * arg.asin()
        }

        // Orthographic: phi = asin(r * sin(max_angle))
        Projection::Orthographic => {
            let arg = (r * max_angle.sin()).clamp(-1.0, 1.0);
            arg.asin()
        }

        // Mercator: inverse of r = ln(tan(π/4 + φ/2)) / ln(tan(π/4 + max_angle/2))
        // => phi = 2 * (atan(exp(r * ln(tan(π/4 + max_angle/2)))) - π/4)
        Projection::Mercator => {
            let quarter_pi = std::f32::consts::FRAC_PI_4;
            let denom = (quarter_pi + max_angle / 2.0).tan().ln();
            if denom.abs() < 1e-6 {
                return r * max_angle; // Fallback to linear for tiny angles
            }
            2.0 * ((r * denom).exp().atan() - quarter_pi)
        }

        // Cosine and Cylindrical are handled separately (not polar-angle based)
        Projection::Cosine | Projection::Cylindrical => {
            // Not used for these projections
            r * max_angle
        }
    }
}

/// Map a 2D point (u, v) in [-1, 1] to a 3D position on the dome surface.
fn grid_to_surface(u: f32, v: f32, config: &SphereMeshConfig) -> Vec3 {
    let max_angle = config.max_angle();

    match config.projection {
        // Cosine dome: flat grid with height displacement
        Projection::Cosine => {
            let r = (u * u + v * v).sqrt().min(1.0);
            let y = config.apex_height * (std::f32::consts::FRAC_PI_2 * r).cos();
            Vec3::new(u * config.radius, y, v * config.radius)
        }

        // Cylindrical: wrap around a horizontal cylinder
        Projection::Cylindrical => {
            // Only the v-axis wraps; u stays flat
            let angle = v * max_angle;
            let y = config.radius * angle.cos();
            let z = config.radius * angle.sin();
            Vec3::new(u * config.radius, y, z)
        }

        // All sphere-based projections
        _ => {
            let r = (u * u + v * v).sqrt().min(1.0);
            let theta = v.atan2(u);
            let phi = project_r_to_phi(r, max_angle, config.projection);

            let x = config.radius * phi.sin() * theta.cos();
            let y = config.radius * phi.cos();
            let z = config.radius * phi.sin() * theta.sin();

            Vec3::new(x, y, z)
        }
    }
}

/// Compute a surface normal via finite differences.
fn surface_normal(u: f32, v: f32, config: &SphereMeshConfig) -> Vec3 {
    let eps = 0.001;
    let p = grid_to_surface(u, v, config);
    let pu = grid_to_surface(u + eps, v, config);
    let pv = grid_to_surface(u, v + eps, config);

    let du = pu - p;
    let dv = pv - p;

    // Cross product gives normal (outward for CCW winding)
    let n = du.cross(dv);
    if n.length_squared() < 1e-12 {
        // Fallback for degenerate cases (e.g., at the pole)
        Vec3::Y
    } else {
        n.normalize()
    }
}

/// Generate the sphere gallery mesh for a grid of `cols × rows` cells.
pub fn generate_sphere_mesh(cols: u32, rows: u32, config: &SphereMeshConfig) -> SphereGalleryMesh {
    let subs = config.subdivisions;
    let verts_per_cell_edge = subs + 1;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for row in 0..rows {
        for col in 0..cols {
            let cell_index = (row * cols + col) as f32;
            let base_vertex = vertices.len() as u32;

            // Cell boundaries in normalized [-1, 1] space
            let u_min = (col as f32 / cols as f32) * 2.0 - 1.0;
            let u_max = ((col + 1) as f32 / cols as f32) * 2.0 - 1.0;
            let v_min = (row as f32 / rows as f32) * 2.0 - 1.0;
            let v_max = ((row + 1) as f32 / rows as f32) * 2.0 - 1.0;

            // Generate subdivided vertices for this cell
            for sy in 0..verts_per_cell_edge {
                for sx in 0..verts_per_cell_edge {
                    let t_u = sx as f32 / subs as f32;
                    let t_v = sy as f32 / subs as f32;

                    let u = u_min + t_u * (u_max - u_min);
                    let v = v_min + t_v * (v_max - v_min);

                    let position = grid_to_surface(u, v, config);
                    let normal = surface_normal(u, v, config);

                    let uv = [t_u, t_v];

                    vertices.push(GalleryVertex::new(position, normal, uv, cell_index));
                }
            }

            // Generate triangle indices for this cell's subdivided quad
            for sy in 0..subs {
                for sx in 0..subs {
                    let tl = base_vertex + sy * verts_per_cell_edge + sx;
                    let tr = tl + 1;
                    let bl = tl + verts_per_cell_edge;
                    let br = bl + 1;

                    indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
                }
            }
        }
    }

    SphereGalleryMesh { vertices, indices }
}

/// Compute the 2D grid center position for a given cell index.
/// Returns (u, v) in [-1, 1] space.
pub fn cell_center(index: u32, cols: u32, rows: u32) -> (f32, f32) {
    let col = index % cols;
    let row = index / cols;
    let u = ((col as f32 + 0.5) / cols as f32) * 2.0 - 1.0;
    let v = ((row as f32 + 0.5) / rows as f32) * 2.0 - 1.0;
    (u, v)
}

/// Get the 3D world position for the center of a cell.
pub fn cell_center_3d(index: u32, cols: u32, rows: u32, config: &SphereMeshConfig) -> Vec3 {
    let (u, v) = cell_center(index, cols, rows);
    grid_to_surface(u, v, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_center_is_highest_all_projections() {
        for proj in Projection::ALL {
            let config = SphereMeshConfig {
                projection: *proj,
                ..Default::default()
            };
            let center = grid_to_surface(0.0, 0.0, &config);
            // Use a corner point (1,1) so both u and v are non-zero.
            // Cylindrical wraps only on v, so (1,0) has the same height
            // as the center — a true edge needs v != 0.
            let edge = grid_to_surface(1.0, 1.0, &config);
            assert!(
                center.y > edge.y,
                "{}: center.y={} should be > edge.y={}",
                proj.name(),
                center.y,
                edge.y
            );
        }
    }

    #[test]
    fn test_flat_when_apex_zero() {
        for proj in Projection::ALL {
            let config = SphereMeshConfig {
                apex_height: 0.0,
                projection: *proj,
                ..Default::default()
            };
            let center = grid_to_surface(0.0, 0.0, &config);
            let edge = grid_to_surface(1.0, 0.0, &config);
            let diff = (center.y - edge.y).abs();
            assert!(
                diff < 0.01,
                "{}: should be ~flat when apex_height=0, but diff={}",
                proj.name(),
                diff
            );
        }
    }

    #[test]
    fn test_mesh_generation() {
        let config = SphereMeshConfig::default();
        let mesh = generate_sphere_mesh(3, 3, &config);

        // 3x3 grid with 4 subdivisions → 5x5 verts per cell → 25 verts per cell, 9 cells
        assert_eq!(mesh.vertices.len(), 9 * 25);

        // 4x4 sub-quads per cell, 2 triangles each, 3 indices per tri = 4*4*6 = 96 per cell
        assert_eq!(mesh.indices.len(), 9 * 96);
    }

    #[test]
    fn test_cell_center() {
        let (u, v) = cell_center(0, 3, 3);
        assert!((u - (-2.0 / 3.0)).abs() < 0.01);
        assert!((v - (-2.0 / 3.0)).abs() < 0.01);

        let (u, v) = cell_center(4, 3, 3);
        assert!(u.abs() < 0.01);
        assert!(v.abs() < 0.01);
    }

    #[test]
    fn test_apex_height_scales_dome() {
        let low = SphereMeshConfig {
            apex_height: 0.2,
            ..Default::default()
        };
        let high = SphereMeshConfig {
            apex_height: 0.8,
            ..Default::default()
        };

        let center_low = grid_to_surface(0.0, 0.0, &low);
        let edge_low = grid_to_surface(1.0, 0.0, &low);
        let center_high = grid_to_surface(0.0, 0.0, &high);
        let edge_high = grid_to_surface(1.0, 0.0, &high);

        let diff_low = center_low.y - edge_low.y;
        let diff_high = center_high.y - edge_high.y;

        assert!(
            diff_high > diff_low,
            "Higher apex should mean bigger height difference: {:.3} > {:.3}",
            diff_high,
            diff_low
        );
    }
}
