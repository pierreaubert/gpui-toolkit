//! Interactive 3D Surface Demo
//!
//! This demo showcases GPU-accelerated 3D surface rendering with interactive controls.
//!
//! ## Controls
//! - **Left Mouse Drag**: Rotate the surface
//! - **Middle Mouse Drag**: Pan the view
//! - **Scroll Wheel**: Zoom in/out
//! - **Double Click**: Reset view to initial position
//!
//! Run with: `cargo run --features gpu-3d --example surface3d_demo`

use d3rs::gpu3d::{Colormap, Surface3DConfig, Surface3DElement, Surface3DState, SurfaceData};
use gpui::*;
use std::cell::RefCell;
use std::rc::Rc;

struct Surface3DDemo {
    data: SurfaceData,
    config: Surface3DConfig,
    state: Rc<RefCell<Surface3DState>>,
    dragging: bool,
    panning: bool,
    last_mouse: Option<Point<Pixels>>,
}

impl Surface3DDemo {
    fn new() -> Self {
        // Create a sample surface: z = sin(sqrt(x^2 + y^2)) * cos(x) * cos(y)
        let data = SurfaceData::from_function(
            (-3.0 * std::f64::consts::PI, 3.0 * std::f64::consts::PI),
            (-3.0 * std::f64::consts::PI, 3.0 * std::f64::consts::PI),
            100,
            100,
            |x, y| {
                let r = (x * x + y * y).sqrt();
                if r < 0.01 {
                    1.0
                } else {
                    (r).sin() / r * (x * 0.3).cos() * (y * 0.3).cos()
                }
            },
        )
        .with_x_label("X")
        .with_y_label("Y")
        .with_z_label("Z");

        let config = Surface3DConfig::new()
            .colormap(Colormap::Viridis)
            .wireframe(false)
            .ambient(0.3)
            .diffuse(0.7)
            .camera_position(4.0, 45.0, 30.0);

        let state = Rc::new(RefCell::new(Surface3DState::new(
            config.camera_distance,
            config.camera_azimuth,
            config.camera_elevation,
        )));

        Self {
            data,
            config,
            state,
            dragging: false,
            panning: false,
            last_mouse: None,
        }
    }

    /// Create demo with spinorama-like dispersion data
    fn spinorama_demo() -> Self {
        // Simulate speaker dispersion: SPL varies with frequency and angle
        let freq_count = 50;
        let angle_count = 37; // -180 to +180 in 10-degree steps

        let freqs: Vec<f64> = (0..freq_count)
            .map(|i| {
                let t = i as f64 / (freq_count - 1) as f64;
                20.0 * (20000.0 / 20.0_f64).powf(t) // Log scale: 20Hz to 20kHz
            })
            .collect();

        let angles: Vec<f64> = (0..angle_count)
            .map(|i| -180.0 + (i as f64 * 10.0))
            .collect();

        // Generate dispersion pattern
        let z_values: Vec<Vec<f64>> = angles
            .iter()
            .map(|&angle| {
                freqs
                    .iter()
                    .map(|&freq| {
                        // Model: narrowing dispersion at higher frequencies
                        let log_freq = freq.log10();
                        let angle_rad = angle.to_radians();

                        // Beaming increases with frequency
                        let beaming = (log_freq - 2.0).max(0.0) * 0.5;

                        // Gaussian-like falloff from on-axis
                        let angle_factor =
                            (-angle_rad.powi(2) / (2.0 * (1.5 - beaming).max(0.3).powi(2))).exp();

                        // Base response with some ripple
                        let base = 85.0 + (log_freq * 2.0).sin() * 2.0;

                        // Final SPL
                        base * angle_factor - 40.0 * (1.0 - angle_factor)
                    })
                    .collect()
            })
            .collect();

        let data = SurfaceData::from_grid(freqs, angles, z_values)
            .with_x_label("Frequency (Hz)")
            .with_y_label("Angle (deg)")
            .with_z_label("SPL (dB)")
            .with_log_x(true)
            .with_z_range(-40.0, 95.0);

        let config = Surface3DConfig::new()
            .colormap(Colormap::Turbo)
            .wireframe(false)
            .ambient(0.25)
            .diffuse(0.75)
            .camera_position(3.5, 60.0, 25.0);

        let state = Rc::new(RefCell::new(Surface3DState::new(
            config.camera_distance,
            config.camera_azimuth,
            config.camera_elevation,
        )));

        Self {
            data,
            config,
            state,
            dragging: false,
            panning: false,
            last_mouse: None,
        }
    }

    /// Create saddle surface (hyperbolic paraboloid)
    fn saddle_demo() -> Self {
        let data =
            SurfaceData::from_function((-2.0, 2.0), (-2.0, 2.0), 80, 80, |x, y| x * x - y * y)
                .with_x_label("X")
                .with_y_label("Y")
                .with_z_label("Z = X² - Y²");

        let config = Surface3DConfig::new()
            .colormap(Colormap::CoolWarm)
            .wireframe(true)
            .camera_position(4.5, 35.0, 25.0);

        let state = Rc::new(RefCell::new(Surface3DState::new(
            config.camera_distance,
            config.camera_azimuth,
            config.camera_elevation,
        )));

        Self {
            data,
            config,
            state,
            dragging: false,
            panning: false,
            last_mouse: None,
        }
    }
}

impl Render for Surface3DDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x1a1a1e))
            .child(
                Surface3DElement::new(self.data.clone(), self.config.clone())
                    .with_state(self.state.clone()),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, event: &MouseDownEvent, _window, _cx| {
                    if event.click_count == 2 {
                        // Double click - reset view
                        let mut state = view.state.borrow_mut();
                        state.controls.reset();
                        state.update_camera();
                    } else {
                        view.dragging = true;
                        view.last_mouse = Some(event.position);
                    }
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|view, event: &MouseDownEvent, _window, _cx| {
                    view.panning = true;
                    view.last_mouse = Some(event.position);
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|view, _event: &MouseUpEvent, _window, _cx| {
                    view.dragging = false;
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|view, _event: &MouseUpEvent, _window, _cx| {
                    view.panning = false;
                }),
            )
            .on_mouse_move(cx.listener(|view, event: &MouseMoveEvent, _window, cx| {
                if let Some(last) = view.last_mouse {
                    let dx: f32 = (event.position.x - last.x).into();
                    let dy: f32 = (event.position.y - last.y).into();

                    if view.dragging {
                        let mut state = view.state.borrow_mut();
                        state.controls.rotate(dx, dy);
                        state.update_camera();
                        cx.notify();
                    } else if view.panning {
                        let mut state = view.state.borrow_mut();
                        let camera_clone = state.camera.clone();
                        state.controls.pan(dx, dy, &camera_clone);
                        state.update_camera();
                        cx.notify();
                    }
                }

                if view.dragging || view.panning {
                    view.last_mouse = Some(event.position);
                }
            }))
            .on_scroll_wheel(cx.listener(|view, event: &ScrollWheelEvent, _window, cx| {
                let delta = match event.delta {
                    ScrollDelta::Lines(lines) => lines.y * 0.5,
                    ScrollDelta::Pixels(pixels) => {
                        let py: f32 = pixels.y.into();
                        py * 0.01
                    }
                };
                let mut state = view.state.borrow_mut();
                state.controls.zoom(delta);
                state.update_camera();
                cx.notify();
            }))
    }
}

fn main() {
    // Parse command line for demo selection
    let args: Vec<String> = std::env::args().collect();
    let demo_type = args.get(1).cloned().unwrap_or_else(|| "sinc".to_string());

    gpui::Application::new().run(move |cx| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::new(px(100.0), px(100.0)),
                    size: Size {
                        width: px(1000.0),
                        height: px(800.0),
                    },
                })),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from(format!(
                        "3D Surface Demo - {}",
                        &demo_type
                    ))),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| {
                let demo = match demo_type.as_str() {
                    "spinorama" => Surface3DDemo::spinorama_demo(),
                    "saddle" => Surface3DDemo::saddle_demo(),
                    _ => Surface3DDemo::new(),
                };
                cx.new(|_| demo)
            },
        )
        .unwrap();
    });
}
