//! Native Metal window-level interaction coverage for the live MeshPlot.
//!
//! This lane is opt-in because ordinary unit-test hosts do not provide a GPU
//! adapter. Run it on macOS with:
//!
//! ```text
//! cargo test -p gpui-px --features native-qa --test mesh_plot_native
//! ```

#![cfg(all(target_os = "macos", feature = "native-qa"))]

use d3rs::gpu3d::Camera3D;
use d3rs::mesh::{CoordinateAxis, RevolveSpec, ScalarAssociation, ScalarField, TriangleMesh};
use glam::Vec3;
use gpui::{
    AnyWindowHandle, AppContext, Context, HeadlessAppContext, InputEvent, InteractiveElement,
    Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render,
    ScrollDelta, ScrollWheelEvent, Styled, TouchPhase, Window, div, point, px, size,
};
use gpui_macos::metal_renderer::MetalHeadlessRenderer;
use gpui_px::{
    FieldInterpolation, MeshPlotPick, MeshPlotState, MeshPlotView, MeshRenderMode, Wireframe,
    mesh_plot,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};

// macOS aborts when TIS/TSM keyboard-layout APIs are initialized from more
// than one Rust test thread at a time. Keep each scenario independently
// named while serializing the complete platform/window lifetime.
static NATIVE_PLATFORM_LOCK: Mutex<()> = Mutex::new(());

fn native_platform_lock() -> MutexGuard<'static, ()> {
    // Do not let an earlier assertion failure hide later native scenario
    // failures behind a poisoned test-serialization mutex.
    NATIVE_PLATFORM_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn native_text_system() -> Arc<dyn gpui::PlatformTextSystem> {
    // Native MeshPlot QA exercises Metal drawing and screenshot capture. A
    // deterministic GPUI text system avoids initializing macOS TIS/TSM
    // services, which can abort headless test processes before assertions.
    Arc::new(gpui::NoopTextSystem::new())
}

fn native_metal_available() -> bool {
    if MetalHeadlessRenderer::try_new().is_some() {
        true
    } else {
        eprintln!("native Metal QA skipped: no compatible Metal device");
        false
    }
}

struct NativeMeshPlotView {
    mesh: TriangleMesh,
    field: ScalarField,
    state: Rc<RefCell<MeshPlotState>>,
    selection: Rc<RefCell<Option<MeshPlotPick>>>,
    view: MeshPlotView,
    mode: MeshRenderMode,
    wireframe: Wireframe,
}

struct ResizableNativeMeshPlotView {
    mesh: TriangleMesh,
    field: ScalarField,
    state: Rc<RefCell<MeshPlotState>>,
    selection: Rc<RefCell<Option<MeshPlotPick>>>,
    size: Rc<RefCell<(f32, f32)>>,
}

impl Render for ResizableNativeMeshPlotView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let (width, height) = *self.size.borrow();
        let selection = self.selection.clone();
        let plot = mesh_plot(self.mesh.clone())
            .field(self.field.clone())
            .size(width, height)
            .view(MeshPlotView::Planar {
                horizontal: CoordinateAxis::X,
                vertical: CoordinateAxis::Y,
            })
            .mode(MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            })
            .with_state(self.state.clone())
            .on_selection(move |pick| {
                *selection.borrow_mut() = pick;
            })
            .build()
            .expect("resizable native MeshPlot should build");
        div().w(px(width)).h(px(height)).child(plot)
    }
}

impl Render for NativeMeshPlotView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let selection = self.selection.clone();
        let plot = mesh_plot(self.mesh.clone())
            .field(self.field.clone())
            .size(600.0, 400.0)
            .view(self.view.clone())
            .mode(self.mode.clone())
            .wireframe(self.wireframe)
            .with_state(self.state.clone())
            .on_selection(move |pick| {
                *selection.borrow_mut() = pick;
            })
            .build()
            .expect("native MeshPlot should build");
        div()
            .id("mesh-plot-native-selection")
            .w(px(600.0))
            .h(px(400.0))
            .child(plot)
    }
}

fn fixture() -> (TriangleMesh, ScalarField) {
    let mesh = TriangleMesh {
        id: "native-selection".into(),
        positions: Arc::from([
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
        vertex_ids: Some(Arc::from([10, 11, 12, 13])),
        cell_ids: Some(Arc::from([100, 101])),
    };
    let field = ScalarField {
        id: "native-field".into(),
        label: "Pressure".into(),
        unit: Some("Pa".into()),
        values: Arc::from([0.0, 0.5, 1.0, 0.25]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    (mesh, field)
}

/// A positive-radius profile with two connected cells. This specifically
/// exercises revolved mesh generation rather than merely the planar source
/// section used by the other native fixture.
fn revolve_fixture() -> (TriangleMesh, ScalarField) {
    let mesh = TriangleMesh {
        id: "native-revolve".into(),
        positions: Arc::from([
            [0.25, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.25, 1.0, 0.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
        vertex_ids: Some(Arc::from([20, 21, 22, 23])),
        cell_ids: Some(Arc::from([200, 201])),
    };
    let field = ScalarField {
        id: "native-revolve-field".into(),
        label: "Velocity".into(),
        unit: Some("m/s".into()),
        values: Arc::from([0.0, 0.25, 1.0, 0.75]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    (mesh, field)
}

/// A connected 10,000-triangle profile, exactly at the live asynchronous
/// revolve threshold. Shared vertices keep the fixture representative without
/// using the memory-heavy independent-triangle pattern.
fn large_revolve_fixture() -> (TriangleMesh, ScalarField) {
    const COLUMNS: usize = 100;
    const ROWS: usize = 50;
    let mut positions = Vec::with_capacity((COLUMNS + 1) * (ROWS + 1));
    let mut values = Vec::with_capacity((COLUMNS + 1) * (ROWS + 1));
    for row in 0..=ROWS {
        let y = row as f64 / ROWS as f64;
        for column in 0..=COLUMNS {
            let x = column as f64 / COLUMNS as f64;
            positions.push([0.25 + 0.75 * x, y, 0.0]);
            values.push(0.3 * x + 0.7 * y);
        }
    }
    let mut triangles = Vec::with_capacity(COLUMNS * ROWS * 2);
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let lower_left = (row * (COLUMNS + 1) + column) as u32;
            let lower_right = lower_left + 1;
            let upper_left = lower_left + (COLUMNS + 1) as u32;
            let upper_right = upper_left + 1;
            triangles.push([lower_left, lower_right, upper_right]);
            triangles.push([lower_left, upper_right, upper_left]);
        }
    }
    assert_eq!(triangles.len(), 10_000);
    (
        TriangleMesh {
            id: "native-large-revolve".into(),
            positions: positions.into(),
            triangles: triangles.into(),
            vertex_ids: None,
            cell_ids: None,
        },
        ScalarField {
            id: "native-large-revolve-field".into(),
            label: "Large profile scalar".into(),
            unit: None,
            values: values.into(),
            association: ScalarAssociation::Vertex,
            valid: None,
        },
    )
}

/// A triangle with one vertex in front of the configured near plane and two
/// visible vertices. Hardware clipping must retain the visible portion rather
/// than discarding the entire primitive.
fn near_clipped_fixture() -> (TriangleMesh, ScalarField) {
    let mesh = TriangleMesh {
        id: "native-near-clipped".into(),
        positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 1.95]]),
        triangles: Arc::from([[0, 1, 2]]),
        vertex_ids: None,
        cell_ids: None,
    };
    let field = ScalarField {
        id: "native-near-clipped-field".into(),
        label: "Near clip scalar".into(),
        unit: None,
        values: Arc::from([0.0, 1.0, 0.5]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    (mesh, field)
}

fn dispatch_click_at(
    cx: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    position: gpui::Point<gpui::Pixels>,
) {
    cx.update_window(window, |_, window, app| {
        window.dispatch_event(
            MouseDownEvent {
                position,
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            app,
        );
        window.dispatch_event(
            MouseUpEvent {
                position,
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: 1,
            }
            .to_platform_input(),
            app,
        );
    })
    .expect("dispatch native MeshPlot click");
    cx.run_until_parked();
}

fn dispatch_move(
    cx: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    position: gpui::Point<gpui::Pixels>,
) {
    cx.update_window(window, |_, window, app| {
        window.dispatch_event(
            MouseMoveEvent {
                position,
                pressed_button: None,
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
            app,
        );
    })
    .expect("dispatch native MeshPlot hover move");
    cx.run_until_parked();
}

fn dispatch_scroll(
    cx: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    position: gpui::Point<gpui::Pixels>,
    lines_y: f32,
) {
    cx.update_window(window, |_, window, app| {
        window.dispatch_event(
            ScrollWheelEvent {
                position,
                delta: ScrollDelta::Lines(point(0.0, lines_y)),
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            }
            .to_platform_input(),
            app,
        );
    })
    .expect("dispatch native MeshPlot scroll");
    cx.run_until_parked();
}

fn dispatch_click(cx: &mut HeadlessAppContext, window: AnyWindowHandle) {
    // A 600×400 equal-aspect plot of this square is letterboxed horizontally
    // by 100 px on either side. At this visible coordinate the correctly
    // inverted point belongs to the second triangle; treating the full 600px
    // width as data would incorrectly select the first triangle.
    let position = point(px(180.0), px(300.0));
    dispatch_click_at(cx, window, position);
}

fn close_window(cx: &mut HeadlessAppContext, window: AnyWindowHandle) {
    cx.update_window(window, |_, window, _app| window.remove_window())
        .expect("close native MeshPlot window");
    cx.run_until_parked();
}

#[test]
fn native_metal_mesh_plot_click_dispatches_selection_and_keyboard_preserves_it() {
    let _platform_guard = native_platform_lock();
    if !native_metal_available() {
        return;
    }
    let text_system = native_text_system();
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
        Some(Box::new(MetalHeadlessRenderer::new()))
    });

    let (mesh, field) = fixture();
    let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
    let selection = Rc::new(RefCell::new(None));
    let window = cx
        .open_window(size(px(600.0), px(400.0)), {
            let state = state.clone();
            let selection = selection.clone();
            move |_window, app| {
                app.new(|_cx| NativeMeshPlotView {
                    mesh,
                    field,
                    state,
                    selection,
                    view: MeshPlotView::Planar {
                        horizontal: d3rs::mesh::CoordinateAxis::X,
                        vertical: d3rs::mesh::CoordinateAxis::Y,
                    },
                    mode: MeshRenderMode::ScalarFill {
                        interpolation: FieldInterpolation::Smooth,
                    },
                    wireframe: Wireframe::hidden(),
                })
            }
        })
        .expect("open native MeshPlot window");
    let any_window: AnyWindowHandle = window.into();
    cx.update_window(any_window, |_, window, app| {
        let _ = window.draw(app);
    })
    .expect("initial native MeshPlot draw");

    dispatch_click(&mut cx, any_window);
    // Pointer focus is committed to the rendered dispatch tree during the
    // following prepaint. A real window performs that frame before the user
    // can press a key; reproduce the same boundary in this headless test.
    cx.run_until_parked();
    cx.update_window(any_window, |_, window, app| {
        window.draw(app).clear();
    })
    .expect("commit focused MeshPlot frame");
    let picked = selection
        .borrow()
        .clone()
        .expect("native click should emit a typed MeshPlotPick");
    assert_eq!(picked.plot_id.as_ref(), "native-selection");
    assert_eq!(picked.mesh_id.as_ref(), "native-selection");
    assert_eq!(picked.cell_id, Some(101));
    assert!(picked.vertex_id.is_some());
    assert!(picked.displayed_value.is_some());

    let before_domain = state.borrow().interaction.x_domain();
    cx.update_window(any_window, |_, window, app| {
        window.dispatch_keystroke(gpui::Keystroke::parse("=").unwrap(), app);
    })
    .expect("dispatch native MeshPlot keyboard zoom");
    cx.run_until_parked();
    let zoomed_domain = state.borrow().interaction.x_domain();
    assert!(
        zoomed_domain.1 - zoomed_domain.0 < before_domain.1 - before_domain.0,
        "keyboard zoom should shrink the viewport before testing a pan"
    );

    cx.update_window(any_window, |_, window, app| {
        window.dispatch_keystroke(gpui::Keystroke::parse("right").unwrap(), app);
    })
    .expect("dispatch native MeshPlot keyboard pan");
    cx.run_until_parked();
    let after_domain = state.borrow().interaction.x_domain();
    assert_ne!(
        zoomed_domain, after_domain,
        "keyboard pan should move a zoomed viewport"
    );
    assert_eq!(selection.borrow().as_ref(), Some(&picked));

    let screenshot = cx
        .capture_screenshot(any_window)
        .expect("capture native MeshPlot framebuffer");
    assert_eq!(screenshot.width(), 1200);
    assert_eq!(screenshot.height(), 800);
    let (min_luma, max_luma) = screenshot
        .pixels()
        .map(|pixel| u16::from(pixel.0[0]) + u16::from(pixel.0[1]) + u16::from(pixel.0[2]))
        .fold((u16::MAX, 0), |(min_luma, max_luma), luma| {
            (min_luma.min(luma), max_luma.max(luma))
        });
    assert!(
        max_luma > min_luma,
        "native MeshPlot framebuffer should contain rendered pixels"
    );
    close_window(&mut cx, any_window);
}

#[test]
fn native_metal_equal_aspect_selection_stays_aligned_after_resize() {
    let _platform_guard = native_platform_lock();
    if !native_metal_available() {
        return;
    }
    let text_system = native_text_system();
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
        Some(Box::new(MetalHeadlessRenderer::new()))
    });
    let (mesh, field) = fixture();
    let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
    let selection = Rc::new(RefCell::new(None));
    let layout = Rc::new(RefCell::new((600.0, 400.0)));
    let window = cx
        .open_window(size(px(600.0), px(400.0)), {
            let state = state.clone();
            let selection = selection.clone();
            let layout = layout.clone();
            move |_window, app| {
                app.new(|_cx| ResizableNativeMeshPlotView {
                    mesh,
                    field,
                    state,
                    selection,
                    size: layout,
                })
            }
        })
        .expect("open resizable native MeshPlot window");
    let view = cx
        .read_window(&window, |view, _| view)
        .expect("read resizable native MeshPlot view");
    let any_window: AnyWindowHandle = window.into();
    cx.update_window(any_window, |_, window, app| {
        let _ = window.draw(app);
    })
    .expect("draw initial equal-aspect MeshPlot");
    dispatch_move(&mut cx, any_window, point(px(180.0), px(300.0)));
    assert_eq!(
        state.borrow().hover.as_ref().and_then(|pick| pick.cell_id),
        Some(101)
    );
    dispatch_move(&mut cx, any_window, point(px(80.0), px(300.0)));
    assert!(state.borrow().hover.is_none(), "letterbox hover must clear");
    dispatch_scroll(&mut cx, any_window, point(px(80.0), px(300.0)), 1.0);
    let (x_min, x_max) = state.borrow().interaction.x_domain();
    assert!(
        x_min == 0.0 && x_max == 1.0,
        "wheel zoom in a letterbox bar must be ignored, got ({x_min}, {x_max})"
    );
    dispatch_scroll(&mut cx, any_window, point(px(180.0), px(300.0)), 1.0);
    let (x_min, x_max) = state.borrow().interaction.x_domain();
    assert!(
        (x_max - x_min - 0.9).abs() < 1e-6,
        "wheel zoom in the visible equal-aspect viewport must shrink by its factor, got ({x_min}, {x_max})"
    );
    dispatch_click_at(&mut cx, any_window, point(px(180.0), px(300.0)));
    assert_eq!(
        selection.borrow().as_ref().and_then(|pick| pick.cell_id),
        Some(101)
    );

    *selection.borrow_mut() = None;
    *layout.borrow_mut() = (800.0, 400.0);
    cx.update_entity(&view, |_view, cx| cx.notify());
    cx.update_window(any_window, |_, window, app| {
        window.resize(size(px(800.0), px(400.0)));
        window.draw(app).clear();
    })
    .expect("resize and draw equal-aspect MeshPlot");
    let screenshot = cx
        .capture_screenshot(any_window)
        .expect("capture resized equal-aspect MeshPlot");
    assert_eq!(screenshot.width(), 1600);
    assert_eq!(screenshot.height(), 800);
    // The resized plot's chart area is 730×360 with a 185px inner letterbox
    // offset. This window coordinate maps to the same second source cell;
    // full-rectangle inversion would land in the first one.
    dispatch_move(&mut cx, any_window, point(px(280.0), px(300.0)));
    assert_eq!(
        state.borrow().hover.as_ref().and_then(|pick| pick.cell_id),
        Some(101)
    );
    dispatch_move(&mut cx, any_window, point(px(100.0), px(300.0)));
    assert!(
        state.borrow().hover.is_none(),
        "resized letterbox hover must clear"
    );
    dispatch_click_at(&mut cx, any_window, point(px(280.0), px(300.0)));
    assert_eq!(
        selection.borrow().as_ref().and_then(|pick| pick.cell_id),
        Some(101)
    );
    drop(view);
    close_window(&mut cx, any_window);
}

#[test]
fn native_metal_surface3d_builds_the_dedicated_depth_and_triad_path() {
    let _platform_guard = native_platform_lock();
    if !native_metal_available() {
        return;
    }
    let text_system = native_text_system();
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
        Some(Box::new(MetalHeadlessRenderer::new()))
    });

    let (mesh, field) = fixture();
    let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
    let selection = Rc::new(RefCell::new(None));
    let window = cx
        .open_window(size(px(600.0), px(400.0)), {
            let state = state.clone();
            let selection = selection.clone();
            move |_window, app| {
                app.new(|_cx| NativeMeshPlotView {
                    mesh,
                    field,
                    state,
                    selection,
                    view: MeshPlotView::Surface3d,
                    mode: MeshRenderMode::ScalarFill {
                        interpolation: FieldInterpolation::Smooth,
                    },
                    wireframe: Wireframe::hidden(),
                })
            }
        })
        .expect("open native Surface3d window");
    let window: AnyWindowHandle = window.into();
    cx.update_window(window, |_, window, app| {
        let _ = window.draw(app);
    })
    .expect("draw native Surface3d");

    let screenshot = cx
        .capture_screenshot(window)
        .expect("capture native Surface3d framebuffer");
    assert_eq!(screenshot.width(), 1200);
    assert_eq!(screenshot.height(), 800);
    assert!(
        screenshot.pixels().any(|pixel| pixel.0[3] != 0),
        "dedicated Metal 3D draw should leave visible framebuffer pixels"
    );
    let colored_pixels = screenshot
        .pixels()
        .filter(|pixel| {
            let [r, g, b, _] = pixel.0;
            r.max(g).max(b).saturating_sub(r.min(g).min(b)) > 12
        })
        .count();
    assert!(
        colored_pixels > 200,
        "dedicated Metal 3D draw should produce a substantial scalar-colored surface, got {colored_pixels} pixels"
    );
    close_window(&mut cx, window);
}

#[test]
fn native_metal_surface3d_wireframe_changes_the_composited_frame() {
    let _platform_guard = native_platform_lock();
    if !native_metal_available() {
        return;
    }

    let render = |wireframe: Wireframe| {
        let text_system = native_text_system();
        let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
            Some(Box::new(MetalHeadlessRenderer::new()))
        });
        let (mesh, field) = fixture();
        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        let selection = Rc::new(RefCell::new(None));
        let window = cx
            .open_window(size(px(600.0), px(400.0)), {
                let state = state.clone();
                let selection = selection.clone();
                move |_window, app| {
                    app.new(|_cx| NativeMeshPlotView {
                        mesh,
                        field,
                        state,
                        selection,
                        view: MeshPlotView::Surface3d,
                        mode: MeshRenderMode::ScalarFill {
                            interpolation: FieldInterpolation::Smooth,
                        },
                        wireframe,
                    })
                }
            })
            .expect("open native Surface3d wireframe window");
        let window: AnyWindowHandle = window.into();
        cx.update_window(window, |_, window, app| {
            let _ = window.draw(app);
        })
        .expect("draw native Surface3d wireframe frame");
        let screenshot = cx
            .capture_screenshot(window)
            .expect("capture native Surface3d wireframe framebuffer");
        let pixels = screenshot.pixels().map(|pixel| pixel.0).collect::<Vec<_>>();
        close_window(&mut cx, window);
        pixels
    };

    let without_wireframe = render(Wireframe::hidden());
    let with_wireframe = render(Wireframe::overlay());
    assert_eq!(with_wireframe.len(), without_wireframe.len());
    let changed_pixels = with_wireframe
        .iter()
        .zip(&without_wireframe)
        .filter(|(wireframe, fill)| wireframe != fill)
        .count();
    assert!(
        changed_pixels > 100,
        "wireframe overlay should visibly alter the native 3D frame, got {changed_pixels} changed pixels"
    );
}

#[test]
fn native_metal_surface3d_keeps_a_partially_near_clipped_triangle() {
    let _platform_guard = native_platform_lock();
    if !native_metal_available() {
        return;
    }
    let text_system = native_text_system();
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
        Some(Box::new(MetalHeadlessRenderer::new()))
    });
    let (mesh, field) = near_clipped_fixture();
    let mut initial_state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
    let mut camera = Camera3D::new()
        .with_position(Vec3::new(0.5, 0.5, 2.0))
        .with_target(Vec3::new(0.5, 0.5, 0.0))
        .with_aspect(1.5)
        .with_fov_degrees(70.0);
    camera.near = 0.2;
    initial_state.camera = camera.clone();
    initial_state.camera_fitted = true;
    assert!(
        camera
            .project_to_screen(Vec3::new(0.5, 1.0, 1.95), 600.0, 400.0)
            .is_none(),
        "fixture vertex must be in front of the camera near plane"
    );
    assert!(
        camera
            .project_to_screen(Vec3::new(0.0, 0.0, 0.0), 600.0, 400.0)
            .is_some(),
        "fixture must retain visible vertices"
    );
    let state = Rc::new(RefCell::new(initial_state));
    let selection = Rc::new(RefCell::new(None));
    let window = cx
        .open_window(size(px(600.0), px(400.0)), {
            let state = state.clone();
            let selection = selection.clone();
            move |_window, app| {
                app.new(|_cx| NativeMeshPlotView {
                    mesh,
                    field,
                    state,
                    selection,
                    view: MeshPlotView::Surface3d,
                    mode: MeshRenderMode::ScalarFill {
                        interpolation: FieldInterpolation::Smooth,
                    },
                    wireframe: Wireframe::overlay(),
                })
            }
        })
        .expect("open native near-clipped Surface3d window");
    let window: AnyWindowHandle = window.into();
    cx.update_window(window, |_, window, app| {
        let _ = window.draw(app);
    })
    .expect("draw native near-clipped Surface3d");
    let screenshot = cx
        .capture_screenshot(window)
        .expect("capture native near-clipped Surface3d framebuffer");
    let colored_pixels = screenshot
        .pixels()
        .filter(|pixel| {
            let [r, g, b, _] = pixel.0;
            r.max(g).max(b).saturating_sub(r.min(g).min(b)) > 12
        })
        .count();
    assert!(
        colored_pixels > 100,
        "partially near-clipped native triangle should retain a visible colored region, got {colored_pixels} pixels"
    );
    close_window(&mut cx, window);
}

#[test]
fn native_metal_large_revolve_shows_preparing_frame_then_completed_surface() {
    let _platform_guard = native_platform_lock();
    if !native_metal_available() {
        return;
    }
    let text_system = native_text_system();
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
        Some(Box::new(MetalHeadlessRenderer::new()))
    });
    let (mesh, field) = large_revolve_fixture();
    let state = Rc::new(RefCell::new(MeshPlotState::new(0.25, 1.0, 0.0, 1.0)));
    let selection = Rc::new(RefCell::new(None));
    let window = cx
        .open_window(size(px(600.0), px(400.0)), {
            let state = state.clone();
            let selection = selection.clone();
            move |_window, app| {
                app.new(|_cx| NativeMeshPlotView {
                    mesh,
                    field,
                    state,
                    selection,
                    view: MeshPlotView::AxisymmetricRevolve(RevolveSpec {
                        radial: CoordinateAxis::X,
                        axial: CoordinateAxis::Y,
                        start_angle: 0.0,
                        sweep_angle: std::f64::consts::TAU,
                        segments: 12,
                        end_caps: false,
                    }),
                    mode: MeshRenderMode::ScalarFill {
                        interpolation: FieldInterpolation::Smooth,
                    },
                    wireframe: Wireframe::hidden(),
                })
            }
        })
        .expect("open native large revolved MeshPlot window");
    let window: AnyWindowHandle = window.into();
    cx.update_window(window, |_, window, app| {
        let _ = window.draw(app);
    })
    .expect("draw native large-revolve preparing frame");
    let preparing = cx
        .capture_screenshot(window)
        .expect("capture native preparing framebuffer");
    let badge_color = [0x30, 0x34, 0x3b];
    let badge_pixels = preparing
        .pixels()
        .take(160 * preparing.width() as usize)
        .filter(|pixel| pixel.0[..3] == badge_color)
        .count();
    assert!(
        badge_pixels > 100,
        "first large-revolve frame should contain the Preparing 3D surface badge, got {badge_pixels} badge pixels"
    );

    // The deterministic headless dispatcher runs both the background derive
    // and its retained-view completion task. The following draw observes the
    // accepted prepared geometry rather than synchronously deriving it.
    cx.run_until_parked();
    cx.update_window(window, |_, window, app| {
        window.draw(app).clear();
    })
    .expect("draw completed native large-revolve frame");
    let completed = cx
        .capture_screenshot(window)
        .expect("capture completed native large-revolve framebuffer");
    let completed_badge_pixels = completed
        .pixels()
        .take(160 * completed.width() as usize)
        .filter(|pixel| pixel.0[..3] == badge_color)
        .count();
    assert_eq!(
        completed_badge_pixels, 0,
        "completed large-revolve frame must remove the preparing badge"
    );
    let colored_pixels = completed
        .pixels()
        .filter(|pixel| {
            let [r, g, b, _] = pixel.0;
            r.max(g).max(b).saturating_sub(r.min(g).min(b)) > 12
        })
        .count();
    assert!(
        colored_pixels > 200,
        "completed large-revolve frame should contain a scalar-colored surface, got {colored_pixels} pixels"
    );
    close_window(&mut cx, window);
}

#[test]
fn native_metal_full_and_partial_revolves_build_depth_tested_frames() {
    let _platform_guard = native_platform_lock();
    if !native_metal_available() {
        return;
    }
    let (mesh, vertex_field) = revolve_fixture();
    let cell_field = ScalarField {
        id: "native-revolve-cell-field".into(),
        label: "Cell velocity".into(),
        unit: Some("m/s".into()),
        values: Arc::from([0.25, 0.75]),
        association: ScalarAssociation::Cell,
        valid: None,
    };
    let views = [
        (
            MeshPlotView::AxisymmetricRevolve(RevolveSpec {
                radial: CoordinateAxis::X,
                axial: CoordinateAxis::Y,
                start_angle: 0.0,
                sweep_angle: std::f64::consts::TAU,
                segments: 32,
                end_caps: false,
            }),
            vertex_field,
            MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            },
        ),
        (
            MeshPlotView::AxisymmetricRevolve(RevolveSpec {
                radial: CoordinateAxis::X,
                axial: CoordinateAxis::Y,
                start_angle: -std::f64::consts::FRAC_PI_2,
                sweep_angle: std::f64::consts::PI,
                segments: 24,
                end_caps: true,
            }),
            cell_field,
            MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Flat,
            },
        ),
    ];

    for (index, (view, field, mode)) in views.into_iter().enumerate() {
        let text_system = native_text_system();
        let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
            Some(Box::new(MetalHeadlessRenderer::new()))
        });
        let state = Rc::new(RefCell::new(MeshPlotState::new(0.25, 1.0, 0.0, 1.0)));
        let selection = Rc::new(RefCell::new(None));
        let window = cx
            .open_window(size(px(600.0), px(400.0)), {
                let state = state.clone();
                let selection = selection.clone();
                let mesh = mesh.clone();
                move |_window, app| {
                    app.new(|_cx| NativeMeshPlotView {
                        mesh,
                        field,
                        state,
                        selection,
                        view,
                        mode,
                        wireframe: Wireframe::hidden(),
                    })
                }
            })
            .expect("open native revolved MeshPlot window");
        let window: AnyWindowHandle = window.into();
        cx.update_window(window, |_, window, app| {
            let _ = window.draw(app);
        })
        .expect("draw native revolved MeshPlot");
        let screenshot = cx
            .capture_screenshot(window)
            .expect("capture native revolved MeshPlot framebuffer");
        assert_eq!(screenshot.width(), 1200, "revolve case {index}");
        assert_eq!(screenshot.height(), 800, "revolve case {index}");
        assert!(
            screenshot.pixels().any(|pixel| pixel.0[3] != 0),
            "revolve case {index} should leave visible framebuffer pixels"
        );
        let colored_pixels = screenshot
            .pixels()
            .filter(|pixel| {
                let [r, g, b, _] = pixel.0;
                r.max(g).max(b).saturating_sub(r.min(g).min(b)) > 12
            })
            .count();
        assert!(
            colored_pixels > 200,
            "revolve case {index} should produce a substantial scalar-colored surface, got {colored_pixels} pixels"
        );
        close_window(&mut cx, window);
    }
}

#[test]
fn native_metal_live_3d_state_exports_deterministic_surface_and_revolve_artifacts() {
    use image::GenericImageView;

    let _platform_guard = native_platform_lock();
    if !native_metal_available() {
        return;
    }
    let (surface_mesh, surface_field) = fixture();
    let (revolve_mesh, revolve_field) = revolve_fixture();
    let cases = [
        (
            surface_mesh,
            surface_field,
            MeshPlotView::Surface3d,
            MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            },
        ),
        (
            revolve_mesh,
            revolve_field,
            MeshPlotView::AxisymmetricRevolve(RevolveSpec {
                radial: CoordinateAxis::X,
                axial: CoordinateAxis::Y,
                start_angle: 0.0,
                sweep_angle: std::f64::consts::TAU,
                segments: 32,
                end_caps: false,
            }),
            MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            },
        ),
    ];

    for (index, (mesh, field, view, mode)) in cases.into_iter().enumerate() {
        let text_system = native_text_system();
        let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
            Some(Box::new(MetalHeadlessRenderer::new()))
        });
        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        let selection = Rc::new(RefCell::new(None));
        let window = cx
            .open_window(size(px(600.0), px(400.0)), {
                let state = state.clone();
                let selection = selection.clone();
                let live_mesh = mesh.clone();
                let live_field = field.clone();
                let live_view = view.clone();
                let live_mode = mode.clone();
                move |_window, app| {
                    app.new(|_cx| NativeMeshPlotView {
                        mesh: live_mesh,
                        field: live_field,
                        state,
                        selection,
                        view: live_view,
                        mode: live_mode,
                        wireframe: Wireframe::hidden(),
                    })
                }
            })
            .expect("open native 3D export-parity MeshPlot window");
        let entity = cx
            .read_window(&window, |view, _| view)
            .expect("read native 3D export-parity MeshPlot entity");
        let any_window: AnyWindowHandle = window.into();
        cx.update_window(any_window, |_, window, app| {
            let _ = window.draw(app);
        })
        .expect("draw native 3D export-parity MeshPlot");

        // The live renderer fits a camera for the first frame. Rotate it so
        // the export must consume the retained live camera, not a default.
        state.borrow_mut().orbit_rotate(13.0, -7.0);
        cx.update_entity(&entity, |_view, cx| cx.notify());
        cx.update_window(any_window, |_, window, app| {
            window.draw(app).clear();
        })
        .expect("redraw rotated native 3D export-parity MeshPlot");
        let live = cx
            .capture_screenshot(any_window)
            .expect("capture native 3D export-parity frame");
        assert_eq!(live.dimensions(), (1200, 800), "3D export case {index}");
        let live_colored = live
            .pixels()
            .filter(|pixel| {
                let [r, g, b, _] = pixel.0;
                r.max(g).max(b).saturating_sub(r.min(g).min(b)) > 12
            })
            .count();
        let stats = state
            .borrow()
            .retained_3d_stats()
            .expect("native 3D export case must retain 3D renderer stats");
        let gpu_uploads = stats.gpu_geometry_upload_count;
        let gpu_resident_bytes = stats.gpu_resident_bytes;
        assert!(
            gpu_uploads > 0 && gpu_resident_bytes > 0,
            "native 3D export case {index} must dispatch an adapter-backed upload (uploads={gpu_uploads}, resident_bytes={gpu_resident_bytes})"
        );
        assert!(
            stats.gpu_geometry_upload_time_ns > 0,
            "native 3D export case {index} must record adapter geometry submission time"
        );
        assert!(
            stats.gpu_frame_count > 0 && stats.gpu_frame_time_ns > 0,
            "native 3D export case {index} must record retained frame timing"
        );
        assert!(
            live_colored > 200,
            "native 3D export case {index} must produce a scalar-coloured live frame"
        );

        // Camera-only navigation must reuse the retained position/normal/
        // index resource and avoid a scalar rewrite as well.
        let camera_geometry_uploads = stats.gpu_geometry_upload_count;
        let camera_field_writes = stats.gpu_field_write_count;
        state.borrow_mut().orbit_rotate(-5.0, 3.0);
        cx.update_entity(&entity, |_view, cx| cx.notify());
        cx.update_window(any_window, |_, window, app| {
            window.draw(app).clear();
        })
        .expect("redraw camera-only native 3D MeshPlot");
        let _camera_frame = cx
            .capture_screenshot(any_window)
            .expect("capture camera-only native 3D MeshPlot");
        let camera_stats = state
            .borrow()
            .retained_3d_stats()
            .expect("camera-only native 3D plot must retain renderer stats");
        assert_eq!(
            camera_stats.gpu_geometry_upload_count, camera_geometry_uploads,
            "camera-only native 3D navigation must not upload geometry"
        );
        assert_eq!(
            camera_stats.gpu_field_write_count, camera_field_writes,
            "camera-only native 3D navigation must not rewrite scalar data"
        );
        assert!(
            camera_stats.gpu_frame_count > stats.gpu_frame_count,
            "camera-only native 3D navigation must record a retained frame"
        );

        // A field-only patch must update the dedicated scalar buffer while
        // preserving the retained position/normal/index geometry resource.
        let geometry_uploads_before_field_patch = stats.gpu_geometry_upload_count;
        let field_writes_before_field_patch = stats.gpu_field_write_count;
        let field_bytes_before_field_patch = stats.gpu_field_write_bytes;
        let updated_field = ScalarField {
            id: field.id.clone(),
            label: field.label.clone(),
            unit: field.unit.clone(),
            values: Arc::from([0.9, 0.2, 0.8, 0.1]),
            association: field.association,
            valid: field.valid.clone(),
        };
        cx.update_entity(&entity, |view, cx| {
            view.field = updated_field;
            cx.notify();
        });
        cx.update_window(any_window, |_, window, app| {
            window.draw(app).clear();
        })
        .expect("redraw field-patched native 3D MeshPlot");
        let _patched = cx
            .capture_screenshot(any_window)
            .expect("capture field-patched native 3D MeshPlot");
        let patched_stats = state
            .borrow()
            .retained_3d_stats()
            .expect("field-patched native 3D plot must retain renderer stats");
        assert_eq!(
            patched_stats.gpu_geometry_upload_count, geometry_uploads_before_field_patch,
            "field-only native 3D patch must not upload geometry"
        );
        assert!(
            patched_stats.gpu_field_write_count > field_writes_before_field_patch,
            "field-only native 3D patch must write the scalar buffer (before={}, after={}, geometry_uploads={})",
            field_writes_before_field_patch,
            patched_stats.gpu_field_write_count,
            patched_stats.gpu_geometry_upload_count,
        );
        assert!(
            patched_stats.gpu_field_write_bytes > field_bytes_before_field_patch,
            "field-only native 3D patch must report scalar bytes"
        );
        assert!(
            patched_stats.gpu_field_write_time_ns > 0,
            "field-only native 3D patch must record scalar submission time"
        );

        let before = {
            let state = state.borrow();
            (
                state.geometry_revision,
                state.field_revision,
                state.selection.clone(),
                state.hover.clone(),
                state.camera.view_projection_matrix().to_cols_array(),
            )
        };
        let plot = mesh_plot(mesh)
            .field(field)
            .size(600.0, 400.0)
            .view(view)
            .mode(mode)
            .wireframe(Wireframe::hidden())
            .with_state(state.clone());
        let svg = plot.to_svg().expect("export live 3D SVG");
        assert!(svg.contains("data-camera=\"current\""));
        assert!(svg.contains("gpui-px-mesh-3d-triangle"));
        assert!(svg.contains("gpui-px-mesh-3d-axes"));
        assert_eq!(
            svg,
            plot.to_svg().expect("re-export deterministic live 3D SVG"),
            "3D export case {index}"
        );
        let png = plot.to_png(1.0).expect("export live 3D PNG");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let decoded = image::load_from_memory(&png).expect("decode exported 3D PNG");
        assert_eq!(decoded.dimensions(), (600, 400), "3D export case {index}");
        let exported = decoded.to_rgba8();
        let exported_colored = exported
            .pixels()
            .filter(|pixel| {
                let [r, g, b, a] = pixel.0;
                a != 0 && r.max(g).max(b).saturating_sub(r.min(g).min(b)) > 12
            })
            .count();
        assert!(
            exported_colored > 50,
            "native/export parity case {index} must contain scalar-coloured PNG pixels"
        );
        // The Metal framebuffer and deterministic export use different
        // rasterizers and layout composition. Verify the shared plot viewport
        // is populated independently; exact cross-adapter silhouette parity
        // remains a release QA gate rather than a unit-test contract.
        let native_scale = live.width() / 600;
        let native_plot = image::imageops::crop_imm(
            &live,
            50 * native_scale,
            10 * native_scale,
            530 * native_scale,
            360 * native_scale,
        )
        .to_image();
        let native_plot_colored = native_plot
            .pixels()
            .filter(|pixel| {
                let [r, g, b, a] = pixel.0;
                a != 0 && r.max(g).max(b).saturating_sub(r.min(g).min(b)) > 12
            })
            .count();
        assert!(
            native_plot_colored > 200,
            "native plot viewport case {index} must contain scalar-coloured pixels, got {native_plot_colored}"
        );
        assert_eq!(
            png,
            plot.to_png(1.0)
                .expect("re-export deterministic live 3D PNG"),
            "3D export case {index}"
        );

        let state = state.borrow();
        assert_eq!(state.geometry_revision, before.0);
        assert_eq!(state.field_revision, before.1);
        assert_eq!(state.selection, before.2);
        assert_eq!(state.hover, before.3);
        assert_eq!(
            state.camera.view_projection_matrix().to_cols_array(),
            before.4,
            "export must not change the live 3D camera for case {index}"
        );
        drop(state);
        drop(entity);
        close_window(&mut cx, any_window);
    }
}
