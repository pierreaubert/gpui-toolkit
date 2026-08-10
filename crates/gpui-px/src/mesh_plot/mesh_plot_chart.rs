use super::interaction::MeshPlotState;
use super::types::*;
use crate::{
    ChartError, ChartSize, ColorRange, ColorScale, Colorbar, DEFAULT_TITLE_FONT_SIZE,
    TITLE_AREA_HEIGHT, apply_chart_size, default_design, resolved_chart_dimensions,
};
use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::grid::{GridConfig, render_grid};
#[cfg(feature = "gpu-3d")]
use d3rs::mesh::MeshBounds;
use d3rs::mesh::{
    ContourBand, CoordinateAxis, IsolineSegment, MarchingTriangles, MeshTopology,
    MeshValidationError, ScalarAssociation, ScalarField, TriangleMesh, project_2d,
};
use d3rs::scale::LinearScale;
use d3rs::text::{GlyphTextConfig, render_glyph_text};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, RenderOnce, Stateful,
    Styled, Window, div, hsla, px, rgb,
};
use gpui_design::DesignSystem;
use gpui_ui_kit::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, apply_native_accessibility,
};
use gpui_ui_kit::plot_toolbar::PlotToolbarAction;
use gpui_ui_kit::tooltip::{Tooltip, TooltipPlacement};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

fn write_mesh_qa_hit_trace(position: [f32; 2], viewport: [f32; 2], picked: bool) {
    let Some(destination) = env::var_os("GPUI_TOOLKIT_QA_LIVE_HIT_TRACE").map(PathBuf::from) else {
        return;
    };
    if let Some(parent) = destination.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        destination,
        format!(
            "position=[{:.1},{:.1}] viewport=[{:.1},{:.1}] picked={}\n",
            position[0], position[1], viewport[0], viewport[1], picked
        ),
    );
}

struct AccessibleMeshPlotElement {
    element: Stateful<Div>,
    node: AccessibilityNode,
}

impl RenderOnce for AccessibleMeshPlotElement {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(self.node);
        self.element
    }
}

impl IntoElement for AccessibleMeshPlotElement {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// Builder for unstructured 2D/axisymmetric mesh charts.
pub struct MeshPlot {
    pub(crate) mesh: TriangleMesh,
    pub(crate) plot_id: Arc<str>,
    pub(crate) field: Option<ScalarField>,
    pub(crate) view: MeshPlotView,
    pub(crate) mode: MeshRenderMode,
    pub(crate) color_scale: ColorScale,
    pub(crate) color_range: ColorRange,
    pub(crate) missing_value_policy: d3rs::mesh::MissingValuePolicy,
    pub(crate) colorbar: Option<Colorbar>,
    pub(crate) wireframe: Wireframe,
    pub(crate) axes: Axes2d,
    pub(crate) interactions: PlotInteractions,
    pub(crate) selection: Option<MeshPlotPick>,
    pub(crate) chart_size: ChartSize,
    pub(crate) title: Option<String>,
    pub(crate) design: Option<Arc<DesignSystem>>,
    pub(crate) state: Option<Rc<RefCell<MeshPlotState>>>,
    pub(crate) selection_callback: Option<Rc<dyn Fn(Option<MeshPlotPick>)>>,
    pub(crate) show_toolbar: bool,
    pub(crate) hidden_toolbar_actions: Vec<PlotToolbarAction>,
}

impl MeshPlot {
    /// Set the stable plot identity carried by hover and selection picks.
    /// Defaults to the geometry ID for backwards compatibility.
    pub fn plot_id(mut self, plot_id: impl Into<String>) -> Self {
        self.plot_id = Arc::from(plot_id.into());
        self
    }

    pub fn field(mut self, field: ScalarField) -> Self {
        self.field = Some(field);
        self
    }
    pub fn view(mut self, view: MeshPlotView) -> Self {
        self.view = view;
        self
    }
    pub fn mode(mut self, mode: MeshRenderMode) -> Self {
        self.mode = mode;
        self
    }
    pub fn color_scale(mut self, scale: ColorScale) -> Self {
        self.color_scale = scale;
        self
    }
    pub fn color_range(mut self, range: ColorRange) -> Self {
        self.color_range = range;
        self
    }
    /// Select how NaN field samples are handled before rendering.
    pub fn missing_value_policy(mut self, policy: d3rs::mesh::MissingValuePolicy) -> Self {
        self.missing_value_policy = policy;
        self
    }
    pub fn colorbar(mut self, colorbar: Colorbar) -> Self {
        self.colorbar = Some(colorbar);
        self
    }
    pub fn wireframe(mut self, value: Wireframe) -> Self {
        self.wireframe = value;
        self
    }
    pub fn axes(mut self, value: Axes2d) -> Self {
        self.axes = value;
        self
    }
    pub fn interactions(mut self, value: PlotInteractions) -> Self {
        self.interactions = value;
        self
    }
    /// Attach retained interaction state shared by the live element and
    /// exporters. The state is optional so static charts remain cheap.
    pub fn with_state(mut self, state: Rc<RefCell<MeshPlotState>>) -> Self {
        self.state = Some(state);
        self
    }
    /// Add the compact fit/reset/mode/wireframe/export toolbar.
    pub fn toolbar(mut self, enabled: bool) -> Self {
        self.show_toolbar = enabled;
        self
    }
    /// Hide or restore one native toolbar action while the toolbar is enabled.
    pub fn toolbar_action_hidden(mut self, action: PlotToolbarAction, hidden: bool) -> Self {
        if hidden {
            if !self.hidden_toolbar_actions.contains(&action) {
                self.hidden_toolbar_actions.push(action);
            }
        } else {
            self.hidden_toolbar_actions
                .retain(|candidate| *candidate != action);
        }
        self
    }
    /// Include a selected cell in static exports and accessibility metadata.
    /// Live interaction state is owned by [`MeshPlotState`].
    pub fn selection(mut self, value: MeshPlotPick) -> Self {
        self.selection = Some(value);
        self
    }
    /// Notify the host whenever a mesh cell is selected or cleared.
    pub fn on_selection<F>(mut self, callback: F) -> Self
    where
        F: Fn(Option<MeshPlotPick>) + 'static,
    {
        self.selection_callback = Some(Rc::new(callback));
        self
    }
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.chart_size = ChartSize::fixed(width, height);
        self
    }
    pub fn fill(mut self) -> Self {
        self.chart_size = ChartSize::fill();
        self
    }
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.chart_size = self.chart_size.min_size(width, height);
        self
    }
    pub fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.chart_size = self.chart_size.aspect_ratio(ratio);
        self
    }

    /// Pick the current surface/revolve view and map hits back to source
    /// profile indices for revolved meshes.
    #[cfg(feature = "gpu-3d")]
    pub fn pick_3d(
        &self,
        camera: &d3rs::gpu3d::Camera3D,
        screen: [f32; 2],
        viewport: [f32; 2],
        plot_id: &str,
    ) -> Option<MeshPlotPick> {
        match &self.view {
            MeshPlotView::AxisymmetricRevolve(spec) => {
                let revolved = d3rs::mesh::revolve(&self.mesh, spec).ok()?;
                super::picking3d::pick_revolved_3d(
                    &self.mesh,
                    &revolved,
                    self.field.as_ref(),
                    camera,
                    screen,
                    viewport,
                    plot_id,
                )
            }
            MeshPlotView::Surface3d => super::picking3d::pick_3d(
                &self.mesh,
                self.field.as_ref(),
                camera,
                screen,
                viewport,
                plot_id,
            ),
            _ => None,
        }
    }

    pub fn build(mut self) -> Result<impl gpui::IntoElement, ChartError> {
        if self.missing_value_policy == d3rs::mesh::MissingValuePolicy::MaskNaN
            && let Some(field) = self.field.take()
        {
            self.field = Some(field.mask_nan()?);
        }
        self.validate()?;
        let accessibility = self.accessibility_summary();
        let design = self.design.clone().unwrap_or_else(default_design);
        let (layout_width, layout_height) = resolved_chart_dimensions(self.chart_size);
        crate::validate::validate_dimensions(layout_width, layout_height)?;

        let (horizontal, vertical) = view_axes(&self.view);
        let projected: Vec<[f64; 2]> = self
            .mesh
            .positions
            .iter()
            .copied()
            .map(|point| project_2d(horizontal, vertical, point))
            .collect();
        let x_domain = finite_domain(&projected, 0).ok_or(ChartError::InvalidData {
            field: "mesh.positions",
            reason: "mesh projection must contain finite coordinates",
        })?;
        let y_domain = finite_domain(&projected, 1).ok_or(ChartError::InvalidData {
            field: "mesh.positions",
            reason: "mesh projection must contain finite coordinates",
        })?;
        let value_range = resolve_value_range(self.field.as_ref(), self.color_range)?;
        let topology = MeshTopology::build(&self.mesh.triangles);

        let margin_left = 50.0;
        let margin_right = if self.colorbar.is_some() { 86.0 } else { 20.0 };
        let margin_top = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            10.0
        };
        let margin_bottom = 30.0;
        let plot_width = (layout_width - margin_left - margin_right).max(1.0);
        let plot_height = (layout_height - margin_top - margin_bottom).max(1.0);

        let x_scale = LinearScale::new()
            .domain(x_domain[0], x_domain[1])
            .range(0.0, plot_width as f64);
        let y_scale = LinearScale::new()
            .domain(y_domain[0], y_domain[1])
            .range(plot_height as f64, 0.0);
        let theme = DefaultAxisTheme;
        let (horizontal_title, vertical_title) = self.axes.titles(&self.view, horizontal, vertical);
        let axis_x = AxisConfig::bottom()
            .with_design(&design)
            .with_title(horizontal_title);
        let axis_y = AxisConfig::left()
            .with_design(&design)
            .with_title(vertical_title);
        let grid = GridConfig::default().with_design(&design);

        let mesh = self.mesh.clone();
        let field = self.field.clone();
        let mode = self.mode.clone();
        let wireframe = self.wireframe;
        let equal_aspect = self.axes.equal_aspect;
        let color_scale = self.color_scale.clone();
        let projected_for_render = projected.clone();
        let range_for_render = value_range;
        let mesh_for_render = mesh.clone();
        let field_for_render = field.clone();
        let selection_callback = self.selection_callback.clone();

        #[cfg(feature = "gpu-3d")]
        let state_is_new = self.state.is_none();
        let interaction_state = if self.interactions == PlotInteractions::InspectAndNavigate {
            let state = self.state.clone().unwrap_or_else(|| {
                Rc::new(RefCell::new(MeshPlotState::new(
                    x_domain[0],
                    x_domain[1],
                    y_domain[0],
                    y_domain[1],
                )))
            });
            {
                let mut state_ref = state.borrow_mut();
                state_ref.interaction = state_ref
                    .interaction
                    .clone()
                    .with_size(plot_width as f32, plot_height as f32);
                state_ref.set_style(self.mode.clone(), self.wireframe, self.color_range);
            }
            Some(state)
        } else {
            None
        };

        let cached_contours = interaction_state.as_ref().and_then(|state| {
            state.borrow().cached_contours(
                &self.mesh,
                self.field.as_ref(),
                horizontal,
                vertical,
                &self.mode,
                value_range,
            )
        });
        let (contour_bands, isolines) = if let Some(cached) = cached_contours {
            cached
        } else {
            let (bands, lines) = contour_geometry(
                &self.mesh,
                self.field.as_ref(),
                &topology,
                horizontal,
                vertical,
                &self.mode,
                value_range,
            )?;
            let bands = Rc::new(bands);
            let lines = Rc::new(lines);
            if let Some(state) = interaction_state.as_ref() {
                state.borrow_mut().store_contours(
                    &self.mesh,
                    self.field.as_ref(),
                    horizontal,
                    vertical,
                    &self.mode,
                    value_range,
                    bands.clone(),
                    lines.clone(),
                );
            }
            (bands, lines)
        };

        #[cfg(feature = "gpu-2d")]
        let retained_state = build_retained_scene_state(
            &mesh,
            field.as_ref(),
            &projected,
            x_domain,
            y_domain,
            plot_width as f32,
            plot_height as f32,
            equal_aspect,
            &mode,
            wireframe,
            &color_scale,
            range_for_render,
        );

        #[cfg(feature = "gpu-3d")]
        let retained_3d_interaction_state = if matches!(
            self.view,
            MeshPlotView::Surface3d | MeshPlotView::AxisymmetricRevolve(_)
        ) && self.interactions
            == PlotInteractions::InspectAndNavigate
        {
            let Some(state) = interaction_state.clone() else {
                return Err(ChartError::UnsupportedView {
                    view: "mesh-3d",
                    reason: "interactive state is unavailable",
                });
            };
            state
                .borrow_mut()
                .set_camera_aspect(plot_width as f32, plot_height as f32);
            if !state.borrow().camera_fitted || state_is_new {
                let render_mesh = render_3d_mesh_for_view(&mesh, &self.view)?;
                let bounds = MeshBounds::from_positions(&render_mesh.positions);
                state
                    .borrow_mut()
                    .fit_camera_to_bounds(bounds, plot_width as f32 / plot_height.max(1.0) as f32);
            }
            Some(state)
        } else {
            None
        };

        #[cfg(all(feature = "gpu-3d", not(test)))]
        let retained_3d_owner = retained_3d_interaction_state
            .clone()
            .or_else(|| self.state.clone());

        #[cfg(all(feature = "gpu-3d", not(test)))]
        let (retained_3d_state, retained_3d_renderer, retained_3d_lod) = if let Some(owner) =
            retained_3d_owner
        {
            let mut owner = owner.borrow_mut();
            if owner.geometry_revision == 0 {
                owner.mark_resources_changed(true, field.is_some());
            }
            let geometry_revision = owner.geometry_revision;
            let field_revision = owner.field_revision;
            let camera = owner.camera.clone();
            if let Some(retained) = owner
                .retained_3d
                .as_ref()
                .filter(|retained| retained.geometry_revision == geometry_revision)
                .cloned()
            {
                // Preserve both the custom draw and its prepared geometry.
                // Field/style-only rebuilds update only the upload values and
                // color configuration; they must not recreate topology,
                // normals, positions, or the 3D scene snapshot.
                let render_field =
                    render_3d_field_for_retained(&mut owner, &mesh, field.as_ref(), &self.view)?;
                update_retained_3d_scene_state(
                    &retained.scene,
                    render_field.as_deref(),
                    &mode,
                    wireframe,
                    &color_scale,
                    range_for_render,
                    field_revision,
                );
                retained
                    .lod
                    .borrow_mut()
                    .update_field(render_field.as_deref());
                retained.renderer.set_camera(&camera);
                (
                    retained.scene.clone(),
                    retained.renderer.clone(),
                    Some(retained.lod.clone()),
                )
            } else {
                let (render_mesh, render_field) = render_3d_mesh_and_field_for_retained(
                    &mut owner,
                    &mesh,
                    field.as_ref(),
                    &self.view,
                )?;
                let fresh_retained_3d_state = build_retained_3d_scene_state(
                    &render_mesh,
                    render_field.as_deref(),
                    &mode,
                    wireframe,
                    &color_scale,
                    range_for_render,
                );
                {
                    use d3rs::mesh::gpu::{FieldRevision, GeometryRevision};
                    let mut fresh = fresh_retained_3d_state.borrow_mut();
                    fresh.geometry_rev = GeometryRevision(geometry_revision);
                    fresh.field_rev = FieldRevision(field_revision);
                }
                let renderer = Rc::new(d3rs::mesh::gpu::WgpuMesh3DRenderer::new_with_camera(
                    fresh_retained_3d_state.clone(),
                    Rc::new(RefCell::new(camera)),
                ));
                let lod = Rc::new(RefCell::new(super::interaction::RetainedMeshLod::new(
                    render_mesh,
                    render_field.as_deref(),
                )));
                owner.retained_3d = Some(super::interaction::RetainedMesh3D {
                    scene: fresh_retained_3d_state.clone(),
                    renderer: renderer.clone(),
                    lod: lod.clone(),
                    geometry_revision,
                });
                (fresh_retained_3d_state, renderer, Some(lod))
            }
        } else {
            let (render_mesh, render_field) =
                render_3d_mesh_and_field_for_view(&mesh, field.as_ref(), &self.view)?;
            let fresh_retained_3d_state = build_retained_3d_scene_state(
                &render_mesh,
                render_field.as_ref(),
                &mode,
                wireframe,
                &color_scale,
                range_for_render,
            );
            let renderer = Rc::new(d3rs::mesh::gpu::WgpuMesh3DRenderer::new(
                fresh_retained_3d_state.clone(),
            ));
            (fresh_retained_3d_state, renderer, None)
        };

        #[cfg(all(feature = "gpu-3d", test))]
        let retained_3d_state = {
            let (render_mesh, render_field) =
                render_3d_mesh_and_field_for_view(&mesh, field.as_ref(), &self.view)?;
            build_retained_3d_scene_state(
                &render_mesh,
                render_field.as_ref(),
                &mode,
                wireframe,
                &color_scale,
                range_for_render,
            )
        };

        #[cfg(all(feature = "gpu-3d", test))]
        let retained_3d_lod: Option<Rc<RefCell<super::interaction::RetainedMeshLod>>> = None;

        #[cfg(all(feature = "gpu-3d", not(test)))]
        let retained_3d_camera = Some(retained_3d_renderer.camera_handle());

        // macOS dispatches its registered Metal custom draw directly. The
        // dedicated 3D constructor consumes the same retained upload as WGPU
        // while selecting the normal-bearing, lit/depth-tested Metal pipeline
        // instead of the legacy scalar 2D pass.
        #[cfg(all(
            feature = "gpu-3d",
            feature = "gpu-metal",
            target_os = "macos",
            not(test)
        ))]
        let retained_3d_custom_id = {
            if let Some(camera) = retained_3d_camera.as_ref() {
                retained_3d_state.borrow_mut().view_transform =
                    camera.borrow().view_projection_matrix().to_cols_array_2d();
            }
            retained_3d_camera
                .as_ref()
                .map_or_else(
                    || d3rs::mesh::gpu::MetalMeshRenderer::new_3d(retained_3d_state.clone()),
                    |camera| {
                        d3rs::mesh::gpu::MetalMeshRenderer::new_3d_with_camera(
                            retained_3d_state.clone(),
                            camera.clone(),
                        )
                    },
                )
                .custom_id()
        };

        #[cfg(all(
            feature = "gpu-3d",
            not(all(feature = "gpu-metal", target_os = "macos")),
            not(test)
        ))]
        let retained_3d_custom_id = retained_3d_renderer.custom_id();

        #[cfg(all(feature = "gpu-3d", test))]
        let retained_3d_camera = retained_3d_interaction_state
            .as_ref()
            .map(|state| Rc::new(RefCell::new(state.borrow().camera.clone())));

        #[cfg(feature = "gpu-2d")]
        if let Some(state) = interaction_state.as_ref()
            && !matches!(
                self.view,
                MeshPlotView::Surface3d | MeshPlotView::AxisymmetricRevolve(_)
            )
        {
            update_scene_view_transform(
                &retained_state,
                &state.borrow(),
                plot_width as f32,
                plot_height as f32,
                equal_aspect,
            );
        }

        #[cfg(feature = "gpu-2d")]
        let plot_element: AnyElement = if matches!(
            self.view,
            MeshPlotView::Surface3d | MeshPlotView::AxisymmetricRevolve(_)
        ) {
            #[cfg(feature = "gpu-3d")]
            {
                let scene = d3rs::mesh::gpu::MeshSceneElement::new(retained_3d_state.clone());
                #[cfg(not(test))]
                let scene = scene.with_custom_id(retained_3d_custom_id);
                scene.into_any_element()
            }
            #[cfg(not(feature = "gpu-3d"))]
            {
                div().size_full().bg(rgb(0xf4f5f7)).into_any_element()
            }
        } else if matches!(
            mode,
            MeshRenderMode::Mesh | MeshRenderMode::ScalarFill { .. }
        ) {
            let scene = d3rs::mesh::gpu::MeshSceneElement::new(retained_state.clone());
            #[cfg(not(test))]
            let scene = {
                #[cfg(all(feature = "gpu-metal", target_os = "macos"))]
                let renderer = d3rs::mesh::gpu::MetalMeshRenderer::new(retained_state.clone());
                #[cfg(not(all(feature = "gpu-metal", target_os = "macos")))]
                let renderer = d3rs::mesh::gpu::WgpuMeshRenderer::new(retained_state.clone());
                scene.with_custom_id(renderer.custom_id())
            };
            scene.into_any_element()
        } else {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let render = d3rs::gpu2d::Chart2DElement::new(move |renderer, bounds| {
                    let width: f32 = bounds.size.width.into();
                    let height: f32 = bounds.size.height.into();
                    let projector = MeshProjector::new(
                        &projected_for_render,
                        width.max(1.0),
                        height.max(1.0),
                        equal_aspect,
                    );
                    let value_to_color = |value: f64| {
                        let t = range_for_render
                            .map(|range| {
                                ((value - range[0]) / (range[1] - range[0]).max(f64::EPSILON))
                                    .clamp(0.0, 1.0)
                            })
                            .unwrap_or(0.5);
                        let color = color_scale.map(t);
                        [color.r, color.g, color.b, color.a]
                    };
                    let default_color = [0.35, 0.39, 0.46, 1.0];

                    if !matches!(mode, MeshRenderMode::Isolines { .. }) {
                        for (cell_index, triangle) in mesh_for_render.triangles.iter().enumerate() {
                            let Some(points) =
                                triangle_points(&projector, &projected_for_render, *triangle)
                            else {
                                continue;
                            };
                            let Some(value) =
                                triangle_value(field_for_render.as_ref(), *triangle, cell_index)
                            else {
                                if field_for_render.is_some() {
                                    continue;
                                }
                                renderer.draw_triangle(
                                    points[0],
                                    points[1],
                                    points[2],
                                    default_color,
                                );
                                continue;
                            };
                            let color = if matches!(mode, MeshRenderMode::Mesh) {
                                default_color
                            } else {
                                value_to_color(value)
                            };
                            renderer.draw_triangle(points[0], points[1], points[2], color);
                        }
                    }

                    for band in contour_bands.iter() {
                        let value = band
                            .lower
                            .unwrap_or_else(|| range_for_render.map_or(0.0, |r| r[0]));
                        let color = value_to_color(value);
                        for triangle in &band.triangles {
                            let Some(points) =
                                triangle_points_from_band(&projector, &band.positions, *triangle)
                            else {
                                continue;
                            };
                            renderer.draw_triangle(points[0], points[1], points[2], color);
                        }
                    }

                    for segment in isolines.iter() {
                        let start = projector.point(segment.start);
                        let end = projector.point(segment.end);
                        if (start[0] - end[0]).abs() <= 1e-6 && (start[1] - end[1]).abs() <= 1e-6 {
                            continue;
                        }
                        renderer.draw_line(
                            start[0],
                            start[1],
                            end[0],
                            end[1],
                            1.25,
                            [0.1, 0.1, 0.1, 0.9],
                        );
                    }

                    if wireframe == Wireframe::Overlay || matches!(mode, MeshRenderMode::Mesh) {
                        for edge in topology.unique_edges.iter() {
                            let Some(a) = projected_for_render.get(edge[0] as usize).copied()
                            else {
                                continue;
                            };
                            let Some(b) = projected_for_render.get(edge[1] as usize).copied()
                            else {
                                continue;
                            };
                            let a = projector.point(a);
                            let b = projector.point(b);
                            renderer.draw_line(
                                a[0],
                                a[1],
                                b[0],
                                b[1],
                                1.0,
                                [0.12, 0.14, 0.18, 0.9],
                            );
                        }
                    }
                })
                .transparent();
                render.into_any_element()
            }))
            .unwrap_or_else(|_| {
                // Chart2DElement creates a renderer eagerly. Keep chart construction usable in
                // headless/unit-test processes where no Metal/wgpu device is available.
                div().size_full().bg(rgb(0xf4f5f7)).into_any_element()
            })
        };

        #[cfg(not(feature = "gpu-2d"))]
        let plot_element: AnyElement = {
            #[cfg(feature = "gpu-3d")]
            if matches!(self.view, MeshPlotView::Surface3d) {
                let scene = d3rs::mesh::gpu::MeshSceneElement::new(retained_3d_state.clone());
                #[cfg(not(test))]
                let scene = scene.with_custom_id(retained_3d_custom_id);
                scene.into_any_element()
            } else {
                div().size_full().bg(rgb(0xf4f5f7)).into_any_element()
            }
            #[cfg(not(feature = "gpu-3d"))]
            {
                div().size_full().bg(rgb(0xf4f5f7)).into_any_element()
            }
        };

        #[cfg(feature = "gpu-2d")]
        let plot_element = if self.interactions == PlotInteractions::InspectAndNavigate
            && !matches!(
                self.view,
                MeshPlotView::Surface3d | MeshPlotView::AxisymmetricRevolve(_)
            ) {
            let Some(state) = interaction_state.clone() else {
                return Err(ChartError::UnsupportedView {
                    view: "mesh-2d",
                    reason: "interactive state is unavailable",
                });
            };
            let index = {
                let mut state = state.borrow_mut();
                state.planar_index_for(&projected, &mesh, horizontal, vertical)
            };
            let hover_mesh = mesh.clone();
            let select_mesh = mesh.clone();
            let hover_field = field.clone();
            let select_field = field.clone();
            let plot_id = self.plot_id.clone();
            let hover_index = index.clone();
            let select_index = index;
            let horizontal = horizontal;
            let vertical = vertical;
            let hover_state = state.clone();
            let hover_clear_state = hover_state.clone();
            let select_state = state.clone();
            let click_state = select_state.clone();
            let key_state = select_state.clone();
            let scroll_state = select_state.clone();
            let key_scene = retained_state.clone();
            let key_scene_click = key_scene.clone();
            let scroll_scene = retained_state.clone();
            let pan_scene = retained_state.clone();
            let pan_scene_move = pan_scene.clone();
            let navigation_width = plot_width as f32;
            let navigation_height = plot_height as f32;
            let hover_plot_id = plot_id.clone();
            let select_plot_id = plot_id;
            let drag_state = Rc::new(RefCell::new(None::<[f32; 2]>));
            let drag_down = drag_state.clone();
            let drag_move = drag_state.clone();
            let drag_up = drag_state.clone();
            let brush_state = state.clone();
            let callback = selection_callback.clone();
            div()
                .size_full()
                .id(format!("mesh-plot-{}", mesh.id))
                .focusable()
                .cursor_grab()
                .child(plot_element)
                .on_mouse_move(move |event: &gpui::MouseMoveEvent, window, _cx| {
                    let x = f32::from(event.position.x);
                    let y = f32::from(event.position.y);
                    let mut state = hover_state.borrow_mut();
                    let Some((x, y)) = state.interaction.update_hover_pixel(x, y) else {
                        state.set_hover(None);
                        return;
                    };
                    if state.interaction.is_brushing() {
                        state.interaction.update_brush(x as f32, y as f32);
                        window.refresh();
                    } else if let Some(previous) = *drag_move.borrow() {
                        let dx = x as f32 - previous[0];
                        let dy = y as f32 - previous[1];
                        if dx.abs() > 0.0 || dy.abs() > 0.0 {
                            state.interaction.pan_by_pixels(dx, dy);
                            *drag_move.borrow_mut() = Some([x as f32, y as f32]);
                            update_scene_view_transform(
                                &pan_scene_move,
                                &state,
                                navigation_width,
                                navigation_height,
                                equal_aspect,
                            );
                            window.refresh();
                        }
                    } else {
                        state.pick_at(
                            &hover_mesh,
                            hover_field.as_ref(),
                            hover_index.as_ref(),
                            horizontal,
                            vertical,
                            [x, y],
                            &hover_plot_id,
                            false,
                        );
                    }
                })
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    move |event: &gpui::MouseDownEvent, _window, _cx| {
                        let screen = [f32::from(event.position.x), f32::from(event.position.y)];
                        let mut state = select_state.borrow_mut();
                        let Some((x, y)) =
                            state.interaction.update_hover_pixel(screen[0], screen[1])
                        else {
                            write_mesh_qa_hit_trace(
                                screen,
                                [navigation_width, navigation_height],
                                false,
                            );
                            return;
                        };
                        if event.modifiers.shift {
                            state.interaction.start_brush(x as f32, y as f32);
                            *drag_down.borrow_mut() = None;
                        } else {
                            let pick = state.pick_at(
                                &select_mesh,
                                select_field.as_ref(),
                                select_index.as_ref(),
                                horizontal,
                                vertical,
                                [x, y],
                                &select_plot_id,
                                true,
                            );
                            write_mesh_qa_hit_trace(
                                [x as f32, y as f32],
                                [navigation_width, navigation_height],
                                pick.is_some(),
                            );
                            *drag_down.borrow_mut() = Some([x as f32, y as f32]);
                            if let Some(callback) = &callback {
                                callback(pick);
                            }
                        }
                    },
                )
                .on_mouse_up(gpui::MouseButton::Left, move |_event, window, _cx| {
                    let mut state = brush_state.borrow_mut();
                    if state.interaction.is_brushing() {
                        state.interaction.end_brush(true);
                        update_scene_view_transform(
                            &pan_scene,
                            &state,
                            navigation_width,
                            navigation_height,
                            equal_aspect,
                        );
                    }
                    *drag_up.borrow_mut() = None;
                    window.refresh();
                })
                .on_hover(move |hovered, window, _cx| {
                    if !hovered {
                        hover_clear_state.borrow_mut().set_hover(None);
                        window.refresh();
                    }
                })
                .on_key_down(move |event: &gpui::KeyDownEvent, window, _cx| {
                    let mut state = key_state.borrow_mut();
                    if state.handle_key(&event.keystroke.key) {
                        update_scene_view_transform(
                            &key_scene,
                            &state,
                            navigation_width,
                            navigation_height,
                            equal_aspect,
                        );
                        window.refresh();
                    }
                })
                .on_click(move |event: &gpui::ClickEvent, window, _cx| {
                    if event.click_count() >= 2 {
                        let mut state = click_state.borrow_mut();
                        state.interaction.reset_zoom();
                        update_scene_view_transform(
                            &key_scene_click,
                            &state,
                            navigation_width,
                            navigation_height,
                            equal_aspect,
                        );
                        window.refresh();
                    }
                })
                .on_scroll_wheel(move |event: &gpui::ScrollWheelEvent, window, _cx| {
                    let delta = match event.delta {
                        gpui::ScrollDelta::Lines(lines) => lines.y,
                        gpui::ScrollDelta::Pixels(pixels) => f32::from(pixels.y) * 0.01,
                    };
                    if !delta.is_finite() || delta == 0.0 {
                        return;
                    }
                    let mut state = scroll_state.borrow_mut();
                    let x = f32::from(event.position.x);
                    let y = f32::from(event.position.y);
                    state
                        .interaction
                        .zoom_around_pixel(x, y, (1.0 - delta * 0.1).max(0.1) as f64);
                    update_scene_view_transform(
                        &scroll_scene,
                        &state,
                        navigation_width,
                        navigation_height,
                        equal_aspect,
                    );
                    window.refresh();
                })
                .into_any_element()
        } else {
            plot_element
        };

        #[cfg(not(feature = "gpu-2d"))]
        let plot_element = plot_element;

        #[cfg(feature = "gpu-3d")]
        let plot_element = if let (Some(state), Some(camera)) = (
            retained_3d_interaction_state.clone(),
            retained_3d_camera.clone(),
        ) {
            let drag_start = Rc::new(RefCell::new(None::<[f32; 2]>));
            let drag_down = drag_start.clone();
            let drag_middle_down = drag_start.clone();
            let drag_move = drag_start.clone();
            let drag_up = drag_start;
            let drag_middle_up = drag_up.clone();
            let pan_drag = Rc::new(RefCell::new(false));
            let pan_drag_down = pan_drag.clone();
            let pan_drag_middle_down = pan_drag.clone();
            let pan_drag_move = pan_drag.clone();
            let pan_drag_up = pan_drag;
            let pan_drag_middle_up = pan_drag_up.clone();
            let pick_mesh = mesh.clone();
            let pick_field = field.clone();
            let pick_view = self.view.clone();
            let plot_id = self.plot_id.clone();
            let hover_mesh = pick_mesh.clone();
            let hover_field = pick_field.clone();
            let hover_view = pick_view.clone();
            let hover_plot_id = plot_id.clone();
            let camera_down = camera.clone();
            let camera_move = camera.clone();
            let camera_scroll = camera.clone();
            let camera_scene_move = retained_3d_state.clone();
            let camera_scene_scroll = retained_3d_state.clone();
            let camera_scene_reset = retained_3d_state.clone();
            let lod_scene_down = retained_3d_state.clone();
            let lod_scene_middle_down = retained_3d_state.clone();
            let lod_scene_up = retained_3d_state.clone();
            let lod_scene_middle_up = retained_3d_state.clone();
            let lod_down = retained_3d_lod.clone();
            let lod_middle_down = retained_3d_lod.clone();
            let lod_up = retained_3d_lod.clone();
            let lod_middle_up = retained_3d_lod.clone();
            let state_down = state.clone();
            let state_move = state.clone();
            let state_scroll = state.clone();
            let state_key = state;
            let selection_callback_3d = selection_callback.clone();
            let clear_selection_callback_3d = selection_callback.clone();
            let viewport = [plot_width as f32, plot_height as f32];
            div()
                .size_full()
                .child(plot_element)
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    move |event: &gpui::MouseDownEvent, _window, _cx| {
                        let screen = [f32::from(event.position.x), f32::from(event.position.y)];
                        let mut state = state_down.borrow_mut();
                        let camera_value = camera_down.borrow().clone();
                        let pick = pick_3d_for_view_retained(
                            &mut state,
                            &hover_mesh,
                            hover_field.as_ref(),
                            &hover_view,
                            &camera_value,
                            screen,
                            viewport,
                            &hover_plot_id,
                        );
                        state.set_selection(pick.clone());
                        if let Some(callback) = &selection_callback_3d {
                            callback(pick);
                        }
                        if let Some(lod) = lod_down.as_ref() {
                            lod.borrow_mut()
                                .begin_drag(&mut lod_scene_down.borrow_mut());
                        }
                        *pan_drag_down.borrow_mut() = false;
                        *drag_down.borrow_mut() = Some(screen);
                    },
                )
                .on_mouse_down(
                    gpui::MouseButton::Middle,
                    move |event: &gpui::MouseDownEvent, _window, _cx| {
                        if let Some(lod) = lod_middle_down.as_ref() {
                            lod.borrow_mut()
                                .begin_drag(&mut lod_scene_middle_down.borrow_mut());
                        }
                        *pan_drag_middle_down.borrow_mut() = true;
                        *drag_middle_down.borrow_mut() =
                            Some([f32::from(event.position.x), f32::from(event.position.y)]);
                    },
                )
                .on_mouse_move(move |event: &gpui::MouseMoveEvent, window, _cx| {
                    let current = [f32::from(event.position.x), f32::from(event.position.y)];
                    let Some(previous) = *drag_move.borrow() else {
                        // Hover inspection is native-only: it updates local
                        // state but deliberately does not invoke the host
                        // selection callback for every pointer movement.
                        let camera_value = camera_move.borrow().clone();
                        let mut state = state_move.borrow_mut();
                        let pick = pick_3d_for_view_retained(
                            &mut state,
                            &pick_mesh,
                            pick_field.as_ref(),
                            &pick_view,
                            &camera_value,
                            current,
                            viewport,
                            &plot_id,
                        );
                        state.set_hover(pick);
                        window.refresh();
                        return;
                    };
                    let delta = [current[0] - previous[0], current[1] - previous[1]];
                    *drag_move.borrow_mut() = Some(current);
                    if delta[0] == 0.0 && delta[1] == 0.0 {
                        return;
                    }
                    let mut state = state_move.borrow_mut();
                    if *pan_drag_move.borrow() {
                        state.orbit_pan(delta[0], delta[1]);
                    } else {
                        state.orbit_rotate(delta[0], delta[1]);
                    }
                    *camera_move.borrow_mut() = state.camera.clone();
                    camera_scene_move.borrow_mut().view_transform =
                        state.camera.view_projection_matrix().to_cols_array_2d();
                    window.refresh();
                })
                .on_mouse_up(gpui::MouseButton::Left, move |_event, _window, _cx| {
                    if let Some(lod) = lod_up.as_ref() {
                        lod.borrow_mut().end_drag(&mut lod_scene_up.borrow_mut());
                    }
                    *drag_up.borrow_mut() = None;
                    *pan_drag_up.borrow_mut() = false;
                })
                .on_mouse_up(gpui::MouseButton::Middle, move |_event, _window, _cx| {
                    if let Some(lod) = lod_middle_up.as_ref() {
                        lod.borrow_mut()
                            .end_drag(&mut lod_scene_middle_up.borrow_mut());
                    }
                    *drag_middle_up.borrow_mut() = None;
                    *pan_drag_middle_up.borrow_mut() = false;
                })
                .on_scroll_wheel(move |event: &gpui::ScrollWheelEvent, window, _cx| {
                    let delta = match event.delta {
                        gpui::ScrollDelta::Lines(lines) => lines.y,
                        gpui::ScrollDelta::Pixels(pixels) => f32::from(pixels.y) * 0.01,
                    };
                    if !delta.is_finite() || delta == 0.0 {
                        return;
                    }
                    let mut state = state_scroll.borrow_mut();
                    state.orbit_zoom(delta);
                    *camera_scroll.borrow_mut() = state.camera.clone();
                    camera_scene_scroll.borrow_mut().view_transform =
                        state.camera.view_projection_matrix().to_cols_array_2d();
                    window.refresh();
                })
                .on_key_down(move |event: &gpui::KeyDownEvent, window, _cx| {
                    {
                        let mut state = state_key.borrow_mut();
                        if state.handle_3d_key(&event.keystroke.key) {
                            *camera.borrow_mut() = state.camera.clone();
                            camera_scene_reset.borrow_mut().view_transform =
                                state.camera.view_projection_matrix().to_cols_array_2d();
                            window.refresh();
                            return;
                        }
                    }
                    if event.keystroke.key == "escape" {
                        let mut state = state_key.borrow_mut();
                        state.orbit_reset();
                        *camera.borrow_mut() = state.camera.clone();
                        camera_scene_reset.borrow_mut().view_transform =
                            state.camera.view_projection_matrix().to_cols_array_2d();
                        let had_selection = state.selection.is_some();
                        state.clear_selection();
                        if had_selection && let Some(callback) = &clear_selection_callback_3d {
                            callback(None);
                        }
                        window.refresh();
                    }
                })
                .into_any_element()
        } else {
            plot_element
        };

        // Hover text is deliberately presented inside the chart's clipped
        // plot area. The interaction handlers update `MeshPlotState` and
        // request a window refresh, allowing normal retained-chart renders to
        // replace this overlay without turning pointer inspection into a host
        // selection event.
        let hover_tooltip = interaction_state.as_ref().and_then(|state| {
            mesh_hover_tooltip_text(&state.borrow(), field.as_ref()).map(|text| {
                div()
                    .absolute()
                    .top(px(8.0))
                    .right(px(8.0))
                    .child(Tooltip::new(text).placement(TooltipPlacement::Bottom))
                    .into_any_element()
            })
        });

        let chart_content = div()
            .flex()
            .child(render_axis(&y_scale, &axis_y, plot_height as f32, &theme))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w(px(plot_width as f32))
                            .h(px(plot_height as f32))
                            .relative()
                            .overflow_hidden()
                            .bg(rgb(0xf8f8f8))
                            .child(render_grid(
                                &x_scale,
                                &y_scale,
                                &grid,
                                plot_width as f32,
                                plot_height as f32,
                                &theme,
                            ))
                            .child(div().absolute().inset_0().size_full().child(plot_element))
                            .children(hover_tooltip),
                    )
                    .child(render_axis(&x_scale, &axis_x, plot_width as f32, &theme)),
            );

        let mut container = apply_chart_size(div(), self.chart_size)
            .relative()
            .flex()
            .flex_col();
        if let Some(title) = &self.title {
            let config = GlyphTextConfig::horizontal(
                design.typography.large_size.max(DEFAULT_TITLE_FONT_SIZE),
                hsla(0.0, 0.0, 0.2, 1.0),
            );
            container = container.child(
                div()
                    .w_full()
                    .h(px(TITLE_AREA_HEIGHT))
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(render_glyph_text(title, &config)),
            );
        }
        let mut body = div().flex().child(chart_content);
        if let Some(colorbar) = &self.colorbar
            && let Some(range) = value_range
        {
            body = body.child(
                colorbar
                    .clone()
                    .color_scale(self.color_scale.clone())
                    .range(ColorRange::Fixed {
                        min: range[0],
                        max: range[1],
                    })
                    .render(&design, plot_height as f32),
            );
        }
        #[cfg(feature = "gpui")]
        if self.show_toolbar {
            use gpui_ui_kit::plot_toolbar::PlotToolbar;
            // `build` creates a live interaction state when the caller did
            // not supply one. The toolbar must operate on that same state,
            // otherwise the default `.toolbar(true)` controls are inert.
            let toolbar_state = interaction_state.clone().or_else(|| self.state.clone());
            let toolbar_is_3d = matches!(
                self.view,
                MeshPlotView::Surface3d | MeshPlotView::AxisymmetricRevolve(_)
            );
            #[cfg(feature = "gpu-2d")]
            let toolbar_2d_scene = retained_state.clone();
            #[cfg(feature = "gpu-3d")]
            let toolbar_3d_scene = retained_3d_state.clone();
            #[cfg(feature = "gpu-3d")]
            let toolbar_3d_camera = retained_3d_camera.clone();
            #[cfg(feature = "gpu-3d")]
            let toolbar_3d_bounds = toolbar_is_3d
                .then(|| render_3d_mesh_for_view(&mesh, &self.view).ok())
                .flatten()
                .map(|mesh| MeshBounds::from_positions(&mesh.positions));
            let toolbar_field = field.clone();
            let toolbar_hidden_actions = self.hidden_toolbar_actions.clone();
            let mut toolbar = PlotToolbar::new("mesh-plot-toolbar")
                .mode(format!("{:?}", self.mode))
                .view(toolbar_view_name(&self.view))
                .wireframe(self.wireframe == Wireframe::Overlay);
            for action in toolbar_hidden_actions {
                toolbar = toolbar.hidden(action, true);
            }
            let toolbar = toolbar
                .on_action(move |action, window, _cx| {
                    let Some(state) = toolbar_state.as_ref() else {
                        return;
                    };
                    let mut state = state.borrow_mut();
                    match action {
                        PlotToolbarAction::Fit | PlotToolbarAction::Reset => {
                            if toolbar_is_3d {
                                #[cfg(feature = "gpu-3d")]
                                {
                                    if matches!(action, PlotToolbarAction::Fit) {
                                        if let Some(bounds) = toolbar_3d_bounds {
                                            state.fit_camera_to_bounds(
                                                bounds,
                                                plot_width as f32 / plot_height.max(1.0) as f32,
                                            );
                                        }
                                    } else {
                                        state.orbit_reset();
                                    }
                                    if let Some(camera) = toolbar_3d_camera.as_ref() {
                                        *camera.borrow_mut() = state.camera.clone();
                                    }
                                    toolbar_3d_scene.borrow_mut().view_transform =
                                        state.camera.view_projection_matrix().to_cols_array_2d();
                                }
                            } else {
                                state.interaction.reset_zoom();
                                #[cfg(feature = "gpu-2d")]
                                update_scene_view_transform(
                                    &toolbar_2d_scene,
                                    &state,
                                    plot_width as f32,
                                    plot_height as f32,
                                    equal_aspect,
                                );
                            }
                        }
                        PlotToolbarAction::ToggleWireframe => {
                            state.toggle_wireframe();
                            #[cfg(feature = "gpu-2d")]
                            {
                                toolbar_2d_scene.borrow_mut().color.wireframe =
                                    state.wireframe == Wireframe::Overlay;
                            }
                            #[cfg(feature = "gpu-3d")]
                            {
                                toolbar_3d_scene.borrow_mut().color.wireframe =
                                    state.wireframe == Wireframe::Overlay;
                            }
                        }
                        PlotToolbarAction::ResetColorRange => {
                            state.reset_color_range();
                            let range =
                                resolve_value_range(toolbar_field.as_ref(), ColorRange::Auto)
                                    .ok()
                                    .flatten()
                                    .unwrap_or([0.0, 1.0]);
                            #[cfg(feature = "gpu-2d")]
                            {
                                toolbar_2d_scene.borrow_mut().color.range =
                                    [range[0] as f32, range[1] as f32];
                            }
                            #[cfg(feature = "gpu-3d")]
                            {
                                toolbar_3d_scene.borrow_mut().color.range =
                                    [range[0] as f32, range[1] as f32];
                            }
                        }
                        PlotToolbarAction::OpenModeMenu
                        | PlotToolbarAction::OpenViewMenu
                        | PlotToolbarAction::Export => {}
                    }
                    window.refresh();
                })
                .build();
            body = body.child(toolbar);
        }
        container = container.child(body);
        let element_id = format!("mesh-plot-{}", self.plot_id);
        let accessibility_label = accessibility.accessible_label();
        let accessibility_props = AriaProps::with_role(AriaRole::Img)
            .description(accessibility.description.clone())
            .value_text(accessibility.accessible_value_text());
        let element = apply_native_accessibility(
            container.id(element_id.clone()),
            accessibility_label.clone(),
            &accessibility_props,
        );
        Ok(AccessibleMeshPlotElement {
            element,
            node: AccessibilityNode {
                element_id: element_id.into(),
                label: accessibility_label.into(),
                props: accessibility_props,
            },
        })
    }

    fn validate(&self) -> Result<(), ChartError> {
        self.mesh.validate()?;
        if let Some(field) = &self.field {
            field.validate(&self.mesh)?;
        }
        let needs_field = !matches!(self.mode, MeshRenderMode::Mesh);
        if needs_field && self.field.is_none() {
            return Err(ChartError::InvalidData {
                field: "field",
                reason: "scalar render mode requires a field",
            });
        }
        if matches!(
            self.mode,
            MeshRenderMode::FilledContours { .. }
                | MeshRenderMode::Isolines { .. }
                | MeshRenderMode::FillAndIsolines { .. }
        ) && self
            .field
            .as_ref()
            .is_some_and(|field| field.association != ScalarAssociation::Vertex)
        {
            return Err(MeshValidationError::ContoursRequireVertexField.into());
        }
        if let (MeshRenderMode::ScalarFill { interpolation }, Some(field)) =
            (&self.mode, self.field.as_ref())
        {
            match (interpolation, field.association) {
                // The retained 2D/3D vertex path uses GPU interpolation.
                (FieldInterpolation::Smooth, ScalarAssociation::Vertex)
                // Per-cell values are expanded once and rendered flat.
                | (FieldInterpolation::Flat, ScalarAssociation::Cell) => {}
                (FieldInterpolation::Smooth, ScalarAssociation::Cell) => {
                    return Err(ChartError::InvalidData {
                        field: "field.interpolation",
                        reason: "smooth interpolation requires a vertex-associated field",
                    });
                }
                (FieldInterpolation::Flat, ScalarAssociation::Vertex) => {
                    return Err(ChartError::InvalidData {
                        field: "field.interpolation",
                        reason: "flat interpolation requires a cell-associated field",
                    });
                }
            }
        }
        if let MeshPlotView::AxisymmetricRevolve(spec) = &self.view {
            // Validate and prepare the retained derivative before a GPU/GPUI
            // element is constructed. Hosts key this derivative by the
            // geometry/spec revision and reuse it across field updates.
            let revolved = d3rs::mesh::revolve(&self.mesh, spec)?;
            if let Some(field) = &self.field {
                let _ = d3rs::mesh::revolve_field(field, &revolved);
            }
        }
        if let MeshPlotView::AxisymmetricSection { radial, .. } = self.view {
            for (index, position) in self.mesh.positions.iter().enumerate() {
                if radial.component(*position) < -1e-12 {
                    return Err(MeshValidationError::InvalidRadius {
                        index,
                        value: radial.component(*position),
                    }
                    .into());
                }
            }
        }
        if let Some(field) = &self.field {
            let mut min = f64::INFINITY;
            let mut max = f64::NEG_INFINITY;
            for (index, value) in field.values.iter().enumerate() {
                if field
                    .valid
                    .as_ref()
                    .is_some_and(|valid| valid.get(index) != Some(&true))
                {
                    continue;
                }
                min = min.min(*value);
                max = max.max(*value);
            }
            if min.is_finite() && max.is_finite() {
                self.color_range.resolve(min, max)?;
            }
        }
        Ok(())
    }
}

pub fn mesh_plot(mesh: TriangleMesh) -> MeshPlot {
    let plot_id = mesh.id.clone();
    MeshPlot {
        mesh,
        plot_id,
        field: None,
        view: MeshPlotView::Planar {
            horizontal: d3rs::mesh::CoordinateAxis::X,
            vertical: d3rs::mesh::CoordinateAxis::Y,
        },
        mode: MeshRenderMode::Mesh,
        color_scale: ColorScale::default(),
        color_range: ColorRange::Auto,
        missing_value_policy: d3rs::mesh::MissingValuePolicy::Reject,
        colorbar: None,
        wireframe: Wireframe::Overlay,
        axes: Axes2d::default(),
        interactions: PlotInteractions::default(),
        selection: None,
        chart_size: ChartSize::default(),
        title: None,
        design: None,
        state: None,
        selection_callback: None,
        show_toolbar: false,
        hidden_toolbar_actions: Vec::new(),
    }
}

#[cfg(feature = "gpu-3d")]
fn render_3d_mesh_for_view(
    mesh: &TriangleMesh,
    view: &MeshPlotView,
) -> Result<TriangleMesh, ChartError> {
    match view {
        MeshPlotView::AxisymmetricRevolve(spec) => Ok(d3rs::mesh::revolve(mesh, spec)?.mesh),
        MeshPlotView::Surface3d => Ok(mesh.clone()),
        _ => Err(ChartError::UnsupportedView {
            view: toolbar_view_name(view),
            reason: "the 3D render path only accepts surface or revolve views",
        }),
    }
}

#[cfg(feature = "gpu-2d")]
fn build_retained_scene_state(
    mesh: &TriangleMesh,
    field: Option<&ScalarField>,
    projected: &[[f64; 2]],
    x_domain: [f64; 2],
    y_domain: [f64; 2],
    plot_width: f32,
    plot_height: f32,
    equal_aspect: bool,
    mode: &MeshRenderMode,
    wireframe: Wireframe,
    color_scale: &ColorScale,
    range: Option<[f64; 2]>,
) -> Rc<RefCell<d3rs::mesh::gpu::MeshSceneState>> {
    use d3rs::mesh::gpu::{FieldRevision, GeometryRevision, MeshColorConfig, MeshSceneState};
    use d3rs::mesh::{prepare_field, prepare_upload};

    let topology = MeshTopology::build(&mesh.triangles);
    let mut upload = prepare_upload(mesh, &topology);
    // The 2D scene consumes projected coordinates. Rebase those projected
    // values in f64 before the final f32 conversion, preserving precision for
    // large world coordinates and arbitrary X/Y/Z axis choices.
    let origin = [
        projected
            .iter()
            .map(|point| point[0])
            .fold(f64::INFINITY, f64::min),
        projected
            .iter()
            .map(|point| point[1])
            .fold(f64::INFINITY, f64::min),
        0.0,
    ];
    upload.origin = origin;
    upload.positions_f32 = projected
        .iter()
        .map(|point| {
            [
                (point[0] - origin[0]) as f32,
                (point[1] - origin[1]) as f32,
                0.0,
            ]
        })
        .collect();
    if let Some(field) = field {
        let values = prepare_field(field);
        match field.association {
            ScalarAssociation::Vertex => upload.values_f32 = Some(values),
            ScalarAssociation::Cell => upload.cell_values_f32 = Some(values),
        }
    }
    let color_range = range.unwrap_or([0.0, 1.0]);
    Rc::new(RefCell::new(MeshSceneState {
        geometry_rev: GeometryRevision(1),
        field_rev: FieldRevision(u64::from(field.is_some())),
        upload: Some(upload),
        geometry_upload_count: 0,
        geometry_upload_bytes: 0,
        view_transform: mesh_view_transform(
            origin,
            x_domain,
            y_domain,
            plot_width,
            plot_height,
            equal_aspect,
        ),
        color: MeshColorConfig {
            colormap: color_scale.to_colormap_index(),
            range: [color_range[0] as f32, color_range[1] as f32],
            wireframe: wireframe == Wireframe::Overlay || matches!(mode, MeshRenderMode::Mesh),
            isoline_step: isoline_step(mode, range).unwrap_or(0.0) as f32,
            isoline_width_px: 1.0,
            unlit: true,
        },
    }))
}

#[cfg(feature = "gpu-3d")]
pub(crate) fn render_3d_mesh_and_field_for_view(
    mesh: &TriangleMesh,
    field: Option<&ScalarField>,
    view: &MeshPlotView,
) -> Result<(TriangleMesh, Option<ScalarField>), ChartError> {
    match view {
        MeshPlotView::AxisymmetricRevolve(spec) => {
            let revolved = d3rs::mesh::revolve(mesh, spec)?;
            let field = field.map(|field| super::picking3d::revolved_field(field, &revolved));
            Ok((revolved.mesh, field))
        }
        _ => Ok((mesh.clone(), field.cloned())),
    }
}

#[cfg(all(feature = "gpu-3d", not(test)))]
fn render_3d_field_for_retained(
    state: &mut MeshPlotState,
    mesh: &TriangleMesh,
    field: Option<&ScalarField>,
    view: &MeshPlotView,
) -> Result<Option<Rc<ScalarField>>, ChartError> {
    match view {
        MeshPlotView::AxisymmetricRevolve(spec) => field
            .map(|field| state.revolved_field_for(mesh, spec, field))
            .transpose()
            .map_err(Into::into),
        _ => Ok(field.map(|field| Rc::new(field.clone()))),
    }
}

#[cfg(all(feature = "gpu-3d", not(test)))]
fn render_3d_mesh_and_field_for_retained(
    state: &mut MeshPlotState,
    mesh: &TriangleMesh,
    field: Option<&ScalarField>,
    view: &MeshPlotView,
) -> Result<(TriangleMesh, Option<Rc<ScalarField>>), ChartError> {
    match view {
        MeshPlotView::AxisymmetricRevolve(spec) => {
            let (revolved, _) = state.revolved_bvh_for(mesh, spec)?;
            let field = field
                .map(|field| state.revolved_field_for(mesh, spec, field))
                .transpose()?;
            Ok((revolved.mesh.clone(), field))
        }
        _ => Ok((mesh.clone(), field.map(|field| Rc::new(field.clone())))),
    }
}

#[cfg(feature = "gpu-3d")]
pub(crate) fn build_retained_3d_scene_state(
    render_mesh: &TriangleMesh,
    render_field: Option<&ScalarField>,
    mode: &MeshRenderMode,
    wireframe: Wireframe,
    color_scale: &ColorScale,
    range: Option<[f64; 2]>,
) -> Rc<RefCell<d3rs::mesh::gpu::MeshSceneState>> {
    use d3rs::mesh::gpu::{FieldRevision, GeometryRevision, MeshColorConfig, MeshSceneState};
    use d3rs::mesh::{prepare_field, prepare_upload};

    let topology = MeshTopology::build(&render_mesh.triangles);
    let mut upload = prepare_upload(render_mesh, &topology);
    if let Some(field) = render_field {
        let values = prepare_field(field);
        match field.association {
            ScalarAssociation::Vertex => upload.values_f32 = Some(values),
            ScalarAssociation::Cell => upload.cell_values_f32 = Some(values),
        }
    }
    let color_range = range.unwrap_or([0.0, 1.0]);
    Rc::new(RefCell::new(MeshSceneState {
        geometry_rev: GeometryRevision(1),
        field_rev: FieldRevision(u64::from(render_field.is_some())),
        upload: Some(upload),
        geometry_upload_count: 0,
        geometry_upload_bytes: 0,
        view_transform: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        color: MeshColorConfig {
            colormap: color_scale.to_colormap_index(),
            range: [color_range[0] as f32, color_range[1] as f32],
            wireframe: wireframe == Wireframe::Overlay || matches!(mode, MeshRenderMode::Mesh),
            isoline_step: isoline_step(mode, range).unwrap_or(0.0) as f32,
            isoline_width_px: 1.0,
            unlit: true,
        },
    }))
}

#[cfg(feature = "gpu-3d")]
fn update_retained_3d_scene_state(
    scene: &Rc<RefCell<d3rs::mesh::gpu::MeshSceneState>>,
    field: Option<&ScalarField>,
    mode: &MeshRenderMode,
    wireframe: Wireframe,
    color_scale: &ColorScale,
    range: Option<[f64; 2]>,
    field_revision: u64,
) {
    use d3rs::mesh::gpu::{FieldRevision, MeshColorConfig};
    use d3rs::mesh::prepare_field;

    let mut scene = scene.borrow_mut();
    scene.field_rev = FieldRevision(field_revision);
    if let Some(upload) = scene.upload.as_mut() {
        upload.values_f32 = None;
        upload.cell_values_f32 = None;
        if let Some(field) = field {
            let values = prepare_field(field);
            match field.association {
                ScalarAssociation::Vertex => upload.values_f32 = Some(values),
                ScalarAssociation::Cell => upload.cell_values_f32 = Some(values),
            }
        }
    }
    let color_range = range.unwrap_or([0.0, 1.0]);
    scene.color = MeshColorConfig {
        colormap: color_scale.to_colormap_index(),
        range: [color_range[0] as f32, color_range[1] as f32],
        wireframe: wireframe == Wireframe::Overlay || matches!(mode, MeshRenderMode::Mesh),
        isoline_step: isoline_step(mode, range).unwrap_or(0.0) as f32,
        isoline_width_px: 1.0,
        unlit: true,
    };
}

#[cfg(feature = "gpu-2d")]
fn mesh_view_transform(
    origin: [f64; 3],
    x_domain: [f64; 2],
    y_domain: [f64; 2],
    plot_width: f32,
    plot_height: f32,
    equal_aspect: bool,
) -> [[f32; 4]; 4] {
    let x_span = (x_domain[1] - x_domain[0]).max(f64::EPSILON);
    let y_span = (y_domain[1] - y_domain[0]).max(f64::EPSILON);
    let (scale_x, scale_y, offset_x, offset_y) = if equal_aspect {
        let pixels_per_unit = (plot_width as f64 / x_span).min(plot_height as f64 / y_span);
        let scale_x = 2.0 * pixels_per_unit / plot_width.max(1.0) as f64;
        let scale_y = 2.0 * pixels_per_unit / plot_height.max(1.0) as f64;
        let used_x = x_span * scale_x;
        let used_y = y_span * scale_y;
        (
            scale_x,
            scale_y,
            (2.0 - used_x) * 0.5 - 1.0,
            (2.0 - used_y) * 0.5 - 1.0,
        )
    } else {
        (2.0 / x_span, 2.0 / y_span, -1.0, -1.0)
    };
    let tx = offset_x - (origin[0] + x_domain[0]) * scale_x;
    let ty = offset_y - (origin[1] + y_domain[0]) * scale_y;
    [
        [scale_x as f32, 0.0, 0.0, 0.0],
        [0.0, scale_y as f32, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [tx as f32, ty as f32, 0.0, 1.0],
    ]
}

#[cfg(feature = "gpu-2d")]
fn update_scene_view_transform(
    scene: &Rc<RefCell<d3rs::mesh::gpu::MeshSceneState>>,
    interaction: &MeshPlotState,
    plot_width: f32,
    plot_height: f32,
    equal_aspect: bool,
) {
    let (x_min, x_max) = interaction.interaction.x_domain();
    let (y_min, y_max) = interaction.interaction.y_domain();
    let mut scene = scene.borrow_mut();
    let Some(origin) = scene.upload.as_ref().map(|upload| upload.origin) else {
        return;
    };
    scene.view_transform = mesh_view_transform(
        origin,
        [x_min, x_max],
        [y_min, y_max],
        plot_width,
        plot_height,
        equal_aspect,
    );
}

fn isoline_step(mode: &MeshRenderMode, range: Option<[f64; 2]>) -> Option<f64> {
    let range = range?;
    let levels = match mode {
        MeshRenderMode::Isolines { levels } | MeshRenderMode::FillAndIsolines { levels } => {
            levels.resolve(range).ok()?
        }
        _ => return None,
    };
    levels.windows(2).find_map(|pair| {
        let step = pair[1] - pair[0];
        (step.is_finite() && step > 0.0).then_some(step)
    })
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

#[cfg(feature = "gpu-3d")]
fn pick_3d_for_view_retained(
    state: &mut MeshPlotState,
    mesh: &TriangleMesh,
    field: Option<&ScalarField>,
    view: &MeshPlotView,
    camera: &d3rs::gpu3d::Camera3D,
    screen: [f32; 2],
    viewport: [f32; 2],
    plot_id: &str,
) -> Option<MeshPlotPick> {
    match view {
        MeshPlotView::Surface3d => {
            let bvh = state.bvh_for(mesh);
            super::picking3d::pick_3d_with_bvh(mesh, field, &bvh, camera, screen, viewport, plot_id)
        }
        MeshPlotView::AxisymmetricRevolve(spec) => {
            let (revolved, bvh) = state.revolved_bvh_for(mesh, spec).ok()?;
            let derived_field = field
                .map(|field| state.revolved_field_for(mesh, spec, field))
                .transpose()
                .ok()?;
            super::picking3d::pick_revolved_3d_with_bvh(
                mesh,
                &revolved,
                derived_field.as_deref(),
                &bvh,
                field.map(|field| field.id.clone()),
                camera,
                screen,
                viewport,
                plot_id,
            )
        }
        _ => None,
    }
}

fn toolbar_view_name(view: &MeshPlotView) -> &'static str {
    match view {
        MeshPlotView::Planar { .. } => "Planar",
        MeshPlotView::AxisymmetricSection { .. } => "Axisymmetric section",
        MeshPlotView::AxisymmetricRevolve(_) => "Axisymmetric revolve",
        MeshPlotView::Surface3d => "Surface 3D",
    }
}

/// Build the native hover-tooltip payload from retained plot state. Keeping
/// formatting outside the element composition makes the visible overlay and
/// accessibility text share the same coordinates, IDs, label, value, and
/// unit contract.
fn mesh_hover_tooltip_text(state: &MeshPlotState, field: Option<&ScalarField>) -> Option<String> {
    state.hover_tooltip_with_field(field)
}

fn finite_domain(points: &[[f64; 2]], axis: usize) -> Option<[f64; 2]> {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for point in points {
        min = min.min(point[axis]);
        max = max.max(point[axis]);
    }
    (min.is_finite() && max.is_finite()).then_some([min, max.max(min + f64::EPSILON)])
}

fn resolve_value_range(
    field: Option<&ScalarField>,
    range: ColorRange,
) -> Result<Option<[f64; 2]>, ChartError> {
    let Some(field) = field else {
        return Ok(None);
    };
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
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
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return Ok(None);
    }
    range.resolve(min, max).map(Some)
}

fn contour_geometry(
    mesh: &TriangleMesh,
    field: Option<&ScalarField>,
    topology: &MeshTopology,
    horizontal: CoordinateAxis,
    vertical: CoordinateAxis,
    mode: &MeshRenderMode,
    range: Option<[f64; 2]>,
) -> Result<(Vec<ContourBand>, Vec<IsolineSegment>), ChartError> {
    let Some(field) = field else {
        return Ok((Vec::new(), Vec::new()));
    };
    let levels = match mode {
        MeshRenderMode::FilledContours { levels }
        | MeshRenderMode::Isolines { levels }
        | MeshRenderMode::FillAndIsolines { levels } => {
            levels.resolve(range.unwrap_or([0.0, 1.0]))?
        }
        _ => return Ok((Vec::new(), Vec::new())),
    };
    let marching = MarchingTriangles::new(mesh, field, topology, horizontal, vertical)?;
    let bands = if matches!(
        mode,
        MeshRenderMode::FilledContours { .. } | MeshRenderMode::FillAndIsolines { .. }
    ) {
        marching.filled_bands(&levels)
    } else {
        Vec::new()
    };
    let lines = if matches!(
        mode,
        MeshRenderMode::Isolines { .. } | MeshRenderMode::FillAndIsolines { .. }
    ) {
        marching.isolines(&levels)
    } else {
        Vec::new()
    };
    Ok((bands, lines))
}

struct MeshProjector {
    min: [f64; 2],
    scale: [f64; 2],
    offset: [f64; 2],
    height: f32,
}

impl MeshProjector {
    fn new(points: &[[f64; 2]], width: f32, height: f32, equal_aspect: bool) -> Self {
        let x = finite_domain(points, 0).unwrap_or([0.0, 1.0]);
        let y = finite_domain(points, 1).unwrap_or([0.0, 1.0]);
        let span = [
            (x[1] - x[0]).max(f64::EPSILON),
            (y[1] - y[0]).max(f64::EPSILON),
        ];
        if equal_aspect {
            let scale = (width as f64 / span[0]).min(height as f64 / span[1]);
            let used = [span[0] * scale, span[1] * scale];
            Self {
                min: [x[0], y[0]],
                scale: [scale, scale],
                offset: [
                    ((width as f64 - used[0]) * 0.5),
                    ((height as f64 - used[1]) * 0.5),
                ],
                height,
            }
        } else {
            Self {
                min: [x[0], y[0]],
                scale: [width as f64 / span[0], height as f64 / span[1]],
                offset: [0.0, 0.0],
                height,
            }
        }
    }
    fn point(&self, point: [f64; 2]) -> [f32; 2] {
        [
            (self.offset[0] + (point[0] - self.min[0]) * self.scale[0]) as f32,
            (self.height as f64 - self.offset[1] - (point[1] - self.min[1]) * self.scale[1]) as f32,
        ]
    }
}

fn triangle_points(
    projector: &MeshProjector,
    points: &[[f64; 2]],
    triangle: [u32; 3],
) -> Option<[[f32; 2]; 3]> {
    Some([
        projector.point(*points.get(triangle[0] as usize)?),
        projector.point(*points.get(triangle[1] as usize)?),
        projector.point(*points.get(triangle[2] as usize)?),
    ])
}

fn triangle_points_from_band(
    projector: &MeshProjector,
    points: &[[f64; 2]],
    triangle: [u32; 3],
) -> Option<[[f32; 2]; 3]> {
    triangle_points(projector, points, triangle)
}

fn triangle_value(field: Option<&ScalarField>, triangle: [u32; 3], cell: usize) -> Option<f64> {
    let field = field?;
    match field.association {
        ScalarAssociation::Vertex => {
            if field.valid.as_ref().is_some_and(|valid| {
                triangle
                    .iter()
                    .any(|index| valid.get(*index as usize) != Some(&true))
            }) {
                return None;
            }
            let values = [
                *field.values.get(triangle[0] as usize)?,
                *field.values.get(triangle[1] as usize)?,
                *field.values.get(triangle[2] as usize)?,
            ];
            values
                .iter()
                .all(|value| value.is_finite())
                .then_some(values.iter().sum::<f64>() / 3.0)
        }
        ScalarAssociation::Cell => {
            if field
                .valid
                .as_ref()
                .is_some_and(|valid| valid.get(cell) != Some(&true))
            {
                return None;
            }
            field
                .values
                .get(cell)
                .copied()
                .filter(|value| value.is_finite())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use d3rs::mesh::{ContourLevels, CoordinateAxis};
    use std::{cell::RefCell, rc::Rc, sync::Arc};

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
    #[test]
    fn spec_example_builds() {
        let result = mesh_plot(square_mesh())
            .field(vertex_field())
            .view(MeshPlotView::Planar {
                horizontal: CoordinateAxis::X,
                vertical: CoordinateAxis::Y,
            })
            .mode(MeshRenderMode::FillAndIsolines {
                levels: ContourLevels::Count(12),
            })
            .color_scale(ColorScale::Viridis)
            .color_range(ColorRange::Auto)
            .colorbar(Colorbar::new("Sound pressure level").unit("dB SPL"))
            .wireframe(Wireframe::overlay())
            .axes(Axes2d::equal_aspect().labels("x", "y").unit("m"))
            .interactions(PlotInteractions::inspect_and_navigate())
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn toolbar_action_visibility_is_builder_configurable() {
        let plot = mesh_plot(square_mesh())
            .toolbar(true)
            .toolbar_action_hidden(PlotToolbarAction::Export, true)
            .toolbar_action_hidden(PlotToolbarAction::Export, false)
            .toolbar_action_hidden(PlotToolbarAction::OpenViewMenu, true);
        assert!(
            !plot
                .hidden_toolbar_actions
                .contains(&PlotToolbarAction::Export)
        );
        assert!(
            plot.hidden_toolbar_actions
                .contains(&PlotToolbarAction::OpenViewMenu)
        );
    }

    #[test]
    fn hover_tooltip_payload_includes_field_metadata() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.set_hover(Some(MeshPlotPick {
            plot_id: "plot".into(),
            mesh_id: "mesh".into(),
            cell_index: 2,
            cell_id: Some(19),
            nearest_vertex_index: Some(1),
            vertex_id: Some(7),
            world_position: [0.25, 0.5, 0.75],
            displayed_value: Some(42.0),
            field_id: Some("pressure".into()),
        }));
        let tooltip = mesh_hover_tooltip_text(&state, Some(&vertex_field())).unwrap();
        assert!(tooltip.contains("Cell 2 (id 19); vertex id 7"));
        assert!(tooltip.contains("Pressure 42.000000 dB SPL"));
        assert!(tooltip.contains("0.750000"));
    }
    #[test]
    fn cell_field_contours_return_structured_error() {
        let field = ScalarField {
            id: "cell".into(),
            label: "Cell".into(),
            unit: None,
            values: Arc::from([0.5, 0.7]),
            association: ScalarAssociation::Cell,
            valid: None,
        };
        let result = mesh_plot(square_mesh())
            .field(field)
            .mode(MeshRenderMode::FilledContours {
                levels: ContourLevels::Count(6),
            })
            .build();
        assert!(matches!(
            result,
            Err(ChartError::MeshValidation(
                MeshValidationError::ContoursRequireVertexField
            ))
        ));
    }

    #[test]
    fn scalar_fill_rejects_mismatched_interpolation_association() {
        let cell_field = ScalarField {
            id: "cell".into(),
            label: "Cell".into(),
            unit: None,
            values: Arc::from([0.5, 0.7]),
            association: ScalarAssociation::Cell,
            valid: None,
        };
        let smooth_cell = mesh_plot(square_mesh())
            .field(cell_field)
            .mode(MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            })
            .build();
        assert!(matches!(
            smooth_cell,
            Err(ChartError::InvalidData {
                field: "field.interpolation",
                ..
            })
        ));

        let flat_vertex = mesh_plot(square_mesh())
            .field(vertex_field())
            .mode(MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Flat,
            })
            .build();
        assert!(matches!(
            flat_vertex,
            Err(ChartError::InvalidData {
                field: "field.interpolation",
                ..
            })
        ));
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn retained_3d_field_update_keeps_prepared_geometry() {
        let mesh = square_mesh();
        let initial = vertex_field();
        let scene = build_retained_3d_scene_state(
            &mesh,
            Some(&initial),
            &MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            },
            Wireframe::Hidden,
            &ColorScale::Viridis,
            Some([0.0, 4.0]),
        );
        let (positions, indices, geometry_rev, upload_count, upload_bytes) = {
            let scene = scene.borrow();
            let upload = scene.upload.as_ref().unwrap();
            (
                upload.positions_f32.clone(),
                upload.indices.clone(),
                scene.geometry_rev,
                scene.geometry_upload_count,
                scene.geometry_upload_bytes,
            )
        };
        let updated = ScalarField {
            values: Arc::from([4.0, 3.0, 2.0, 1.0]),
            ..initial
        };
        update_retained_3d_scene_state(
            &scene,
            Some(&updated),
            &MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            },
            Wireframe::Overlay,
            &ColorScale::Magma,
            Some([1.0, 4.0]),
            7,
        );
        let scene = scene.borrow();
        let upload = scene.upload.as_ref().unwrap();
        assert_eq!(scene.geometry_rev, geometry_rev);
        assert_eq!(scene.geometry_upload_count, upload_count);
        assert_eq!(scene.geometry_upload_bytes, upload_bytes);
        assert_eq!(upload.positions_f32, positions);
        assert_eq!(upload.indices, indices);
        assert_eq!(
            upload.values_f32.as_deref(),
            Some(&[4.0, 3.0, 2.0, 1.0][..])
        );
        assert!(upload.cell_values_f32.is_none());
        assert_eq!(scene.field_rev.0, 7);
        assert!(scene.color.wireframe);
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn retained_3d_thousand_alternating_field_updates_reuse_geometry_and_field_storage() {
        let mesh = square_mesh();
        let initial = vertex_field();
        let scene = build_retained_3d_scene_state(
            &mesh,
            Some(&initial),
            &MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            },
            Wireframe::Hidden,
            &ColorScale::Viridis,
            Some([0.0, 4.0]),
        );
        let (positions, indices, geometry_revision, upload_count, upload_bytes, field_capacity) = {
            let scene = scene.borrow();
            let upload = scene.upload.as_ref().expect("prepared 3D upload");
            (
                upload.positions_f32.clone(),
                upload.indices.clone(),
                scene.geometry_rev,
                scene.geometry_upload_count,
                scene.geometry_upload_bytes,
                upload.values_f32.as_ref().expect("vertex field").capacity(),
            )
        };
        for iteration in 0..1_000_u64 {
            let field = ScalarField {
                values: if iteration % 2 == 0 {
                    Arc::from([4.0, 3.0, 2.0, 1.0])
                } else {
                    Arc::from([0.0, 1.0, 2.0, 3.0])
                },
                ..initial.clone()
            };
            update_retained_3d_scene_state(
                &scene,
                Some(&field),
                &MeshRenderMode::ScalarFill {
                    interpolation: FieldInterpolation::Smooth,
                },
                if iteration % 2 == 0 {
                    Wireframe::Overlay
                } else {
                    Wireframe::Hidden
                },
                if iteration % 2 == 0 {
                    &ColorScale::Magma
                } else {
                    &ColorScale::Viridis
                },
                Some([0.0, 4.0]),
                iteration + 1,
            );
        }
        let scene = scene.borrow();
        let upload = scene.upload.as_ref().expect("retained 3D upload");
        assert_eq!(scene.geometry_rev, geometry_revision);
        assert_eq!(scene.geometry_upload_count, upload_count);
        assert_eq!(scene.geometry_upload_bytes, upload_bytes);
        assert_eq!(upload.positions_f32, positions);
        assert_eq!(upload.indices, indices);
        assert_eq!(
            upload.values_f32.as_ref().expect("field").capacity(),
            field_capacity
        );
        assert_eq!(scene.field_rev.0, 1_000);
        assert!(!scene.color.wireframe);
        assert_eq!(
            upload.values_f32.as_deref(),
            Some(&[0.0, 1.0, 2.0, 3.0][..])
        );
    }

    #[test]
    fn mask_nan_policy_turns_nan_samples_into_invalid_mask_entries() {
        let mut field = vertex_field();
        field.values = Arc::from([0.0, f64::NAN, 1.0, 2.0]);
        assert!(
            mesh_plot(square_mesh())
                .field(field.clone())
                .build()
                .is_err()
        );
        assert!(
            mesh_plot(square_mesh())
                .field(field)
                .missing_value_policy(d3rs::mesh::MissingValuePolicy::MaskNaN)
                .build()
                .is_ok()
        );
    }
    #[test]
    fn negative_radius_axisymmetric_rejected() {
        let mut mesh = square_mesh();
        mesh.positions = Arc::from([
            [-0.1, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]);
        let result = mesh_plot(mesh)
            .view(MeshPlotView::AxisymmetricSection {
                radial: CoordinateAxis::X,
                axial: CoordinateAxis::Y,
            })
            .build();
        assert!(matches!(
            result,
            Err(ChartError::MeshValidation(
                MeshValidationError::InvalidRadius { .. }
            ))
        ));
    }
    #[test]
    fn mesh_only_mode_needs_no_field() {
        assert!(
            mesh_plot(square_mesh())
                .mode(MeshRenderMode::Mesh)
                .build()
                .is_ok()
        );
    }
    #[test]
    fn invalid_mesh_surfaces_validation_error() {
        let mut mesh = square_mesh();
        mesh.triangles = Arc::from([[0, 0, 1], [0, 2, 3]]);
        assert!(matches!(
            mesh_plot(mesh).build(),
            Err(ChartError::MeshValidation(_))
        ));
    }

    #[test]
    fn projector_maps_each_domain_to_viewport_without_equal_aspect() {
        let points = [[10.0, 20.0], [30.0, 80.0]];
        let projector = MeshProjector::new(&points, 400.0, 300.0, false);
        assert_eq!(projector.point(points[0]), [0.0, 300.0]);
        assert_eq!(projector.point(points[1]), [400.0, 0.0]);
    }

    #[test]
    fn projector_centers_equal_aspect_domain() {
        let points = [[0.0, 0.0], [2.0, 1.0]];
        let projector = MeshProjector::new(&points, 400.0, 300.0, true);
        assert_eq!(projector.point(points[0]), [0.0, 250.0]);
        assert_eq!(projector.point(points[1]), [400.0, 50.0]);
    }

    #[test]
    fn selection_callback_receives_pick_and_clear_events() {
        let events = Rc::new(RefCell::new(Vec::<Option<MeshPlotPick>>::new()));
        let callback_events = events.clone();
        let plot = mesh_plot(square_mesh()).on_selection(move |selection| {
            callback_events.borrow_mut().push(selection);
        });
        let callback = plot
            .selection_callback
            .expect("on_selection must retain the callback");
        callback(Some(MeshPlotPick {
            plot_id: "plot".into(),
            mesh_id: "square".into(),
            cell_index: 1,
            cell_id: None,
            nearest_vertex_index: Some(2),
            vertex_id: None,
            world_position: [0.5, 0.5, 0.0],
            displayed_value: Some(1.0),
            field_id: None,
        }));
        callback(None);

        let events = events.borrow();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].as_ref().map(|pick| pick.cell_index), Some(1));
        assert!(events[1].is_none());
    }

    #[test]
    fn plot_id_defaults_to_mesh_id_and_can_be_overridden() {
        let plot = mesh_plot(square_mesh()).plot_id("plot");
        assert_eq!(plot.plot_id.as_ref(), "plot");
    }
}
