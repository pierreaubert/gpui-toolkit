//! Interactive 3D viewer: orbit, zoom, pan, and a cutting plane.
//!
//! The classical viewport controls, wired to `gpui-scene3d`:
//! - left-drag rotates (orbit), double-click resets the view
//! - mouse wheel zooms (dolly)
//! - right-drag or Shift+drag pans (grab the world)
//! - the panel adds a cutting plane: toggle it, pick its axis, then
//!   step its offset with the buttons or Alt+drag in the viewport
//!
//! The scene is the PC-fan demo dataset (120 mm fan, pressure field).
//! Rendering is the deterministic software rasterizer through
//! `SceneElement`, so the same scene renders headlessly for captures.
//!
//! Run: `cargo run -p gpui-scene3d --example interactive_viewer --features gpui`

// Rust guideline compliant 2026-02-21

use gpui::{
    Context, MouseButton, MouseDownEvent, MouseMoveEvent, Render, ScrollDelta, ScrollWheelEvent,
    Window, div, prelude::*, px, rgb,
};
use gpui_mesh::{CellType, Cells, DataSet, Field};
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_scene3d::{
    Actor, Camera, ClipPlane, Scene, TransferFunction,
    element::{SceneCamera, SceneElement, fitted_camera},
};
use std::f64::consts::TAU;

const HUB_R: f64 = 0.018;
const TIP_R: f64 = 0.054;

/// The fan demo dataset (same generator as the pcfan example, compact).
fn fan_dataset() -> DataSet {
    let (nr, ntheta, nz) = (10usize, 36usize, 28usize);
    let mut positions = Vec::new();
    for k in 0..nz {
        let z = -0.05 + 0.15 * k as f64 / (nz - 1) as f64;
        for j in 0..ntheta {
            let theta = TAU * j as f64 / ntheta as f64;
            for i in 0..nr {
                let r = HUB_R + (TIP_R - HUB_R) * i as f64 / (nr - 1) as f64;
                positions.push([r * theta.cos(), r * theta.sin(), z]);
            }
        }
    }
    let idx = |i: usize, j: usize, k: usize| (k * ntheta * nr + (j % ntheta) * nr + i) as u32;
    let mut cells: Vec<(CellType, [u32; 8])> = Vec::new();
    for k in 0..nz - 1 {
        for j in 0..ntheta {
            for i in 0..nr - 1 {
                cells.push((
                    CellType::Hexahedron,
                    [
                        idx(i, j, k),
                        idx(i + 1, j, k),
                        idx(i + 1, j + 1, k),
                        idx(i, j + 1, k),
                        idx(i, j, k + 1),
                        idx(i + 1, j, k + 1),
                        idx(i + 1, j + 1, k + 1),
                        idx(i, j + 1, k + 1),
                    ],
                ));
            }
        }
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    let mut dataset = DataSet::new("pcfan", positions, Cells::from_cells(&refs));
    let omega = 1500.0 * TAU / 60.0;
    let mut velocity = Vec::with_capacity(dataset.point_count());
    let mut pressure = Vec::with_capacity(dataset.point_count());
    for p in dataset.positions.iter() {
        let r = (p[0] * p[0] + p[1] * p[1]).sqrt();
        let swirl = if p[2] < 0.0 {
            0.0
        } else if p[2] <= 0.025 {
            p[2] / 0.025
        } else {
            (-(p[2] - 0.025) / 0.05).exp()
        };
        let v_theta = 0.38 * omega * r * swirl;
        velocity.push([
            -v_theta * p[1] / r.max(1e-9),
            v_theta * p[0] / r.max(1e-9),
            3.2,
        ]);
        pressure.push(-40.0 + 0.6 * v_theta * v_theta + 900.0 * (p[2] + 0.05));
    }
    dataset
        .point_data
        .push(Field::vector("velocity", velocity).labelled("Velocity", "m/s"));
    dataset
        .point_data
        .push(Field::scalar("pressure", pressure).labelled("Pressure", "Pa"));
    dataset
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragMode {
    Rotate,
    Pan,
    Plane,
}

struct DragState {
    mode: DragMode,
    last: gpui::Point<gpui::Pixels>,
}

/// Observable viewer state, mirrored on every render for tests and
/// diagnostics (the headless visual harness reads this after each
/// simulated input).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewerProbe {
    /// Camera azimuth in radians.
    pub azimuth: f32,
    /// Camera elevation in radians.
    pub elevation: f32,
    /// Camera distance.
    pub distance: f32,
    /// Camera target.
    pub target: [f32; 3],
    /// Whether the cutting plane is enabled.
    pub clip_enabled: bool,
    /// Cutting-plane axis (0 = X, 1 = Y, 2 = Z).
    pub clip_axis: usize,
    /// Cutting-plane offset along its axis.
    pub clip_offset: f64,
    /// Enabled state of the three axis planes.
    pub planes_enabled: [bool; 3],
    /// Whether the clip box is enabled.
    pub box_enabled: bool,
    /// Clip-box inset as a fraction of the bounds extent per side.
    pub box_inset: f64,
}

impl Default for ViewerProbe {
    fn default() -> Self {
        Self {
            azimuth: 0.0,
            elevation: 0.0,
            distance: 0.0,
            target: [0.0; 3],
            clip_enabled: false,
            clip_axis: 1,
            clip_offset: 0.0,
            planes_enabled: [false; 3],
            box_enabled: false,
            box_inset: 0.0,
        }
    }
}

/// Shared probe handle.
pub type SharedProbe = std::rc::Rc<std::cell::RefCell<ViewerProbe>>;

struct Viewer {
    dataset: DataSet,
    camera: SceneCamera,
    home: Camera,
    drag: Option<DragState>,
    plane_enabled: [bool; 3],
    clip_axis: usize,
    plane_offset: [f64; 3],
    box_enabled: bool,
    box_inset: f64,
    bounds: gpui_mesh::Bounds,
    probe: SharedProbe,
    panel_scroll: gpui::ScrollHandle,
}

impl Viewer {
    fn new(cx: &mut Context<Self>) -> Self {
        Self::with_probe(
            cx,
            std::rc::Rc::new(std::cell::RefCell::new(ViewerProbe::default())),
        )
    }

    fn with_probe(_cx: &mut Context<Self>, probe: SharedProbe) -> Self {
        let dataset = fan_dataset();
        let bounds0 = dataset.bounds().unwrap();
        let scene = Self::scene_for(&dataset, &[(false, 0.0); 3], bounds0, false, 0.0);
        let camera = fitted_camera(&scene);
        let home = *camera.borrow();
        let bounds = dataset.bounds().unwrap();
        let viewer = Self {
            dataset,
            camera,
            home,
            drag: None,
            plane_enabled: [false; 3],
            clip_axis: 1,
            plane_offset: [0.0; 3],
            box_enabled: false,
            box_inset: 0.0,
            bounds,
            probe,
            panel_scroll: gpui::ScrollHandle::new(),
        };
        viewer.publish_probe();
        viewer
    }

    fn publish_probe(&self) {
        let camera = self.camera.borrow();
        *self.probe.borrow_mut() = ViewerProbe {
            azimuth: camera.azimuth,
            elevation: camera.elevation,
            distance: camera.distance,
            target: camera.target,
            clip_enabled: self.plane_enabled[self.clip_axis],
            clip_axis: self.clip_axis,
            clip_offset: self.plane_offset[self.clip_axis],
            planes_enabled: self.plane_enabled,
            box_enabled: self.box_enabled,
            box_inset: self.box_inset,
        };
    }

    fn scene_for(
        dataset: &DataSet,
        planes: &[(bool, f64); 3],
        bounds: gpui_mesh::Bounds,
        box_enabled: bool,
        box_inset: f64,
    ) -> Scene {
        let mut actor = Actor::new(dataset.clone())
            .coloured_by("pressure", TransferFunction::coolwarm([-40.0, 160.0]));
        for (axis, (enabled, offset)) in planes.iter().enumerate() {
            if *enabled {
                let mut origin = [0.0; 3];
                let mut normal = [0.0; 3];
                origin[axis] = *offset;
                normal[axis] = 1.0;
                actor = actor.clip(ClipPlane { origin, normal });
            }
        }
        if box_enabled {
            let mut min = bounds.min;
            let mut max = bounds.max;
            for axis in 0..3 {
                let inset = (max[axis] - min[axis]) * box_inset;
                min[axis] += inset;
                max[axis] -= inset;
            }
            actor = actor.clip_box(gpui_scene3d::ClipBox { min, max });
        }
        Scene::new().with_actor(actor)
    }

    fn current_scene(&self) -> Scene {
        let planes = [
            (self.plane_enabled[0], self.plane_offset[0]),
            (self.plane_enabled[1], self.plane_offset[1]),
            (self.plane_enabled[2], self.plane_offset[2]),
        ];
        Self::scene_for(
            &self.dataset,
            &planes,
            self.bounds,
            self.box_enabled,
            self.box_inset,
        )
    }

    fn axis_extent(&self) -> (f64, f64) {
        (
            self.bounds.min[self.clip_axis],
            self.bounds.max[self.clip_axis],
        )
    }

    fn step_plane(&mut self, direction: f64) {
        let (min, max) = self.axis_extent();
        let step = (max - min) * 0.02 * direction;
        let offset = &mut self.plane_offset[self.clip_axis];
        *offset = (*offset + step).clamp(min, max);
    }
}

impl Render for Viewer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.publish_probe();
        let scene = self.current_scene();
        let element = SceneElement::new(scene, self.camera.clone()).id("interactive-scene");
        let camera_state = self.camera.clone();
        let (azimuth, elevation, distance) = {
            let camera = self.camera.borrow();
            (camera.azimuth, camera.elevation, camera.distance)
        };
        let (min, max) = self.axis_extent();
        let plane_percent = if max > min {
            (self.plane_offset[self.clip_axis] - min) / (max - min) * 100.0
        } else {
            0.0
        };
        let axis_name = ["X", "Y", "Z"][self.clip_axis];

        let viewport = div()
            .relative()
            .flex_1()
            .h_full()
            .bg(rgb(0x14161a))
            .child(element)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    if event.click_count == 2 {
                        *this.camera.borrow_mut() = this.home;
                    } else {
                        this.drag = Some(DragState {
                            mode: if event.modifiers.alt {
                                DragMode::Plane
                            } else if event.modifiers.shift {
                                DragMode::Pan
                            } else {
                                DragMode::Rotate
                            },
                            last: event.position,
                        });
                    }
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    this.drag = Some(DragState {
                        mode: DragMode::Pan,
                        last: event.position,
                    });
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                let Some(drag) = &mut this.drag else { return };
                let dx: f32 = (event.position.x - drag.last.x).into();
                let dy: f32 = (event.position.y - drag.last.y).into();
                drag.last = event.position;
                match drag.mode {
                    DragMode::Rotate => {
                        this.camera.borrow_mut().orbit(dx * 0.01, -dy * 0.01);
                    }
                    DragMode::Pan => {
                        this.camera.borrow_mut().pan(dx, dy);
                    }
                    DragMode::Plane => {
                        this.plane_enabled[this.clip_axis] = true;
                        let (min, max) = this.axis_extent();
                        let fraction = f64::from(dy) * 0.002;
                        let offset = &mut this.plane_offset[this.clip_axis];
                        *offset = (*offset - fraction * (max - min)).clamp(min, max);
                    }
                }
                cx.notify();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.drag = None;
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, _, _, cx| {
                    this.drag = None;
                    cx.notify();
                }),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                let delta = match event.delta {
                    ScrollDelta::Lines(lines) => lines.y,
                    ScrollDelta::Pixels(pixels) => f32::from(pixels.y) * 0.02,
                };
                this.camera
                    .borrow_mut()
                    .dolly((1.0 - delta * 0.08).clamp(0.5, 1.8));
                cx.notify();
            }));

        let panel = div()
            .id("control-panel")
            .w(px(250.0))
            .h_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .overflow_y_scroll()
            .track_scroll(&self.panel_scroll)
            .bg(rgb(0x1d2026))
            .text_color(rgb(0xe8eaf0))
            .child(div().text_lg().child("PC fan — pressure"))
            .child(
                div().text_sm().text_color(rgb(0x9aa1ad)).child(
                    "Drag: rotate · Right/Shift+drag: pan · Wheel: zoom · Double-click: reset",
                ),
            )
            .child(div().mt_2().text_base().child("Cutting plane"))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.toggle_button(cx))
                    .child(self.axis_button(0, "X", cx))
                    .child(self.axis_button(1, "Y", cx))
                    .child(self.axis_button(2, "Z", cx)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(self.step_button("- step", -1.0, cx))
                    .child(self.step_button("+ step", 1.0, cx)),
            )
            .child(div().text_sm().child(format!(
                "Plane: {} at {:.1}% (Alt+drag in view moves it)",
                if self.plane_enabled[self.clip_axis] {
                    axis_name
                } else {
                    "off"
                },
                plane_percent
            )))
            .child(div().text_sm().child(format!(
                "Planes on: {}",
                ["X", "Y", "Z"]
                    .iter()
                    .zip(self.plane_enabled.iter())
                    .filter(|(_, on)| **on)
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>()
                    .join(" ")
            )))
            .child(div().mt_2().text_base().child("Clip box"))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(self.box_toggle_button(cx))
                    .child(self.box_step_button("- inset", -1.0, cx))
                    .child(self.box_step_button("+ inset", 1.0, cx)),
            )
            .child(div().text_sm().child(format!(
                "Box: {} at {:.0}% inset",
                if self.box_enabled { "on" } else { "off" },
                self.box_inset * 100.0
            )))
            .child(
                div()
                    .mt_4()
                    .text_sm()
                    .text_color(rgb(0x9aa1ad))
                    .child(format!(
                        "Azimuth {azimuth:.2} · Elevation {elevation:.2} · Distance {distance:.2}"
                    )),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x9aa1ad))
                    .child("Every control repaints the same scene."),
            );

        let _ = camera_state;
        div().flex().size_full().child(viewport).child(panel)
    }
}

impl Viewer {
    fn toggle_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.plane_enabled[self.clip_axis];
        div()
            .id("plane-toggle")
            .debug_selector(|| "plane-toggle".to_string())
            .px_3()
            .py_1()
            .rounded_md()
            .bg(if active { rgb(0x2f6fed) } else { rgb(0x2a2e37) })
            .cursor_pointer()
            .child(if active { "Plane on" } else { "Plane off" })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    let axis = this.clip_axis;
                    this.plane_enabled[axis] = !this.plane_enabled[axis];
                    cx.notify();
                }),
            )
    }

    fn axis_button(
        &self,
        axis: usize,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.plane_enabled[axis];
        div()
            .id(match axis {
                0 => "plane-axis-x",
                1 => "plane-axis-y",
                _ => "plane-axis-z",
            })
            .debug_selector(move || match axis {
                0 => "plane-axis-x".to_string(),
                1 => "plane-axis-y".to_string(),
                _ => "plane-axis-z".to_string(),
            })
            .px_3()
            .py_1()
            .rounded_md()
            .bg(if active { rgb(0x2f6fed) } else { rgb(0x2a2e37) })
            .cursor_pointer()
            .child(label)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    if this.clip_axis == axis && this.plane_enabled[axis] {
                        this.plane_enabled[axis] = false;
                    } else {
                        this.plane_enabled[axis] = true;
                    }
                    this.clip_axis = axis;
                    let (min, max) = this.axis_extent();
                    let offset = &mut this.plane_offset[axis];
                    *offset = (*offset).clamp(min, max);
                    cx.notify();
                }),
            )
    }

    fn step_button(
        &self,
        label: &'static str,
        direction: f64,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id(if direction < 0.0 {
                "plane-step-minus"
            } else {
                "plane-step-plus"
            })
            .debug_selector(move || {
                if direction < 0.0 {
                    "plane-step-minus".to_string()
                } else {
                    "plane-step-plus".to_string()
                }
            })
            .px_3()
            .py_1()
            .rounded_md()
            .bg(rgb(0x2a2e37))
            .cursor_pointer()
            .child(label)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.plane_enabled[this.clip_axis] = true;
                    this.step_plane(direction);
                    cx.notify();
                }),
            )
    }

    fn box_toggle_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.box_enabled;
        div()
            .id("box-toggle")
            .debug_selector(|| "box-toggle".to_string())
            .px_3()
            .py_1()
            .rounded_md()
            .bg(if active { rgb(0x2f6fed) } else { rgb(0x2a2e37) })
            .cursor_pointer()
            .child(if active { "Box on" } else { "Box off" })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.box_enabled = !this.box_enabled;
                    if this.box_enabled && this.box_inset == 0.0 {
                        this.box_inset = 0.15;
                    }
                    cx.notify();
                }),
            )
    }

    fn box_step_button(
        &self,
        label: &'static str,
        direction: f64,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id(if direction < 0.0 {
                "box-step-minus"
            } else {
                "box-step-plus"
            })
            .debug_selector(move || {
                if direction < 0.0 {
                    "box-step-minus".to_string()
                } else {
                    "box-step-plus".to_string()
                }
            })
            .px_3()
            .py_1()
            .rounded_md()
            .bg(rgb(0x2a2e37))
            .cursor_pointer()
            .child(label)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.box_enabled = true;
                    this.box_inset = (this.box_inset + direction * 0.05).clamp(0.0, 0.45);
                    cx.notify();
                }),
            )
    }
}

fn main() {
    let config = MiniAppConfig::new("Scene3D interactive viewer").size(1100.0, 700.0);
    MiniApp::run(config, |cx| cx.new(Viewer::new));
}

#[cfg(all(test, target_os = "macos"))]
mod interaction_tests {
    use super::*;
    use gpui::{
        AnyWindowHandle, AppContext, HeadlessAppContext, InputEvent, Modifiers, MouseButton,
        MouseDownEvent, MouseMoveEvent, MouseUpEvent, ScrollDelta, ScrollWheelEvent, TouchPhase,
        point, px, size,
    };
    use gpui_macos::metal_renderer::MetalHeadlessRenderer;
    use std::sync::{Arc, Mutex, MutexGuard};
    use std::time::Instant;

    static PLATFORM_LOCK: Mutex<()> = Mutex::new(());

    fn platform_lock() -> MutexGuard<'static, ()> {
        PLATFORM_LOCK.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn dispatch(cx: &mut HeadlessAppContext, window: AnyWindowHandle, event: impl InputEvent) {
        cx.update_window(window, |_, w, app| {
            w.dispatch_event(event.to_platform_input(), app);
        })
        .expect("dispatch viewer event");
        cx.run_until_parked();
        cx.update_window(window, |_, w, app| {
            let _ = w.draw(app);
        })
        .expect("draw viewer frame");
        cx.run_until_parked();
    }

    fn down(
        cx: &mut HeadlessAppContext,
        window: AnyWindowHandle,
        position: gpui::Point<gpui::Pixels>,
        button: MouseButton,
        modifiers: Modifiers,
    ) {
        dispatch(
            cx,
            window,
            MouseDownEvent {
                position,
                modifiers,
                button,
                click_count: 1,
                first_mouse: false,
            },
        );
    }
    fn down2(
        cx: &mut HeadlessAppContext,
        window: AnyWindowHandle,
        position: gpui::Point<gpui::Pixels>,
    ) {
        dispatch(
            cx,
            window,
            MouseDownEvent {
                position,
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: 2,
                first_mouse: false,
            },
        );
    }
    fn up(
        cx: &mut HeadlessAppContext,
        window: AnyWindowHandle,
        position: gpui::Point<gpui::Pixels>,
        button: MouseButton,
        modifiers: Modifiers,
    ) {
        dispatch(
            cx,
            window,
            MouseUpEvent {
                position,
                modifiers,
                button,
                click_count: 1,
            },
        );
    }
    fn mv(
        cx: &mut HeadlessAppContext,
        window: AnyWindowHandle,
        position: gpui::Point<gpui::Pixels>,
        button: MouseButton,
        modifiers: Modifiers,
    ) {
        dispatch(
            cx,
            window,
            MouseMoveEvent {
                position,
                pressed_button: Some(button),
                modifiers,
            },
        );
    }
    fn click(
        cx: &mut HeadlessAppContext,
        window: AnyWindowHandle,
        position: gpui::Point<gpui::Pixels>,
    ) {
        down(
            cx,
            window,
            position,
            MouseButton::Left,
            Modifiers::default(),
        );
        up(
            cx,
            window,
            position,
            MouseButton::Left,
            Modifiers::default(),
        );
    }

    fn pixel_diff(a: &image::RgbaImage, b: &image::RgbaImage) -> usize {
        a.as_raw()
            .as_chunks::<4>()
            .0
            .iter()
            .zip(b.as_raw().as_chunks::<4>().0.iter())
            .filter(|(x, y)| (0..3).any(|c| (i32::from(x[c]) - i32::from(y[c])).abs() > 8))
            .count()
    }

    /// Drives every viewport and panel control headlessly (Metal
    /// offscreen) and asserts probe state plus rendered-frame changes
    /// after each interaction, with per-frame timing for drags.
    #[test]
    fn drives_every_control() {
        let _lock = platform_lock();
        if MetalHeadlessRenderer::try_new().is_none() {
            eprintln!("viewer interaction test skipped: no Metal device");
            return;
        }
        let probe: SharedProbe = std::rc::Rc::new(std::cell::RefCell::new(ViewerProbe::default()));
        let mut cx = HeadlessAppContext::with_platform(
            Arc::new(gpui::NoopTextSystem::new()),
            Arc::new(()),
            || Some(Box::new(MetalHeadlessRenderer::new())),
        );
        let probe_in_window = probe.clone();
        let window = cx
            .open_window(size(px(1100.0), px(700.0)), move |_window, app| {
                app.new(|cx| Viewer::with_probe(cx, probe_in_window.clone()))
            })
            .expect("open viewer window");
        let window: AnyWindowHandle = window.into();
        cx.update_window(window, |_, w, app| {
            let _ = w.draw(app);
        })
        .expect("initial draw");
        cx.run_until_parked();

        let state = |probe: &SharedProbe| *probe.borrow();
        let initial = state(&probe);
        let centre = point(px(425.0), px(350.0));

        let started = Instant::now();
        let frame0 = cx.capture_screenshot(window).expect("initial capture");
        let initial_ms = started.elapsed().as_secs_f64() * 1e3;
        // Captures are device pixels: a 1100-logical-pixel window on a
        // 2x headless display reads back at 2200.
        assert_eq!(frame0.width(), 2200);

        // Left-drag rotate (24 stepped moves, timed per frame).
        let mut times = Vec::new();
        down(
            &mut cx,
            window,
            centre,
            MouseButton::Left,
            Modifiers::default(),
        );
        let mut pos = centre;
        for _ in 0..24 {
            let t = Instant::now();
            pos = point(pos.x + px(6.0), pos.y + px(2.0));
            mv(
                &mut cx,
                window,
                pos,
                MouseButton::Left,
                Modifiers::default(),
            );
            times.push(t.elapsed().as_secs_f64() * 1e3);
        }
        up(
            &mut cx,
            window,
            pos,
            MouseButton::Left,
            Modifiers::default(),
        );
        let rotated = state(&probe);
        assert!(
            (rotated.azimuth - initial.azimuth - 24.0 * 6.0 * 0.01).abs() < 0.05,
            "rotate azimuth {} -> {}",
            initial.azimuth,
            rotated.azimuth
        );
        let frame1 = cx.capture_screenshot(window).expect("rotated capture");
        assert!(
            pixel_diff(&frame0, &frame1) > 5_000,
            "rotate changes the frame"
        );

        // Wheel zoom.
        let before = state(&probe);
        dispatch(
            &mut cx,
            window,
            ScrollWheelEvent {
                position: centre,
                delta: ScrollDelta::Lines(point(0.0, 3.0)),
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            },
        );
        let after = state(&probe);
        assert!(
            after.distance < before.distance,
            "wheel zooms in {} -> {}",
            before.distance,
            after.distance
        );
        let frame2 = cx.capture_screenshot(window).expect("zoom capture");
        assert!(pixel_diff(&frame1, &frame2) > 1_000);

        // Right-drag pan.
        let before = state(&probe);
        down(
            &mut cx,
            window,
            centre,
            MouseButton::Right,
            Modifiers::default(),
        );
        mv(
            &mut cx,
            window,
            point(px(485.0), px(320.0)),
            MouseButton::Right,
            Modifiers::default(),
        );
        up(
            &mut cx,
            window,
            point(px(485.0), px(320.0)),
            MouseButton::Right,
            Modifiers::default(),
        );
        let after = state(&probe);
        assert_ne!(after.target, before.target, "right-drag pans");
        assert!((after.distance - before.distance).abs() < 1e-6);

        // Shift+drag pan.
        let before = state(&probe);
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        down(&mut cx, window, centre, MouseButton::Left, shift);
        mv(
            &mut cx,
            window,
            point(px(385.0), px(390.0)),
            MouseButton::Left,
            shift,
        );
        up(
            &mut cx,
            window,
            point(px(385.0), px(390.0)),
            MouseButton::Left,
            shift,
        );
        assert_ne!(state(&probe).target, before.target, "shift-drag pans");

        // Double-click reset.
        down2(&mut cx, window, centre);
        let reset = state(&probe);
        assert_eq!(reset.azimuth, initial.azimuth);
        assert_eq!(reset.distance, initial.distance);
        assert_eq!(reset.target, initial.target);
        let frame_reset = cx.capture_screenshot(window).expect("reset capture");
        assert!(
            pixel_diff(&frame0, &frame_reset) < 500,
            "reset matches initial frame"
        );

        // Cutting plane toggle.
        let toggle = cx
            .debug_bounds(window, "plane-toggle")
            .expect("window")
            .expect("toggle bounds")
            .center();
        click(&mut cx, window, toggle);
        assert!(state(&probe).clip_enabled, "toggle enables plane");
        let frame_plane = cx.capture_screenshot(window).expect("plane capture");
        assert!(
            pixel_diff(&frame_reset, &frame_plane) > 5_000,
            "plane cuts the frame"
        );
        click(&mut cx, window, toggle);
        assert!(!state(&probe).clip_enabled, "toggle disables plane");
        click(&mut cx, window, toggle);
        assert!(state(&probe).clip_enabled);

        // Axis buttons.
        for (id, axis) in [
            ("plane-axis-x", 0usize),
            ("plane-axis-y", 1),
            ("plane-axis-z", 2),
        ] {
            let button = cx
                .debug_bounds(window, id)
                .expect("window")
                .expect("axis bounds")
                .center();
            click(&mut cx, window, button);
            let now = state(&probe);
            assert_eq!(now.clip_axis, axis, "{id} selects axis {axis}");
            assert!(now.clip_enabled);
        }

        // Step buttons.
        let before = state(&probe);
        let plus = cx
            .debug_bounds(window, "plane-step-plus")
            .expect("window")
            .expect("plus bounds")
            .center();
        click(&mut cx, window, plus);
        let after_plus = state(&probe);
        assert!(
            after_plus.clip_offset > before.clip_offset,
            "+ step advances {} -> {}",
            before.clip_offset,
            after_plus.clip_offset
        );
        let minus = cx
            .debug_bounds(window, "plane-step-minus")
            .expect("window")
            .expect("minus bounds")
            .center();
        click(&mut cx, window, minus);
        assert!(
            state(&probe).clip_offset < after_plus.clip_offset,
            "- step retreats"
        );

        // All three axis planes are enabled simultaneously after the
        // axis-button loop.
        assert_eq!(state(&probe).planes_enabled, [true, true, true]);

        // Clip box toggle and inset steps.
        let frame_pre_box = cx.capture_screenshot(window).expect("pre-box capture");
        let box_toggle = cx
            .debug_bounds(window, "box-toggle")
            .expect("window")
            .expect("box toggle bounds")
            .center();
        click(&mut cx, window, box_toggle);
        let boxed = state(&probe);
        assert!(boxed.box_enabled, "box toggle enables the clip box");
        assert!((boxed.box_inset - 0.15).abs() < 1e-9);
        let box_plus = cx
            .debug_bounds(window, "box-step-plus")
            .expect("window")
            .expect("box plus bounds")
            .center();
        click(&mut cx, window, box_plus);
        assert!((state(&probe).box_inset - 0.20).abs() < 1e-9);
        let frame_box = cx.capture_screenshot(window).expect("box capture");
        assert!(
            pixel_diff(&frame_pre_box, &frame_box) > 1_000,
            "clip box changes the frame"
        );
        click(&mut cx, window, box_toggle);
        assert!(!state(&probe).box_enabled, "box toggle disables");

        // Alt+drag moves the plane.
        let before = state(&probe);
        let alt = Modifiers {
            alt: true,
            ..Modifiers::default()
        };
        down(&mut cx, window, centre, MouseButton::Left, alt);
        mv(
            &mut cx,
            window,
            point(px(425.0), px(290.0)),
            MouseButton::Left,
            alt,
        );
        up(
            &mut cx,
            window,
            point(px(425.0), px(290.0)),
            MouseButton::Left,
            alt,
        );
        assert_ne!(
            state(&probe).clip_offset,
            before.clip_offset,
            "Alt+drag moves plane"
        );
        let frame_final = cx.capture_screenshot(window).expect("final capture");
        if let Ok(dir) = std::env::var("VIEWER_CAPTURE_DIR") {
            let _ = std::fs::create_dir_all(&dir);
            let _ = frame0.save(format!("{dir}/viewer_initial.png"));
            let _ = frame_plane.save(format!("{dir}/viewer_plane_cut.png"));
            let _ = frame_final.save(format!("{dir}/viewer_final.png"));
        }
        assert!(pixel_diff(&frame_plane, &frame_final) > 100);

        times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = times[times.len() / 2];
        let worst = times[times.len() - 1];
        println!(
            "viewer timing: initial {initial_ms:.1} ms, rotate median {median:.1} ms, worst {worst:.1} ms ({} frames)",
            times.len()
        );
        assert!(worst < 5_000.0, "rotate frame took {worst:.1} ms");

        cx.update_window(window, |_, w, _| w.remove_window())
            .expect("close");
        cx.run_until_parked();
    }
}
