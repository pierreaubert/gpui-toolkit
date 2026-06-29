use super::super::camera::{Camera3D, OrbitControls};
use gpui::*;

/// Interactive state for 3D surface element
#[derive(Debug, Clone)]
pub struct Surface3DState {
    /// Orbit camera controls
    pub controls: OrbitControls,
    /// Camera
    pub camera: Camera3D,
    /// Is mouse currently dragging (for rotation)
    pub dragging: bool,
    /// Is middle mouse dragging (for pan)
    pub panning: bool,
    /// Last mouse position
    pub last_mouse: Option<Point<Pixels>>,
}

impl Default for Surface3DState {
    fn default() -> Self {
        let controls = OrbitControls::default();
        let camera = controls.to_camera();
        Self {
            controls,
            camera,
            dragging: false,
            panning: false,
            last_mouse: None,
        }
    }
}

impl Surface3DState {
    /// Create state with custom initial camera position
    pub fn new(distance: f32, azimuth_deg: f32, elevation_deg: f32) -> Self {
        let controls = OrbitControls::default().with_position(distance, azimuth_deg, elevation_deg);
        let camera = controls.to_camera();
        Self {
            controls,
            camera,
            dragging: false,
            panning: false,
            last_mouse: None,
        }
    }

    /// Update camera from controls
    pub fn update_camera(&mut self) {
        self.controls.update_camera(&mut self.camera);
    }
}
