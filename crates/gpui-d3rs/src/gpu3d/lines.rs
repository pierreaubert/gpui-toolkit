//! Sparse 3D line/polygon primitives rendered via CPU projection + GPUI paths.
//!
//! For low-vertex-count scenes (axis frames, wireframe spheres, spider
//! polygons) the dense-mesh wgpu path used by [`Surface3DElement`] is
//! overkill — we'd burn an entire render pipeline on ~50 vertices. Instead,
//! we project to screen space on the CPU and let GPUI rasterise the
//! resulting paths, which is the same approach `Surface3DElement` already
//! uses for axis labels and tick lines (see `element.rs`).
//!
//! The element re-uses [`OrbitControls`] / [`Camera3D`] from this module so
//! orbit / pan / zoom is identical to the surface renderer. Mouse handling
//! lives on the parent view: the view forwards `on_mouse_*` events to the
//! shared [`Lines3DState`] (typed `Rc<RefCell<_>>`), and the element reads
//! the latest camera on each paint.
//!
//! This module deliberately avoids any new wgpu state so it stays under the
//! ~250 LoC budget called out in the spatial-spider plan.

use super::camera::{Camera3D, OrbitControls};
use glam::Vec3;
use gpui::*;
use std::cell::RefCell;
use std::panic;
use std::rc::Rc;

/// Shared interactive state for a [`Lines3DElement`].
///
/// Mirrors `Surface3DState` so a future migration to a unified
/// "GPU-or-CPU 3D viewport" state can share types.
#[derive(Debug, Clone)]
pub struct Lines3DState {
    pub controls: OrbitControls,
    pub camera: Camera3D,
    pub dragging: bool,
    pub panning: bool,
    pub last_mouse: Option<Point<Pixels>>,
}

impl Default for Lines3DState {
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

impl Lines3DState {
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

    /// Recompute the camera from the current orbit controls. Call after
    /// any rotate / pan / zoom mutation.
    pub fn update_camera(&mut self) {
        self.controls.update_camera(&mut self.camera);
    }
}

/// A single straight line segment in world space.
#[derive(Debug, Clone)]
pub struct Line3D {
    pub from: Vec3,
    pub to: Vec3,
    pub color: Rgba,
    /// Stroke width in pixels.
    pub width: f32,
}

/// A closed polygon — vertices traced in order, filled with `fill` and
/// outlined with `stroke`.
#[derive(Debug, Clone)]
pub struct Polygon3D {
    pub vertices: Vec<Vec3>,
    pub fill: Option<Rgba>,
    pub stroke: Option<(Rgba, f32)>,
}

/// All primitives the element should draw on the next paint.
#[derive(Debug, Clone, Default)]
pub struct Lines3DScene {
    /// Background colour for the viewport (alpha is honoured).
    pub background: Option<Rgba>,
    pub lines: Vec<Line3D>,
    pub polygons: Vec<Polygon3D>,
}

/// GPUI element. Construct fresh each paint with the scene to render. State
/// lives behind an `Rc<RefCell<_>>` so the owning view can mutate it from
/// mouse handlers.
#[derive(Clone)]
pub struct Lines3DElement {
    state: Rc<RefCell<Lines3DState>>,
    scene: Lines3DScene,
}

impl Lines3DElement {
    pub fn new(state: Rc<RefCell<Lines3DState>>, scene: Lines3DScene) -> Self {
        Self { state, scene }
    }

    fn project(camera: &Camera3D, p: Vec3, w: f32, h: f32) -> Option<(f32, f32)> {
        let s = camera.project_to_screen(p, w, h)?;
        // wgpu/glam perspective projects into z ∈ [0, 1]. Clip behind near plane.
        if (0.0..=1.0).contains(&s.z) {
            Some((s.x, s.y))
        } else {
            None
        }
    }
}

impl IntoElement for Lines3DElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Lines3DElement {
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
        let style = Style {
            size: Size {
                width: relative(1.0).into(),
                height: relative(1.0).into(),
            },
            ..Default::default()
        };
        (window.request_layout(style, [], cx), ())
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
        let w: f32 = bounds.size.width.into();
        let h: f32 = bounds.size.height.into();
        if h > 0.0 {
            self.state.borrow_mut().camera.aspect = w / h;
            // Recompute camera so the freshly-set aspect ratio actually applies.
            // controls.update_camera preserves aspect rather than overwriting it.
            self.state.borrow_mut().update_camera();
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
        let w: f32 = bounds.size.width.into();
        let h: f32 = bounds.size.height.into();
        if w <= 0.0 || h <= 0.0 {
            return;
        }

        // Background fill, if requested. Painted as a rectangle path so we
        // stay on GPUI's PathBuilder API exclusively.
        if let Some(bg) = self.scene.background {
            let mut b = PathBuilder::fill();
            let o = bounds.origin;
            let r = Point {
                x: o.x + bounds.size.width,
                y: o.y + bounds.size.height,
            };
            b.move_to(Point { x: o.x, y: o.y });
            b.line_to(Point { x: r.x, y: o.y });
            b.line_to(Point { x: r.x, y: r.y });
            b.line_to(Point { x: o.x, y: r.y });
            b.line_to(Point { x: o.x, y: o.y });
            if let Ok(p) = b.build() {
                window.paint_path(p, bg);
            }
        }

        let camera = self.state.borrow().camera.clone();

        // Filled polygons + their outlines.
        for poly in &self.scene.polygons {
            let projected: Vec<(f32, f32)> = poly
                .vertices
                .iter()
                .filter_map(|v| Self::project(&camera, *v, w, h))
                .collect();
            // Need at least a triangle.
            if projected.len() < 3 {
                continue;
            }
            if let Some(fill) = poly.fill {
                let mut b = PathBuilder::fill();
                let (x0, y0) = projected[0];
                b.move_to(Point {
                    x: px(x0) + bounds.origin.x,
                    y: px(y0) + bounds.origin.y,
                });
                for &(x, y) in &projected[1..] {
                    b.line_to(Point {
                        x: px(x) + bounds.origin.x,
                        y: px(y) + bounds.origin.y,
                    });
                }
                b.line_to(Point {
                    x: px(x0) + bounds.origin.x,
                    y: px(y0) + bounds.origin.y,
                });
                if let Ok(p) = b.build() {
                    window.paint_path(p, fill);
                }
            }
            if let Some((stroke, width)) = poly.stroke {
                let mut b = PathBuilder::stroke(px(width));
                let (x0, y0) = projected[0];
                b.move_to(Point {
                    x: px(x0) + bounds.origin.x,
                    y: px(y0) + bounds.origin.y,
                });
                for &(x, y) in &projected[1..] {
                    b.line_to(Point {
                        x: px(x) + bounds.origin.x,
                        y: px(y) + bounds.origin.y,
                    });
                }
                b.line_to(Point {
                    x: px(x0) + bounds.origin.x,
                    y: px(y0) + bounds.origin.y,
                });
                if let Ok(p) = b.build() {
                    window.paint_path(p, stroke);
                }
            }
        }

        // Free-floating line segments (axes, grid, wireframe spheres).
        for line in &self.scene.lines {
            let Some((x0, y0)) = Self::project(&camera, line.from, w, h) else {
                continue;
            };
            let Some((x1, y1)) = Self::project(&camera, line.to, w, h) else {
                continue;
            };
            let mut b = PathBuilder::stroke(px(line.width));
            b.move_to(Point {
                x: px(x0) + bounds.origin.x,
                y: px(y0) + bounds.origin.y,
            });
            b.line_to(Point {
                x: px(x1) + bounds.origin.x,
                y: px(y1) + bounds.origin.y,
            });
            if let Ok(p) = b.build() {
                window.paint_path(p, line.color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_has_camera_initialized_from_controls() {
        let state = Lines3DState::default();
        // Camera should reflect the orbit controls' initial spherical pose.
        let expected = state.controls.to_camera();
        assert!((state.camera.position - expected.position).length() < 1e-5);
        assert_eq!(state.camera.target, expected.target);
    }

    #[test]
    fn new_state_applies_overrides() {
        let s = Lines3DState::new(5.0, 90.0, 30.0);
        assert!((s.controls.distance - 5.0).abs() < 1e-5);
        assert!((s.controls.azimuth - 90.0_f32.to_radians()).abs() < 1e-5);
        assert!((s.controls.elevation - 30.0_f32.to_radians()).abs() < 1e-5);
    }

    #[test]
    fn update_camera_propagates_orbit_changes() {
        let mut s = Lines3DState::default();
        let before = s.camera.position;
        s.controls.azimuth += 0.5;
        s.update_camera();
        assert!(
            (s.camera.position - before).length() > 1e-3,
            "camera should have moved after rotating orbit",
        );
    }

    #[test]
    fn scene_default_is_empty() {
        let s = Lines3DScene::default();
        assert!(s.lines.is_empty());
        assert!(s.polygons.is_empty());
        assert!(s.background.is_none());
    }
}
