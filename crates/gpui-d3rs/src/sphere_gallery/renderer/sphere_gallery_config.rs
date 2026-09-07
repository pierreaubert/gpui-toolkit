use super::super::mesh::{Projection, SphereMeshConfig};

/// Configuration for the sphere gallery renderer
#[derive(Debug, Clone)]
pub struct SphereGalleryConfig {
    /// Number of columns in the gallery grid
    pub cols: u32,
    /// Number of rows in the gallery grid
    pub rows: u32,
    /// Size of each cell in the texture atlas (pixels)
    pub cell_size: u32,
    /// Sphere mesh configuration
    pub mesh_config: SphereMeshConfig,
    /// Background color [r, g, b]
    pub background_color: [f32; 3],
    /// Ambient lighting factor
    pub ambient: f32,
    /// Diffuse lighting factor
    pub diffuse: f32,
    /// Render each item's label as a world-anchored SDF billboard under its
    /// sphere in the wgpu pass. Labels stay upright at every camera angle.
    /// Defaults off (labels exist on the items but were never displayed).
    pub billboard_labels: bool,
    /// Em size of item billboard labels in screen px.
    pub label_size_px: f32,
}

impl Default for SphereGalleryConfig {
    fn default() -> Self {
        Self {
            cols: 5,
            rows: 5,
            cell_size: 256,
            mesh_config: SphereMeshConfig::default(),
            background_color: [0.05, 0.05, 0.07],
            ambient: 0.4,
            diffuse: 0.6,
            billboard_labels: false,
            label_size_px: 11.0,
        }
    }
}

impl SphereGalleryConfig {
    pub fn new(cols: u32, rows: u32) -> Self {
        Self {
            cols,
            rows,
            ..Default::default()
        }
    }

    pub fn cell_size(mut self, size: u32) -> Self {
        self.cell_size = size;
        self
    }

    /// Set the map projection type.
    pub fn projection(mut self, projection: Projection) -> Self {
        self.mesh_config.projection = projection;
        self
    }

    /// Set how high the center rises above the edges.
    ///
    /// - `0.0` = flat
    /// - `0.5` = moderate dome (default)
    /// - `1.0` = hemisphere
    pub fn apex_height(mut self, height: f32) -> Self {
        self.mesh_config.apex_height = height;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.mesh_config.radius = radius;
        self
    }

    pub fn subdivisions(mut self, subs: u32) -> Self {
        self.mesh_config.subdivisions = subs;
        self
    }

    /// Render item labels as world-anchored SDF billboards in the wgpu pass
    pub fn billboard_labels(mut self, enabled: bool) -> Self {
        self.billboard_labels = enabled;
        self
    }

    /// Em size of item billboard labels in screen px
    pub fn label_size_px(mut self, size: f32) -> Self {
        self.label_size_px = size.max(1.0);
        self
    }
}
