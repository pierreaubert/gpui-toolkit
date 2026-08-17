//! Deterministic MeshPlot smoke fixtures for Sonium-style result viewers.
//!
//! The executable deliberately stays renderer-independent: it exercises the
//! public builder, validation, contour preparation, axisymmetric derivative,
//! accessibility summary, and SVG export without requiring an external
//! plotting viewer. Native component-lab and host runs cover the GPU paths.
//!
//! Set `GPUI_MESHPLOT_WINDOW=1` to open the same fixtures in a live GPUI
//! window with tabs for mesh-only, contours, axisymmetric section, revolve,
//! and Surface3d inspection.
use d3rs::mesh::{
    ContourLevels, CoordinateAxis, RevolveSpec, ScalarAssociation, ScalarField, TriangleMesh,
    revolve,
};
use gpui::{
    AppContext, Context, ElementId, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, div, px, rgb,
};
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_px::{
    Axes2d, ColorRange, ColorScale, Colorbar, FieldInterpolation, MeshPlotView, MeshRenderMode,
    Wireframe, mesh_plot,
};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DemoView {
    Mesh,
    Contours,
    AxisymmetricSection,
    Revolve,
    Surface3d,
}

impl DemoView {
    const ALL: [Self; 5] = [
        Self::Mesh,
        Self::Contours,
        Self::AxisymmetricSection,
        Self::Revolve,
        Self::Surface3d,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Mesh => "Mesh",
            Self::Contours => "Contours",
            Self::AxisymmetricSection => "r-z section",
            Self::Revolve => "Revolve",
            Self::Surface3d => "Surface 3D",
        }
    }
}

struct MeshPlotWindow {
    selected: DemoView,
}

impl MeshPlotWindow {
    fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            selected: DemoView::Mesh,
        }
    }

    fn render_plot(&self) -> gpui::AnyElement {
        match self.selected {
            DemoView::Mesh => mesh_plot(square_mesh("window-mesh"))
                .title("Unstructured mesh")
                .mode(MeshRenderMode::Mesh)
                .wireframe(Wireframe::overlay())
                .build()
                .expect("mesh-only MeshPlot window fixture")
                .into_any_element(),
            DemoView::Contours => mesh_plot(square_mesh("window-contours"))
                .field(vertex_field(
                    "window-pressure",
                    &[0.0, 0.5, 1.0, 0.25],
                    "Pressure",
                    "Pa",
                ))
                .title("Unstructured contour field")
                .mode(MeshRenderMode::FillAndIsolines {
                    levels: ContourLevels::Count(6),
                })
                .color_scale(ColorScale::Viridis)
                .color_range(ColorRange::Fixed { min: 0.0, max: 1.0 })
                .colorbar(Colorbar::new("Pressure").unit("Pa"))
                .wireframe(Wireframe::overlay())
                .build()
                .expect("contour MeshPlot window fixture")
                .into_any_element(),
            DemoView::AxisymmetricSection => {
                let (mesh, field) = axisymmetric_fixture();
                mesh_plot(mesh)
                    .field(field)
                    .view(MeshPlotView::AxisymmetricSection {
                        radial: CoordinateAxis::X,
                        axial: CoordinateAxis::Z,
                    })
                    .title("Axisymmetric temperature section")
                    .mode(MeshRenderMode::ScalarFill {
                        interpolation: FieldInterpolation::Smooth,
                    })
                    .colorbar(Colorbar::new("Temperature").unit("K"))
                    .build()
                    .expect("axisymmetric section MeshPlot window fixture")
                    .into_any_element()
            }
            DemoView::Revolve => {
                let (mesh, field) = axisymmetric_fixture();
                mesh_plot(mesh)
                    .field(field)
                    .view(MeshPlotView::AxisymmetricRevolve(RevolveSpec {
                        radial: CoordinateAxis::X,
                        axial: CoordinateAxis::Z,
                        segments: 24,
                        ..RevolveSpec::default()
                    }))
                    .title("Axisymmetric revolved surface")
                    .mode(MeshRenderMode::ScalarFill {
                        interpolation: FieldInterpolation::Smooth,
                    })
                    .colorbar(Colorbar::new("Temperature").unit("K"))
                    .wireframe(Wireframe::overlay())
                    .build()
                    .expect("revolve MeshPlot window fixture")
                    .into_any_element()
            }
            DemoView::Surface3d => mesh_plot(square_mesh("window-surface"))
                .field(vertex_field(
                    "window-surface-pressure",
                    &[0.0, 0.5, 1.0, 0.25],
                    "Pressure",
                    "Pa",
                ))
                .view(MeshPlotView::Surface3d)
                .title("Unstructured Surface3d")
                .mode(MeshRenderMode::ScalarFill {
                    interpolation: FieldInterpolation::Smooth,
                })
                .colorbar(Colorbar::new("Pressure").unit("Pa"))
                .wireframe(Wireframe::overlay())
                .build()
                .expect("Surface3d MeshPlot window fixture")
                .into_any_element(),
        }
    }
}

impl Render for MeshPlotWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected;
        let tabs = DemoView::ALL.into_iter().map(|view| {
            let active = view == selected;
            div()
                .id(ElementId::Name(view.label().into()))
                .px(px(12.0))
                .py(px(7.0))
                .rounded(px(5.0))
                .cursor_pointer()
                .bg(if active { rgb(0x2563eb) } else { rgb(0x1e293b) })
                .text_color(rgb(0xf8fafc))
                .child(view.label())
                .on_click(cx.listener(move |this, _, _window, cx| {
                    this.selected = view;
                    cx.notify();
                }))
        });

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x0f172a))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .p(px(12.0))
                    .child(
                        div()
                            .text_color(rgb(0xe2e8f0))
                            .child("MeshPlot live viewer"),
                    )
                    .children(tabs),
            )
            .child(div().flex_1().p(px(16.0)).child(self.render_plot()))
    }
}

fn run_window() {
    MiniApp::run(
        MiniAppConfig::new("MeshPlot live viewer")
            .size(1100.0, 760.0)
            .with_theme(true)
            .scrollable(false),
        |cx| cx.new(MeshPlotWindow::new),
    );
}

fn square_mesh(id: &str) -> TriangleMesh {
    TriangleMesh {
        id: id.into(),
        positions: Arc::from([
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.1],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, -0.1],
        ]),
        triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
        vertex_ids: Some(Arc::from([10, 11, 12, 13])),
        cell_ids: Some(Arc::from([100, 101])),
    }
}

fn vertex_field(id: &str, values: &[f64], label: &str, unit: &str) -> ScalarField {
    ScalarField {
        id: id.into(),
        label: label.into(),
        unit: Some(unit.into()),
        values: values.to_vec().into(),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}

fn axisymmetric_fixture() -> (TriangleMesh, ScalarField) {
    (
        TriangleMesh {
            id: "window-axisymmetric-profile".into(),
            positions: Arc::from([
                [0.0, 0.0, 0.0],
                [0.4, 0.0, 0.0],
                [0.4, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ]),
            triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
            vertex_ids: None,
            cell_ids: None,
        },
        vertex_field(
            "window-temperature",
            &[0.0, 0.4, 0.9, 0.2],
            "Temperature",
            "K",
        ),
    )
}

fn export(plot: gpui_px::MeshPlot, marker: &str) -> String {
    let summary = plot.accessibility_summary();
    assert_eq!(
        summary.chart_type, "mesh_plot",
        "{marker}: wrong chart type"
    );
    let svg = plot.to_svg().unwrap_or_else(|error| {
        panic!("{marker}: MeshPlot export failed: {error}");
    });
    assert!(svg.contains("<svg"), "{marker}: export is not SVG");
    svg
}

fn main() {
    if std::env::var_os("GPUI_MESHPLOT_WINDOW").is_some() {
        run_window();
        return;
    }

    // 2D BEM/FEM mesh inspection: topology and stable IDs are retained.
    let mesh = square_mesh("bem-like");
    let mesh_svg = export(
        mesh_plot(mesh.clone())
            .title("BEM mesh")
            .axes(Axes2d::equal_aspect().labels("x", "y").unit("m"))
            .wireframe(Wireframe::overlay())
            .mode(MeshRenderMode::Mesh),
        "mesh-only-2d",
    );
    assert!(mesh_svg.contains("BEM mesh"));

    // Unstructured scalar field-map result: contours and isolines stay on the
    // triangle topology rather than passing through a rectangular resampler.
    let field = vertex_field("pressure", &[0.0, 0.5, 1.0, 0.25], "Pressure", "Pa");
    let contour_svg = export(
        mesh_plot(mesh.clone())
            .field(field.clone())
            .title("Pressure field map")
            .mode(MeshRenderMode::FillAndIsolines {
                levels: ContourLevels::Count(6),
            })
            .color_scale(ColorScale::Viridis)
            .color_range(ColorRange::Fixed { min: 0.0, max: 1.0 })
            .colorbar(Colorbar::new("Pressure").unit("Pa"))
            .wireframe(Wireframe::overlay()),
        "field-map-contours",
    );
    assert!(contour_svg.contains("gpui-px-colorbar"));

    // Axisymmetric r-z profile and its retained revolved 3D derivative.
    let profile = TriangleMesh {
        id: "axisymmetric-profile".into(),
        positions: Arc::from([
            [0.0, 0.0, 0.0],
            [0.4, 0.0, 0.0],
            [0.4, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
        vertex_ids: None,
        cell_ids: None,
    };
    let profile_field = vertex_field("temperature", &[0.0, 0.4, 0.9, 0.2], "Temperature", "K");
    let revolve_spec = RevolveSpec {
        radial: CoordinateAxis::X,
        axial: CoordinateAxis::Z,
        segments: 16,
        ..RevolveSpec::default()
    };
    let revolved = revolve(&profile, &revolve_spec).expect("valid axisymmetric profile");
    assert!(!revolved.mesh.triangles.is_empty());
    let axisymmetric_svg = export(
        mesh_plot(profile.clone())
            .field(profile_field.clone())
            .view(MeshPlotView::AxisymmetricSection {
                radial: CoordinateAxis::X,
                axial: CoordinateAxis::Z,
            })
            .title("Axisymmetric temperature")
            .mode(MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            })
            .colorbar(Colorbar::new("Temperature").unit("K")),
        "axisymmetric-section",
    );
    assert!(axisymmetric_svg.contains("Axisymmetric temperature"));

    let revolve_svg = export(
        mesh_plot(profile.clone())
            .field(profile_field.clone())
            .view(MeshPlotView::AxisymmetricRevolve(revolve_spec.clone()))
            .title("Revolved temperature surface")
            .mode(MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            })
            .colorbar(Colorbar::new("Temperature").unit("K"))
            .wireframe(Wireframe::overlay()),
        "axisymmetric-revolve",
    );
    assert!(revolve_svg.contains("data-view=\"axisymmetric-revolve\""));
    assert!(revolve_svg.contains("gpui-px-mesh-3d-triangle"));

    // 3D BEM surface result: export metadata identifies the 3D view while
    // native GPU composition remains the responsibility of the live builder.
    let surface = mesh_plot(mesh)
        .field(field)
        .view(MeshPlotView::Surface3d)
        .title("BEM surface pressure")
        .mode(MeshRenderMode::ScalarFill {
            interpolation: FieldInterpolation::Smooth,
        })
        .colorbar(Colorbar::new("Pressure").unit("Pa"));
    let surface_png = surface.to_png(1.5).expect("Surface3d PNG export");
    assert_eq!(&surface_png[..8], b"\x89PNG\r\n\x1a\n");
    let surface_svg = export(surface, "surface-3d");
    assert!(surface_svg.contains("BEM surface pressure"));

    println!(
        "{}",
        concat!(
            "mesh_plot_showcase: 5 Sonium smoke scenarios passed ",
            "(2D mesh, contour field-map, axisymmetric section, revolve, 3D surface)"
        )
    );
}
