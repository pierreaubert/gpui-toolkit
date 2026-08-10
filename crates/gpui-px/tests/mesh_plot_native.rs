//! Native Metal window-level interaction coverage for the live MeshPlot.
//!
//! This lane is opt-in because ordinary unit-test hosts do not provide a GPU
//! adapter. Run it on macOS with:
//!
//! ```text
//! cargo test -p gpui-px --features native-qa --test mesh_plot_native
//! ```

#![cfg(all(target_os = "macos", feature = "native-qa"))]

use d3rs::mesh::{CoordinateAxis, RevolveSpec, ScalarAssociation, ScalarField, TriangleMesh};
use gpui::{
    AnyWindowHandle, AppContext, Context, HeadlessAppContext, InputEvent, InteractiveElement,
    Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, ParentElement, Platform, Render, Styled,
    Window, div, point, px, size,
};
use gpui_macos::{MacPlatform, metal_renderer::MetalHeadlessRenderer};
use gpui_px::{
    FieldInterpolation, MeshPlotPick, MeshPlotState, MeshPlotView, MeshRenderMode, mesh_plot,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

struct NativeMeshPlotView {
    mesh: TriangleMesh,
    field: ScalarField,
    state: Rc<RefCell<MeshPlotState>>,
    selection: Rc<RefCell<Option<MeshPlotPick>>>,
    view: MeshPlotView,
    mode: MeshRenderMode,
}

impl Render for NativeMeshPlotView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let selection = self.selection.clone();
        let plot = mesh_plot(self.mesh.clone())
            .field(self.field.clone())
            .size(600.0, 400.0)
            .view(self.view.clone())
            .mode(self.mode.clone())
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

fn dispatch_click(cx: &mut HeadlessAppContext, window: AnyWindowHandle) {
    let position = point(px(300.0), px(200.0));
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

#[test]
fn native_metal_mesh_plot_click_dispatches_selection_and_keyboard_preserves_it() {
    let platform = MacPlatform::new(true);
    let text_system = platform.text_system();
    drop(platform);
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
    let picked = selection
        .borrow()
        .clone()
        .expect("native click should emit a typed MeshPlotPick");
    assert_eq!(picked.plot_id.as_ref(), "native-selection");
    assert_eq!(picked.mesh_id.as_ref(), "native-selection");
    assert!(picked.cell_id.is_some());
    assert!(picked.vertex_id.is_some());
    assert!(picked.displayed_value.is_some());

    let before_domain = state.borrow().interaction.x_domain();
    cx.update_window(any_window, |_, window, app| {
        window.dispatch_keystroke(gpui::Keystroke::parse("right").unwrap(), app);
    })
    .expect("dispatch native MeshPlot keyboard pan");
    cx.run_until_parked();
    let after_domain = state.borrow().interaction.x_domain();
    assert_ne!(
        before_domain, after_domain,
        "keyboard pan should move the viewport"
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
}

#[test]
fn native_metal_surface3d_builds_the_dedicated_depth_and_triad_path() {
    let platform = MacPlatform::new(true);
    let text_system = platform.text_system();
    drop(platform);
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
}

#[test]
fn native_metal_full_and_partial_revolves_build_depth_tested_frames() {
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
        let platform = MacPlatform::new(true);
        let text_system = platform.text_system();
        drop(platform);
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
    }
}
