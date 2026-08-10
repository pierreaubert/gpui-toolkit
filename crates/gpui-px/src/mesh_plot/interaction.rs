use super::MeshPlotPick;
use crate::interaction::ChartInteraction;
#[cfg(feature = "gpu-3d")]
use d3rs::gpu3d::{Camera3D, OrbitControls};
#[cfg(feature = "gpu-3d")]
use d3rs::mesh::MeshBounds;
use d3rs::mesh::{CoordinateAxis, ScalarField, TriGridIndex, TriangleMesh};

#[derive(Clone)]
pub struct MeshPlotState {
    pub interaction: ChartInteraction,
    pub hover: Option<MeshPlotPick>,
    pub selection: Option<MeshPlotPick>,
    pub geometry_revision: u64,
    pub field_revision: u64,
    field_values: Vec<f32>,
    /// Current plot style as seen by retained toolbar/render callbacks.
    pub wireframe: super::Wireframe,
    pub render_mode: super::MeshRenderMode,
    pub color_range: crate::ColorRange,
    #[cfg(feature = "gpu-3d")]
    pub camera: Camera3D,
    #[cfg(feature = "gpu-3d")]
    pub orbit: OrbitControls,
    #[cfg(feature = "gpu-3d")]
    pub camera_fitted: bool,
}

impl MeshPlotState {
    pub fn new(x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> Self {
        #[cfg(feature = "gpu-3d")]
        let orbit = OrbitControls::default();
        Self {
            interaction: ChartInteraction::new(x_min, x_max, y_min, y_max),
            hover: None,
            selection: None,
            geometry_revision: 0,
            field_revision: 0,
            field_values: Vec::new(),
            wireframe: super::Wireframe::Overlay,
            render_mode: super::MeshRenderMode::Mesh,
            color_range: crate::ColorRange::Auto,
            #[cfg(feature = "gpu-3d")]
            camera: orbit.to_camera(),
            #[cfg(feature = "gpu-3d")]
            orbit,
            #[cfg(feature = "gpu-3d")]
            camera_fitted: false,
        }
    }
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Replace the retained hover result without affecting click selection.
    pub fn set_hover(&mut self, pick: Option<MeshPlotPick>) {
        self.hover = pick;
    }

    /// Replace the retained click selection.
    pub fn set_selection(&mut self, pick: Option<MeshPlotPick>) {
        self.selection = pick;
    }

    /// Pick one plot-relative data point and update hover/selection state.
    ///
    /// The caller supplies the retained spatial index so pointer motion does
    /// not rebuild an accelerator or allocate a candidate list.
    pub fn pick_at(
        &mut self,
        mesh: &TriangleMesh,
        field: Option<&ScalarField>,
        index: &TriGridIndex,
        horizontal: CoordinateAxis,
        vertical: CoordinateAxis,
        point_2d: [f64; 2],
        plot_id: &str,
        select: bool,
    ) -> Option<MeshPlotPick> {
        let pick = super::super::mesh_plot::picking::pick_2d(
            mesh, field, index, horizontal, vertical, point_2d, plot_id,
        );
        self.hover = pick.clone();
        if select {
            self.selection = pick.clone();
        }
        pick
    }

    /// Human-readable tooltip text for the current hover target.
    pub fn hover_tooltip(&self) -> Option<String> {
        self.hover_tooltip_with_field(None)
    }

    /// Format the native tooltip payload with coordinates, IDs, field label,
    /// value, and unit. The field is borrowed so hover updates do not clone
    /// large scalar arrays.
    pub fn hover_tooltip_with_field(&self, field: Option<&ScalarField>) -> Option<String> {
        self.hover.as_ref().map(|pick| {
            let cell = pick
                .cell_id
                .map_or_else(String::new, |id| format!(" (id {id})"));
            let vertex = pick
                .vertex_id
                .map_or_else(String::new, |id| format!("; vertex id {id}"));
            let value = pick.displayed_value.map_or_else(String::new, |value| {
                let label = field.map_or("Value", |field| field.label.as_ref());
                let unit = field
                    .and_then(|field| field.unit.as_deref())
                    .map_or(String::new(), |unit| format!(" {unit}"));
                format!("; {label} {value:.6}{unit}")
            });
            format!(
                "({:.6}, {:.6}, {:.6}); Cell {}{cell}{vertex}{value}",
                pick.world_position[0],
                pick.world_position[1],
                pick.world_position[2],
                pick.cell_index,
            )
        })
    }

    /// Configure the style values shared by the native toolbar and retained
    /// render callbacks.
    pub fn set_style(
        &mut self,
        mode: super::MeshRenderMode,
        wireframe: super::Wireframe,
        color_range: crate::ColorRange,
    ) {
        self.render_mode = mode;
        self.wireframe = wireframe;
        self.color_range = color_range;
    }

    /// Apply a keyboard navigation action while retaining selection/hover.
    pub fn handle_key(&mut self, key: &str) -> bool {
        let Some(action) = crate::interaction::keyboard_action_for_key(key) else {
            return false;
        };
        use crate::interaction::ChartKeyboardAction;
        match action {
            ChartKeyboardAction::ZoomIn => self.interaction.zoom_around_pixel(300.0, 200.0, 0.8),
            ChartKeyboardAction::ZoomOut => self.interaction.zoom_around_pixel(300.0, 200.0, 1.25),
            ChartKeyboardAction::PanLeft => self.interaction.pan_by_pixels(-24.0, 0.0),
            ChartKeyboardAction::PanRight => self.interaction.pan_by_pixels(24.0, 0.0),
            ChartKeyboardAction::PanUp => self.interaction.pan_by_pixels(0.0, -24.0),
            ChartKeyboardAction::PanDown => self.interaction.pan_by_pixels(0.0, 24.0),
            ChartKeyboardAction::ResetZoom => self.interaction.reset_zoom(),
        }
        true
    }

    /// Reserve the retained field buffer before entering the render loop.
    pub fn reserve_field_capacity(&mut self, capacity: usize) {
        self.field_values
            .reserve(capacity.saturating_sub(self.field_values.len()));
    }

    /// Replace field values while preserving viewport and selection state.
    pub fn replace_field_values(&mut self, revision: u64, values: &[f32]) {
        self.field_values.clear();
        self.field_values.extend_from_slice(values);
        self.field_revision = revision;
    }

    /// Return the retained field values for a backend upload.
    pub fn field_values(&self) -> &[f32] {
        &self.field_values
    }

    /// Apply camera navigation without growing zoom history.
    pub fn set_viewport_without_history(&mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) {
        self.interaction
            .set_viewport_without_history(x_min, x_max, y_min, y_max);
    }

    #[cfg(feature = "gpu-3d")]
    /// Rotate the retained 3D camera and preserve its current target/distance.
    pub fn orbit_rotate(&mut self, delta_x: f32, delta_y: f32) {
        self.orbit.rotate(delta_x, delta_y);
        self.orbit.update_camera(&mut self.camera);
    }

    #[cfg(feature = "gpu-3d")]
    /// Zoom the retained 3D camera around its orbit target.
    pub fn orbit_zoom(&mut self, delta: f32) {
        self.orbit.zoom(delta);
        self.orbit.update_camera(&mut self.camera);
    }

    #[cfg(feature = "gpu-3d")]
    /// Restore the initial retained 3D camera.
    pub fn orbit_reset(&mut self) {
        self.orbit.reset();
        self.orbit.update_camera(&mut self.camera);
    }

    #[cfg(feature = "gpu-3d")]
    /// Update the camera aspect ratio after a plot resize.
    pub fn set_camera_aspect(&mut self, width: f32, height: f32) {
        if width.is_finite() && height.is_finite() && height > 0.0 {
            self.camera.aspect = (width / height).max(f32::EPSILON);
        }
    }

    #[cfg(feature = "gpu-3d")]
    /// Fit the retained orbit to a mesh and make that fit the reset state.
    pub fn fit_camera_to_bounds(&mut self, bounds: MeshBounds, viewport_aspect: f32) {
        let mut fitted = OrbitControls::default();
        fitted.fit_to_bounds(bounds, viewport_aspect);
        self.orbit = OrbitControls::default()
            .with_target(fitted.target)
            .with_position(
                fitted.distance,
                fitted.azimuth.to_degrees(),
                fitted.elevation.to_degrees(),
            );
        self.orbit.update_camera(&mut self.camera);
        self.camera_fitted = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn pick() -> MeshPlotPick {
        MeshPlotPick {
            plot_id: Arc::from("plot"),
            mesh_id: Arc::from("mesh"),
            cell_index: 3,
            cell_id: Some(42),
            nearest_vertex_index: Some(1),
            vertex_id: Some(7),
            world_position: [0.25, 0.5, 0.0],
            displayed_value: Some(2.5),
            field_id: Some(Arc::from("field")),
        }
    }

    #[test]
    fn keyboard_navigation_preserves_selection_and_updates_view() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.set_selection(Some(pick()));
        let before = state.interaction.x_domain();
        assert!(state.handle_key("+") || state.handle_key("equal"));
        assert_ne!(state.interaction.x_domain(), before);
        assert_eq!(
            state.selection.as_ref().and_then(|pick| pick.cell_id),
            Some(42)
        );
        assert_eq!(state.hover_tooltip().as_deref(), None);
        state.set_hover(state.selection.clone());
        assert!(
            state
                .hover_tooltip()
                .is_some_and(|text| text.contains("42"))
        );
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn orbit_navigation_changes_camera_without_touching_2d_state() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        let initial = state.camera.position;
        state.orbit_rotate(20.0, 5.0);
        assert_ne!(state.camera.position, initial);
        state.orbit_zoom(0.5);
        assert!(state.camera.position.is_finite());
        state.orbit_reset();
        assert_eq!(state.interaction.x_domain(), (0.0, 1.0));
    }
}
