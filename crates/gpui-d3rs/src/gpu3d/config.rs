//! Configuration for 3D surface rendering

use glam::Vec3;

/// Available colormaps for surface visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Colormap {
    /// Viridis (perceptually uniform, colorblind-friendly)
    #[default]
    Viridis,
    /// Plasma (perceptually uniform, warm colors)
    Plasma,
    /// Inferno (perceptually uniform, hot colors)
    Inferno,
    /// Turbo (Google's improved rainbow)
    Turbo,
    /// Cool-Warm diverging colormap
    CoolWarm,
}

impl Colormap {
    /// Get colormap index for shader
    pub fn shader_index(&self) -> u32 {
        match self {
            Colormap::Viridis => 0,
            Colormap::Plasma => 1,
            Colormap::Inferno => 2,
            Colormap::Turbo => 3,
            Colormap::CoolWarm => 4,
        }
    }

    /// Get RGB color at position t (0.0 to 1.0) for colorbar rendering
    /// Uses the same polynomial approximations as the GPU shader for consistency
    pub fn color_at(&self, t: f32) -> (f32, f32, f32) {
        let t = t.clamp(0.0, 1.0);
        match self {
            Colormap::Viridis => {
                // Polynomial approximation matching shader
                let t2 = t * t;
                let t3 = t2 * t;
                let t4 = t3 * t;
                let t5 = t4 * t;
                let t6 = t5 * t;

                let r =
                    (0.2777 + 0.1050 * t - 0.3308 * t2 - 4.6342 * t3 + 6.2282 * t4 + 4.7763 * t5
                        - 5.4354 * t6)
                        .clamp(0.0, 1.0);
                let g = (0.0054 + 0.6387 * t + 0.3143 * t2 - 5.7991 * t3 + 14.1799 * t4
                    - 13.7451 * t5
                    + 4.6456 * t6)
                    .clamp(0.0, 1.0);
                let b = (0.3340 + 0.2383 * t + 0.5287 * t2 - 19.3324 * t3 + 56.6905 * t4
                    - 65.3530 * t5
                    + 26.3124 * t6)
                    .clamp(0.0, 1.0);
                (r, g, b)
            }
            Colormap::Plasma => {
                // Polynomial approximation matching shader
                let t2 = t * t;
                let t3 = t2 * t;
                let t4 = t3 * t;
                let t5 = t4 * t;

                let r = (0.0504 + 1.3656 * t + 0.4324 * t2 - 6.8475 * t3 + 5.5523 * t4
                    - 0.5571 * t5)
                    .clamp(0.0, 1.0);
                let g = (0.0298 + 0.0099 * t + 2.2891 * t2 - 6.4044 * t3 + 5.4343 * t4
                    - 1.3609 * t5)
                    .clamp(0.0, 1.0);
                let b = (0.5280 + 1.8654 * t - 6.4178 * t2 + 10.0276 * t3 - 6.5861 * t4
                    + 1.5724 * t5)
                    .clamp(0.0, 1.0);
                (r, g, b)
            }
            Colormap::Inferno => {
                // Polynomial approximation matching shader
                let t2 = t * t;
                let t3 = t2 * t;
                let t4 = t3 * t;
                let t5 = t4 * t;

                let r = (0.0002 + 0.4366 * t + 4.1934 * t2 - 13.6829 * t3 + 16.1821 * t4
                    - 6.1307 * t5)
                    .clamp(0.0, 1.0);
                let g = (0.0003 + 0.0888 * t + 3.5044 * t2 - 8.7954 * t3 + 8.4731 * t4
                    - 2.2655 * t5)
                    .clamp(0.0, 1.0);
                let b = (0.0139 + 2.0252 * t - 6.4560 * t2 + 10.8598 * t3 - 9.6524 * t4
                    + 3.2059 * t5)
                    .clamp(0.0, 1.0);
                (r, g, b)
            }
            Colormap::Turbo => {
                // Polynomial approximation matching shader
                let r = (0.13572
                    + t * (4.6153
                        + t * (-42.6592 + t * (138.5676 + t * (-152.3494 + t * 59.2859)))))
                    .clamp(0.0, 1.0);
                let g = (0.09140
                    + t * (2.2537 + t * (0.6487 + t * (-23.3910 + t * (38.3522 - t * 18.0858)))))
                    .clamp(0.0, 1.0);
                let b = (0.10667
                    + t * (12.5925
                        + t * (-60.5820 + t * (109.7316 + t * (-88.2949 + t * 26.7236)))))
                    .clamp(0.0, 1.0);
                (r, g, b)
            }
            Colormap::CoolWarm => {
                // Matching shader implementation
                let mid = 0.5;
                if t < mid {
                    let s = t / mid;
                    let r = 0.23 + (0.87 - 0.23) * s;
                    let g = 0.30 + (0.87 - 0.30) * s;
                    let b = 0.75 + (0.87 - 0.75) * s;
                    (r, g, b)
                } else {
                    let s = (t - mid) / (1.0 - mid);
                    let r = 0.87 + (0.71 - 0.87) * s;
                    let g = 0.87 + (0.02 - 0.87) * s;
                    let b = 0.87 + (0.15 - 0.87) * s;
                    (r, g, b)
                }
            }
        }
    }
}

/// Type of 3D surface plot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SurfacePlotType {
    /// Standard Cartesian plot (X, Y, Z)
    #[default]
    Cartesian,
    /// Spherical plot (Globe/Balloon)
    Spherical,
}

/// Configuration for 3D surface rendering
#[derive(Debug, Clone)]
pub struct Surface3DConfig {
    /// Colormap for surface coloring
    pub colormap: Colormap,
    /// Show wireframe overlay
    pub wireframe: bool,
    /// Wireframe color (RGB)
    pub wireframe_color: [f32; 3],
    /// Background color (RGB)
    pub background_color: [f32; 3],
    /// Ambient lighting intensity (0.0 - 1.0)
    pub ambient: f32,
    /// Diffuse lighting intensity (0.0 - 1.0)
    pub diffuse: f32,
    /// Light direction (will be normalized)
    pub light_direction: Vec3,
    /// Enable anti-aliasing (MSAA)
    pub msaa_samples: u32,
    /// Initial camera distance
    pub camera_distance: f32,
    /// Initial camera azimuth (degrees)
    pub camera_azimuth: f32,
    /// Initial camera elevation (degrees)
    pub camera_elevation: f32,
    /// Show axis labels
    pub show_axes: bool,
    /// Show colorbar legend
    pub show_colorbar: bool,
    /// Surface opacity (0.0 - 1.0)
    pub opacity: f32,
    /// Show isolines projection
    pub isolines: bool,
    /// Isoline step size (normalized 0-1)
    pub isoline_step: f32,
    /// Plot type (Cartesian or Spherical)
    pub plot_type: SurfacePlotType,
    /// Show grid/bounding box
    pub show_grid: bool,
}

impl Default for Surface3DConfig {
    fn default() -> Self {
        Self {
            colormap: Colormap::Viridis,
            wireframe: false,
            wireframe_color: [0.2, 0.2, 0.2],
            background_color: [0.1, 0.1, 0.12],
            ambient: 0.3,
            diffuse: 0.7,
            light_direction: Vec3::new(1.0, 1.0, 1.0),
            msaa_samples: 4,
            camera_distance: 3.5,
            camera_azimuth: 45.0,
            camera_elevation: 30.0,
            show_axes: true,
            show_colorbar: true,
            opacity: 1.0,
            isolines: false,
            isoline_step: 0.05,
            plot_type: SurfacePlotType::Cartesian,
            show_grid: true,
        }
    }
}

impl Surface3DConfig {
    /// Create a new configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the colormap
    pub fn colormap(mut self, colormap: Colormap) -> Self {
        self.colormap = colormap;
        self
    }

    /// Enable or disable wireframe overlay
    pub fn wireframe(mut self, enabled: bool) -> Self {
        self.wireframe = enabled;
        self
    }

    /// Set wireframe color
    pub fn wireframe_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.wireframe_color = [r, g, b];
        self
    }

    /// Set background color
    pub fn background_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.background_color = [r, g, b];
        self
    }

    /// Set ambient lighting (0.0 - 1.0)
    pub fn ambient(mut self, ambient: f32) -> Self {
        self.ambient = ambient.clamp(0.0, 1.0);
        self
    }

    /// Set diffuse lighting (0.0 - 1.0)
    pub fn diffuse(mut self, diffuse: f32) -> Self {
        self.diffuse = diffuse.clamp(0.0, 1.0);
        self
    }

    /// Set light direction
    pub fn light_direction(mut self, x: f32, y: f32, z: f32) -> Self {
        self.light_direction = Vec3::new(x, y, z).normalize();
        self
    }

    /// Set MSAA sample count (1, 2, 4, or 8)
    pub fn msaa_samples(mut self, samples: u32) -> Self {
        self.msaa_samples = match samples {
            1 | 2 | 4 | 8 => samples,
            _ => 4,
        };
        self
    }

    /// Set initial camera position
    pub fn camera_position(mut self, distance: f32, azimuth_deg: f32, elevation_deg: f32) -> Self {
        self.camera_distance = distance;
        self.camera_azimuth = azimuth_deg;
        self.camera_elevation = elevation_deg;
        self
    }

    /// Enable or disable axis display
    pub fn show_axes(mut self, enabled: bool) -> Self {
        self.show_axes = enabled;
        self
    }

    /// Show colorbar legend
    pub fn show_colorbar(mut self, enabled: bool) -> Self {
        self.show_colorbar = enabled;
        self
    }

    /// Set surface opacity (0.0 - 1.0)
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Enable or disable isolines
    pub fn isolines(mut self, enabled: bool) -> Self {
        self.isolines = enabled;
        self
    }

    /// Set isoline step size (in normalized 0-1 range)
    pub fn isoline_step(mut self, step: f32) -> Self {
        self.isoline_step = step.clamp(0.001, 1.0);
        self
    }

    /// Set plot type
    pub fn plot_type(mut self, plot_type: SurfacePlotType) -> Self {
        self.plot_type = plot_type;
        self
    }

    /// Enable or disable grid/bounding box display
    pub fn show_grid(mut self, enabled: bool) -> Self {
        self.show_grid = enabled;
        self
    }

    /// Get normalized light direction
    pub fn normalized_light_direction(&self) -> Vec3 {
        self.light_direction.normalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Surface3DConfig::default();
        assert_eq!(config.colormap, Colormap::Viridis);
        assert!(!config.wireframe);
        assert!(config.ambient > 0.0);
        assert!(config.diffuse > 0.0);
    }

    #[test]
    fn test_builder_pattern() {
        let config = Surface3DConfig::new()
            .colormap(Colormap::Plasma)
            .wireframe(true)
            .ambient(0.5)
            .diffuse(0.5)
            .camera_position(5.0, 60.0, 45.0);

        assert_eq!(config.colormap, Colormap::Plasma);
        assert!(config.wireframe);
        assert!((config.ambient - 0.5).abs() < 0.01);
        assert!((config.camera_distance - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_ambient_clamping() {
        let config = Surface3DConfig::new().ambient(1.5);
        assert!((config.ambient - 1.0).abs() < 0.01);

        let config = Surface3DConfig::new().ambient(-0.5);
        assert!((config.ambient - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_msaa_validation() {
        let config = Surface3DConfig::new().msaa_samples(3);
        assert_eq!(config.msaa_samples, 4); // Should default to 4

        let config = Surface3DConfig::new().msaa_samples(8);
        assert_eq!(config.msaa_samples, 8);
    }
}
