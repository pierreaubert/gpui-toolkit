use super::interaction::MeshPlotState;
#[cfg(feature = "gpu-3d")]
use super::interaction::PreparedRevolve;
use super::types::*;
use crate::{
    ChartError, ChartSize, ColorRange, ColorScale, Colorbar, DEFAULT_TITLE_FONT_SIZE,
    TITLE_AREA_HEIGHT, apply_chart_size, default_design, resolved_chart_dimensions,
};
use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::grid::{GridConfig, render_grid};
#[cfg(feature = "gpu-3d")]
use d3rs::mesh::MeshBounds;
#[cfg(feature = "gpu-3d")]
use d3rs::mesh::gpu::compute::MeshCompute;
use d3rs::mesh::{
    ContourBand, CoordinateAxis, IsolineSegment, MarchingTriangles, MeshTopology,
    MeshValidationError, ScalarAssociation, ScalarField, TriangleMesh, project_2d,
};
use d3rs::scale::LinearScale;
use d3rs::text::{GlyphTextConfig, render_glyph_text};
#[cfg(any(feature = "gpu-3d", test))]
use gpui::Point;
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Context, Div, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Pixels, Render, RenderOnce, Stateful, Styled, WeakEntity, Window, canvas, div,
    hsla, point, px, rgb,
};
use gpui_design::DesignSystem;
use gpui_ui_kit::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, apply_native_accessibility,
};
use gpui_ui_kit::plot_toolbar::PlotToolbarAction;
use gpui_ui_kit::tooltip::{Tooltip, TooltipPlacement};
#[cfg(feature = "gpui")]
use gpui_ui_kit::{ContextMenu, menu::MenuItem};
use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(feature = "gpu-2d")]
use std::env;
#[cfg(feature = "gpu-2d")]
use std::fs;
#[cfg(feature = "gpu-2d")]
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

type MeshPlotExportCallback = Rc<dyn Fn(Result<String, ChartError>)>;

#[derive(Default)]
struct MeshPlotOccurrenceTracker {
    draw_epoch: u64,
    counts: HashMap<(gpui::WindowId, Arc<str>), usize>,
}

impl MeshPlotOccurrenceTracker {
    fn next(&mut self, draw_epoch: u64, key: (gpui::WindowId, Arc<str>)) -> usize {
        if self.draw_epoch != draw_epoch {
            self.draw_epoch = draw_epoch;
            self.counts.clear();
        }
        let occurrence = self.counts.entry(key).or_default();
        let current = *occurrence;
        *occurrence += 1;
        current
    }
}

#[cfg(all(feature = "gpu-2d", any(not(test), feature = "native-qa")))]
enum Mesh2dDrawOwner {
    #[cfg(all(feature = "gpu-metal", target_os = "macos"))]
    Metal(d3rs::mesh::gpu::MetalMeshRenderer),
    Wgpu(d3rs::mesh::gpu::WgpuMeshRenderer),
}

#[cfg(all(feature = "gpu-2d", any(not(test), feature = "native-qa")))]
impl Mesh2dDrawOwner {
    fn custom_id(&self) -> gpui::CustomDrawId {
        match self {
            #[cfg(all(feature = "gpu-metal", target_os = "macos"))]
            Self::Metal(renderer) => renderer.custom_id(),
            Self::Wgpu(renderer) => renderer.custom_id(),
        }
    }
}

#[cfg(all(feature = "gpu-2d", any(not(test), feature = "native-qa")))]
fn new_mesh_2d_draw_owner(
    state: Rc<RefCell<d3rs::mesh::gpu::MeshSceneState>>,
    backend: MeshPlotBackend,
) -> Mesh2dDrawOwner {
    if matches!(backend, MeshPlotBackend::Wgpu) {
        return Mesh2dDrawOwner::Wgpu(d3rs::mesh::gpu::WgpuMeshRenderer::new(state));
    }
    #[cfg(all(feature = "gpu-metal", target_os = "macos"))]
    {
        Mesh2dDrawOwner::Metal(d3rs::mesh::gpu::MetalMeshRenderer::new(state))
    }
    #[cfg(not(all(feature = "gpu-metal", target_os = "macos")))]
    {
        Mesh2dDrawOwner::Wgpu(d3rs::mesh::gpu::WgpuMeshRenderer::new(state))
    }
}

thread_local! {
    /// MeshPlot builders are often recreated by a parent `Render` after a
    /// Python/resource patch. Distinguish same-ID siblings by their encounter
    /// order in the current draw; GPUI's keyed element-state store then owns
    /// each live entity exactly while that component remains mounted.
    static MESH_PLOT_OCCURRENCES: RefCell<MeshPlotOccurrenceTracker> =
        RefCell::new(MeshPlotOccurrenceTracker::default());
}

/// Classify a declarative rebuild by the immutable data that determines
/// prepared mesh buffers. Parent `Render` implementations commonly recreate a
/// builder on each notification, and resource adapters may decode fresh `Arc`
/// buffers even when the resource generation is unchanged. The fallback
/// comparison is therefore exact and uses f64 bit patterns so a masked NaN is
/// stable across rebuilds.
fn mesh_plot_resource_domains_changed(previous: &MeshPlot, next: &MeshPlot) -> (bool, bool) {
    let positions_changed = previous.mesh.positions.as_ptr() != next.mesh.positions.as_ptr()
        && (previous.mesh.positions.len() != next.mesh.positions.len()
            || previous
                .mesh
                .positions
                .iter()
                .zip(next.mesh.positions.iter())
                .any(|(previous, next)| {
                    previous
                        .iter()
                        .zip(next.iter())
                        .any(|(previous, next)| previous.to_bits() != next.to_bits())
                }));
    let geometry_changed = positions_changed
        || (previous.mesh.triangles.as_ptr() != next.mesh.triangles.as_ptr()
            && previous.mesh.triangles.as_ref() != next.mesh.triangles.as_ref())
        || match (&previous.mesh.vertex_ids, &next.mesh.vertex_ids) {
            (Some(previous), Some(next)) => {
                previous.as_ptr() != next.as_ptr() && previous.as_ref() != next.as_ref()
            }
            (None, None) => false,
            _ => true,
        }
        || match (&previous.mesh.cell_ids, &next.mesh.cell_ids) {
            (Some(previous), Some(next)) => {
                previous.as_ptr() != next.as_ptr() && previous.as_ref() != next.as_ref()
            }
            (None, None) => false,
            _ => true,
        }
        // Axisymmetric views derive a different render mesh from the same
        // source buffers, so a projection/spec switch is a geometry change.
        || previous.view != next.view
        || previous.renderer_backend != next.renderer_backend;
    let field_changed = match (&previous.field, &next.field) {
        (Some(previous), Some(next)) => {
            (previous.values.as_ptr() != next.values.as_ptr()
                && (previous.values.len() != next.values.len()
                    || previous
                        .values
                        .iter()
                        .zip(next.values.iter())
                        .any(|(previous, next)| previous.to_bits() != next.to_bits())))
                || match (&previous.valid, &next.valid) {
                    (Some(previous), Some(next)) => {
                        previous.as_ptr() != next.as_ptr() && previous.as_ref() != next.as_ref()
                    }
                    (None, None) => false,
                    _ => true,
                }
                || previous.association != next.association
        }
        (None, None) => false,
        _ => true,
    };
    (geometry_changed, field_changed)
}

#[cfg(feature = "gpu-2d")]
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

#[cfg(any(feature = "gpu-3d", test))]
fn plot_local_position(position: Point<Pixels>, bounds: Bounds<Pixels>) -> [f32; 2] {
    [
        f32::from(position.x) - f32::from(bounds.origin.x),
        f32::from(position.y) - f32::from(bounds.origin.y),
    ]
}

/// Windows GPUI currently has no native custom-mesh primitive. Leaving the
/// custom ID unset makes `MeshSceneElement` render its retained upload through
/// `render_offscreen` and paint the resulting GPUI image, so DirectX receives
/// ordinary image primitives instead of silently discarding the chart.
const fn mesh_custom_draw_supported(target_os: &str) -> bool {
    !matches!(target_os.as_bytes(), b"windows")
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

/// A one-shot builder wrapper that promotes a `MeshPlot` to a retained GPUI
/// view at layout time.  The builder API stays ergonomic while the live view
/// gains a stable owner for toolbar/menu state, completion notifications, and
/// future background preparation results.
struct MeshPlotElement {
    plot: MeshPlot,
}

struct MeshPlotLiveElement {
    plot: MeshPlot,
    /// The most recent plot that successfully built a frame. Retaining the
    /// declarative input lets a recoverable resource/renderer error keep the
    /// last valid frame visible instead of replacing it with a blank panel.
    last_valid_plot: Option<MeshPlot>,
    first_frame: bool,
    toolbar_menu: Option<MeshPlotToolbarMenu>,
    focus_handle: FocusHandle,
    toolbar_menu_focus_handle: FocusHandle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MeshPlotToolbarMenu {
    Mode,
    View,
}

impl RenderOnce for MeshPlotElement {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let MeshPlotElement { mut plot } = self;
        let window_id = window.window_handle().window_id();
        let occurrence = MESH_PLOT_OCCURRENCES.with(|occurrences| {
            occurrences
                .borrow_mut()
                .next(window.draw_epoch(), (window_id, plot.plot_id.clone()))
        });
        let element_key = format!("mesh-plot-live-{}-{occurrence}", plot.plot_id);
        let initial_plot = plot.clone();
        let entity =
            window.use_keyed_state(element_key, cx, move |_window, cx| MeshPlotLiveElement {
                plot: initial_plot,
                last_valid_plot: None,
                first_frame: true,
                toolbar_menu: None,
                focus_handle: cx.focus_handle(),
                toolbar_menu_focus_handle: cx.focus_handle(),
            });
        // When this builder is rendered again, retain the entity and its local
        // menu/preparation ownership while atomically replacing declarative
        // data and configuration.
        entity.update(cx, |live, cx| {
            // Most declarative callers create a fresh builder without an
            // explicit state handle. Preserve the retained owner in that
            // common case so preparation, camera, selection, and toolbar
            // changes survive the parent redraw that delivered new resources.
            if plot.state.is_none() {
                plot.state = live.plot.state.clone();
            }
            let shares_retained_state = match (&live.plot.state, &plot.state) {
                (Some(previous), Some(next)) => Rc::ptr_eq(previous, next),
                _ => false,
            };
            if shares_retained_state {
                let (geometry_changed, field_changed) =
                    mesh_plot_resource_domains_changed(&live.plot, &plot);
                #[cfg(all(feature = "gpu-2d", any(not(test), feature = "native-qa")))]
                if !geometry_changed {
                    // Keep the platform custom draw registered across
                    // declarative field/style rebuilds. Its backend resources
                    // are keyed by the retained scene revision, so a field
                    // patch can write only the scalar buffer instead of
                    // allocating a new geometry resource.
                    plot.retained_2d_draw_owner = live.plot.retained_2d_draw_owner.clone();
                }
                if (geometry_changed || field_changed)
                    && let Some(state) = plot.state.as_ref()
                {
                    state
                        .borrow_mut()
                        .mark_resources_changed(geometry_changed, field_changed);
                }
            }
            live.plot = plot;
            cx.notify();
        });
        entity
    }
}

impl IntoElement for MeshPlotElement {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

impl Render for MeshPlotLiveElement {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let frame = self.plot.build_frame(
            cx,
            self.first_frame,
            self.toolbar_menu,
            &self.focus_handle,
            &self.toolbar_menu_focus_handle,
        );
        self.first_frame = false;
        match frame {
            Ok(frame) => {
                self.last_valid_plot = Some(self.plot.clone());
                frame
            }
            Err(error) => {
                // `MeshPlot::build` performs validation before this retained
                // view is created. This branch protects a later live
                // rebuild from a recoverable renderer/resource failure while
                // preserving the last complete frame and its camera/state.
                let error_text = error.to_string();
                if let Some(last_valid) = self.last_valid_plot.as_mut()
                    && let Ok(frame) = last_valid.build_frame(
                        cx,
                        false,
                        self.toolbar_menu,
                        &self.focus_handle,
                        &self.toolbar_menu_focus_handle,
                    )
                {
                    return div()
                        .size_full()
                        .relative()
                        .child(frame)
                        .child(
                            div()
                                .absolute()
                                .top(px(8.0))
                                .right(px(8.0))
                                .px(px(8.0))
                                .py(px(4.0))
                                .bg(rgb(0x6b2737))
                                .text_color(rgb(0xffffff))
                                .child(format!("Mesh update rejected: {error_text}")),
                        )
                        .into_any_element();
                }
                div()
                    .size_full()
                    .bg(rgb(0xf4f5f7))
                    .child(format!("Mesh plot unavailable: {error_text}"))
                    .into_any_element()
            }
        }
    }
}

/// Builder for unstructured 2D/axisymmetric mesh charts.
#[derive(Clone)]
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
    pub(crate) export_callback: Option<MeshPlotExportCallback>,
    pub(crate) show_toolbar: bool,
    pub(crate) hidden_toolbar_actions: Vec<PlotToolbarAction>,
    pub(crate) renderer_backend: MeshPlotBackend,
    #[cfg(all(feature = "gpu-2d", any(not(test), feature = "native-qa")))]
    retained_2d_draw_owner: Option<Rc<Mesh2dDrawOwner>>,
}

impl MeshPlot {
    /// Set the stable plot identity carried by hover and selection picks.
    /// Defaults to the geometry ID for backwards compatibility.
    pub fn plot_id(mut self, plot_id: impl Into<String>) -> Self {
        self.plot_id = Arc::from(plot_id.into());
        self
    }

    /// Select the retained GPU backend for live rendering.
    ///
    /// `Auto` is the default platform choice. `Wgpu` is intended for explicit
    /// cross-adapter rendering and is especially useful on macOS builds that
    /// also include the native Metal feature.
    pub fn renderer_backend(mut self, backend: MeshPlotBackend) -> Self {
        self.renderer_backend = backend;
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
    /// Receive the deterministic SVG produced by the toolbar Export action.
    /// The host owns file saving, so the component stays usable in sandboxed
    /// and embedded windows.
    pub fn on_export<F>(mut self, callback: F) -> Self
    where
        F: Fn(Result<String, ChartError>) + 'static,
    {
        self.export_callback = Some(Rc::new(callback));
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
        let (layout_width, layout_height) = resolved_chart_dimensions(self.chart_size);
        crate::validate::validate_dimensions(layout_width, layout_height)?;
        Ok(MeshPlotElement { plot: self })
    }

    /// Compose the current frame of a retained live plot. The public builder
    /// validates once, then this method is re-entered whenever the live owner
    /// is notified by navigation, toolbar actions, or preparation completion.
    fn build_frame(
        &mut self,
        cx: &mut Context<MeshPlotLiveElement>,
        first_frame: bool,
        toolbar_menu: Option<MeshPlotToolbarMenu>,
        focus_handle: &FocusHandle,
        toolbar_menu_focus_handle: &FocusHandle,
    ) -> Result<AnyElement, ChartError> {
        let live = cx.entity().clone();
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
        let mesh_x_domain = finite_domain(&projected, 0).ok_or(ChartError::InvalidData {
            field: "mesh.positions",
            reason: "mesh projection must contain finite coordinates",
        })?;
        let mesh_y_domain = finite_domain(&projected, 1).ok_or(ChartError::InvalidData {
            field: "mesh.positions",
            reason: "mesh projection must contain finite coordinates",
        })?;
        let (configured_x_domain, configured_y_domain) = self.axes.configured_ranges();
        let x_domain = configured_x_domain.unwrap_or(mesh_x_domain);
        let y_domain = configured_y_domain.unwrap_or(mesh_y_domain);
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

        let theme = DefaultAxisTheme;
        let (horizontal_title, vertical_title) = self.axes.titles(&self.view, horizontal, vertical);
        let axis_x = AxisConfig::bottom()
            .with_design(&design)
            .with_title(horizontal_title);
        let axis_y = AxisConfig::left()
            .with_design(&design)
            .with_title(vertical_title);
        let grid = GridConfig::default()
            .with_design(&design)
            .with_dots(self.axes.show_grid());

        let mesh = self.mesh.clone();
        let field = self.field.clone();
        #[cfg(feature = "gpu-2d")]
        let equal_aspect = self.axes.equal_aspect;
        let color_scale = self.color_scale.clone();
        #[cfg(feature = "gpu-2d")]
        let projected_for_render = projected.clone();
        #[cfg(feature = "gpu-2d")]
        let mesh_for_render = mesh.clone();
        #[cfg(feature = "gpu-2d")]
        let field_for_render = field.clone();
        let selection_callback = self.selection_callback.clone();

        #[cfg(feature = "gpu-3d")]
        let state_is_new = self.state.is_none();
        #[cfg(feature = "gpu-3d")]
        let needs_revolve_preparation_state =
            matches!(self.view, MeshPlotView::AxisymmetricRevolve(_))
                && self.mesh.triangles.len() >= ASYNC_REVOLVE_TRIANGLE_THRESHOLD;
        let interaction_state = if self.interactions.is_interactive() || self.show_toolbar || {
            #[cfg(feature = "gpu-3d")]
            {
                needs_revolve_preparation_state
            }
            #[cfg(not(feature = "gpu-3d"))]
            {
                false
            }
        } {
            let state = self.state.clone().unwrap_or_else(|| {
                Rc::new(RefCell::new(MeshPlotState::new(
                    x_domain[0],
                    x_domain[1],
                    y_domain[0],
                    y_domain[1],
                )))
            });
            // A builder-created state must live beyond this frame so toolbar
            // actions, async preparation, and exporter snapshots all address
            // the same retained plot instance.
            self.state = Some(state.clone());
            {
                let mut state_ref = state.borrow_mut();
                state_ref.interaction = state_ref
                    .interaction
                    .clone()
                    .with_size(plot_width, plot_height);
                if first_frame {
                    state_ref.set_style(self.mode.clone(), self.wireframe, self.color_range);
                    if state_ref.selection.is_none() {
                        state_ref.selection = self.selection.clone();
                    }
                }
            }
            Some(state)
        } else {
            None
        };

        // After the initial frame, the live state is authoritative for all
        // style values that native controls can mutate. Rebuilding from the
        // original builder values would immediately undo a toolbar action.
        let (mode, wireframe, active_color_range) = interaction_state
            .as_ref()
            .map(|state| {
                let state = state.borrow();
                (
                    state.render_mode.clone(),
                    state.wireframe,
                    state.color_range,
                )
            })
            .unwrap_or_else(|| (self.mode.clone(), self.wireframe, self.color_range));
        let value_range = resolve_value_range(self.field.as_ref(), active_color_range)?;
        let range_for_render = value_range;
        let toolbar_mode_label = format!("{mode:?}");
        let (visible_x_domain, visible_y_domain) = interaction_state
            .as_ref()
            .map(|state| {
                let state = state.borrow();
                let x = state.interaction.x_domain();
                let y = state.interaction.y_domain();
                ([x.0, x.1], [y.0, y.1])
            })
            .unwrap_or((x_domain, y_domain));
        let x_scale = LinearScale::new()
            .domain(visible_x_domain[0], visible_x_domain[1])
            .range(0.0, plot_width as f64);
        let y_scale = LinearScale::new()
            .domain(visible_y_domain[0], visible_y_domain[1])
            .range(plot_height as f64, 0.0);

        #[cfg(all(feature = "gpu-3d", not(test)))]
        let revolve_preparing = match (&self.view, interaction_state.as_ref()) {
            (MeshPlotView::AxisymmetricRevolve(spec), Some(state))
                if self.mesh.triangles.len() >= ASYNC_REVOLVE_TRIANGLE_THRESHOLD =>
            {
                let already_prepared =
                    state
                        .borrow()
                        .has_prepared_revolve(&self.mesh, spec, self.field.as_ref());
                if !already_prepared {
                    let key = state.borrow_mut().begin_revolve_preparation(
                        &self.mesh,
                        spec,
                        self.field.as_ref(),
                    );
                    if let Some(key) = key {
                        let background_mesh = self.mesh.clone();
                        let background_spec = spec.clone();
                        let background_field = self.field.clone();
                        let task = cx.background_spawn(async move {
                            let started = Instant::now();
                            let prepared = prepare_revolve(
                                &background_mesh,
                                &background_spec,
                                background_field.as_ref(),
                            );
                            (prepared, started.elapsed())
                        });
                        cx.spawn(async move |this: WeakEntity<MeshPlotLiveElement>, cx| {
                            let (prepared, elapsed) = task.await;
                            let _ = this.update(cx, |live, cx| {
                                let Some(state) = live.plot.state.as_ref() else {
                                    return;
                                };
                                let mut state = state.borrow_mut();
                                state.record_revolve_preparation(elapsed);
                                if !state.finish_revolve_preparation(&key) {
                                    return;
                                }
                                if let Ok(prepared) = prepared
                                    && state.store_prepared_revolve(
                                        &key,
                                        &live.plot.mesh,
                                        live.plot.field.as_ref(),
                                        prepared,
                                    )
                                {
                                    // The previous ready scene remains visible
                                    // until this point. Build the new upload on
                                    // the following retained render and fit to
                                    // the new derived bounds exactly once.
                                    state.retained_3d = None;
                                    state.camera_fitted = false;
                                }
                                cx.notify();
                            });
                        })
                        .detach();
                    }
                    true
                } else {
                    false
                }
            }
            _ => false,
        };

        #[cfg(not(feature = "gpu-3d"))]
        let _revolve_preparing = false;

        #[cfg(all(feature = "gpu-3d", test))]
        let revolve_preparing = false;

        let cached_contours = interaction_state.as_ref().and_then(|state| {
            state.borrow().cached_contours(
                &self.mesh,
                self.field.as_ref(),
                horizontal,
                vertical,
                &mode,
                value_range,
            )
        });
        let run_contours_in_background = interaction_state.is_some()
            && requires_contour_preparation(&mode)
            && self.mesh.triangles.len() >= ASYNC_CONTOUR_TRIANGLE_THRESHOLD;
        let (contour_bands, isolines) = if let Some(cached) = cached_contours {
            cached
        } else if run_contours_in_background {
            let Some(state) = interaction_state.as_ref() else {
                unreachable!("background contour preparation requires retained state")
            };
            let previous = state.borrow().previous_contours();
            let key = state.borrow_mut().begin_contour_preparation(
                &self.mesh,
                self.field.as_ref(),
                horizontal,
                vertical,
                &mode,
                value_range,
            );
            if let Some(key) = key {
                let background_mesh = self.mesh.clone();
                let background_field = self.field.clone();
                let background_mode = mode.clone();
                let completion_mode = mode.clone();
                let task = cx.background_spawn(async move {
                    let background_topology = MeshTopology::build(&background_mesh.triangles);
                    let started = Instant::now();
                    let result = {
                        #[cfg(feature = "gpu-3d")]
                        {
                            contour_geometry_with_compute(
                                &background_mesh,
                                background_field.as_ref(),
                                &background_topology,
                                horizontal,
                                vertical,
                                &background_mode,
                                value_range,
                            )
                        }
                        #[cfg(not(feature = "gpu-3d"))]
                        {
                            contour_geometry(
                                &background_mesh,
                                background_field.as_ref(),
                                &background_topology,
                                horizontal,
                                vertical,
                                &background_mode,
                                value_range,
                            )
                        }
                    };
                    (result, started.elapsed())
                });
                cx.spawn(async move |this: WeakEntity<MeshPlotLiveElement>, cx| {
                    let (prepared, elapsed) = task.await;
                    let _ = this.update(cx, |live, cx| {
                        let Some(state) = live.plot.state.as_ref() else {
                            return;
                        };
                        let mut state = state.borrow_mut();
                        state.record_contour_preparation(elapsed);
                        if !state.finish_contour_preparation(&key) {
                            return;
                        }
                        if let Ok((bands, lines)) = prepared {
                            state.store_contours(
                                &live.plot.mesh,
                                live.plot.field.as_ref(),
                                horizontal,
                                vertical,
                                &completion_mode,
                                value_range,
                                Rc::new(bands),
                                Rc::new(lines),
                            );
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
            previous.unwrap_or_else(|| (Rc::new(Vec::new()), Rc::new(Vec::new())))
        } else {
            let started = Instant::now();
            let prepared = contour_geometry(
                &self.mesh,
                self.field.as_ref(),
                &topology,
                horizontal,
                vertical,
                &mode,
                value_range,
            );
            if let Some(state) = interaction_state.as_ref() {
                state
                    .borrow_mut()
                    .record_contour_preparation(started.elapsed());
            }
            let (bands, lines) = prepared?;
            let bands = Rc::new(bands);
            let lines = Rc::new(lines);
            if let Some(state) = interaction_state.as_ref() {
                state.borrow_mut().store_contours(
                    &self.mesh,
                    self.field.as_ref(),
                    horizontal,
                    vertical,
                    &mode,
                    value_range,
                    bands.clone(),
                    lines.clone(),
                );
            }
            (bands, lines)
        };

        #[cfg(not(feature = "gpu-2d"))]
        let _ = (&contour_bands, &isolines);

        #[cfg(feature = "gpu-2d")]
        let retained_state = build_retained_scene_state(
            &mesh,
            field.as_ref(),
            &projected,
            x_domain,
            y_domain,
            plot_width,
            plot_height,
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
        ) && self.interactions.is_interactive()
        {
            let Some(state) = interaction_state.clone() else {
                return Err(ChartError::UnsupportedView {
                    view: "mesh-3d",
                    reason: "interactive state is unavailable",
                });
            };
            state
                .borrow_mut()
                .set_camera_aspect(plot_width, plot_height);
            if (!state.borrow().camera_fitted || state_is_new) && !revolve_preparing {
                let bounds = match &self.view {
                    MeshPlotView::AxisymmetricRevolve(spec) => {
                        let (revolved, _) = state.borrow_mut().revolved_bvh_for(&mesh, spec)?;
                        MeshBounds::from_positions(&revolved.mesh.positions)
                    }
                    MeshPlotView::Surface3d => MeshBounds::from_positions(&mesh.positions),
                    _ => unreachable!("retained 3D state only accepts 3D views"),
                };
                state
                    .borrow_mut()
                    .fit_camera_to_bounds(bounds, plot_width / plot_height.max(1.0));
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
            if revolve_preparing {
                if let Some(retained) = owner.retained_3d.as_ref().cloned() {
                    // A geometry patch may be expensive to revolve. Keep the
                    // last complete upload/camera visible until the worker
                    // delivers an atomically accepted replacement.
                    retained.renderer.set_camera(&camera);
                    retained.scene.borrow_mut().view_transform =
                        camera.view_projection_matrix().to_cols_array_2d();
                    (
                        retained.scene.clone(),
                        retained.renderer.clone(),
                        Some(retained.lod.clone()),
                    )
                } else {
                    // First large revolve: render the source profile as a
                    // deliberately lightweight preparing representation. This
                    // avoids a hidden synchronous `revolve()` while making it
                    // clear that the final surface is still being prepared.
                    let fresh_retained_3d_state = build_retained_3d_scene_state(
                        &mesh,
                        field.as_ref(),
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
                        mesh.clone(),
                        field.as_ref(),
                    )));
                    owner.retained_3d = Some(super::interaction::RetainedMesh3D {
                        scene: fresh_retained_3d_state.clone(),
                        renderer: renderer.clone(),
                        lod: lod.clone(),
                        geometry_revision,
                    });
                    (fresh_retained_3d_state, renderer, Some(lod))
                }
            } else if let Some(retained) = owner
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
                retained.scene.borrow_mut().view_transform =
                    camera.view_projection_matrix().to_cols_array_2d();
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
            any(not(test), feature = "native-qa")
        ))]
        let retained_3d_custom_id = {
            if let Some(camera) = retained_3d_camera.as_ref() {
                retained_3d_state.borrow_mut().view_transform =
                    camera.borrow().view_projection_matrix().to_cols_array_2d();
            }
            if matches!(self.renderer_backend, MeshPlotBackend::Wgpu) {
                retained_3d_renderer.custom_id()
            } else if let Some(renderer) = self.retained_2d_draw_owner.as_ref() {
                renderer.custom_id()
            } else {
                let renderer = retained_3d_camera.as_ref().map_or_else(
                    || d3rs::mesh::gpu::MetalMeshRenderer::new_3d(retained_3d_state.clone()),
                    |camera| {
                        d3rs::mesh::gpu::MetalMeshRenderer::new_3d_with_camera(
                            retained_3d_state.clone(),
                            camera.clone(),
                        )
                    },
                );
                let custom_id = renderer.custom_id();
                self.retained_2d_draw_owner = Some(Rc::new(Mesh2dDrawOwner::Metal(renderer)));
                custom_id
            }
        };

        #[cfg(all(
            feature = "gpu-3d",
            not(all(feature = "gpu-metal", target_os = "macos")),
            any(not(test), feature = "native-qa")
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
                plot_width,
                plot_height,
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
                #[cfg(any(not(test), feature = "native-qa"))]
                let scene = if mesh_custom_draw_supported(std::env::consts::OS) {
                    scene.with_custom_id(retained_3d_custom_id)
                } else {
                    scene
                };
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
            #[cfg(any(not(test), feature = "native-qa"))]
            let scene = if mesh_custom_draw_supported(std::env::consts::OS) {
                let renderer =
                    new_mesh_2d_draw_owner(retained_state.clone(), self.renderer_backend);
                let custom_id = renderer.custom_id();
                self.retained_2d_draw_owner = Some(Rc::new(renderer));
                scene.with_custom_id(custom_id)
            } else {
                scene
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
                    )
                    .with_viewport(visible_x_domain, visible_y_domain);
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
                #[cfg(any(not(test), feature = "native-qa"))]
                let scene = if mesh_custom_draw_supported(std::env::consts::OS) {
                    scene.with_custom_id(retained_3d_custom_id)
                } else {
                    scene
                };
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
        let plot_element = if self.interactions.is_interactive()
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
            let navigation_width = plot_width;
            let navigation_height = plot_height;
            let hover_plot_id = plot_id.clone();
            let select_plot_id = plot_id;
            let drag_state = Rc::new(RefCell::new(None::<[f32; 2]>));
            let drag_down = drag_state.clone();
            let drag_move = drag_state.clone();
            let drag_up = drag_state.clone();
            let brush_state = state.clone();
            let callback = selection_callback.clone();
            let focus_handle = focus_handle.clone();
            let focus_on_pointer_down = focus_handle.clone();
            let focus_on_click = focus_handle.clone();
            // Mouse events arrive in window coordinates. Cache the actual
            // painted plot bounds so nested layouts, titles, axes, and resize
            // all share one local-coordinate conversion.
            let interaction_bounds: Rc<RefCell<Option<Bounds<Pixels>>>> =
                Rc::new(RefCell::new(None));
            let bounds_for_paint = interaction_bounds.clone();
            let bounds_for_move = interaction_bounds.clone();
            let bounds_for_down = interaction_bounds.clone();
            let bounds_for_scroll = interaction_bounds.clone();
            let bounds_recorder = canvas(
                move |_bounds, _window, _cx| (),
                move |bounds, (), _window, _cx| {
                    let _ = bounds_for_paint.borrow_mut().replace(bounds);
                },
            )
            .absolute()
            .inset_0();
            // GPUI dispatches keyboard events along the focused element's
            // ancestor path. Keep the focus target as the interactive canvas,
            // and install the key handler on its stable parent so it remains
            // reachable after retained-frame rebuilds.
            let allow_pan = self.interactions.allows_pan();
            let allow_zoom = self.interactions.allows_zoom();
            let allow_inspect = self.interactions.allows_inspect();
            let allow_select = self.interactions.allows_select();
            let allow_reset = self.interactions.allows_reset();
            let allow_fit = self.interactions.allows_fit();

            let interaction_surface = div()
                .size_full()
                .id(format!("mesh-plot-{}", mesh.id))
                .track_focus(&focus_handle)
                .focusable()
                .cursor_grab()
                .child(plot_element)
                .child(bounds_recorder)
                .on_mouse_move(move |event: &gpui::MouseMoveEvent, window, _cx| {
                    let Some(bounds) = *bounds_for_move.borrow() else {
                        return;
                    };
                    let screen_x = f32::from(event.position.x) - f32::from(bounds.origin.x);
                    let screen_y = f32::from(event.position.y) - f32::from(bounds.origin.y);
                    let mut state = hover_state.borrow_mut();
                    if state
                        .interaction
                        .update_hover_pixel(screen_x, screen_y)
                        .is_none()
                    {
                        state.set_hover(None);
                        return;
                    }
                    let Some([x, y]) = mesh_point_to_domain(
                        &state,
                        screen_x,
                        screen_y,
                        navigation_width,
                        navigation_height,
                        equal_aspect,
                    ) else {
                        // Letterbox bars are outside the rendered mesh. Do
                        // not turn their pixel coordinates into false picks.
                        state.set_hover(None);
                        return;
                    };
                    if state.interaction.is_brushing() {
                        if allow_zoom {
                            state.interaction.update_brush(screen_x, screen_y);
                            window.refresh();
                        }
                    } else if let Some(previous) = *drag_move.borrow() {
                        if !allow_pan {
                            return;
                        }
                        let dx = screen_x - previous[0];
                        let dy = screen_y - previous[1];
                        if dx.abs() > 0.0 || dy.abs() > 0.0 {
                            state.interaction.pan_by_pixels(dx, dy);
                            *drag_move.borrow_mut() = Some([screen_x, screen_y]);
                            update_scene_view_transform(
                                &pan_scene_move,
                                &state,
                                navigation_width,
                                navigation_height,
                                equal_aspect,
                            );
                            window.refresh();
                        }
                    } else if allow_inspect {
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
                    move |event: &gpui::MouseDownEvent, window, cx| {
                        window.focus(&focus_on_pointer_down, cx);
                        let Some(bounds) = *bounds_for_down.borrow() else {
                            return;
                        };
                        let screen = [
                            f32::from(event.position.x) - f32::from(bounds.origin.x),
                            f32::from(event.position.y) - f32::from(bounds.origin.y),
                        ];
                        let mut state = select_state.borrow_mut();
                        if state
                            .interaction
                            .update_hover_pixel(screen[0], screen[1])
                            .is_none()
                        {
                            write_mesh_qa_hit_trace(
                                screen,
                                [navigation_width, navigation_height],
                                false,
                            );
                            return;
                        }
                        let Some([x, y]) = mesh_point_to_domain(
                            &state,
                            screen[0],
                            screen[1],
                            navigation_width,
                            navigation_height,
                            equal_aspect,
                        ) else {
                            write_mesh_qa_hit_trace(
                                screen,
                                [navigation_width, navigation_height],
                                false,
                            );
                            return;
                        };
                        if event.modifiers.shift && allow_zoom {
                            state.interaction.start_brush(screen[0], screen[1]);
                            *drag_down.borrow_mut() = None;
                        } else {
                            if allow_select {
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
                                if let Some(callback) = &callback {
                                    callback(pick);
                                }
                            }
                            *drag_down.borrow_mut() = allow_pan.then_some(screen);
                        }
                    },
                )
                .on_mouse_up(gpui::MouseButton::Left, move |_event, window, _cx| {
                    let mut state = brush_state.borrow_mut();
                    if state.interaction.is_brushing() && allow_zoom {
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
                .on_click(move |event: &gpui::ClickEvent, window, cx| {
                    window.focus(&focus_on_click, cx);
                    if event.click_count() >= 2 && allow_reset {
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
                    let Some(bounds) = *bounds_for_scroll.borrow() else {
                        return;
                    };
                    if !allow_zoom {
                        return;
                    }
                    let mut state = scroll_state.borrow_mut();
                    let x = f32::from(event.position.x) - f32::from(bounds.origin.x);
                    let y = f32::from(event.position.y) - f32::from(bounds.origin.y);
                    let Some([focus_x, focus_y]) = mesh_point_to_domain(
                        &state,
                        x,
                        y,
                        navigation_width,
                        navigation_height,
                        equal_aspect,
                    ) else {
                        return;
                    };
                    state.interaction.zoom_around_domain(
                        focus_x,
                        focus_y,
                        (1.0 - delta * 0.1).max(0.1) as f64,
                    );
                    update_scene_view_transform(
                        &scroll_scene,
                        &state,
                        navigation_width,
                        navigation_height,
                        equal_aspect,
                    );
                    window.refresh();
                });
            div()
                .size_full()
                .on_key_down(move |event: &gpui::KeyDownEvent, window, _cx| {
                    let mut state = key_state.borrow_mut();
                    if state.handle_key_with_permissions(
                        &event.keystroke.key,
                        allow_pan,
                        allow_zoom,
                        allow_reset,
                        allow_fit,
                    ) {
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
                .child(interaction_surface)
                .into_any_element()
        } else {
            plot_element
        };

        #[cfg(not(feature = "gpu-2d"))]
        let plot_element = plot_element;

        // Selection is a live visual state, not only an export annotation.
        // Keep this as a transparent retained 2D layer so it follows the
        // same viewport/equal-aspect mapping as the mesh and remains visible
        // for both the GPU mesh path and the CPU contour path. The state is
        // borrowed only while the draw callback runs; pointer handlers can
        // therefore update it and request a normal GPUI repaint.
        #[cfg(feature = "gpu-2d")]
        let selection_overlay = if matches!(
            &self.view,
            MeshPlotView::Planar { .. } | MeshPlotView::AxisymmetricSection { .. }
        ) && (self.selection.is_some() || interaction_state.is_some())
        {
            let selection_state = interaction_state.clone();
            let static_selection = self.selection.clone();
            let selection_mesh = mesh.clone();
            let selection_projected = projected.clone();
            let selection_equal_aspect = equal_aspect;
            Some(
                canvas(
                    move |bounds, _window, _cx| {
                        let selection = retained_overlay_selection(
                            selection_state.as_ref(),
                            static_selection.as_ref(),
                        );
                        let Some(selection) = selection else {
                            return None;
                        };
                        let width = f32::from(bounds.size.width).max(1.0);
                        let height = f32::from(bounds.size.height).max(1.0);
                        let projector = MeshProjector::new(
                            &selection_projected,
                            width,
                            height,
                            selection_equal_aspect,
                        )
                        .with_viewport(visible_x_domain, visible_y_domain);
                        selected_triangle_points(
                            &selection_mesh,
                            &selection,
                            &projector,
                            &selection_projected,
                        )
                    },
                    move |bounds, points, window, _cx| {
                        let Some(points) = points else {
                            return;
                        };
                        let origin_x = f32::from(bounds.origin.x);
                        let origin_y = f32::from(bounds.origin.y);
                        let mut builder = gpui::PathBuilder::stroke(px(2.0));
                        builder.move_to(point(
                            px(origin_x + points[0][0]),
                            px(origin_y + points[0][1]),
                        ));
                        for point_position in points.iter().skip(1) {
                            builder.line_to(point(
                                px(origin_x + point_position[0]),
                                px(origin_y + point_position[1]),
                            ));
                        }
                        builder.close();
                        if let Ok(path) = builder.build() {
                            window.paint_path(
                                path,
                                gpui::Rgba {
                                    r: 1.0,
                                    g: 0.55,
                                    b: 0.0,
                                    a: 1.0,
                                },
                            );
                        }
                    },
                )
                .absolute()
                .inset_0()
                .into_any_element(),
            )
        } else {
            None
        };

        #[cfg(not(feature = "gpu-2d"))]
        let selection_overlay: Option<AnyElement> = None;

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
            let state_key = state.clone();
            let selection_callback_3d = selection_callback.clone();
            let clear_selection_callback_3d = selection_callback.clone();
            let viewport = [plot_width, plot_height];
            let allow_pan = self.interactions.allows_pan();
            let allow_zoom = self.interactions.allows_zoom();
            let allow_inspect = self.interactions.allows_inspect();
            let allow_select = self.interactions.allows_select();
            let allow_reset = self.interactions.allows_reset();
            let allow_fit = self.interactions.allows_fit();
            let keyboard_fit_bounds_3d = if revolve_preparing {
                None
            } else {
                match &self.view {
                    MeshPlotView::Surface3d => Some(MeshBounds::from_positions(&mesh.positions)),
                    MeshPlotView::AxisymmetricRevolve(spec) => {
                        state.borrow_mut().revolved_bvh_for(&mesh, spec).ok().map(
                            |(revolved, _)| MeshBounds::from_positions(&revolved.mesh.positions),
                        )
                    }
                    _ => None,
                }
            }
            .map(|bounds| (bounds, plot_width / plot_height.max(1.0)));
            let focus_handle_3d = focus_handle.clone();
            let focus_on_pointer_down_3d = focus_handle_3d.clone();
            let interaction_bounds: Rc<RefCell<Option<Bounds<Pixels>>>> =
                Rc::new(RefCell::new(None));
            let bounds_for_paint = interaction_bounds.clone();
            let bounds_for_down = interaction_bounds.clone();
            let bounds_for_middle_down = interaction_bounds.clone();
            let bounds_for_move = interaction_bounds;
            let bounds_recorder = canvas(
                move |_bounds, _window, _cx| (),
                move |bounds, (), _window, _cx| {
                    let _ = bounds_for_paint.borrow_mut().replace(bounds);
                },
            )
            .absolute()
            .inset_0();
            let preparing_overlay = revolve_preparing.then(|| {
                div()
                    .absolute()
                    .top(px(8.0))
                    .left(px(8.0))
                    .px(px(8.0))
                    .py(px(4.0))
                    .bg(rgb(0x30343b))
                    .text_color(rgb(0xffffff))
                    .child("Preparing 3D surface…")
                    .into_any_element()
            });
            div()
                .size_full()
                .relative()
                .id(format!("mesh-plot-3d-{}", mesh.id))
                .track_focus(&focus_handle_3d)
                .focusable()
                .child(plot_element)
                .child(bounds_recorder)
                .children(preparing_overlay)
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    move |event: &gpui::MouseDownEvent, window, cx| {
                        window.focus(&focus_on_pointer_down_3d, cx);
                        let Some(bounds) = *bounds_for_down.borrow() else {
                            return;
                        };
                        let screen = plot_local_position(event.position, bounds);
                        let mut state = state_down.borrow_mut();
                        if allow_select {
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
                        }
                        if allow_pan {
                            if let Some(lod) = lod_down.as_ref() {
                                lod.borrow_mut()
                                    .begin_drag(&mut lod_scene_down.borrow_mut());
                            }
                            *pan_drag_down.borrow_mut() = false;
                            *drag_down.borrow_mut() = Some(screen);
                        }
                    },
                )
                .on_mouse_down(
                    gpui::MouseButton::Middle,
                    move |event: &gpui::MouseDownEvent, _window, _cx| {
                        let Some(bounds) = *bounds_for_middle_down.borrow() else {
                            return;
                        };
                        if !allow_pan {
                            return;
                        }
                        if let Some(lod) = lod_middle_down.as_ref() {
                            lod.borrow_mut()
                                .begin_drag(&mut lod_scene_middle_down.borrow_mut());
                        }
                        *pan_drag_middle_down.borrow_mut() = true;
                        *drag_middle_down.borrow_mut() =
                            Some(plot_local_position(event.position, bounds));
                    },
                )
                .on_mouse_move(move |event: &gpui::MouseMoveEvent, window, _cx| {
                    let Some(bounds) = *bounds_for_move.borrow() else {
                        return;
                    };
                    let current = plot_local_position(event.position, bounds);
                    let Some(previous) = *drag_move.borrow() else {
                        if !allow_inspect {
                            return;
                        }
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
                    if !allow_pan {
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
                    if !allow_zoom {
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
                        if state.handle_3d_key_with_fit(
                            &event.keystroke.key,
                            allow_pan,
                            allow_zoom,
                            allow_reset,
                            allow_fit,
                            keyboard_fit_bounds_3d,
                        ) {
                            *camera.borrow_mut() = state.camera.clone();
                            camera_scene_reset.borrow_mut().view_transform =
                                state.camera.view_projection_matrix().to_cols_array_2d();
                            window.refresh();
                            return;
                        }
                    }
                    if event.keystroke.key == "escape" && allow_reset {
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
            .child(render_axis(&y_scale, &axis_y, plot_height, &theme))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w(px(plot_width))
                            .h(px(plot_height))
                            .relative()
                            .overflow_hidden()
                            .bg(rgb(0xf8f8f8))
                            .child(render_grid(
                                &x_scale,
                                &y_scale,
                                &grid,
                                plot_width,
                                plot_height,
                                &theme,
                            ))
                            .child(div().absolute().inset_0().size_full().child(plot_element))
                            .children(selection_overlay)
                            .children(hover_tooltip),
                    )
                    .child(render_axis(&x_scale, &axis_x, plot_width, &theme)),
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
        let mut body = div().relative().flex().child(chart_content);
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
                    .render(&design, plot_height),
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
            let toolbar_3d_bounds = toolbar_state.as_ref().and_then(|state| {
                let mut state = state.borrow_mut();
                match &self.view {
                    MeshPlotView::Surface3d => Some(MeshBounds::from_positions(&mesh.positions)),
                    MeshPlotView::AxisymmetricRevolve(spec)
                        if !state.revolve_preparation_pending(&mesh, spec, field.as_ref()) =>
                    {
                        state
                            .revolved_bvh_for(&mesh, spec)
                            .ok()
                            .map(|(revolved, _)| {
                                MeshBounds::from_positions(&revolved.mesh.positions)
                            })
                    }
                    _ => None,
                }
            });
            let toolbar_field = field.clone();
            let toolbar_hidden_actions = self.hidden_toolbar_actions.clone();
            let toolbar_live = live.clone();
            let toolbar_export = self.export_callback.clone();
            let toolbar_action_state = toolbar_state.clone();
            let toolbar_action_field = toolbar_field.clone();
            let toolbar_menu_focus = toolbar_menu_focus_handle.clone();
            let mut toolbar = PlotToolbar::new("mesh-plot-toolbar")
                .mode(toolbar_mode_label)
                .view(toolbar_view_name(&self.view))
                .wireframe(wireframe == Wireframe::Overlay)
                .disabled(PlotToolbarAction::Export, toolbar_export.is_none());
            if !toolbar_is_3d {
                toolbar = toolbar.hidden(PlotToolbarAction::OpenViewMenu, true);
            }
            if !self.interactions.allows_fit() {
                toolbar = toolbar.hidden(PlotToolbarAction::Fit, true);
            }
            if !self.interactions.allows_reset() {
                toolbar = toolbar.hidden(PlotToolbarAction::Reset, true);
            }
            for action in toolbar_hidden_actions {
                toolbar = toolbar.hidden(action, true);
            }
            let toolbar = toolbar
                .on_action(move |action, window, _cx| {
                    if matches!(action, PlotToolbarAction::OpenModeMenu) {
                        toolbar_live.update(_cx, |plot, cx| {
                            plot.toolbar_menu = Some(MeshPlotToolbarMenu::Mode);
                            cx.notify();
                        });
                        window.focus(&toolbar_menu_focus, _cx);
                        return;
                    }
                    if matches!(action, PlotToolbarAction::OpenViewMenu) {
                        toolbar_live.update(_cx, |plot, cx| {
                            plot.toolbar_menu = Some(MeshPlotToolbarMenu::View);
                            cx.notify();
                        });
                        window.focus(&toolbar_menu_focus, _cx);
                        return;
                    }
                    if matches!(action, PlotToolbarAction::Export) {
                        if let Some(callback) = toolbar_export.as_ref() {
                            toolbar_live.update(_cx, |plot, cx| {
                                callback(plot.plot.to_svg());
                                cx.notify();
                            });
                        }
                        return;
                    }
                    let Some(state) = toolbar_action_state.as_ref() else {
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
                                                plot_width / plot_height.max(1.0),
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
                                    plot_width,
                                    plot_height,
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
                            let range = resolve_value_range(
                                toolbar_action_field.as_ref(),
                                ColorRange::Auto,
                            )
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
                        | PlotToolbarAction::Export => unreachable!("handled before state access"),
                    }
                    window.refresh();
                    toolbar_live.update(_cx, |_plot, cx| cx.notify());
                })
                .build();
            body = body.child(toolbar);

            if let Some(menu) = toolbar_menu {
                let active_menu_mode = toolbar_state
                    .as_ref()
                    .map(|state| state.borrow().render_mode.clone())
                    .unwrap_or(MeshRenderMode::Mesh);
                let items = match menu {
                    MeshPlotToolbarMenu::Mode => {
                        mesh_toolbar_mode_items(toolbar_field.as_ref(), &active_menu_mode)
                    }
                    MeshPlotToolbarMenu::View => mesh_toolbar_view_items(),
                };
                let menu_live = live.clone();
                let close_live = live.clone();
                let menu_state = toolbar_state.clone();
                let menu_is_3d = toolbar_is_3d;
                let menu_focus = toolbar_menu_focus_handle.clone();
                let plot_focus_on_select = focus_handle.clone();
                let plot_focus_on_close = focus_handle.clone();
                body = body.child(
                    ContextMenu::new("mesh-plot-toolbar-menu", items)
                        .position(point(px(4.0), px(38.0)))
                        .aria_label(match menu {
                            MeshPlotToolbarMenu::Mode => "Mesh plot mode menu",
                            MeshPlotToolbarMenu::View => "Mesh plot view menu",
                        })
                        .focused_index(0)
                        .focus_handle(menu_focus)
                        .on_select(move |id, window, cx| {
                            menu_live.update(cx, |plot, cx| {
                                if let Some(state) = menu_state.as_ref() {
                                    let mut state = state.borrow_mut();
                                    apply_mesh_toolbar_menu_selection(
                                        &mut state,
                                        menu,
                                        id.as_ref(),
                                        menu_is_3d,
                                    );
                                }
                                plot.toolbar_menu = None;
                                cx.notify();
                            });
                            window.focus(&plot_focus_on_select, cx);
                            window.refresh();
                        })
                        .on_close(move |window, cx| {
                            close_live.update(cx, |plot, cx| {
                                plot.toolbar_menu = None;
                                cx.notify();
                            });
                            window.focus(&plot_focus_on_close, cx);
                        }),
                );
            }
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
        }
        .into_any_element())
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
            // Keep builder validation cheap. `revolve()` performs the same
            // mesh/spec/radius checks below, but it also allocates the full
            // derived surface, computes normals, and builds topology. Large
            // interactive profiles must defer that work to the retained
            // background preparation path rather than doing it before the
            // first "Preparing 3D surface…" frame can be presented.
            if spec.segments < 3
                || !spec.start_angle.is_finite()
                || !spec.sweep_angle.is_finite()
                || spec.sweep_angle <= 0.0
                || spec.sweep_angle > std::f64::consts::TAU
            {
                return Err(MeshValidationError::InvalidRevolveSpec.into());
            }
            for (index, position) in self.mesh.positions.iter().enumerate() {
                let radius = spec.radial.component(*position);
                if radius < -1e-12 {
                    return Err(MeshValidationError::InvalidRadius {
                        index,
                        value: radius,
                    }
                    .into());
                }
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
        for (field, range) in [
            ("axes.horizontal_range", self.axes.configured_ranges().0),
            ("axes.vertical_range", self.axes.configured_ranges().1),
        ] {
            if let Some([min, max]) = range
                && (!min.is_finite() || !max.is_finite() || max <= min)
            {
                return Err(ChartError::InvalidData {
                    field,
                    reason: "axis range must be finite and strictly increasing",
                });
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
        export_callback: None,
        show_toolbar: false,
        hidden_toolbar_actions: Vec::new(),
        renderer_backend: MeshPlotBackend::default(),
        #[cfg(all(feature = "gpu-2d", any(not(test), feature = "native-qa")))]
        retained_2d_draw_owner: None,
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
        field_write_count: 0,
        field_write_bytes: 0,
        gpu_field_write_count: 0,
        gpu_field_write_bytes: 0,
        gpu_geometry_upload_count: 0,
        gpu_geometry_upload_bytes: 0,
        gpu_field_capacity_bytes: 0,
        gpu_resident_bytes: 0,
        gpu_driver_allocated_bytes: None,
        gpu_peak_driver_allocated_bytes: 0,
        gpu_peak_resident_bytes: 0,
        gpu_peak_field_capacity_bytes: 0,
        gpu_memory_release_count: 0,
        gpu_geometry_upload_time_ns: 0,
        gpu_field_write_time_ns: 0,
        gpu_frame_time_ns: 0,
        gpu_frame_count: 0,
        gpu_frame_gpu_time_ns: 0,
        gpu_frame_gpu_time_count: 0,
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
        field_write_count: 0,
        field_write_bytes: 0,
        gpu_field_write_count: 0,
        gpu_field_write_bytes: 0,
        gpu_geometry_upload_count: 0,
        gpu_geometry_upload_bytes: 0,
        gpu_field_capacity_bytes: 0,
        gpu_resident_bytes: 0,
        gpu_driver_allocated_bytes: None,
        gpu_peak_driver_allocated_bytes: 0,
        gpu_peak_resident_bytes: 0,
        gpu_peak_field_capacity_bytes: 0,
        gpu_memory_release_count: 0,
        gpu_geometry_upload_time_ns: 0,
        gpu_field_write_time_ns: 0,
        gpu_frame_time_ns: 0,
        gpu_frame_count: 0,
        gpu_frame_gpu_time_ns: 0,
        gpu_frame_gpu_time_count: 0,
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
fn mesh_point_to_domain(
    state: &MeshPlotState,
    screen_x: f32,
    screen_y: f32,
    plot_width: f32,
    plot_height: f32,
    equal_aspect: bool,
) -> Option<[f64; 2]> {
    if !screen_x.is_finite()
        || !screen_y.is_finite()
        || plot_width <= 0.0
        || plot_height <= 0.0
        || screen_x < 0.0
        || screen_y < 0.0
        || screen_x > plot_width
        || screen_y > plot_height
    {
        return None;
    }
    let (x_domain_min, x_domain_max) = state.interaction.x_domain();
    let (y_domain_min, y_domain_max) = state.interaction.y_domain();
    let x_span = (x_domain_max - x_domain_min).max(f64::EPSILON);
    let y_span = (y_domain_max - y_domain_min).max(f64::EPSILON);
    if !equal_aspect {
        return Some([
            x_domain_min + f64::from(screen_x) / f64::from(plot_width) * x_span,
            y_domain_max - f64::from(screen_y) / f64::from(plot_height) * y_span,
        ]);
    }

    let pixels_per_unit = (f64::from(plot_width) / x_span).min(f64::from(plot_height) / y_span);
    let used_width = x_span * pixels_per_unit;
    let used_height = y_span * pixels_per_unit;
    let left = (f64::from(plot_width) - used_width) * 0.5;
    let top = (f64::from(plot_height) - used_height) * 0.5;
    let screen_x = f64::from(screen_x);
    let screen_y = f64::from(screen_y);
    if screen_x < left
        || screen_x > left + used_width
        || screen_y < top
        || screen_y > top + used_height
    {
        return None;
    }
    Some([
        x_domain_min + (screen_x - left) / pixels_per_unit,
        y_domain_max - (screen_y - top) / pixels_per_unit,
    ])
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
    let started = Instant::now();
    let result = (|| match view {
        MeshPlotView::Surface3d => {
            let bvh = state.bvh_for(mesh);
            super::picking3d::pick_3d_with_bvh(mesh, field, &bvh, camera, screen, viewport, plot_id)
        }
        MeshPlotView::AxisymmetricRevolve(spec) => {
            if state.revolve_preparation_pending(mesh, spec, field) {
                // The visible fallback belongs to a previous complete scene
                // (or the lightweight initial profile), so do not synchronously
                // build a new derived BVH merely to service a pointer event.
                None
            } else {
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
        }
        _ => None,
    })();
    state.record_pick(started.elapsed());
    result
}

fn toolbar_view_name(view: &MeshPlotView) -> &'static str {
    match view {
        MeshPlotView::Planar { .. } => "Planar",
        MeshPlotView::AxisymmetricSection { .. } => "Axisymmetric section",
        MeshPlotView::AxisymmetricRevolve(_) => "Axisymmetric revolve",
        MeshPlotView::Surface3d => "Surface 3D",
    }
}

#[cfg(feature = "gpui")]
fn mesh_toolbar_mode_items(field: Option<&ScalarField>, current: &MeshRenderMode) -> Vec<MenuItem> {
    let mut items = vec![MenuItem::checkbox(
        "mesh",
        "Mesh",
        matches!(current, MeshRenderMode::Mesh),
    )];
    let Some(field) = field else {
        return items;
    };
    match field.association {
        ScalarAssociation::Vertex => {
            items.push(MenuItem::checkbox(
                "smooth-fill",
                "Smooth scalar fill",
                matches!(
                    current,
                    MeshRenderMode::ScalarFill {
                        interpolation: FieldInterpolation::Smooth
                    }
                ),
            ));
            items.push(MenuItem::checkbox(
                "filled-contours",
                "Filled contours",
                matches!(current, MeshRenderMode::FilledContours { .. }),
            ));
            items.push(MenuItem::checkbox(
                "isolines",
                "Isolines",
                matches!(current, MeshRenderMode::Isolines { .. }),
            ));
            items.push(MenuItem::checkbox(
                "fill-and-isolines",
                "Fill and isolines",
                matches!(current, MeshRenderMode::FillAndIsolines { .. }),
            ));
        }
        ScalarAssociation::Cell => items.push(MenuItem::checkbox(
            "flat-fill",
            "Flat scalar fill",
            matches!(
                current,
                MeshRenderMode::ScalarFill {
                    interpolation: FieldInterpolation::Flat
                }
            ),
        )),
    }
    items
}

#[cfg(feature = "gpui")]
fn mesh_toolbar_view_items() -> Vec<MenuItem> {
    [
        ("front", "Front"),
        ("back", "Back"),
        ("left", "Left"),
        ("right", "Right"),
        ("top", "Top"),
        ("bottom", "Bottom"),
        ("isometric", "Isometric"),
        ("projection", "Toggle perspective / orthographic"),
    ]
    .into_iter()
    .map(|(id, label)| MenuItem::new(id, label))
    .collect()
}

#[cfg(feature = "gpui")]
fn apply_mesh_toolbar_menu_selection(
    state: &mut MeshPlotState,
    menu: MeshPlotToolbarMenu,
    id: &str,
    is_3d: bool,
) {
    match menu {
        MeshPlotToolbarMenu::Mode => {
            let mode = match id {
                "mesh" => MeshRenderMode::Mesh,
                "smooth-fill" => MeshRenderMode::ScalarFill {
                    interpolation: FieldInterpolation::Smooth,
                },
                "flat-fill" => MeshRenderMode::ScalarFill {
                    interpolation: FieldInterpolation::Flat,
                },
                "filled-contours" => MeshRenderMode::FilledContours {
                    levels: d3rs::mesh::ContourLevels::Count(12),
                },
                "isolines" => MeshRenderMode::Isolines {
                    levels: d3rs::mesh::ContourLevels::Count(12),
                },
                "fill-and-isolines" => MeshRenderMode::FillAndIsolines {
                    levels: d3rs::mesh::ContourLevels::Count(12),
                },
                _ => return,
            };
            state.set_render_mode(mode);
        }
        MeshPlotToolbarMenu::View if is_3d =>
        {
            #[cfg(feature = "gpu-3d")]
            match id {
                "front" => state.orbit_standard_view(d3rs::gpu3d::StandardView::Front),
                "back" => state.orbit_standard_view(d3rs::gpu3d::StandardView::Back),
                "left" => state.orbit_standard_view(d3rs::gpu3d::StandardView::Left),
                "right" => state.orbit_standard_view(d3rs::gpu3d::StandardView::Right),
                "top" => state.orbit_standard_view(d3rs::gpu3d::StandardView::Top),
                "bottom" => state.orbit_standard_view(d3rs::gpu3d::StandardView::Bottom),
                "isometric" => state.orbit_standard_view(d3rs::gpu3d::StandardView::Isometric),
                "projection" => state.toggle_projection(),
                _ => {}
            }
        }
        MeshPlotToolbarMenu::View => {}
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

/// Adapter-backed isolines are an opportunistic acceleration for large live
/// plots. Filled-band clipping stays on the deterministic CPU implementation
/// until a matching GPU band pipeline exists. Any adapter creation/dispatch
/// failure deliberately falls back to `contour_geometry`.
#[cfg(feature = "gpu-3d")]
fn contour_geometry_with_compute(
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
    let cpu_bands = || marching.filled_bands(&levels);
    let cpu_lines = || marching.isolines(&levels);
    let Some(compute) = MeshCompute::try_new() else {
        return Ok((
            if matches!(
                mode,
                MeshRenderMode::FilledContours { .. } | MeshRenderMode::FillAndIsolines { .. }
            ) {
                cpu_bands()
            } else {
                Vec::new()
            },
            if matches!(
                mode,
                MeshRenderMode::Isolines { .. } | MeshRenderMode::FillAndIsolines { .. }
            ) {
                cpu_lines()
            } else {
                Vec::new()
            },
        ));
    };
    let bands = if matches!(
        mode,
        MeshRenderMode::FilledContours { .. } | MeshRenderMode::FillAndIsolines { .. }
    ) {
        compute
            .band_triangles_projected(mesh, field, topology, horizontal, vertical, &levels)
            .unwrap_or_else(|_| cpu_bands())
    } else {
        Vec::new()
    };
    let lines = if matches!(
        mode,
        MeshRenderMode::Isolines { .. } | MeshRenderMode::FillAndIsolines { .. }
    ) {
        compute
            .marching_segments_projected(mesh, field, topology, horizontal, vertical, &levels)
            .unwrap_or_else(|_| cpu_lines())
    } else {
        Vec::new()
    };
    Ok((bands, lines))
}

/// Above this size, marching-triangle bands and isolines are prepared on the
/// GPUI background executor. The threshold keeps small plots synchronous and
/// deterministic while protecting interactive render frames for solver-scale
/// meshes.
const ASYNC_CONTOUR_TRIANGLE_THRESHOLD: usize = 10_000;

/// Revolve generation expands every profile vertex and cell by the requested
/// angular resolution and then builds a BVH, so use the same large-mesh policy
/// as contours: never do that work during a live frame once the profile is
/// large enough to be user-visible.
#[cfg(feature = "gpu-3d")]
const ASYNC_REVOLVE_TRIANGLE_THRESHOLD: usize = 10_000;

#[cfg(feature = "gpu-3d")]
#[cfg_attr(test, allow(dead_code))]
fn prepare_revolve(
    mesh: &TriangleMesh,
    spec: &d3rs::mesh::RevolveSpec,
    field: Option<&ScalarField>,
) -> Result<PreparedRevolve, ChartError> {
    let revolved = d3rs::mesh::revolve(mesh, spec)?;
    let bvh = d3rs::mesh::MeshBvh::build(&revolved.mesh);
    let field = field.map(|field| super::picking3d::revolved_field(field, &revolved));
    Ok(PreparedRevolve {
        revolved,
        bvh,
        field,
    })
}

fn requires_contour_preparation(mode: &MeshRenderMode) -> bool {
    matches!(
        mode,
        MeshRenderMode::FilledContours { .. }
            | MeshRenderMode::Isolines { .. }
            | MeshRenderMode::FillAndIsolines { .. }
    )
}

#[cfg(any(feature = "gpu-2d", test))]
struct MeshProjector {
    min: [f64; 2],
    scale: [f64; 2],
    offset: [f64; 2],
    width: f32,
    height: f32,
    equal_aspect: bool,
}

#[cfg(any(feature = "gpu-2d", test))]
impl MeshProjector {
    fn new(points: &[[f64; 2]], width: f32, height: f32, equal_aspect: bool) -> Self {
        let x = finite_domain(points, 0).unwrap_or([0.0, 1.0]);
        let y = finite_domain(points, 1).unwrap_or([0.0, 1.0]);
        Self::from_domains(x, y, width, height, equal_aspect)
    }

    fn with_viewport(self, x: [f64; 2], y: [f64; 2]) -> Self {
        Self::from_domains(x, y, self.width, self.height, self.equal_aspect)
    }

    fn from_domains(x: [f64; 2], y: [f64; 2], width: f32, height: f32, equal_aspect: bool) -> Self {
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
                width,
                height,
                equal_aspect,
            }
        } else {
            Self {
                min: [x[0], y[0]],
                scale: [width as f64 / span[0], height as f64 / span[1]],
                offset: [0.0, 0.0],
                width,
                height,
                equal_aspect,
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

#[cfg(any(feature = "gpu-2d", test))]
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

#[cfg(any(feature = "gpu-2d", test))]
fn selected_triangle_points(
    mesh: &TriangleMesh,
    selection: &MeshPlotPick,
    projector: &MeshProjector,
    projected: &[[f64; 2]],
) -> Option<[[f32; 2]; 3]> {
    if selection.mesh_id.as_ref() != mesh.id.as_ref() {
        return None;
    }
    triangle_points(
        projector,
        projected,
        *mesh.triangles.get(selection.cell_index as usize)?,
    )
}

#[cfg(any(feature = "gpu-2d", test))]
fn retained_overlay_selection(
    state: Option<&Rc<RefCell<MeshPlotState>>>,
    fallback: Option<&MeshPlotPick>,
) -> Option<MeshPlotPick> {
    match state {
        Some(state) => state
            .try_borrow()
            .ok()
            .and_then(|state| state.selection.clone()),
        None => fallback.cloned(),
    }
}

#[cfg(any(feature = "gpu-2d", test))]
fn triangle_points_from_band(
    projector: &MeshProjector,
    points: &[[f64; 2]],
    triangle: [u32; 3],
) -> Option<[[f32; 2]; 3]> {
    triangle_points(projector, points, triangle)
}

#[cfg(any(feature = "gpu-2d", test))]
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
    fn selected_triangle_overlay_uses_the_live_mesh_projection() {
        let mesh = square_mesh();
        let projected = mesh
            .positions
            .iter()
            .copied()
            .map(|point| project_2d(CoordinateAxis::X, CoordinateAxis::Y, point))
            .collect::<Vec<_>>();
        let projector = MeshProjector::from_domains([0.0, 1.0], [0.0, 1.0], 100.0, 80.0, true);
        let selection = MeshPlotPick {
            plot_id: "plot".into(),
            mesh_id: mesh.id.clone(),
            cell_index: 1,
            cell_id: None,
            nearest_vertex_index: None,
            vertex_id: None,
            world_position: [0.25, 0.75, 0.0],
            displayed_value: None,
            field_id: None,
        };
        let points = selected_triangle_points(&mesh, &selection, &projector, &projected)
            .expect("valid selection should produce an overlay triangle");
        assert_eq!(points[0], [10.0, 80.0]);
        assert_eq!(points[1], [90.0, 0.0]);
        assert_eq!(points[2], [10.0, 0.0]);
    }

    #[test]
    fn selected_triangle_overlay_rejects_foreign_and_invalid_picks() {
        let mesh = square_mesh();
        let projected = mesh
            .positions
            .iter()
            .copied()
            .map(|point| project_2d(CoordinateAxis::X, CoordinateAxis::Y, point))
            .collect::<Vec<_>>();
        let projector = MeshProjector::from_domains([0.0, 1.0], [0.0, 1.0], 100.0, 80.0, false);
        let mut selection = MeshPlotPick {
            plot_id: "plot".into(),
            mesh_id: "other-mesh".into(),
            cell_index: 0,
            cell_id: None,
            nearest_vertex_index: None,
            vertex_id: None,
            world_position: [0.0, 0.0, 0.0],
            displayed_value: None,
            field_id: None,
        };
        assert!(selected_triangle_points(&mesh, &selection, &projector, &projected).is_none());
        selection.mesh_id = mesh.id.clone();
        selection.cell_index = 99;
        assert!(selected_triangle_points(&mesh, &selection, &projector, &projected).is_none());
    }

    #[test]
    fn live_selection_clearing_does_not_restore_static_selection() {
        let static_selection = MeshPlotPick {
            plot_id: "plot".into(),
            mesh_id: "square".into(),
            cell_index: 0,
            cell_id: None,
            nearest_vertex_index: None,
            vertex_id: None,
            world_position: [0.0, 0.0, 0.0],
            displayed_value: None,
            field_id: None,
        };
        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        state.borrow_mut().selection = Some(static_selection.clone());
        assert_eq!(
            retained_overlay_selection(Some(&state), Some(&static_selection)),
            Some(static_selection.clone())
        );
        state.borrow_mut().clear_selection();
        assert_eq!(
            retained_overlay_selection(Some(&state), Some(&static_selection)),
            None
        );
        assert_eq!(
            retained_overlay_selection(None, Some(&static_selection)),
            Some(static_selection)
        );
    }

    #[test]
    fn declarative_rebuild_classifies_geometry_and_field_dirty_domains() {
        let mesh = square_mesh();
        let field = vertex_field();
        let previous = mesh_plot(mesh.clone()).field(field.clone());
        let unchanged = mesh_plot(mesh.clone()).field(field.clone());
        assert_eq!(
            mesh_plot_resource_domains_changed(&previous, &unchanged),
            (false, false),
            "cloned builders retain their immutable backing stores"
        );

        let mut replacement_mesh = mesh.clone();
        replacement_mesh.positions = Arc::from([
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]);
        let geometry_update = mesh_plot(replacement_mesh).field(field.clone());
        assert_eq!(
            mesh_plot_resource_domains_changed(&previous, &geometry_update),
            (true, false)
        );

        let mut replacement_field = field.clone();
        replacement_field.values = Arc::from([2.0, 1.0, 1.0, 0.0]);
        let field_update = mesh_plot(mesh.clone()).field(replacement_field);
        assert_eq!(
            mesh_plot_resource_domains_changed(&previous, &field_update),
            (false, true)
        );

        let mut masked_nan_field = field.clone();
        masked_nan_field.values = Arc::from([0.0, f64::NAN, 1.0, 2.0]);
        let masked_nan_previous = mesh_plot(mesh.clone()).field(masked_nan_field.clone());
        let mut rebuilt_masked_nan_field = masked_nan_field.clone();
        rebuilt_masked_nan_field.values = Arc::from([0.0, f64::NAN, 1.0, 2.0]);
        let rebuilt_masked_nan = mesh_plot(mesh.clone()).field(rebuilt_masked_nan_field);
        assert_eq!(
            mesh_plot_resource_domains_changed(&masked_nan_previous, &rebuilt_masked_nan),
            (false, false),
            "equivalent masked NaN samples must not dirty a rebuilt live plot"
        );

        let revolve_update = mesh_plot(mesh)
            .field(field)
            .view(MeshPlotView::AxisymmetricRevolve(Default::default()));
        assert_eq!(
            mesh_plot_resource_domains_changed(&previous, &revolve_update),
            (true, false),
            "a view that derives revolved geometry must invalidate prepared buffers"
        );

        let backend_update = mesh_plot(square_mesh())
            .field(vertex_field())
            .renderer_backend(MeshPlotBackend::Wgpu);
        assert_eq!(
            mesh_plot_resource_domains_changed(&previous, &backend_update),
            (true, false),
            "switching retained GPU backends must replace the custom draw owner"
        );
    }

    #[test]
    fn declarative_rebuild_classifies_ids_masks_and_associations() {
        let mut mesh = square_mesh();
        mesh.vertex_ids = Some(Arc::from([10, 11, 12, 13]));
        mesh.cell_ids = Some(Arc::from([20, 21]));
        let mut field = vertex_field();
        field.valid = Some(Arc::from([true, true, true, true]));
        let previous = mesh_plot(mesh.clone()).field(field.clone());

        let mut changed_vertex_ids = mesh.clone();
        changed_vertex_ids.vertex_ids = Some(Arc::from([10, 11, 12, 99]));
        assert_eq!(
            mesh_plot_resource_domains_changed(
                &previous,
                &mesh_plot(changed_vertex_ids).field(field.clone())
            ),
            (true, false)
        );

        let mut changed_cell_ids = mesh.clone();
        changed_cell_ids.cell_ids = Some(Arc::from([20, 99]));
        assert_eq!(
            mesh_plot_resource_domains_changed(
                &previous,
                &mesh_plot(changed_cell_ids).field(field.clone())
            ),
            (true, false)
        );

        let mut missing_vertex_ids = mesh.clone();
        missing_vertex_ids.vertex_ids = None;
        assert_eq!(
            mesh_plot_resource_domains_changed(
                &previous,
                &mesh_plot(missing_vertex_ids).field(field.clone())
            ),
            (true, false)
        );

        let mut changed_mask = field.clone();
        changed_mask.valid = Some(Arc::from([true, false, true, true]));
        assert_eq!(
            mesh_plot_resource_domains_changed(
                &previous,
                &mesh_plot(mesh.clone()).field(changed_mask)
            ),
            (false, true)
        );

        let mut cell_field = field;
        cell_field.values = Arc::from([0.5, 0.75]);
        cell_field.valid = None;
        cell_field.association = ScalarAssociation::Cell;
        assert_eq!(
            mesh_plot_resource_domains_changed(
                &previous,
                &mesh_plot(mesh.clone()).field(cell_field)
            ),
            (false, true)
        );

        assert_eq!(
            mesh_plot_resource_domains_changed(&previous, &mesh_plot(mesh).field(vertex_field())),
            (false, true),
            "removing the validity mask is a field-domain change"
        );
    }

    #[test]
    fn repeated_plot_ids_receive_stable_occurrence_keys_per_draw() {
        let window = gpui::WindowId::from(1);
        let mut tracker = MeshPlotOccurrenceTracker::default();
        assert_eq!(tracker.next(10, (window, Arc::from("shared"))), 0);
        assert_eq!(tracker.next(10, (window, Arc::from("shared"))), 1);
        assert_eq!(tracker.next(10, (window, Arc::from("other"))), 0);
        assert_eq!(tracker.next(11, (window, Arc::from("shared"))), 0);
        assert_eq!(tracker.next(11, (window, Arc::from("shared"))), 1);
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

    #[cfg(feature = "gpui")]
    #[test]
    fn toolbar_mode_menu_offers_only_valid_field_association_choices() {
        let vertex_items = mesh_toolbar_mode_items(Some(&vertex_field()), &MeshRenderMode::Mesh);
        let vertex_ids: Vec<_> = vertex_items.iter().map(|item| item.id().as_ref()).collect();
        assert!(vertex_ids.contains(&"smooth-fill"));
        assert!(vertex_ids.contains(&"filled-contours"));

        let cell_field = ScalarField {
            id: "cell".into(),
            label: "Cell".into(),
            unit: None,
            values: Arc::from([0.5, 0.7]),
            association: ScalarAssociation::Cell,
            valid: None,
        };
        let cell_items = mesh_toolbar_mode_items(Some(&cell_field), &MeshRenderMode::Mesh);
        let cell_ids: Vec<_> = cell_items.iter().map(|item| item.id().as_ref()).collect();
        assert!(cell_ids.contains(&"flat-fill"));
        assert!(!cell_ids.contains(&"filled-contours"));
    }

    #[cfg(feature = "gpui")]
    #[test]
    fn toolbar_mode_selection_updates_the_retained_style() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        apply_mesh_toolbar_menu_selection(
            &mut state,
            MeshPlotToolbarMenu::Mode,
            "fill-and-isolines",
            false,
        );
        assert!(matches!(
            state.render_mode,
            MeshRenderMode::FillAndIsolines {
                levels: ContourLevels::Count(12)
            }
        ));
    }

    #[cfg(feature = "gpui")]
    #[test]
    fn toolbar_mode_and_view_selection_cover_supported_actions() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        for (id, expected) in [
            ("mesh", MeshRenderMode::Mesh),
            (
                "smooth-fill",
                MeshRenderMode::ScalarFill {
                    interpolation: FieldInterpolation::Smooth,
                },
            ),
            (
                "flat-fill",
                MeshRenderMode::ScalarFill {
                    interpolation: FieldInterpolation::Flat,
                },
            ),
            (
                "filled-contours",
                MeshRenderMode::FilledContours {
                    levels: ContourLevels::Count(12),
                },
            ),
            (
                "isolines",
                MeshRenderMode::Isolines {
                    levels: ContourLevels::Count(12),
                },
            ),
            (
                "fill-and-isolines",
                MeshRenderMode::FillAndIsolines {
                    levels: ContourLevels::Count(12),
                },
            ),
        ] {
            apply_mesh_toolbar_menu_selection(&mut state, MeshPlotToolbarMenu::Mode, id, false);
            assert_eq!(state.render_mode, expected);
        }
        let before_unknown = state.render_mode.clone();
        apply_mesh_toolbar_menu_selection(&mut state, MeshPlotToolbarMenu::Mode, "unknown", false);
        assert_eq!(state.render_mode, before_unknown);

        for id in [
            "front",
            "back",
            "left",
            "right",
            "top",
            "bottom",
            "isometric",
            "projection",
        ] {
            apply_mesh_toolbar_menu_selection(&mut state, MeshPlotToolbarMenu::View, id, true);
        }
        apply_mesh_toolbar_menu_selection(&mut state, MeshPlotToolbarMenu::View, "unknown", true);
        apply_mesh_toolbar_menu_selection(&mut state, MeshPlotToolbarMenu::View, "front", false);
        assert_eq!(mesh_toolbar_view_items().len(), 8);
    }

    #[test]
    fn export_callback_is_builder_configurable() {
        let plot = mesh_plot(square_mesh()).on_export(|result| assert!(result.is_ok()));
        assert!(plot.export_callback.is_some());
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

    fn contour_band_area(band: &ContourBand) -> f64 {
        band.triangles
            .iter()
            .map(|&[a, b, c]| {
                let a = band.positions[a as usize];
                let b = band.positions[b as usize];
                let c = band.positions[c as usize];
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5
            })
            .sum()
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn computed_fill_and_isolines_matches_cpu_geometry_with_adapter_precision() {
        let mesh = square_mesh();
        let field = vertex_field();
        let topology = MeshTopology::build(&mesh.triangles);
        let mode = MeshRenderMode::FillAndIsolines {
            levels: d3rs::mesh::ContourLevels::Count(3),
        };
        let cpu = contour_geometry(
            &mesh,
            Some(&field),
            &topology,
            CoordinateAxis::X,
            CoordinateAxis::Y,
            &mode,
            Some([0.0, 2.0]),
        )
        .unwrap();
        let computed = contour_geometry_with_compute(
            &mesh,
            Some(&field),
            &topology,
            CoordinateAxis::X,
            CoordinateAxis::Y,
            &mode,
            Some([0.0, 2.0]),
        )
        .unwrap();
        assert_eq!(computed.0.len(), cpu.0.len());
        for (computed_band, cpu_band) in computed.0.iter().zip(&cpu.0) {
            // Keep f64 level boundaries exact even though the adapter clips with f32
            // interpolation internally.
            assert_eq!(computed_band.lower, cpu_band.lower);
            assert_eq!(computed_band.upper, cpu_band.upper);
            assert!(
                (contour_band_area(computed_band) - contour_band_area(cpu_band)).abs() < 2e-4,
                "computed area {} != CPU area {} for {:?}..{:?}",
                contour_band_area(computed_band),
                contour_band_area(cpu_band),
                cpu_band.lower,
                cpu_band.upper,
            );
        }
        assert_eq!(computed.1, cpu.1);
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
    fn retained_3d_cell_field_update_clears_old_storage() {
        let mesh = square_mesh();
        let cell_field = ScalarField {
            id: "cell".into(),
            label: "Cell".into(),
            unit: None,
            values: Arc::from([0.5, 0.75]),
            association: ScalarAssociation::Cell,
            valid: None,
        };
        let scene = build_retained_3d_scene_state(
            &mesh,
            Some(&cell_field),
            &MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Flat,
            },
            Wireframe::Hidden,
            &ColorScale::Viridis,
            Some([0.0, 1.0]),
        );
        {
            let scene = scene.borrow();
            let upload = scene.upload.as_ref().expect("prepared 3D upload");
            assert_eq!(upload.cell_values_f32.as_deref(), Some(&[0.5, 0.75][..]));
            assert!(upload.values_f32.is_none());
        }
        update_retained_3d_scene_state(
            &scene,
            None,
            &MeshRenderMode::Mesh,
            Wireframe::Hidden,
            &ColorScale::Magma,
            None,
            2,
        );
        let scene = scene.borrow();
        let upload = scene.upload.as_ref().expect("retained 3D upload");
        assert!(upload.cell_values_f32.is_none());
        assert!(upload.values_f32.is_none());
        assert_eq!(scene.field_rev.0, 2);
        assert!(scene.color.wireframe);
        assert_eq!(scene.color.range, [0.0, 1.0]);
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn render_view_materializes_revolved_geometry_and_field() {
        let mesh = square_mesh();
        let field = vertex_field();
        let (surface, surface_field) =
            render_3d_mesh_and_field_for_view(&mesh, Some(&field), &MeshPlotView::Surface3d)
                .unwrap();
        assert_eq!(surface.positions, mesh.positions);
        assert_eq!(surface.triangles, mesh.triangles);
        assert_eq!(surface_field.unwrap().values, field.values);

        let view = MeshPlotView::AxisymmetricRevolve(d3rs::mesh::RevolveSpec {
            radial: CoordinateAxis::X,
            axial: CoordinateAxis::Y,
            ..Default::default()
        });
        let (revolved, revolved_field) =
            render_3d_mesh_and_field_for_view(&mesh, Some(&field), &view).unwrap();
        assert!(revolved.positions.len() > mesh.positions.len());
        assert_eq!(
            revolved_field.unwrap().values.len(),
            revolved.positions.len()
        );
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn background_revolve_preparation_builds_derived_bvh_and_field() {
        let mesh = square_mesh();
        let field = vertex_field();
        let prepared = prepare_revolve(
            &mesh,
            &d3rs::mesh::RevolveSpec {
                segments: 8,
                ..Default::default()
            },
            Some(&field),
        )
        .expect("valid revolve preparation should complete");
        assert!(prepared.revolved.mesh.positions.len() > mesh.positions.len());
        assert!(!prepared.revolved.mesh.triangles.is_empty());
        assert_eq!(
            prepared.field.as_ref().map(|field| field.values.len()),
            Some(prepared.revolved.mesh.positions.len())
        );

        let invalid = prepare_revolve(
            &mesh,
            &d3rs::mesh::RevolveSpec {
                segments: 2,
                ..Default::default()
            },
            None,
        );
        assert!(matches!(
            invalid,
            Err(ChartError::MeshValidation(
                MeshValidationError::InvalidRevolveSpec
            ))
        ));
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

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn retained_3d_viewport_and_selection_updates_keep_geometry_payload() {
        let mesh = square_mesh();
        let field = vertex_field();
        let scene = build_retained_3d_scene_state(
            &mesh,
            Some(&field),
            &MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            },
            Wireframe::Hidden,
            &ColorScale::Viridis,
            Some([0.0, 2.0]),
        );
        let (positions, indices, geometry_revision, upload_count, upload_bytes) = {
            let scene = scene.borrow();
            let upload = scene.upload.as_ref().expect("prepared 3D upload");
            (
                upload.positions_f32.clone(),
                upload.indices.clone(),
                scene.geometry_rev,
                scene.geometry_upload_count,
                scene.geometry_upload_bytes,
            )
        };
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        update_scene_view_transform(&scene, &state, 600.0, 400.0, true);
        let initial_transform = scene.borrow().view_transform;

        state.interaction.zoom_around_domain(0.5, 0.5, 0.8);
        update_scene_view_transform(&scene, &state, 800.0, 400.0, true);
        let viewport_transform = scene.borrow().view_transform;
        assert_ne!(viewport_transform, initial_transform);

        state.set_selection(Some(MeshPlotPick {
            plot_id: "plot".into(),
            mesh_id: "square".into(),
            cell_index: 1,
            cell_id: Some(101),
            nearest_vertex_index: Some(2),
            vertex_id: Some(12),
            world_position: [0.5, 0.5, 0.0],
            displayed_value: Some(1.0),
            field_id: Some("pressure".into()),
        }));
        update_scene_view_transform(&scene, &state, 800.0, 400.0, true);

        let scene = scene.borrow();
        let upload = scene.upload.as_ref().expect("retained 3D upload");
        assert_eq!(scene.view_transform, viewport_transform);
        assert_eq!(scene.geometry_rev, geometry_revision);
        assert_eq!(scene.geometry_upload_count, upload_count);
        assert_eq!(scene.geometry_upload_bytes, upload_bytes);
        assert_eq!(upload.positions_f32, positions);
        assert_eq!(upload.indices, indices);
        assert_eq!(
            state.selection.as_ref().and_then(|pick| pick.cell_id),
            Some(101)
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
    fn invalid_axes_ranges_are_rejected() {
        let horizontal = mesh_plot(square_mesh())
            .axes(Axes2d::default().horizontal_range(1.0, 1.0))
            .build();
        assert!(matches!(
            horizontal,
            Err(ChartError::InvalidData {
                field: "axes.horizontal_range",
                ..
            })
        ));

        let vertical = mesh_plot(square_mesh())
            .axes(Axes2d::default().vertical_range(f64::NAN, 2.0))
            .build();
        assert!(matches!(
            vertical,
            Err(ChartError::InvalidData {
                field: "axes.vertical_range",
                ..
            })
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
    fn validation_reports_missing_field_invalid_revolve_and_color_range() {
        let missing_field = mesh_plot(square_mesh())
            .mode(MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            })
            .build();
        assert!(matches!(
            missing_field,
            Err(ChartError::InvalidData { field: "field", .. })
        ));

        let mut invalid_revolve = d3rs::mesh::RevolveSpec::default();
        invalid_revolve.segments = 2;
        let invalid_revolve = mesh_plot(square_mesh())
            .view(MeshPlotView::AxisymmetricRevolve(invalid_revolve))
            .build();
        assert!(matches!(
            invalid_revolve,
            Err(ChartError::MeshValidation(
                MeshValidationError::InvalidRevolveSpec
            ))
        ));

        let invalid_range = mesh_plot(square_mesh())
            .field(vertex_field())
            .color_range(ColorRange::Fixed { min: 1.0, max: 1.0 })
            .build();
        assert!(matches!(
            invalid_range,
            Err(ChartError::InvalidColorRange { .. })
        ));
    }

    #[test]
    fn range_domain_and_contour_helpers_cover_empty_and_active_paths() {
        assert_eq!(resolve_value_range(None, ColorRange::Auto).unwrap(), None);
        let mut masked = vertex_field();
        masked.valid = Some(Arc::from([false, false, false, false]));
        assert_eq!(
            resolve_value_range(Some(&masked), ColorRange::Auto).unwrap(),
            None
        );
        assert_eq!(
            resolve_value_range(
                Some(&vertex_field()),
                ColorRange::Fixed { min: 0.0, max: 2.0 }
            )
            .unwrap(),
            Some([0.0, 2.0])
        );

        assert_eq!(finite_domain(&[], 0), None);
        assert_eq!(
            finite_domain(&[[2.0, 3.0]], 0),
            Some([2.0, 2.0 + f64::EPSILON])
        );
        assert_eq!(finite_domain(&[[f64::NAN, 3.0]], 0), None);

        let mesh = square_mesh();
        let topology = MeshTopology::build(&mesh.triangles);
        let empty = contour_geometry(
            &mesh,
            None,
            &topology,
            CoordinateAxis::X,
            CoordinateAxis::Y,
            &MeshRenderMode::Mesh,
            None,
        )
        .unwrap();
        assert!(empty.0.is_empty() && empty.1.is_empty());
        let inactive = contour_geometry(
            &mesh,
            Some(&vertex_field()),
            &topology,
            CoordinateAxis::X,
            CoordinateAxis::Y,
            &MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            },
            Some([0.0, 2.0]),
        )
        .unwrap();
        assert!(inactive.0.is_empty() && inactive.1.is_empty());
        let isolines = contour_geometry(
            &mesh,
            Some(&vertex_field()),
            &topology,
            CoordinateAxis::X,
            CoordinateAxis::Y,
            &MeshRenderMode::Isolines {
                levels: ContourLevels::Count(3),
            },
            Some([0.0, 2.0]),
        )
        .unwrap();
        assert!(!isolines.1.is_empty());
    }

    #[test]
    fn view_and_toolbar_labels_cover_all_mesh_plot_variants() {
        let revolve = MeshPlotView::AxisymmetricRevolve(d3rs::mesh::RevolveSpec::default());
        let views = [
            MeshPlotView::Planar {
                horizontal: CoordinateAxis::X,
                vertical: CoordinateAxis::Y,
            },
            MeshPlotView::AxisymmetricSection {
                radial: CoordinateAxis::X,
                axial: CoordinateAxis::Y,
            },
            revolve,
            MeshPlotView::Surface3d,
        ];
        assert_eq!(toolbar_view_name(&views[0]), "Planar");
        assert_eq!(toolbar_view_name(&views[1]), "Axisymmetric section");
        assert_eq!(toolbar_view_name(&views[2]), "Axisymmetric revolve");
        assert_eq!(toolbar_view_name(&views[3]), "Surface 3D");
        assert_eq!(view_axes(&views[0]), (CoordinateAxis::X, CoordinateAxis::Y));
        assert_eq!(view_axes(&views[1]), (CoordinateAxis::X, CoordinateAxis::Y));
        assert_eq!(view_axes(&views[2]), (CoordinateAxis::X, CoordinateAxis::Z));
        assert_eq!(view_axes(&views[3]), (CoordinateAxis::X, CoordinateAxis::Y));
        assert_eq!(isoline_step(&MeshRenderMode::Mesh, Some([0.0, 1.0])), None);
        assert_eq!(
            isoline_step(
                &MeshRenderMode::Isolines {
                    levels: ContourLevels::Count(3),
                },
                None
            ),
            None
        );
        assert!(
            isoline_step(
                &MeshRenderMode::FillAndIsolines {
                    levels: ContourLevels::Count(3),
                },
                Some([0.0, 2.0]),
            )
            .is_some()
        );
        assert!(requires_contour_preparation(
            &MeshRenderMode::FilledContours {
                levels: ContourLevels::Count(3),
            }
        ));
        assert!(!requires_contour_preparation(&MeshRenderMode::ScalarFill {
            interpolation: FieldInterpolation::Smooth,
        }));
    }

    #[test]
    fn triangle_projection_and_value_helpers_reject_invalid_samples() {
        let points = [[0.0, 0.0], [2.0, 0.0], [0.0, 2.0]];
        let projector = MeshProjector::new(&points, 200.0, 200.0, false);
        let triangle = triangle_points(&projector, &points, [0, 1, 2])
            .expect("valid triangle indices should project");
        assert_eq!(triangle[0], [0.0, 200.0]);
        assert!(triangle_points(&projector, &points, [0, 1, 9]).is_none());
        assert_eq!(
            triangle_points_from_band(&projector, &points, [0, 2, 1]),
            triangle_points(&projector, &points, [0, 2, 1])
        );

        let vertex = ScalarField {
            values: Arc::from([1.0, 2.0, 3.0]),
            ..vertex_field()
        };
        assert_eq!(triangle_value(Some(&vertex), [0, 1, 2], 0), Some(2.0));

        let mut invalid_vertex = vertex.clone();
        invalid_vertex.values = Arc::from([1.0, f64::NAN, 3.0]);
        assert_eq!(triangle_value(Some(&invalid_vertex), [0, 1, 2], 0), None);
        invalid_vertex.valid = Some(Arc::from([true, false, true]));
        assert_eq!(triangle_value(Some(&invalid_vertex), [0, 1, 2], 0), None);

        let cell = ScalarField {
            id: "cell".into(),
            label: "Cell".into(),
            unit: None,
            values: Arc::from([4.0]),
            association: ScalarAssociation::Cell,
            valid: Some(Arc::from([true])),
        };
        assert_eq!(triangle_value(Some(&cell), [0, 1, 2], 0), Some(4.0));
        assert_eq!(triangle_value(Some(&cell), [0, 1, 2], 1), None);
        assert_eq!(triangle_value(None, [0, 1, 2], 0), None);
    }

    #[test]
    fn pointer_inversion_rejects_invalid_and_outside_points() {
        let state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        for (x, y, width, height) in [
            (f32::NAN, 1.0, 100.0, 100.0),
            (1.0, f32::INFINITY, 100.0, 100.0),
            (1.0, 1.0, 0.0, 100.0),
            (1.0, 1.0, 100.0, 0.0),
            (-1.0, 1.0, 100.0, 100.0),
            (101.0, 1.0, 100.0, 100.0),
            (1.0, -1.0, 100.0, 100.0),
            (1.0, 101.0, 100.0, 100.0),
        ] {
            assert!(mesh_point_to_domain(&state, x, y, width, height, false).is_none());
        }

        let letterboxed = mesh_point_to_domain(&state, 0.0, 0.0, 100.0, 200.0, true);
        assert!(
            letterboxed.is_none(),
            "the top letterbox bar is not drawable"
        );
        assert!(mesh_point_to_domain(&state, 50.0, 100.0, 100.0, 200.0, true).is_some());
    }

    #[cfg(feature = "gpu-2d")]
    #[test]
    fn retained_2d_scene_prepares_cell_values_and_rebased_positions() {
        let mesh = square_mesh();
        let cell_field = ScalarField {
            id: "cell".into(),
            label: "Cell".into(),
            unit: None,
            values: Arc::from([0.5, 0.75]),
            association: ScalarAssociation::Cell,
            valid: None,
        };
        let projected = [[10.0, 20.0], [11.0, 20.0], [11.0, 21.0], [10.0, 21.0]];
        let scene = build_retained_scene_state(
            &mesh,
            Some(&cell_field),
            &projected,
            [10.0, 11.0],
            [20.0, 21.0],
            400.0,
            300.0,
            true,
            &MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Flat,
            },
            Wireframe::Hidden,
            &ColorScale::Viridis,
            Some([0.0, 1.0]),
        );
        let scene = scene.borrow();
        let upload = scene.upload.as_ref().expect("prepared 2D upload");
        assert_eq!(upload.origin, [10.0, 20.0, 0.0]);
        assert_eq!(upload.positions_f32[0], [0.0, 0.0, 0.0]);
        assert_eq!(upload.cell_values_f32.as_deref(), Some(&[0.5, 0.75][..]));
        assert!(upload.values_f32.is_none());
        assert!(!scene.color.wireframe);
    }

    #[cfg(feature = "gpu-2d")]
    #[test]
    fn retained_2d_scene_supports_fill_aspect_and_pointer_inversion() {
        let mesh = square_mesh();
        let projected = [[10.0, 20.0], [11.0, 20.0], [11.0, 21.0], [10.0, 21.0]];
        let scene = build_retained_scene_state(
            &mesh,
            None,
            &projected,
            [10.0, 11.0],
            [20.0, 21.0],
            400.0,
            300.0,
            false,
            &MeshRenderMode::Mesh,
            Wireframe::Overlay,
            &ColorScale::Viridis,
            None,
        );
        let scene = scene.borrow();
        let upload = scene.upload.as_ref().expect("prepared 2D upload");
        assert!(upload.values_f32.is_none());
        assert!(upload.cell_values_f32.is_none());
        assert!(scene.color.wireframe);
        drop(scene);

        let state = MeshPlotState::new(10.0, 11.0, 20.0, 21.0);
        let point = mesh_point_to_domain(&state, 200.0, 150.0, 400.0, 300.0, false)
            .expect("point inside fill-aspect plot");
        assert_eq!(point, [10.5, 20.5]);
        assert!(mesh_point_to_domain(&state, f32::NAN, 10.0, 400.0, 300.0, false).is_none());
        assert!(mesh_point_to_domain(&state, 10.0, 10.0, 0.0, 300.0, false).is_none());
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
    fn projector_uses_the_live_interaction_viewport() {
        let points = [[0.0, 0.0], [4.0, 2.0]];
        let projector =
            MeshProjector::new(&points, 400.0, 200.0, false).with_viewport([1.0, 3.0], [0.5, 1.5]);
        assert_eq!(projector.point([1.0, 0.5]), [0.0, 200.0]);
        assert_eq!(projector.point([3.0, 1.5]), [400.0, 0.0]);
    }

    #[test]
    fn pointer_positions_are_converted_to_plot_local_coordinates() {
        let bounds = Bounds {
            origin: point(px(50.0), px(75.0)),
            size: gpui::size(px(400.0), px(300.0)),
        };
        assert_eq!(
            plot_local_position(point(px(250.0), px(225.0)), bounds),
            [200.0, 150.0]
        );
    }

    #[test]
    fn windows_uses_the_software_mesh_scene_fallback() {
        assert!(!mesh_custom_draw_supported("windows"));
        assert!(mesh_custom_draw_supported("macos"));
        assert!(mesh_custom_draw_supported("linux"));
    }

    #[cfg(feature = "gpu-2d")]
    #[test]
    fn equal_aspect_pointer_inversion_uses_the_visible_letterboxed_viewport() {
        let state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        // 530×360 leaves 85px bars left/right. This local point is within
        // the visible square, not at the same normalized x of the full rect.
        let point = mesh_point_to_domain(&state, 130.0, 290.0, 530.0, 360.0, true)
            .expect("point inside equal-aspect plot");
        assert!((point[0] - 0.125).abs() < 1e-12);
        assert!((point[1] - (70.0 / 360.0)).abs() < 1e-12);
        assert!(mesh_point_to_domain(&state, 50.0, 180.0, 530.0, 360.0, true).is_none());
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

    #[test]
    fn builder_setters_retain_layout_state_and_callbacks() {
        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        let selection = MeshPlotPick {
            plot_id: "plot".into(),
            mesh_id: "square".into(),
            cell_index: 0,
            cell_id: None,
            nearest_vertex_index: None,
            vertex_id: None,
            world_position: [0.25, 0.25, 0.0],
            displayed_value: None,
            field_id: None,
        };
        let result = mesh_plot(square_mesh())
            .plot_id("plot")
            .renderer_backend(MeshPlotBackend::Wgpu)
            .missing_value_policy(d3rs::mesh::MissingValuePolicy::Reject)
            .with_state(state)
            .toolbar(true)
            .selection(selection)
            .title("Mesh")
            .design(default_design())
            .size(400.0, 300.0)
            .min_size(200.0, 150.0)
            .aspect_ratio(4.0 / 3.0)
            .fill()
            .on_selection(|_| {})
            .on_export(|_| {})
            .build();
        assert!(result.is_ok());
    }
}
