//! Deterministic SVG export and accessibility metadata for mesh plots.

use super::{MeshPlot, MeshPlotPick, MeshPlotView, MeshRenderMode, Wireframe};
use crate::static_export::{draw_title, escape_xml, svg_header, validate_plot_area};
use crate::{ChartAccessibilitySummary, ChartError, ColorRange, ScaleType, StaticSvgOptions};
#[cfg(feature = "gpu-3d")]
use d3rs::gpu3d::OrbitControls;
use d3rs::mesh::{
    ContourBand, ContourLevels, CoordinateAxis, MarchingTriangles, MeshTopology,
    MeshValidationError, ScalarAssociation, ScalarField, TriangleMesh, project_2d,
};
#[cfg(feature = "gpu-3d")]
use glam::{Vec3, Vec4};
#[cfg(feature = "gpu-3d")]
use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use std::fmt::Write as _;

const MICRO_BAND_COUNT: usize = 64;
const ZERO_LENGTH_EPSILON: f64 = 1e-24;

#[derive(Debug, Clone, Copy)]
struct SvgLayout {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl SvgLayout {
    fn new(options: StaticSvgOptions) -> Result<Self, ChartError> {
        let left = options.margin_left as f64;
        let top = options.margin_top as f64;
        let right = options.width as f64 - options.margin_right as f64;
        let bottom = options.height as f64 - options.margin_bottom as f64;
        if right <= left || bottom <= top {
            return Err(ChartError::InvalidDimension {
                field: "width",
                value: options.width,
            });
        }
        Ok(Self {
            left,
            top,
            right,
            bottom,
        })
    }

    fn width(self) -> f64 {
        self.right - self.left
    }

    fn height(self) -> f64 {
        self.bottom - self.top
    }
}

#[derive(Debug, Clone, Copy)]
struct Projector {
    min: [f64; 2],
    extent: [f64; 2],
    origin: [f64; 2],
    size: [f64; 2],
    container_origin: [f64; 2],
    container_size: [f64; 2],
    equal_aspect: bool,
}

impl Projector {
    fn new(points: &[[f64; 2]], layout: SvgLayout, equal_aspect: bool) -> Self {
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        for point in points {
            min[0] = min[0].min(point[0]);
            min[1] = min[1].min(point[1]);
            max[0] = max[0].max(point[0]);
            max[1] = max[1].max(point[1]);
        }

        let extent = [
            (max[0] - min[0]).max(1.0e-12),
            (max[1] - min[1]).max(1.0e-12),
        ];
        let available = [layout.width(), layout.height()];
        let (origin, size) =
            fitted_frame([layout.left, layout.top], available, extent, equal_aspect);

        Self {
            min,
            extent,
            origin,
            size,
            container_origin: [layout.left, layout.top],
            container_size: available,
            equal_aspect,
        }
    }

    fn point(self, point: [f64; 2]) -> [f64; 2] {
        let x = (point[0] - self.min[0]) / self.extent[0];
        let y = (point[1] - self.min[1]) / self.extent[1];
        [
            self.origin[0] + x * self.size[0],
            self.origin[1] + (1.0 - y) * self.size[1],
        ]
    }

    fn with_viewport(mut self, x: [f64; 2], y: [f64; 2]) -> Self {
        self.min = [x[0], y[0]];
        self.extent = [
            (x[1] - x[0]).abs().max(1.0e-12),
            (y[1] - y[0]).abs().max(1.0e-12),
        ];
        (self.origin, self.size) = fitted_frame(
            self.container_origin,
            self.container_size,
            self.extent,
            self.equal_aspect,
        );
        self
    }

    fn frame(self) -> SvgLayout {
        SvgLayout {
            left: self.origin[0],
            top: self.origin[1],
            right: self.origin[0] + self.size[0],
            bottom: self.origin[1] + self.size[1],
        }
    }
}

fn fitted_frame(
    origin: [f64; 2],
    available: [f64; 2],
    extent: [f64; 2],
    equal_aspect: bool,
) -> ([f64; 2], [f64; 2]) {
    if !equal_aspect {
        return (origin, available);
    }
    let scale = (available[0] / extent[0]).min(available[1] / extent[1]);
    let size = [extent[0] * scale, extent[1] * scale];
    (
        [
            origin[0] + (available[0] - size[0]) * 0.5,
            origin[1] + (available[1] - size[1]) * 0.5,
        ],
        size,
    )
}

impl MeshPlot {
    /// Export the current mesh-plot view as deterministic SVG.
    pub fn to_svg(&self) -> Result<String, ChartError> {
        let (width, height) = self.chart_size.layout_dimensions();
        self.to_svg_with_options(StaticSvgOptions::new(width, height))
    }

    /// Export the current mesh-plot view using explicit SVG dimensions/margins.
    pub fn to_svg_with_options(&self, options: StaticSvgOptions) -> Result<String, ChartError> {
        validate_for_export(self)?;
        crate::validate::validate_dimensions(options.width, options.height)?;
        validate_plot_area(options)?;
        let layout = SvgLayout::new(options)?;
        #[cfg(feature = "gpu-3d")]
        if matches!(
            self.view,
            MeshPlotView::Surface3d | MeshPlotView::AxisymmetricRevolve(_)
        ) {
            return self.to_svg_3d_with_layout(options, layout);
        }
        let (horizontal, vertical) = view_axes(&self.view);
        let points: Vec<[f64; 2]> = self
            .mesh
            .positions
            .iter()
            .copied()
            .map(|point| project_2d(horizontal, vertical, point))
            .collect();
        let full_x_domain = domain(&points, 0);
        let full_y_domain = domain(&points, 1);
        let (x_domain, y_domain) = live_viewport(self, full_x_domain, full_y_domain);
        let projector = Projector::new(&points, layout, self.axes.equal_aspect)
            .with_viewport(x_domain, y_domain);
        let value_range = resolved_value_range(self)?;
        let summary = self.accessibility_summary();

        let mut svg = svg_header(options);
        draw_title(&mut svg, self.title.as_deref(), options.width);
        if self.title.is_none() {
            let _ = writeln!(
                svg,
                "<title>{}</title>",
                escape_xml(&summary.accessible_label())
            );
        }
        let _ = writeln!(svg, "<desc>{}</desc>", escape_xml(&summary.description));
        let _ = writeln!(
            svg,
            "<g class=\"gpui-px-mesh-plot\" data-viewport=\"current\" data-view=\"{}\" data-x-range=\"{:.6},{:.6}\" data-y-range=\"{:.6},{:.6}\">",
            view_name(&self.view),
            x_domain[0],
            x_domain[1],
            y_domain[0],
            y_domain[1]
        );
        if let Some([min, max]) = value_range {
            let _ = writeln!(
                svg,
                "<metadata class=\"gpui-px-mesh-value-range\" min=\"{min:.6}\" max=\"{max:.6}\" association=\"{}\"/>",
                self.field
                    .as_ref()
                    .map(|field| association_name(field.association))
                    .unwrap_or("none")
            );
        }

        draw_mesh_axes(
            &mut svg,
            options,
            projector.frame(),
            x_domain,
            y_domain,
            horizontal,
            vertical,
        );
        render_mode(self, &mut svg, &projector, value_range)?;

        let topology = MeshTopology::build(&self.mesh.triangles);
        if self.wireframe == Wireframe::Overlay || matches!(self.mode, MeshRenderMode::Mesh) {
            render_wireframe(
                &mut svg, &self.mesh, &topology, projector, horizontal, vertical,
            );
        }

        if let Some(colorbar) = &self.colorbar {
            let colorbar_range = value_range.map_or(self.color_range, |range| ColorRange::Fixed {
                min: range[0],
                max: range[1],
            });
            let colorbar = colorbar
                .clone()
                .color_scale(self.color_scale.clone())
                .range(colorbar_range);
            svg.push_str(&colorbar.to_svg(layout.right + 16.0, layout.top, layout.height()));
        }

        let live_selection = self
            .state
            .as_ref()
            .and_then(|state| {
                state
                    .try_borrow()
                    .ok()
                    .and_then(|state| state.selection.clone())
            })
            .or_else(|| self.selection.clone());
        render_selection(
            &mut svg,
            live_selection.as_ref(),
            &self.mesh,
            projector,
            horizontal,
            vertical,
        );
        svg.push_str("</g>\n</svg>\n");
        Ok(svg)
    }

    /// Export a 3D or revolved mesh plot as PNG at an explicit display scale.
    ///
    /// The image uses the same current camera, scalar range, wireframe, and
    /// mesh/revolve preparation as the interactive 3D scene. It deliberately
    /// creates a short-lived scene snapshot, so exporting cannot mutate live
    /// interaction state. Platforms with framebuffer readback may replace the
    /// deterministic CPU fallback behind this API without changing callers.
    #[cfg(feature = "gpu-3d")]
    pub fn to_png(&self, scale: f32) -> Result<Vec<u8>, ChartError> {
        validate_for_export(self)?;
        if !scale.is_finite() || scale <= 0.0 {
            return Err(ChartError::InvalidDimension {
                field: "png scale",
                value: scale,
            });
        }
        if !matches!(
            self.view,
            MeshPlotView::Surface3d | MeshPlotView::AxisymmetricRevolve(_)
        ) {
            return Err(ChartError::UnsupportedView {
                view: view_name(&self.view),
                reason: "PNG export is currently available for 3D mesh views",
            });
        }
        let (base_width, base_height) = self.chart_size.layout_dimensions();
        let width = scaled_png_dimension(base_width, scale, "png width")?;
        let height = scaled_png_dimension(base_height, scale, "png height")?;
        let (mesh, field) = super::mesh_plot_chart::render_3d_mesh_and_field_for_view(
            &self.mesh,
            self.field.as_ref(),
            &self.view,
        )?;
        let range = resolved_value_range(self)?;
        let scene = super::mesh_plot_chart::build_retained_3d_scene_state(
            &mesh,
            field.as_ref(),
            &self.mode,
            self.wireframe,
            &self.color_scale,
            range,
        );
        let mut camera = self
            .state
            .as_ref()
            .and_then(|state| state.try_borrow().ok().map(|state| state.camera.clone()))
            .unwrap_or_else(|| {
                let mut orbit = OrbitControls::default();
                orbit.fit_to_bounds(
                    d3rs::mesh::MeshBounds::from_positions(&mesh.positions),
                    width as f32 / height.max(1) as f32,
                );
                orbit.to_camera()
            });
        camera.aspect = width as f32 / height.max(1) as f32;
        {
            let mut scene_state = scene.borrow_mut();
            let origin = scene_state
                .upload
                .as_ref()
                .map(|upload| upload.origin)
                .unwrap_or([0.0; 3]);
            // The CPU fallback consumes already-rebased upload positions, so
            // compose the same model-origin translation used by GPU backends.
            scene_state.view_transform = (camera.view_projection_matrix()
                * glam::Mat4::from_translation(Vec3::new(
                    origin[0] as f32,
                    origin[1] as f32,
                    origin[2] as f32,
                )))
            .to_cols_array_2d();
        }
        let image = {
            let scene = scene.borrow();
            d3rs::mesh::gpu::render_offscreen(scene.upload.as_ref(), &scene, width, height)
        };
        let mut output = Vec::new();
        PngEncoder::new(&mut output)
            .write_image(image.as_raw(), width, height, ColorType::Rgba8.into())
            .map_err(|error| ChartError::ImageExport {
                reason: error.to_string(),
            })?;
        Ok(output)
    }

    #[cfg(feature = "gpu-3d")]
    fn to_svg_3d_with_layout(
        &self,
        options: StaticSvgOptions,
        layout: SvgLayout,
    ) -> Result<String, ChartError> {
        let (mesh, field) = match &self.view {
            MeshPlotView::Surface3d => (self.mesh.clone(), self.field.clone()),
            MeshPlotView::AxisymmetricRevolve(spec) => {
                let revolved = d3rs::mesh::revolve(&self.mesh, spec)?;
                let field = self
                    .field
                    .as_ref()
                    .map(|field| super::picking3d::revolved_field(field, &revolved));
                (revolved.mesh, field)
            }
            _ => unreachable!(),
        };
        let mut camera = self
            .state
            .as_ref()
            .and_then(|state| state.try_borrow().ok().map(|state| state.camera.clone()))
            .unwrap_or_else(|| {
                let mut orbit = OrbitControls::default();
                orbit.fit_to_bounds(
                    d3rs::mesh::MeshBounds::from_positions(&mesh.positions),
                    layout.width() as f32 / layout.height() as f32,
                );
                orbit.to_camera()
            });
        camera.aspect = (layout.width() / layout.height()).max(f64::EPSILON) as f32;
        let project_clip = |clip: Vec4| project_clip_to_svg(clip, layout);
        let clip_point = |point: [f64; 3]| {
            camera.view_projection_matrix()
                * Vec3::new(point[0] as f32, point[1] as f32, point[2] as f32).extend(1.0)
        };
        let range = field
            .as_ref()
            .and_then(finite_field_range)
            .and_then(|r| self.color_range.resolve(r[0], r[1]).ok());
        let contour_levels = match &self.mode {
            MeshRenderMode::FilledContours { levels }
            | MeshRenderMode::Isolines { levels }
            | MeshRenderMode::FillAndIsolines { levels } => range
                .map(|range| levels.resolve(range).map_err(ChartError::from))
                .transpose()?,
            _ => None,
        };
        let mut triangles = Vec::new();
        for (index, triangle) in mesh.triangles.iter().enumerate() {
            let (Some(a), Some(b), Some(c)) = (
                mesh.positions
                    .get(triangle[0] as usize)
                    .copied()
                    .map(clip_point),
                mesh.positions
                    .get(triangle[1] as usize)
                    .copied()
                    .map(clip_point),
                mesh.positions
                    .get(triangle[2] as usize)
                    .copied()
                    .map(clip_point),
            ) else {
                continue;
            };
            let value = field.as_ref().and_then(|field| match field.association {
                ScalarAssociation::Cell => (field
                    .valid
                    .as_ref()
                    .is_none_or(|valid| valid.get(index) == Some(&true)))
                .then(|| field.values.get(index).copied())
                .flatten()
                .filter(|value| value.is_finite()),
                ScalarAssociation::Vertex => triangle
                    .iter()
                    .all(|vertex| {
                        field
                            .valid
                            .as_ref()
                            .is_none_or(|valid| valid.get(*vertex as usize) == Some(&true))
                    })
                    .then(|| {
                        (field.values[triangle[0] as usize]
                            + field.values[triangle[1] as usize]
                            + field.values[triangle[2] as usize])
                            / 3.0
                    })
                    .filter(|value| value.is_finite()),
            });
            if field.is_some() && value.is_none() {
                continue;
            }
            for clipped in clip_triangle_to_frustum([a, b, c]) {
                let Some(a) = project_clip(clipped[0]) else {
                    continue;
                };
                let Some(b) = project_clip(clipped[1]) else {
                    continue;
                };
                let Some(c) = project_clip(clipped[2]) else {
                    continue;
                };
                triangles.push((
                    (a[2] + b[2] + c[2]) / 3.0,
                    [[a[0], a[1]], [b[0], b[1]], [c[0], c[1]]],
                    value,
                ));
            }
        }
        triangles.sort_by(|a, b| b.0.total_cmp(&a.0));
        let summary = self.accessibility_summary();
        let mut svg = svg_header(options);
        draw_title(&mut svg, self.title.as_deref(), options.width);
        let _ = writeln!(
            svg,
            "<desc>{}</desc><g class=\"gpui-px-mesh-plot-3d\" data-camera=\"current\" data-view=\"{}\">",
            escape_xml(&summary.description),
            view_name(&self.view)
        );
        let draw_surface = !matches!(self.mode, MeshRenderMode::Isolines { .. });
        for (_, points, value) in triangles {
            if !draw_surface {
                continue;
            }
            let value = if matches!(self.mode, MeshRenderMode::FilledContours { .. }) {
                value.map(|value| contour_band_midpoint(value, contour_levels.as_deref()))
            } else {
                value
            };
            let color = match (value, range) {
                (Some(value), Some(range)) => {
                    self.color_scale.map(normalize(value, range)).to_hex()
                }
                _ => "#596371".into(),
            };
            write_triangle_path(
                &mut svg,
                "gpui-px-mesh-3d-triangle",
                &points,
                &color,
                None,
                None,
            );
        }
        if matches!(
            self.mode,
            MeshRenderMode::Isolines { .. } | MeshRenderMode::FillAndIsolines { .. }
        ) && let (Some(field), Some(levels)) = (field.as_ref(), contour_levels.as_deref())
            && field.association == ScalarAssociation::Vertex
        {
            svg.push_str("<g class=\"gpui-px-mesh-3d-isolines\" fill=\"none\" stroke=\"#141820\" stroke-width=\"1\">\n");
            for (cell_index, triangle) in mesh.triangles.iter().enumerate() {
                let Some(points) = triangle_world_positions(&mesh, *triangle) else {
                    continue;
                };
                let Some(values) = triangle_vertex_values(field, *triangle) else {
                    continue;
                };
                if field.valid.as_ref().is_some_and(|valid| {
                    triangle
                        .iter()
                        .any(|&vertex| valid.get(vertex as usize) != Some(&true))
                }) {
                    continue;
                }
                for &level in levels {
                    let Some((start, end)) = isoline_segment_3d(points, values, level) else {
                        continue;
                    };
                    let Some((start, end)) =
                        clip_line_to_frustum(clip_point(start), clip_point(end)).and_then(
                            |(start, end)| Some((project_clip(start)?, project_clip(end)?)),
                        )
                    else {
                        continue;
                    };
                    let _ = writeln!(
                        svg,
                        "<path class=\"gpui-px-mesh-3d-isoline\" data-cell=\"{cell_index}\" data-level=\"{level:.6}\" d=\"M {:.2},{:.2} L {:.2},{:.2}\"/>",
                        start[0], start[1], end[0], end[1]
                    );
                }
            }
            svg.push_str("</g>\n");
        }
        if self.wireframe == Wireframe::Overlay || matches!(self.mode, MeshRenderMode::Mesh) {
            svg.push_str("<g class=\"gpui-px-mesh-wireframe\" fill=\"none\" stroke=\"#222\">\n");
            for edge in MeshTopology::build(&mesh.triangles).unique_edges {
                let Some(start) = mesh
                    .positions
                    .get(edge[0] as usize)
                    .copied()
                    .map(clip_point)
                else {
                    continue;
                };
                let Some(end) = mesh
                    .positions
                    .get(edge[1] as usize)
                    .copied()
                    .map(clip_point)
                else {
                    continue;
                };
                let Some((start, end)) = clip_line_to_frustum(start, end)
                    .and_then(|(start, end)| Some((project_clip(start)?, project_clip(end)?)))
                else {
                    continue;
                };
                let _ = writeln!(
                    svg,
                    "<path class=\"gpui-px-mesh-wireframe-edge\" d=\"M {:.2},{:.2} L {:.2},{:.2}\" stroke-width=\"1\"/>",
                    start[0], start[1], end[0], end[1]
                );
            }
            svg.push_str("</g>\n");
        }
        let bounds = d3rs::mesh::MeshBounds::from_positions(&mesh.positions);
        let axis_origin = [bounds.min[0], bounds.min[1], bounds.min[2]];
        let axes = [
            (
                "X",
                [bounds.max[0], bounds.min[1], bounds.min[2]],
                "#d14b45",
            ),
            (
                "Y",
                [bounds.min[0], bounds.max[1], bounds.min[2]],
                "#359653",
            ),
            (
                "Z",
                [bounds.min[0], bounds.min[1], bounds.max[2]],
                "#357edb",
            ),
        ];
        svg.push_str("<g class=\"gpui-px-mesh-3d-axes\" fill=\"none\" stroke-width=\"1.25\">\n");
        for (label, endpoint, color) in axes {
            let Some((start, end)) =
                clip_line_to_frustum(clip_point(axis_origin), clip_point(endpoint))
                    .and_then(|(start, end)| Some((project_clip(start)?, project_clip(end)?)))
            else {
                continue;
            };
            let _ = writeln!(
                svg,
                "<path class=\"gpui-px-mesh-3d-axis\" data-axis=\"{label}\" d=\"M {:.2},{:.2} L {:.2},{:.2}\" stroke=\"{color}\"/><text class=\"gpui-px-mesh-3d-axis-label\" x=\"{:.2}\" y=\"{:.2}\" fill=\"{color}\">{label}</text>",
                start[0],
                start[1],
                end[0],
                end[1],
                end[0] + 3.0,
                end[1] - 3.0
            );
        }
        svg.push_str("</g>\n");
        let selection = self
            .state
            .as_ref()
            .and_then(|state| {
                state
                    .try_borrow()
                    .ok()
                    .and_then(|state| state.selection.clone())
            })
            .or_else(|| self.selection.clone());
        if let Some(selection) = selection
            && let Some(point) = project_clip(clip_point(selection.world_position))
        {
            let title = format!(
                "Cell {}{}{}",
                selection.cell_index,
                selection
                    .cell_id
                    .map_or_else(String::new, |id| format!(" (id {id})")),
                selection
                    .displayed_value
                    .map_or_else(String::new, |value| format!("; value {value:.6}")),
            );
            let _ = writeln!(
                svg,
                "<circle class=\"gpui-px-mesh-selection\" data-selected=\"true\" cx=\"{:.2}\" cy=\"{:.2}\" r=\"4\" fill=\"none\" stroke=\"#ff8c00\" stroke-width=\"2\"><title>{}</title></circle>",
                point[0],
                point[1],
                escape_xml(&title)
            );
        }
        svg.push_str("</g>\n</svg>\n");
        Ok(svg)
    }

    /// Return structured accessibility metadata for this mesh plot.
    pub fn accessibility_summary(&self) -> ChartAccessibilitySummary {
        let (horizontal, vertical) = view_axes(&self.view);
        let points: Vec<[f64; 2]> = self
            .mesh
            .positions
            .iter()
            .copied()
            .map(|point| project_2d(horizontal, vertical, point))
            .collect();
        let x_range = finite_domain(&points, 0);
        let y_range = finite_domain(&points, 1);
        let (x_range, y_range) = match (x_range, y_range) {
            (Some(x), Some(y)) => {
                let (x, y) = live_viewport(self, x, y);
                (Some(x), Some(y))
            }
            (x, y) => (x, y),
        };
        let value_range = self.field.as_ref().and_then(finite_field_range);
        let title = self.title.clone();
        let name = title.as_deref().unwrap_or("Mesh plot");
        let field_text = self.field.as_ref().map_or_else(
            || "No scalar field is active.".to_string(),
            |field| {
                let unit = field
                    .unit
                    .as_deref()
                    .map_or_else(String::new, |unit| format!(" ({unit})"));
                format!(
                    "Field {}{unit}, {} association, {} values.",
                    field.label,
                    association_name(field.association),
                    field.values.len()
                )
            },
        );
        let live_selection = self
            .state
            .as_ref()
            .and_then(|state| {
                state
                    .try_borrow()
                    .ok()
                    .and_then(|state| state.selection.clone())
            })
            .or_else(|| self.selection.clone());
        let selected = live_selection.as_ref().map_or_else(
            || "No mesh element is selected.".to_string(),
            |pick| {
                format!(
                    "Selected cell {}{}{}.",
                    pick.cell_index,
                    pick.cell_id
                        .map_or_else(String::new, |id| format!(" (id {id})")),
                    pick.displayed_value
                        .map_or_else(String::new, |value| format!("; value {value:.3}")),
                )
            },
        );
        let controls = match self.interactions {
            super::PlotInteractions::InspectAndNavigate => {
                "Available controls: inspect, select, pan, zoom, fit, and reset."
            }
            super::PlotInteractions::None => "Available controls: none.",
        };
        let description = format!(
            "{name}: {} view with {} vertices and {} triangles. {}, X range {:.3} to {:.3}, Y range {:.3} to {:.3}. {field_text} Displayed value range {}. Wireframe {}. {selected} {controls}",
            view_name(&self.view),
            self.mesh.positions.len(),
            self.mesh.triangles.len(),
            field_text,
            x_range.map_or(0.0, |range| range[0]),
            x_range.map_or(0.0, |range| range[1]),
            y_range.map_or(0.0, |range| range[0]),
            y_range.map_or(0.0, |range| range[1]),
            self.field
                .as_ref()
                .and_then(|_| resolved_value_range(self).ok().flatten())
                .map_or_else(
                    || "unavailable".to_string(),
                    |range| format!("{:.3} to {:.3}", range[0], range[1])
                ),
            if self.wireframe == Wireframe::Overlay {
                "enabled."
            } else {
                "hidden."
            },
        );

        ChartAccessibilitySummary {
            chart_type: "mesh_plot",
            title,
            series_count: usize::from(self.field.is_some()),
            datum_count: self
                .field
                .as_ref()
                .map_or(self.mesh.triangles.len(), |field| field.values.len()),
            x_range,
            y_range,
            value_range,
            x_scale: Some(ScaleType::Linear),
            y_scale: Some(ScaleType::Linear),
            series_labels: self.field.as_ref().map_or_else(
                || vec!["Mesh geometry".to_string()],
                |field| vec![field.label.to_string()],
            ),
            description,
        }
    }
}

#[cfg(feature = "gpu-3d")]
fn scaled_png_dimension(base: f32, scale: f32, field: &'static str) -> Result<u32, ChartError> {
    let scaled = base * scale;
    if !scaled.is_finite() || scaled <= 0.0 || scaled > 16_384.0 {
        return Err(ChartError::InvalidDimension {
            field,
            value: scaled,
        });
    }
    Ok(scaled.round().max(1.0) as u32)
}

fn validate_for_export(plot: &MeshPlot) -> Result<(), ChartError> {
    plot.mesh.validate()?;
    if let Some(field) = &plot.field {
        field.validate(&plot.mesh)?;
    }
    if !matches!(plot.mode, MeshRenderMode::Mesh) && plot.field.is_none() {
        return Err(ChartError::InvalidData {
            field: "field",
            reason: "scalar render mode requires a field",
        });
    }
    if matches!(
        plot.mode,
        MeshRenderMode::FilledContours { .. }
            | MeshRenderMode::Isolines { .. }
            | MeshRenderMode::FillAndIsolines { .. }
    ) && plot
        .field
        .as_ref()
        .is_some_and(|field| field.association != ScalarAssociation::Vertex)
    {
        return Err(MeshValidationError::ContoursRequireVertexField.into());
    }
    if let MeshPlotView::AxisymmetricSection { radial, .. } = plot.view {
        for (index, position) in plot.mesh.positions.iter().enumerate() {
            let radius = radial.component(*position);
            if radius < -1e-12 {
                return Err(MeshValidationError::InvalidRadius {
                    index,
                    value: radius,
                }
                .into());
            }
        }
    }
    if let Some(range) = plot.field.as_ref().and_then(finite_field_range) {
        plot.color_range.resolve(range[0], range[1])?;
    }
    Ok(())
}

fn resolved_value_range(plot: &MeshPlot) -> Result<Option<[f64; 2]>, ChartError> {
    plot.field
        .as_ref()
        .and_then(finite_field_range)
        .map_or(Ok(None), |range| {
            plot.color_range.resolve(range[0], range[1]).map(Some)
        })
}

fn finite_field_range(field: &ScalarField) -> Option<[f64; 2]> {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut seen = false;
    for (index, value) in field.values.iter().enumerate() {
        if field
            .valid
            .as_ref()
            .is_some_and(|valid| valid.get(index) != Some(&true))
        {
            continue;
        }
        if value.is_finite() {
            min = min.min(*value);
            max = max.max(*value);
            seen = true;
        }
    }
    seen.then_some([min, max])
}

fn view_axes(view: &MeshPlotView) -> (CoordinateAxis, CoordinateAxis) {
    match view {
        MeshPlotView::Planar {
            horizontal,
            vertical,
        } => (*horizontal, *vertical),
        MeshPlotView::AxisymmetricSection { radial, axial } => (*radial, *axial),
        MeshPlotView::AxisymmetricRevolve(spec) => (spec.radial, spec.axial),
        MeshPlotView::Surface3d => (CoordinateAxis::X, CoordinateAxis::Y),
    }
}

fn view_name(view: &MeshPlotView) -> &'static str {
    match view {
        MeshPlotView::Planar { .. } => "planar",
        MeshPlotView::AxisymmetricSection { .. } => "axisymmetric-section",
        MeshPlotView::AxisymmetricRevolve(_) => "axisymmetric-revolve",
        MeshPlotView::Surface3d => "surface3d",
    }
}

fn live_viewport(plot: &MeshPlot, full_x: [f64; 2], full_y: [f64; 2]) -> ([f64; 2], [f64; 2]) {
    let Some(state) = plot.state.as_ref() else {
        return (full_x, full_y);
    };
    let Ok(state) = state.try_borrow() else {
        return (full_x, full_y);
    };
    let (x0, x1) = state.interaction.x_domain();
    let (y0, y1) = state.interaction.y_domain();
    if ![x0, x1, y0, y1].iter().all(|value| value.is_finite()) || x1 <= x0 || y1 <= y0 {
        return (full_x, full_y);
    }
    ([x0, x1], [y0, y1])
}

fn association_name(association: ScalarAssociation) -> &'static str {
    match association {
        ScalarAssociation::Vertex => "vertex",
        ScalarAssociation::Cell => "cell",
    }
}

fn axis_name(axis: CoordinateAxis) -> &'static str {
    match axis {
        CoordinateAxis::X => "X",
        CoordinateAxis::Y => "Y",
        CoordinateAxis::Z => "Z",
    }
}

fn domain(points: &[[f64; 2]], axis: usize) -> [f64; 2] {
    finite_domain(points, axis).unwrap_or([0.0, 1.0])
}

fn finite_domain(points: &[[f64; 2]], axis: usize) -> Option<[f64; 2]> {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for point in points {
        if point[axis].is_finite() {
            min = min.min(point[axis]);
            max = max.max(point[axis]);
        }
    }
    min.is_finite().then_some([min, max])
}

fn draw_mesh_axes(
    svg: &mut String,
    options: StaticSvgOptions,
    layout: SvgLayout,
    x_domain: [f64; 2],
    y_domain: [f64; 2],
    horizontal: CoordinateAxis,
    vertical: CoordinateAxis,
) {
    if !options.show_axes {
        return;
    }
    let grid_color = "#e6e6e6";
    let axis_color = "#666";
    for step in 0..=4 {
        let t = step as f64 / 4.0;
        let x = layout.left + layout.width() * t;
        let y = layout.bottom - layout.height() * t;
        let x_value = x_domain[0] + (x_domain[1] - x_domain[0]) * t;
        let y_value = y_domain[0] + (y_domain[1] - y_domain[0]) * t;
        let _ = writeln!(
            svg,
            "<line class=\"gpui-px-mesh-grid\" x1=\"{x:.2}\" y1=\"{:.2}\" x2=\"{x:.2}\" y2=\"{:.2}\" stroke=\"{grid_color}\" stroke-width=\"1\"/>",
            layout.top, layout.bottom
        );
        let _ = writeln!(
            svg,
            "<line class=\"gpui-px-mesh-grid\" x1=\"{:.2}\" y1=\"{y:.2}\" x2=\"{:.2}\" y2=\"{y:.2}\" stroke=\"{grid_color}\" stroke-width=\"1\"/>",
            layout.left, layout.right
        );
        let _ = writeln!(
            svg,
            "<text class=\"gpui-px-mesh-axis-label\" x=\"{x:.2}\" y=\"{:.2}\" text-anchor=\"middle\" fill=\"{axis_color}\">{x_value:.3}</text>",
            layout.bottom + 18.0
        );
        let _ = writeln!(
            svg,
            "<text class=\"gpui-px-mesh-axis-label\" x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\" fill=\"{axis_color}\">{y_value:.3}</text>",
            layout.left - 6.0,
            y + 3.0
        );
    }
    let _ = writeln!(
        svg,
        "<line class=\"gpui-px-mesh-axis\" x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{axis_color}\" stroke-width=\"1\"/>",
        layout.left, layout.bottom, layout.right, layout.bottom
    );
    let _ = writeln!(
        svg,
        "<line class=\"gpui-px-mesh-axis\" x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{axis_color}\" stroke-width=\"1\"/>",
        layout.left, layout.top, layout.left, layout.bottom
    );
    let _ = writeln!(
        svg,
        "<text class=\"gpui-px-mesh-axis-title\" x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\">{}</text>",
        (layout.left + layout.right) * 0.5,
        layout.bottom + 34.0,
        axis_name(horizontal)
    );
    let _ = writeln!(
        svg,
        "<text class=\"gpui-px-mesh-axis-title\" x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" transform=\"rotate(-90 {:.2} {:.2})\">{}</text>",
        layout.left - 38.0,
        (layout.top + layout.bottom) * 0.5,
        layout.left - 38.0,
        (layout.top + layout.bottom) * 0.5,
        axis_name(vertical)
    );
}

fn render_mode(
    plot: &MeshPlot,
    svg: &mut String,
    projector: &Projector,
    value_range: Option<[f64; 2]>,
) -> Result<(), ChartError> {
    let Some(field) = plot.field.as_ref() else {
        return Ok(());
    };
    let Some(range) = value_range else {
        return Ok(());
    };
    match &plot.mode {
        MeshRenderMode::Mesh => {}
        MeshRenderMode::ScalarFill { .. } => {
            render_scalar_fill(svg, plot, field, projector, range)?;
        }
        MeshRenderMode::FilledContours { levels } => {
            render_contours(svg, plot, field, projector, range, levels, false)?;
        }
        MeshRenderMode::Isolines { levels } => {
            render_contours(svg, plot, field, projector, range, levels, true)?;
        }
        MeshRenderMode::FillAndIsolines { levels } => {
            render_contours(svg, plot, field, projector, range, levels, false)?;
            render_contours(svg, plot, field, projector, range, levels, true)?;
        }
    }
    Ok(())
}

fn render_scalar_fill(
    svg: &mut String,
    plot: &MeshPlot,
    field: &ScalarField,
    projector: &Projector,
    range: [f64; 2],
) -> Result<(), ChartError> {
    if field.association == ScalarAssociation::Cell {
        render_cell_fill(svg, plot, field, projector, range);
        return Ok(());
    }
    let topology = MeshTopology::build(&plot.mesh.triangles);
    let (horizontal, vertical) = view_axes(&plot.view);
    let marching = MarchingTriangles::new(&plot.mesh, field, &topology, horizontal, vertical)?;
    let levels = linear_levels(range, MICRO_BAND_COUNT);
    render_bands(
        svg,
        &marching.filled_bands(&levels),
        projector,
        range,
        &plot.color_scale,
        "scalar",
    );
    Ok(())
}

fn render_contours(
    svg: &mut String,
    plot: &MeshPlot,
    field: &ScalarField,
    projector: &Projector,
    range: [f64; 2],
    requested: &ContourLevels,
    isolines: bool,
) -> Result<(), ChartError> {
    let levels = requested.resolve(range)?;
    let topology = MeshTopology::build(&plot.mesh.triangles);
    let (horizontal, vertical) = view_axes(&plot.view);
    let marching = MarchingTriangles::new(&plot.mesh, field, &topology, horizontal, vertical)?;
    if isolines {
        render_isolines(svg, &marching.isolines(&levels), projector);
    } else {
        let boundaries = with_range_bounds(&levels, range);
        render_bands(
            svg,
            &marching.filled_bands(&boundaries),
            projector,
            range,
            &plot.color_scale,
            "contour",
        );
    }
    Ok(())
}

fn render_cell_fill(
    svg: &mut String,
    plot: &MeshPlot,
    field: &ScalarField,
    projector: &Projector,
    range: [f64; 2],
) {
    svg.push_str("<g class=\"gpui-px-mesh-cell-fill\" data-association=\"cell\">\n");
    for (index, triangle) in plot.mesh.triangles.iter().enumerate() {
        if field
            .valid
            .as_ref()
            .is_some_and(|valid| valid.get(index) != Some(&true))
        {
            continue;
        }
        let value = field.values[index];
        let color = plot.color_scale.map(normalize(value, range)).to_hex();
        let points = triangle.map(|vertex| {
            projector.point(project_2d(
                view_axes(&plot.view).0,
                view_axes(&plot.view).1,
                plot.mesh.positions[vertex as usize],
            ))
        });
        write_triangle_path(svg, "gpui-px-mesh-cell", &points, &color, None, None);
    }
    svg.push_str("</g>\n");
}

fn render_bands(
    svg: &mut String,
    bands: &[ContourBand],
    projector: &Projector,
    range: [f64; 2],
    color_scale: &crate::ColorScale,
    class_name: &str,
) {
    for (band_index, band) in bands.iter().enumerate() {
        let lower = band.lower.unwrap_or(range[0]);
        let upper = band.upper.unwrap_or(range[1]);
        let color = color_scale
            .map(normalize((lower + upper) * 0.5, range))
            .to_hex();
        let _ = writeln!(
            svg,
            "<g class=\"gpui-px-mesh-{class_name}-band\" data-band-index=\"{band_index}\" data-lower=\"{lower:.6}\" data-upper=\"{upper:.6}\" fill=\"{color}\">"
        );
        for triangle in &band.triangles {
            let points = triangle.map(|vertex| projector.point(band.positions[vertex as usize]));
            write_triangle_path(svg, "gpui-px-mesh-band", &points, &color, None, None);
        }
        svg.push_str("</g>\n");
    }
}

fn write_triangle_path(
    svg: &mut String,
    class_name: &str,
    points: &[[f64; 2]; 3],
    color: &str,
    lower: Option<f64>,
    upper: Option<f64>,
) {
    if triangle_area2(points) <= ZERO_LENGTH_EPSILON {
        return;
    }
    let _ = write!(
        svg,
        "<path class=\"{class_name}\" d=\"M {:.2},{:.2} L {:.2},{:.2} L {:.2},{:.2} Z\" fill=\"{color}\"",
        points[0][0], points[0][1], points[1][0], points[1][1], points[2][0], points[2][1]
    );
    if let Some(lower) = lower {
        let _ = write!(svg, " data-lower=\"{lower:.6}\"");
    }
    if let Some(upper) = upper {
        let _ = write!(svg, " data-upper=\"{upper:.6}\"");
    }
    svg.push_str("/>\n");
}

/// Clip a homogeneous point to the SVG viewport after frustum clipping. The
/// camera uses WebGPU/Metal depth convention (`0 <= z <= w`).
#[cfg(feature = "gpu-3d")]
fn project_clip_to_svg(clip: Vec4, layout: SvgLayout) -> Option<[f64; 3]> {
    (clip.w > 1e-6).then(|| {
        let ndc = clip.truncate() / clip.w;
        [
            layout.left + (ndc.x as f64 + 1.0) * 0.5 * layout.width(),
            layout.top + (1.0 - (ndc.y as f64 + 1.0) * 0.5) * layout.height(),
            ndc.z as f64,
        ]
    })
}

/// Return the signed distance from `point` to each homogeneous clip plane.
/// A non-negative distance is inside. Keeping this in homogeneous space is
/// essential: discarding a triangle merely because one vertex is behind the
/// near plane loses all partially visible surfaces and wireframe edges.
#[cfg(feature = "gpu-3d")]
fn frustum_distances(point: Vec4) -> [f32; 6] {
    [
        point.w + point.x,
        point.w - point.x,
        point.w + point.y,
        point.w - point.y,
        point.z,
        point.w - point.z,
    ]
}

#[cfg(feature = "gpu-3d")]
fn clip_triangle_to_frustum(triangle: [Vec4; 3]) -> Vec<[Vec4; 3]> {
    let mut polygon = triangle.to_vec();
    for plane in 0..6 {
        if polygon.is_empty() {
            return Vec::new();
        }
        let mut clipped = Vec::with_capacity(polygon.len() + 1);
        let Some(mut previous) = polygon.last().copied() else {
            return Vec::new();
        };
        let mut previous_distance = frustum_distances(previous)[plane];
        for current in polygon {
            let current_distance = frustum_distances(current)[plane];
            let previous_inside = previous_distance >= 0.0;
            let current_inside = current_distance >= 0.0;
            if previous_inside != current_inside {
                let t = previous_distance / (previous_distance - current_distance);
                clipped.push(previous.lerp(current, t.clamp(0.0, 1.0)));
            }
            if current_inside {
                clipped.push(current);
            }
            previous = current;
            previous_distance = current_distance;
        }
        polygon = clipped;
    }
    if polygon.len() < 3 {
        return Vec::new();
    }
    (1..polygon.len() - 1)
        .map(|index| [polygon[0], polygon[index], polygon[index + 1]])
        .collect()
}

#[cfg(feature = "gpu-3d")]
fn clip_line_to_frustum(start: Vec4, end: Vec4) -> Option<(Vec4, Vec4)> {
    let start_distances = frustum_distances(start);
    let end_distances = frustum_distances(end);
    let (mut lower, mut upper) = (0.0_f32, 1.0_f32);
    for plane in 0..6 {
        let a = start_distances[plane];
        let b = end_distances[plane];
        if a < 0.0 && b < 0.0 {
            return None;
        }
        if (a < 0.0) != (b < 0.0) {
            let t = a / (a - b);
            if a < 0.0 {
                lower = lower.max(t);
            } else {
                upper = upper.min(t);
            }
        }
    }
    (lower <= upper).then(|| (start.lerp(end, lower), start.lerp(end, upper)))
}

#[cfg(feature = "gpu-3d")]
fn triangle_world_positions(mesh: &TriangleMesh, triangle: [u32; 3]) -> Option<[[f64; 3]; 3]> {
    Some([
        *mesh.positions.get(triangle[0] as usize)?,
        *mesh.positions.get(triangle[1] as usize)?,
        *mesh.positions.get(triangle[2] as usize)?,
    ])
}

#[cfg(feature = "gpu-3d")]
fn triangle_vertex_values(field: &ScalarField, triangle: [u32; 3]) -> Option<[f64; 3]> {
    let values = [
        *field.values.get(triangle[0] as usize)?,
        *field.values.get(triangle[1] as usize)?,
        *field.values.get(triangle[2] as usize)?,
    ];
    values
        .iter()
        .all(|value| value.is_finite())
        .then_some(values)
}

/// Mesh-native 3D equivalent of the documented marching-triangle tie-break:
/// an exactly-on-level endpoint is classified as above. It returns one
/// segment per non-degenerate triangle/level crossing.
#[cfg(feature = "gpu-3d")]
fn isoline_segment_3d(
    points: [[f64; 3]; 3],
    values: [f64; 3],
    level: f64,
) -> Option<([f64; 3], [f64; 3])> {
    let mut hits = Vec::with_capacity(2);
    for (a, b) in [(0usize, 1usize), (1, 2), (2, 0)] {
        if (values[a] >= level) == (values[b] >= level) {
            continue;
        }
        let t = (level - values[a]) / (values[b] - values[a]);
        hits.push(std::array::from_fn(|axis| {
            points[a][axis] + t * (points[b][axis] - points[a][axis])
        }));
    }
    (hits.len() == 2 && hits[0] != hits[1]).then(|| (hits[0], hits[1]))
}

fn contour_band_midpoint(value: f64, levels: Option<&[f64]>) -> f64 {
    let Some(levels) = levels else {
        return value;
    };
    levels
        .windows(2)
        .find(|band| value >= band[0] && value <= band[1])
        .map(|band| 0.5 * (band[0] + band[1]))
        .unwrap_or(value)
}

fn render_isolines(
    svg: &mut String,
    segments: &[d3rs::mesh::IsolineSegment],
    projector: &Projector,
) {
    svg.push_str("<g class=\"gpui-px-mesh-isolines\" fill=\"none\">\n");
    for segment in segments {
        let start = projector.point(segment.start);
        let end = projector.point(segment.end);
        let dx = start[0] - end[0];
        let dy = start[1] - end[1];
        if dx * dx + dy * dy <= ZERO_LENGTH_EPSILON {
            continue;
        }
        let _ = writeln!(
            svg,
            "<path class=\"gpui-px-mesh-isoline\" data-level=\"{:.6}\" d=\"M {:.2},{:.2} L {:.2},{:.2}\" stroke=\"#333\" stroke-width=\"1\"/>",
            segment.level, start[0], start[1], end[0], end[1]
        );
    }
    svg.push_str("</g>\n");
}

fn render_wireframe(
    svg: &mut String,
    mesh: &TriangleMesh,
    topology: &MeshTopology,
    projector: Projector,
    horizontal: CoordinateAxis,
    vertical: CoordinateAxis,
) {
    svg.push_str("<g class=\"gpui-px-mesh-wireframe\" fill=\"none\" stroke=\"#222\">\n");
    for edge in &topology.unique_edges {
        let start = projector.point(project_2d(
            horizontal,
            vertical,
            mesh.positions[edge[0] as usize],
        ));
        let end = projector.point(project_2d(
            horizontal,
            vertical,
            mesh.positions[edge[1] as usize],
        ));
        let _ = writeln!(
            svg,
            "<line class=\"gpui-px-mesh-wire\" data-edge=\"{},{}\" x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke-width=\"1\"/>",
            edge[0], edge[1], start[0], start[1], end[0], end[1]
        );
    }
    svg.push_str("</g>\n");
}

fn render_selection(
    svg: &mut String,
    selection: Option<&MeshPlotPick>,
    mesh: &TriangleMesh,
    projector: Projector,
    horizontal: CoordinateAxis,
    vertical: CoordinateAxis,
) {
    let Some(selection) = selection else {
        svg.push_str(
            "<g class=\"gpui-px-mesh-selection\" data-selected=\"false\"><title>No mesh element selected</title></g>\n",
        );
        return;
    };
    if selection.mesh_id.as_ref() != mesh.id.as_ref() {
        svg.push_str(
            "<g class=\"gpui-px-mesh-selection\" data-selected=\"false\"><title>Selection belongs to another mesh</title></g>\n",
        );
        return;
    }
    let Some(triangle) = mesh.triangles.get(selection.cell_index as usize) else {
        svg.push_str(
            "<g class=\"gpui-px-mesh-selection\" data-selected=\"false\"><title>Invalid mesh selection</title></g>\n",
        );
        return;
    };
    let Some(vertices) = triangle
        .iter()
        .map(|&index| mesh.positions.get(index as usize))
        .collect::<Option<Vec<_>>>()
    else {
        svg.push_str(
            "<g class=\"gpui-px-mesh-selection\" data-selected=\"false\"><title>Invalid mesh selection</title></g>\n",
        );
        return;
    };
    let points = vertices
        .iter()
        .map(|point| projector.point(project_2d(horizontal, vertical, **point)))
        .collect::<Vec<_>>();
    let tooltip = format!(
        "Cell {}{}{}",
        selection.cell_index,
        selection
            .cell_id
            .map_or_else(String::new, |id| format!(" (id {id})")),
        selection
            .displayed_value
            .map_or_else(String::new, |value| format!("; value {value:.6}")),
    );
    let _ = writeln!(
        svg,
        "<path class=\"gpui-px-mesh-selection\" data-selected=\"true\" d=\"M {:.2},{:.2} L {:.2},{:.2} L {:.2},{:.2} Z\" fill=\"none\" stroke=\"#ff8c00\" stroke-width=\"2\"><title>{}</title></path>",
        points[0][0],
        points[0][1],
        points[1][0],
        points[1][1],
        points[2][0],
        points[2][1],
        escape_xml(&tooltip)
    );
}

fn normalize(value: f64, range: [f64; 2]) -> f64 {
    ((value - range[0]) / (range[1] - range[0])).clamp(0.0, 1.0)
}

fn linear_levels(range: [f64; 2], bands: usize) -> Vec<f64> {
    (0..=bands)
        .map(|index| range[0] + (range[1] - range[0]) * index as f64 / bands as f64)
        .collect()
}

fn with_range_bounds(levels: &[f64], range: [f64; 2]) -> Vec<f64> {
    let mut boundaries = levels.to_vec();
    boundaries.push(range[0]);
    boundaries.push(range[1]);
    boundaries.sort_by(|a, b| a.total_cmp(b));
    boundaries.dedup_by(|a, b| *a == *b);
    boundaries
}

fn triangle_area2(points: &[[f64; 2]; 3]) -> f64 {
    ((points[1][0] - points[0][0]) * (points[2][1] - points[0][1])
        - (points[1][1] - points[0][1]) * (points[2][0] - points[0][0]))
        .abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorRange, ColorScale, Colorbar, MeshPlotView, MeshRenderMode, mesh_plot};
    use d3rs::mesh::{ContourLevels, CoordinateAxis, ScalarAssociation};
    use std::sync::Arc;

    fn square_mesh() -> TriangleMesh {
        TriangleMesh {
            id: "square".into(),
            positions: Arc::from([
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ]),
            triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
            vertex_ids: None,
            cell_ids: None,
        }
    }

    fn vertex_field() -> ScalarField {
        ScalarField {
            id: "pressure".into(),
            label: "Pressure".into(),
            unit: Some("dB SPL".into()),
            values: Arc::from([0.0, 1.0, 1.0, 2.0]),
            association: ScalarAssociation::Vertex,
            valid: None,
        }
    }

    fn plot() -> MeshPlot {
        mesh_plot(square_mesh())
            .field(vertex_field())
            .view(MeshPlotView::Planar {
                horizontal: CoordinateAxis::X,
                vertical: CoordinateAxis::Y,
            })
            .mode(MeshRenderMode::FillAndIsolines {
                levels: ContourLevels::Explicit(Arc::from([0.5, 1.5])),
            })
            .color_scale(ColorScale::Viridis)
            .color_range(ColorRange::Fixed { min: 0.0, max: 2.0 })
            .colorbar(Colorbar::new("Pressure").unit("dB SPL"))
            .size(320.0, 240.0)
            .title("Mesh pressure")
    }

    #[test]
    fn mesh_plot_svg_is_deterministic() {
        let plot = plot();
        assert_eq!(plot.to_svg().unwrap(), plot.to_svg().unwrap());
    }

    #[test]
    fn equal_aspect_frame_tracks_the_current_viewport() {
        let layout = SvgLayout {
            left: 0.0,
            top: 0.0,
            right: 400.0,
            bottom: 200.0,
        };
        let projector = Projector::new(&[[0.0, 0.0], [4.0, 1.0]], layout, true);
        assert_eq!(projector.frame().top, 50.0);
        assert_eq!(projector.frame().bottom, 150.0);
        let zoomed = projector.with_viewport([1.0, 3.0], [0.0, 1.0]);
        assert_eq!(zoomed.frame().top, 0.0);
        assert_eq!(zoomed.frame().bottom, 200.0);
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn surface_svg_uses_camera_projected_triangles() {
        let plot = mesh_plot(square_mesh())
            .view(MeshPlotView::Surface3d)
            .mode(MeshRenderMode::Mesh)
            .size(320.0, 240.0);
        let svg = plot.to_svg().unwrap();
        assert!(svg.contains("gpui-px-mesh-plot-3d"));
        assert!(svg.contains("data-camera=\"current\""));
        assert!(svg.contains("gpui-px-mesh-3d-triangle"));
        assert!(svg.contains("gpui-px-mesh-wireframe-edge"));
        assert_eq!(svg, plot.to_svg().unwrap());
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn surface_svg_projects_render_mode_contours_and_orientation_axes() {
        let plot = mesh_plot(square_mesh())
            .field(vertex_field())
            .view(MeshPlotView::Surface3d)
            .mode(MeshRenderMode::FillAndIsolines {
                levels: ContourLevels::Explicit(Arc::from([0.5, 1.5])),
            })
            .color_range(ColorRange::Fixed { min: 0.0, max: 2.0 });
        let svg = plot.to_svg().unwrap();
        assert!(svg.contains("gpui-px-mesh-3d-isolines"));
        assert!(svg.contains("gpui-px-mesh-3d-isoline"));
        assert!(svg.contains("data-level=\"0.500000\""));
        assert!(svg.contains("gpui-px-mesh-3d-axes"));
        assert!(svg.contains("data-axis=\"X\""));
        assert_eq!(svg, plot.to_svg().unwrap());
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn homogeneous_clipping_keeps_partially_visible_triangles_and_edges() {
        // One vertex is behind the near plane (`z < 0`) while the other two
        // are visible. Export must retain the clipped quad as two triangles
        // instead of dropping the entire source triangle.
        let triangle = [
            Vec4::new(-0.4, -0.4, -0.5, 1.0),
            Vec4::new(0.7, -0.4, 0.5, 1.0),
            Vec4::new(0.0, 0.7, 0.5, 1.0),
        ];
        let clipped = clip_triangle_to_frustum(triangle);
        assert_eq!(clipped.len(), 2);
        assert!(clipped.iter().flatten().all(|point| {
            frustum_distances(*point)
                .into_iter()
                .all(|distance| distance >= 0.0)
        }));

        let (start, end) = clip_line_to_frustum(triangle[0], triangle[1]).unwrap();
        assert!(
            frustum_distances(start)
                .into_iter()
                .all(|distance| distance >= 0.0)
        );
        assert!(
            frustum_distances(end)
                .into_iter()
                .all(|distance| distance >= 0.0)
        );
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn surface_svg_projects_selected_annotation() {
        let selection = MeshPlotPick {
            plot_id: "plot".into(),
            mesh_id: "square".into(),
            cell_index: 0,
            cell_id: Some(42),
            nearest_vertex_index: None,
            vertex_id: None,
            world_position: [0.0, 0.0, 0.0],
            displayed_value: Some(1.0),
            field_id: None,
        };
        let svg = mesh_plot(square_mesh())
            .view(MeshPlotView::Surface3d)
            .selection(selection)
            .to_svg()
            .unwrap();
        assert!(svg.contains("<circle class=\"gpui-px-mesh-selection\""));
        assert!(svg.contains("Cell 0 (id 42); value 1.000000"));
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn surface_exports_do_not_mutate_retained_interaction_state() {
        use crate::MeshPlotState;
        use std::cell::RefCell;
        use std::rc::Rc;

        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        {
            let mut state = state.borrow_mut();
            state.geometry_revision = 7;
            state.field_revision = 11;
            state.orbit_rotate(18.0, -9.0);
            state.set_selection(Some(MeshPlotPick {
                plot_id: "plot".into(),
                mesh_id: "square".into(),
                cell_index: 1,
                cell_id: Some(9),
                nearest_vertex_index: None,
                vertex_id: None,
                world_position: [1.0, 1.0, 0.0],
                displayed_value: Some(2.0),
                field_id: Some("pressure".into()),
            }));
        }
        let before = {
            let state = state.borrow();
            (
                state.geometry_revision,
                state.field_revision,
                state.selection.clone(),
                state.hover.clone(),
                state.camera.view_projection_matrix().to_cols_array(),
                state.interaction.x_domain(),
                state.interaction.y_domain(),
            )
        };

        let plot = mesh_plot(square_mesh())
            .field(vertex_field())
            .view(MeshPlotView::Surface3d)
            .with_state(state.clone());
        let svg = plot.to_svg().unwrap();
        assert!(svg.contains("data-camera=\"current\""));
        let png = plot.to_png(1.0).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");

        let state = state.borrow();
        assert_eq!(state.geometry_revision, before.0);
        assert_eq!(state.field_revision, before.1);
        assert_eq!(state.selection, before.2);
        assert_eq!(state.hover, before.3);
        assert_eq!(
            state.camera.view_projection_matrix().to_cols_array(),
            before.4
        );
        assert_eq!(state.interaction.x_domain(), before.5);
        assert_eq!(state.interaction.y_domain(), before.6);
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn surface_png_export_uses_requested_scale_and_is_deterministic() {
        use image::GenericImageView;

        let plot = mesh_plot(square_mesh())
            .field(vertex_field())
            .view(MeshPlotView::Surface3d)
            .mode(MeshRenderMode::ScalarFill {
                interpolation: super::super::types::FieldInterpolation::Smooth,
            })
            .size(80.0, 50.0);
        let first = plot.to_png(2.0).unwrap();
        let second = plot.to_png(2.0).unwrap();
        assert_eq!(first, second);
        assert_eq!(&first[..8], b"\x89PNG\r\n\x1a\n");
        let image = image::load_from_memory(&first).unwrap();
        assert_eq!(image.dimensions(), (160, 100));
        assert!(image.pixels().any(|(_, _, pixel)| pixel.0[3] != 0));
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn surface_png_restores_rebased_large_world_origin_before_projection() {
        use image::GenericImageView;

        let mut mesh = square_mesh();
        mesh.positions = mesh
            .positions
            .iter()
            .map(|point| {
                [
                    point[0] + 1_000_000.0,
                    point[1] - 2_000_000.0,
                    point[2] + 500_000.0,
                ]
            })
            .collect::<Vec<_>>()
            .into();
        let png = mesh_plot(mesh)
            .field(vertex_field())
            .view(MeshPlotView::Surface3d)
            .mode(MeshRenderMode::ScalarFill {
                interpolation: super::super::types::FieldInterpolation::Smooth,
            })
            .size(80.0, 50.0)
            .to_png(1.0)
            .unwrap();
        let image = image::load_from_memory(&png).unwrap();
        assert!(image.pixels().any(|(_, _, pixel)| pixel.0[3] != 0));
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn png_export_rejects_invalid_scale_and_non_3d_views() {
        let surface = mesh_plot(square_mesh()).view(MeshPlotView::Surface3d);
        assert!(matches!(
            surface.to_png(0.0),
            Err(ChartError::InvalidDimension {
                field: "png scale",
                ..
            })
        ));
        assert!(matches!(
            plot().to_png(1.0),
            Err(ChartError::UnsupportedView { .. })
        ));
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn surface_svg_omits_triangles_with_masked_vertex_samples() {
        let mut field = vertex_field();
        field.valid = Some(Arc::from([false, true, true, true]));
        let svg = mesh_plot(square_mesh())
            .field(field)
            .view(MeshPlotView::Surface3d)
            .mode(MeshRenderMode::ScalarFill {
                interpolation: super::super::types::FieldInterpolation::Smooth,
            })
            .to_svg()
            .unwrap();
        assert!(!svg.contains("gpui-px-mesh-3d-triangle"));
    }

    #[test]
    fn mesh_plot_svg_contains_scalar_isolines_wireframe_colorbar_and_a11y() {
        let svg = plot().to_svg().unwrap();
        assert!(svg.contains("gpui-px-mesh-contour-band"));
        assert!(svg.contains("gpui-px-mesh-isoline"));
        assert!(svg.contains("gpui-px-mesh-wireframe"));
        assert!(svg.contains("gpui-px-colorbar"));
        assert!(svg.contains("<desc>"));
        assert!(svg.contains("<title>Mesh pressure</title>"));
    }

    #[test]
    fn mesh_plot_svg_uses_the_displayed_value_range_for_colorbar_and_metadata() {
        let svg = plot().to_svg().unwrap();
        assert!(svg.contains("min=\"0.000000\" max=\"2.000000\""));
        assert!(svg.contains("data-lower=\"0.000000\""));
        assert!(svg.contains("data-upper=\"2.000000\""));
    }

    #[test]
    fn mesh_plot_accessibility_summary_describes_mesh_field_and_controls() {
        let summary = plot().accessibility_summary();
        assert_eq!(summary.chart_type, "mesh_plot");
        assert_eq!(summary.value_range, Some([0.0, 2.0]));
        assert!(summary.description.contains("4 vertices"));
        assert!(summary.description.contains("2 triangles"));
        assert!(summary.description.contains("Pressure"));
        assert!(summary.description.contains("dB SPL"));
        assert!(summary.description.contains("vertex association"));
        assert!(summary.description.contains("Available controls"));
    }

    #[test]
    fn mesh_plot_svg_includes_selected_cell_annotation() {
        let pick = MeshPlotPick {
            plot_id: "plot".into(),
            mesh_id: "square".into(),
            cell_index: 0,
            cell_id: Some(42),
            nearest_vertex_index: Some(0),
            vertex_id: Some(7),
            world_position: [0.2, 0.2, 0.0],
            displayed_value: Some(0.4),
            field_id: Some("pressure".into()),
        };
        let plot = plot().selection(pick);
        let svg = plot.to_svg().unwrap();
        assert!(svg.contains("data-selected=\"true\""));
        assert!(svg.contains("Cell 0 (id 42); value 0.400000"));
        assert!(
            plot.accessibility_summary()
                .description
                .contains("Selected cell 0")
        );
    }
}
