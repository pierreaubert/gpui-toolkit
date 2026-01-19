//! Surface rendering for GPUI
//!
//! Provides 3D surface visualization using 2D projection and painter's algorithm.

use super::data::SurfaceData;
use super::mesh::SurfaceMesh;
use super::projection::{
    Camera2D, IsometricProjection, ObliqueProjection, OrthographicProjection,
    PerspectiveProjection, Point2D, Projection, ProjectionType,
};
use crate::color::D3Color;
use crate::text::paint_vector_text_at;
use gpui::prelude::*;
use gpui::*;
use std::f32::consts::PI;
use std::panic;
use std::sync::Arc;

/// Enum wrapper for projections to enable dynamic dispatch without trait objects.
/// This is needed because the Projection trait has Clone bound which makes it not dyn-compatible.
#[derive(Clone)]
enum ProjectionImpl {
    Isometric(IsometricProjection),
    Oblique(ObliqueProjection),
    Orthographic(OrthographicProjection),
    Perspective(PerspectiveProjection),
}

impl ProjectionImpl {
    fn project(&self, x: f64, y: f64, z: f64) -> Point2D {
        match self {
            ProjectionImpl::Isometric(p) => p.project(x, y, z),
            ProjectionImpl::Oblique(p) => p.project(x, y, z),
            ProjectionImpl::Orthographic(p) => p.project(x, y, z),
            ProjectionImpl::Perspective(p) => p.project(x, y, z),
        }
    }

    fn depth(&self, x: f64, y: f64, z: f64) -> f64 {
        match self {
            ProjectionImpl::Isometric(p) => p.depth(x, y, z),
            ProjectionImpl::Oblique(p) => p.depth(x, y, z),
            ProjectionImpl::Orthographic(p) => p.depth(x, y, z),
            ProjectionImpl::Perspective(p) => p.depth(x, y, z),
        }
    }

    fn point_depth(&self, p: &super::data::SurfacePoint3D) -> f64 {
        self.depth(p.x, p.y, p.z)
    }
}

/// Color scale types for surface coloring
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ColorScaleType {
    /// Viridis color scale (perceptually uniform, colorblind-friendly)
    #[default]
    Viridis,
    /// Heat color scale (blue -> white -> red)
    Heat,
    /// Cool color scale (cyan -> magenta)
    Cool,
    /// Spectral color scale (rainbow)
    Spectral,
    /// Grayscale
    Grayscale,
    /// Single color with brightness variation
    Monochrome(D3Color),
}

impl ColorScaleType {
    /// Get the color for a normalized value [0, 1]
    pub fn color(&self, t: f64) -> D3Color {
        let t = t.clamp(0.0, 1.0);
        match self {
            ColorScaleType::Viridis => viridis_color(t),
            ColorScaleType::Heat => heat_color(t),
            ColorScaleType::Cool => cool_color(t),
            ColorScaleType::Spectral => spectral_color(t),
            ColorScaleType::Grayscale => grayscale_color(t),
            ColorScaleType::Monochrome(base) => monochrome_color(*base, t),
        }
    }
}

/// Viridis color scale
fn viridis_color(t: f64) -> D3Color {
    let colors = [
        D3Color::from_hex(0x440154),
        D3Color::from_hex(0x482878),
        D3Color::from_hex(0x3e4a89),
        D3Color::from_hex(0x31688e),
        D3Color::from_hex(0x26838f),
        D3Color::from_hex(0x1f9e89),
        D3Color::from_hex(0x35b779),
        D3Color::from_hex(0x6ece58),
        D3Color::from_hex(0xb5de2b),
        D3Color::from_hex(0xfde725),
    ];
    interpolate_colors(&colors, t)
}

/// Heat color scale (blue -> white -> red)
fn heat_color(t: f64) -> D3Color {
    if t < 0.5 {
        let local_t = t * 2.0;
        D3Color::from_hex(0x0571b0).interpolate(&D3Color::from_hex(0xf7f7f7), local_t as f32)
    } else {
        let local_t = (t - 0.5) * 2.0;
        D3Color::from_hex(0xf7f7f7).interpolate(&D3Color::from_hex(0xca0020), local_t as f32)
    }
}

/// Cool color scale (cyan -> magenta)
fn cool_color(t: f64) -> D3Color {
    D3Color::from_hex(0x00ffff).interpolate(&D3Color::from_hex(0xff00ff), t as f32)
}

/// Spectral color scale
fn spectral_color(t: f64) -> D3Color {
    let colors = [
        D3Color::from_hex(0x9e0142),
        D3Color::from_hex(0xd53e4f),
        D3Color::from_hex(0xf46d43),
        D3Color::from_hex(0xfdae61),
        D3Color::from_hex(0xfee08b),
        D3Color::from_hex(0xe6f598),
        D3Color::from_hex(0xabdda4),
        D3Color::from_hex(0x66c2a5),
        D3Color::from_hex(0x3288bd),
        D3Color::from_hex(0x5e4fa2),
    ];
    interpolate_colors(&colors, t)
}

/// Grayscale color
fn grayscale_color(t: f64) -> D3Color {
    let v = (t * 255.0) as u8;
    D3Color::rgb(v, v, v)
}

/// Monochrome color with brightness variation
fn monochrome_color(base: D3Color, t: f64) -> D3Color {
    // Interpolate from dark version to light version
    let dark = D3Color {
        r: base.r * 0.2,
        g: base.g * 0.2,
        b: base.b * 0.2,
        a: base.a,
    };
    dark.interpolate(&base, t as f32)
}

/// Helper to interpolate through a color array
fn interpolate_colors(colors: &[D3Color], t: f64) -> D3Color {
    let idx = (t * (colors.len() - 1) as f64) as usize;
    let idx = idx.min(colors.len() - 2);
    let local_t = (t * (colors.len() - 1) as f64) - idx as f64;
    colors[idx].interpolate(&colors[idx + 1], local_t as f32)
}

/// Configuration for surface rendering
#[derive(Clone)]
pub struct SurfaceConfig {
    /// Projection type
    pub projection_type: ProjectionType,
    /// Camera settings
    pub camera: Camera2D,
    /// Color scale for the surface
    pub color_scale: ColorScaleType,
    /// Custom color function (overrides color_scale if set)
    pub custom_color: Option<Arc<dyn Fn(f64) -> D3Color + Send + Sync>>,
    /// Surface opacity (0.0 - 1.0)
    pub opacity: f32,
    /// Whether to draw wireframe
    pub wireframe: bool,
    /// Wireframe color
    pub wireframe_color: D3Color,
    /// Wireframe stroke width
    pub wireframe_width: f32,
    /// Wireframe opacity
    pub wireframe_opacity: f32,
    /// Whether to apply lighting
    pub lighting: bool,
    /// Ambient light intensity (0.0 - 1.0)
    pub ambient: f64,
    /// Diffuse light intensity (0.0 - 1.0)
    pub diffuse: f64,
    /// Light direction (normalized)
    pub light_direction: (f64, f64, f64),
    /// Scale factor for the projection
    pub scale: f64,
    /// Whether to show 3D axes
    pub show_axes: bool,
    /// Axis color
    pub axis_color: D3Color,
    /// Axis line width
    pub axis_width: f32,
    /// Axis labels (X, Y, Z)
    pub axis_labels: Option<(String, String, String)>,
    /// Axis value ranges for tick labels (X, Y, Z) - ((min, max), (min, max), (min, max))
    pub axis_ranges: Option<((f64, f64), (f64, f64), (f64, f64))>,
    /// Font size for axis labels and tick values
    pub axis_font_size: f32,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            projection_type: ProjectionType::Isometric,
            camera: Camera2D::default(),
            color_scale: ColorScaleType::Viridis,
            custom_color: None,
            opacity: 1.0,
            wireframe: false,
            wireframe_color: D3Color::rgb(0, 0, 0),
            wireframe_width: 0.5,
            wireframe_opacity: 0.5,
            lighting: true,
            ambient: 0.4,
            diffuse: 0.6,
            light_direction: normalize_vec((-0.5, -0.5, 1.0)),
            scale: 1.0,
            show_axes: false,
            axis_color: D3Color::rgb(100, 100, 100),
            axis_width: 1.5,
            axis_labels: None,
            axis_ranges: None,
            axis_font_size: 10.0,
        }
    }
}

impl SurfaceConfig {
    /// Create a new surface configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Use isometric projection
    pub fn isometric(mut self) -> Self {
        self.projection_type = ProjectionType::Isometric;
        self
    }

    /// Use oblique projection
    pub fn oblique(mut self) -> Self {
        self.projection_type = ProjectionType::Oblique;
        self
    }

    /// Use orthographic projection
    pub fn orthographic(mut self) -> Self {
        self.projection_type = ProjectionType::Orthographic;
        self
    }

    /// Use perspective projection
    pub fn perspective(mut self) -> Self {
        self.projection_type = ProjectionType::Perspective;
        self
    }

    /// Set camera rotation (pitch, yaw) in degrees
    pub fn rotation(mut self, pitch: f64, yaw: f64) -> Self {
        self.camera.rotation_x = pitch;
        self.camera.rotation_z = yaw;
        self
    }

    /// Set camera zoom
    pub fn zoom(mut self, zoom: f64) -> Self {
        self.camera.zoom = zoom;
        self
    }

    /// Set camera pan offset
    pub fn pan(mut self, x: f64, y: f64) -> Self {
        self.camera.pan = (x, y);
        self
    }

    /// Set the color scale
    pub fn color_scale(mut self, scale: ColorScaleType) -> Self {
        self.color_scale = scale;
        self.custom_color = None;
        self
    }

    /// Set a custom color function
    pub fn custom_color<F>(mut self, f: F) -> Self
    where
        F: Fn(f64) -> D3Color + Send + Sync + 'static,
    {
        self.custom_color = Some(Arc::new(f));
        self
    }

    /// Set surface opacity
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Enable wireframe overlay
    pub fn wireframe(mut self, enabled: bool) -> Self {
        self.wireframe = enabled;
        self
    }

    /// Set wireframe color
    pub fn wireframe_color(mut self, color: D3Color) -> Self {
        self.wireframe_color = color;
        self
    }

    /// Set wireframe stroke width
    pub fn wireframe_width(mut self, width: f32) -> Self {
        self.wireframe_width = width;
        self
    }

    /// Set wireframe opacity
    pub fn wireframe_opacity(mut self, opacity: f32) -> Self {
        self.wireframe_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Enable or disable lighting
    pub fn lighting(mut self, enabled: bool) -> Self {
        self.lighting = enabled;
        self
    }

    /// Set ambient light intensity
    pub fn ambient(mut self, intensity: f64) -> Self {
        self.ambient = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set diffuse light intensity
    pub fn diffuse(mut self, intensity: f64) -> Self {
        self.diffuse = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set light direction
    pub fn light_direction(mut self, x: f64, y: f64, z: f64) -> Self {
        self.light_direction = normalize_vec((x, y, z));
        self
    }

    /// Set projection scale
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    /// Show 3D axes
    pub fn show_axes(mut self, show: bool) -> Self {
        self.show_axes = show;
        self
    }

    /// Set axis color
    pub fn axis_color(mut self, color: D3Color) -> Self {
        self.axis_color = color;
        self
    }

    /// Set axis line width
    pub fn axis_width(mut self, width: f32) -> Self {
        self.axis_width = width;
        self
    }

    /// Set axis labels (X, Y, Z)
    pub fn axis_labels(
        mut self,
        x: impl Into<String>,
        y: impl Into<String>,
        z: impl Into<String>,
    ) -> Self {
        self.axis_labels = Some((x.into(), y.into(), z.into()));
        self
    }

    /// Set axis value ranges for tick labels (X, Y, Z)
    pub fn axis_ranges(
        mut self,
        x_range: (f64, f64),
        y_range: (f64, f64),
        z_range: (f64, f64),
    ) -> Self {
        self.axis_ranges = Some((x_range, y_range, z_range));
        self
    }

    /// Set font size for axis labels and tick values
    pub fn axis_font_size(mut self, size: f32) -> Self {
        self.axis_font_size = size;
        self
    }

    /// Get color for a normalized t value
    fn get_color(&self, t: f64) -> D3Color {
        if let Some(ref custom) = self.custom_color {
            custom(t)
        } else {
            self.color_scale.color(t)
        }
    }
}

/// Normalize a vector
fn normalize_vec(v: (f64, f64, f64)) -> (f64, f64, f64) {
    let len = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
    if len > 1e-10 {
        (v.0 / len, v.1 / len, v.2 / len)
    } else {
        (0.0, 0.0, 1.0)
    }
}

/// Format tick value for display based on the range
fn format_tick_value(value: f64, min: f64, max: f64) -> String {
    let range = (max - min).abs();

    // For large numbers (like frequency), use k suffix
    if value.abs() >= 1000.0 && range >= 1000.0 {
        let k = value / 1000.0;
        if k == k.floor() {
            format!("{}k", k as i32)
        } else {
            format!("{:.1}k", k)
        }
    } else if range < 10.0 {
        // Small range - show one decimal
        format!("{:.1}", value)
    } else if value == value.floor() {
        // Integer value
        format!("{}", value as i32)
    } else {
        format!("{:.0}", value)
    }
}

/// A custom GPUI element for rendering 3D surfaces
pub struct SurfaceElement {
    /// The surface mesh
    mesh: SurfaceMesh,
    /// The surface data (for range info)
    data: SurfaceData,
    /// Configuration
    config: SurfaceConfig,
    /// Element width
    width: Pixels,
    /// Element height
    height: Pixels,
}

impl SurfaceElement {
    /// Create a new surface element
    pub fn new(data: &SurfaceData, config: SurfaceConfig, width: f32, height: f32) -> Self {
        // Normalize data to unit cube for consistent rendering
        let normalized = data.normalize();

        // Create mesh from normalized data
        let mut mesh = SurfaceMesh::from_surface_data(&normalized);

        // Apply color scale
        mesh.apply_color_scale(normalized.t_range, |t| config.get_color(t));

        Self {
            mesh,
            data: normalized,
            config,
            width: px(width),
            height: px(height),
        }
    }

    /// Set element dimensions
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = px(width);
        self.height = px(height);
        self
    }

    /// Create the projection based on config
    fn create_projection(&self, bounds: &Bounds<Pixels>) -> ProjectionImpl {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let origin_x: f32 = bounds.origin.x.into();
        let origin_y: f32 = bounds.origin.y.into();

        // Center the projection in the bounds
        let center_x = origin_x + width / 2.0;
        let center_y = origin_y + height / 2.0;

        // Scale based on the smaller dimension to fit
        let base_scale = width.min(height) as f64 * 0.35 * self.config.scale;

        match self.config.projection_type {
            ProjectionType::Isometric => ProjectionImpl::Isometric(
                IsometricProjection::new()
                    .scale(base_scale)
                    .origin(center_x as f64, center_y as f64)
                    .camera(self.config.camera.clone()),
            ),
            ProjectionType::Oblique => ProjectionImpl::Oblique(
                ObliqueProjection::cabinet()
                    .scale(base_scale)
                    .origin(center_x as f64, center_y as f64),
            ),
            ProjectionType::Orthographic => ProjectionImpl::Orthographic(
                OrthographicProjection::new()
                    .scale(base_scale)
                    .rotation(
                        self.config.camera.rotation_x,
                        0.0,
                        self.config.camera.rotation_z,
                    )
                    .origin(center_x as f64, center_y as f64),
            ),
            ProjectionType::Perspective => ProjectionImpl::Perspective(
                PerspectiveProjection::new()
                    .scale(base_scale)
                    .distance(3.0)
                    .rotation(
                        self.config.camera.rotation_x,
                        0.0,
                        self.config.camera.rotation_z,
                    )
                    .origin(center_x as f64, center_y as f64),
            ),
        }
    }
}

impl IntoElement for SurfaceElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SurfaceElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = window.request_layout(
            Style {
                size: size(self.width.into(), self.height.into()),
                min_size: size(px(100.0).into(), px(100.0).into()),
                ..Default::default()
            },
            [],
            cx,
        );
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        // Create projection and sort triangles by depth
        let projection = self.create_projection(&bounds);

        // Apply depth sorting using painter's algorithm
        self.mesh.triangles.sort_by(|a, b| {
            let centroid_a = a.centroid();
            let centroid_b = b.centroid();

            let depth_a = projection.point_depth(&centroid_a);
            let depth_b = projection.point_depth(&centroid_b);

            // Sort by depth: larger depth (further) should come first
            depth_b
                .partial_cmp(&depth_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply lighting if enabled
        if self.config.lighting {
            // Re-apply color scale first (lighting modifies colors)
            self.mesh
                .apply_color_scale(self.data.t_range, |t| self.config.get_color(t));
            self.mesh.apply_lighting(
                self.config.light_direction,
                self.config.ambient,
                self.config.diffuse,
            );
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let projection = self.create_projection(&bounds);

        // Center the data (normalized data is in [0,1]^3, center at origin)
        let offset = -0.5;

        // Paint each triangle (already sorted back-to-front)
        for triangle in &self.mesh.triangles {
            // Project vertices
            let projected: Vec<Point2D> = triangle
                .vertices
                .iter()
                .map(|v| projection.project(v.x + offset, v.y + offset, v.z + offset))
                .collect();

            // Create fill path
            let mut builder = PathBuilder::fill();
            builder.move_to(point(px(projected[0].x as f32), px(projected[0].y as f32)));
            builder.line_to(point(px(projected[1].x as f32), px(projected[1].y as f32)));
            builder.line_to(point(px(projected[2].x as f32), px(projected[2].y as f32)));

            if let Ok(path) = builder.build() {
                let mut fill_color = triangle.color.to_rgba();
                fill_color.a *= self.config.opacity;
                window.paint_path(path, fill_color);
            }

            // Draw wireframe if enabled
            if self.config.wireframe {
                let mut stroke_builder = PathBuilder::stroke(px(self.config.wireframe_width));
                stroke_builder.move_to(point(px(projected[0].x as f32), px(projected[0].y as f32)));
                stroke_builder.line_to(point(px(projected[1].x as f32), px(projected[1].y as f32)));
                stroke_builder.line_to(point(px(projected[2].x as f32), px(projected[2].y as f32)));
                stroke_builder.line_to(point(px(projected[0].x as f32), px(projected[0].y as f32)));

                if let Ok(stroke_path) = stroke_builder.build() {
                    let mut wireframe_rgba = self.config.wireframe_color.to_rgba();
                    wireframe_rgba.a *= self.config.wireframe_opacity;
                    window.paint_path(stroke_path, wireframe_rgba);
                }
            }
        }

        // Draw 3D axes if enabled
        if self.config.show_axes {
            let axis_color = self.config.axis_color.to_rgba();

            // Origin point (corner of the unit cube, offset to center)
            let origin = (-0.5, -0.5, -0.5);

            // Axis endpoints (extend slightly beyond the surface for visibility)
            let x_end = (0.6, -0.5, -0.5);
            let y_end = (-0.5, 0.6, -0.5);
            let z_end = (-0.5, -0.5, 0.6);

            // Project axis endpoints
            let p_origin = projection.project(origin.0, origin.1, origin.2);
            let p_x = projection.project(x_end.0, x_end.1, x_end.2);
            let p_y = projection.project(y_end.0, y_end.1, y_end.2);
            let p_z = projection.project(z_end.0, z_end.1, z_end.2);

            // Draw X axis
            let mut x_builder = PathBuilder::stroke(px(self.config.axis_width));
            x_builder.move_to(point(px(p_origin.x as f32), px(p_origin.y as f32)));
            x_builder.line_to(point(px(p_x.x as f32), px(p_x.y as f32)));
            if let Ok(path) = x_builder.build() {
                window.paint_path(path, axis_color);
            }

            // Draw Y axis
            let mut y_builder = PathBuilder::stroke(px(self.config.axis_width));
            y_builder.move_to(point(px(p_origin.x as f32), px(p_origin.y as f32)));
            y_builder.line_to(point(px(p_y.x as f32), px(p_y.y as f32)));
            if let Ok(path) = y_builder.build() {
                window.paint_path(path, axis_color);
            }

            // Draw Z axis
            let mut z_builder = PathBuilder::stroke(px(self.config.axis_width));
            z_builder.move_to(point(px(p_origin.x as f32), px(p_origin.y as f32)));
            z_builder.line_to(point(px(p_z.x as f32), px(p_z.y as f32)));
            if let Ok(path) = z_builder.build() {
                window.paint_path(path, axis_color);
            }

            // Draw tick marks on each axis (5 ticks)
            let tick_size = 0.03;
            let num_ticks = 5;
            let tick_font_size = self.config.axis_font_size;
            let tick_stroke = 1.0;

            for i in 0..=num_ticks {
                let t = i as f64 / num_ticks as f64;

                // X axis ticks (perpendicular to X in YZ plane)
                let x_tick_pos = (-0.5 + t * 1.0, -0.5, -0.5);
                let x_tick_end = (-0.5 + t * 1.0, -0.5 - tick_size, -0.5);
                let p1 = projection.project(x_tick_pos.0, x_tick_pos.1, x_tick_pos.2);
                let p2 = projection.project(x_tick_end.0, x_tick_end.1, x_tick_end.2);
                let mut tick_builder = PathBuilder::stroke(px(1.0));
                tick_builder.move_to(point(px(p1.x as f32), px(p1.y as f32)));
                tick_builder.line_to(point(px(p2.x as f32), px(p2.y as f32)));
                if let Ok(path) = tick_builder.build() {
                    window.paint_path(path, axis_color);
                }

                // Draw X tick value if ranges provided
                if let Some(((x_min, x_max), _, _)) = self.config.axis_ranges {
                    let value = x_min + t * (x_max - x_min);
                    let label = format_tick_value(value, x_min, x_max);
                    paint_vector_text_at(
                        window,
                        &label,
                        p2.x as f32,
                        p2.y as f32 + 12.0,
                        tick_font_size,
                        tick_stroke,
                        axis_color,
                        0.0,
                    );
                }

                // Y axis ticks
                let y_tick_pos = (-0.5, -0.5 + t * 1.0, -0.5);
                let y_tick_end = (-0.5 - tick_size, -0.5 + t * 1.0, -0.5);
                let p1 = projection.project(y_tick_pos.0, y_tick_pos.1, y_tick_pos.2);
                let p2 = projection.project(y_tick_end.0, y_tick_end.1, y_tick_end.2);
                let mut tick_builder = PathBuilder::stroke(px(1.0));
                tick_builder.move_to(point(px(p1.x as f32), px(p1.y as f32)));
                tick_builder.line_to(point(px(p2.x as f32), px(p2.y as f32)));
                if let Ok(path) = tick_builder.build() {
                    window.paint_path(path, axis_color);
                }

                // Draw Y tick value if ranges provided
                if let Some((_, (y_min, y_max), _)) = self.config.axis_ranges {
                    let value = y_min + t * (y_max - y_min);
                    let label = format_tick_value(value, y_min, y_max);
                    paint_vector_text_at(
                        window,
                        &label,
                        p2.x as f32 - 18.0,
                        p2.y as f32,
                        tick_font_size,
                        tick_stroke,
                        axis_color,
                        0.0,
                    );
                }

                // Z axis ticks
                let z_tick_pos = (-0.5, -0.5, -0.5 + t * 1.0);
                let z_tick_end = (-0.5 - tick_size, -0.5, -0.5 + t * 1.0);
                let p1 = projection.project(z_tick_pos.0, z_tick_pos.1, z_tick_pos.2);
                let p2 = projection.project(z_tick_end.0, z_tick_end.1, z_tick_end.2);
                let mut tick_builder = PathBuilder::stroke(px(1.0));
                tick_builder.move_to(point(px(p1.x as f32), px(p1.y as f32)));
                tick_builder.line_to(point(px(p2.x as f32), px(p2.y as f32)));
                if let Ok(path) = tick_builder.build() {
                    window.paint_path(path, axis_color);
                }

                // Draw Z tick value if ranges provided
                if let Some((_, _, (z_min, z_max))) = self.config.axis_ranges {
                    let value = z_min + t * (z_max - z_min);
                    let label = format_tick_value(value, z_min, z_max);
                    paint_vector_text_at(
                        window,
                        &label,
                        p2.x as f32 - 20.0,
                        p2.y as f32,
                        tick_font_size,
                        tick_stroke,
                        axis_color,
                        0.0,
                    );
                }
            }

            // Draw axis labels if configured
            if let Some((x_label, y_label, z_label)) = &self.config.axis_labels {
                let font_size = 11.0;
                let stroke_width = 1.2;

                // X axis label - positioned at the end of the X axis
                let x_label_pos = projection.project(0.7, -0.5, -0.5);
                paint_vector_text_at(
                    window,
                    x_label,
                    x_label_pos.x as f32,
                    x_label_pos.y as f32 + 15.0,
                    font_size,
                    stroke_width,
                    axis_color,
                    0.0,
                );

                // Y axis label - positioned at the end of the Y axis
                let y_label_pos = projection.project(-0.5, 0.7, -0.5);
                paint_vector_text_at(
                    window,
                    y_label,
                    y_label_pos.x as f32 - 15.0,
                    y_label_pos.y as f32,
                    font_size,
                    stroke_width,
                    axis_color,
                    -PI / 2.0, // Vertical text
                );

                // Z axis label - positioned at the end of the Z axis
                let z_label_pos = projection.project(-0.5, -0.5, 0.7);
                paint_vector_text_at(
                    window,
                    z_label,
                    z_label_pos.x as f32 - 15.0,
                    z_label_pos.y as f32,
                    font_size,
                    stroke_width,
                    axis_color,
                    -PI / 2.0, // Vertical text
                );
            }
        }
    }
}

/// Render a 3D surface
///
/// # Arguments
///
/// * `data` - The surface data to render
/// * `config` - Configuration for rendering
/// * `width` - Element width in pixels
/// * `height` - Element height in pixels
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::surface::{SurfaceData, SurfaceConfig, render_surface, ColorScaleType};
///
/// let data = SurfaceData::from_z_function(
///     (-2.0, 2.0),
///     (-2.0, 2.0),
///     50,
///     |x, y| (x * x + y * y).sin(),
/// );
///
/// let element = render_surface(
///     &data,
///     SurfaceConfig::new()
///         .isometric()
///         .rotation(30.0, 45.0)
///         .color_scale(ColorScaleType::Viridis)
///         .opacity(0.9)
///         .wireframe(true),
///     600.0,
///     400.0,
/// );
/// ```
pub fn render_surface(
    data: &SurfaceData,
    config: SurfaceConfig,
    width: f32,
    height: f32,
) -> SurfaceElement {
    SurfaceElement::new(data, config, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_scales() {
        // Test that all color scales produce valid colors
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let _ = viridis_color(t);
            let _ = heat_color(t);
            let _ = cool_color(t);
            let _ = spectral_color(t);
            let _ = grayscale_color(t);
        }
    }

    #[test]
    fn test_config_builder() {
        let config = SurfaceConfig::new()
            .isometric()
            .rotation(45.0, 60.0)
            .zoom(1.5)
            .color_scale(ColorScaleType::Heat)
            .opacity(0.8)
            .wireframe(true)
            .lighting(true)
            .ambient(0.3)
            .diffuse(0.7);

        assert_eq!(config.projection_type, ProjectionType::Isometric);
        assert_eq!(config.camera.rotation_x, 45.0);
        assert_eq!(config.camera.rotation_z, 60.0);
        assert_eq!(config.camera.zoom, 1.5);
        assert_eq!(config.color_scale, ColorScaleType::Heat);
        assert!((config.opacity - 0.8).abs() < 1e-6);
        assert!(config.wireframe);
        assert!(config.lighting);
        assert!((config.ambient - 0.3).abs() < 1e-6);
        assert!((config.diffuse - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_vec() {
        let v = normalize_vec((3.0, 4.0, 0.0));
        let len = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
        assert!((len - 1.0).abs() < 1e-10);
    }
}
