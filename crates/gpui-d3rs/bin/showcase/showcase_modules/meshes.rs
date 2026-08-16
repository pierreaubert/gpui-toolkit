//! Mesh feature showcase.
//!
//! The examples intentionally use the real retained mesh upload path and the
//! deterministic offscreen renderer. This keeps the showcase useful in
//! snapshots while exercising the same mesh model used by native backends.

use super::ShowcaseApp;
use d3rs::mesh::gpu::{MeshColorConfig, MeshSceneState};
use d3rs::mesh::{
    MeshTopology, RevolveSpec, ScalarAssociation, ScalarField, TriangleMesh, prepare_field,
    prepare_upload, revolve,
};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

struct MeshExample {
    state: Rc<RefCell<MeshSceneState>>,
}

impl MeshExample {
    fn new(mesh: TriangleMesh, field: Option<ScalarField>, color: MeshColorConfig) -> Self {
        let state = Rc::new(RefCell::new(MeshSceneState {
            color,
            ..MeshSceneState::default()
        }));
        let topology = MeshTopology::build(&mesh.triangles);
        let mut upload = prepare_upload(&mesh, &topology);
        if let Some(field) = field {
            let values = prepare_field(&field);
            match field.association {
                ScalarAssociation::Vertex => upload.values_f32 = Some(values),
                ScalarAssociation::Cell => upload.cell_values_f32 = Some(values),
            }
        }
        {
            let mut retained = state.borrow_mut();
            retained.geometry_rev = d3rs::mesh::gpu::GeometryRevision(1);
            retained.field_rev = d3rs::mesh::gpu::FieldRevision(1);
            retained.geometry_upload_count = 1;
            retained.geometry_upload_bytes = upload.geometry_byte_len();
            retained.upload = Some(upload);
        }
        Self { state }
    }

    fn element(&self) -> impl IntoElement {
        d3rs::mesh::gpu::MeshSceneElement::new(self.state.clone())
    }

    fn set_camera(&self, camera: &d3rs::surface::SurfaceCamera) {
        let angle = (camera.camera.rotation_z as f32).to_radians();
        let scale = camera.camera.zoom as f32;
        let (sin, cos) = angle.sin_cos();
        self.state.borrow_mut().view_transform = [
            [scale * cos, -scale * sin, 0.0, 0.0],
            [scale * sin, scale * cos, 0.0, 0.0],
            [0.0, 0.0, scale, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
    }
}

fn scalar_mesh() -> TriangleMesh {
    // An intentionally irregular plate: unlike a regular surface grid, this
    // makes the unstructured-triangle nature of the API visible immediately.
    let positions = vec![
        [-1.0, -1.0, 0.0],
        [-0.15, -1.05, 0.12],
        [0.8, -0.9, -0.05],
        [-1.1, -0.2, 0.18],
        [-0.25, -0.1, 0.42],
        [0.95, -0.15, 0.1],
        [-0.9, 0.75, -0.08],
        [0.05, 0.9, 0.28],
        [1.05, 0.72, -0.12],
        [-0.35, 1.18, 0.02],
    ];
    let triangles = vec![
        [0, 1, 4],
        [0, 4, 3],
        [1, 2, 5],
        [1, 5, 4],
        [3, 4, 6],
        [4, 7, 6],
        [4, 5, 7],
        [5, 8, 7],
        [6, 7, 9],
        [7, 8, 9],
    ];
    TriangleMesh {
        id: Arc::from("irregular-plate"),
        positions: positions.into(),
        triangles: triangles.into(),
        vertex_ids: Some((100..110).collect::<Vec<_>>().into()),
        cell_ids: Some((200..210).collect::<Vec<_>>().into()),
    }
}

fn scalar_field(mesh: &TriangleMesh) -> ScalarField {
    let values = mesh
        .positions
        .iter()
        .map(|p| (p[0] * 2.1).sin() * 0.65 + (p[1] * 1.7).cos() * 0.35 + p[2])
        .collect::<Vec<_>>();
    ScalarField {
        id: Arc::from("temperature"),
        label: Arc::from("Temperature"),
        unit: Some(Arc::from("°C")),
        values: values.into(),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}

fn cell_field(mesh: &TriangleMesh) -> ScalarField {
    ScalarField {
        id: Arc::from("stress"),
        label: Arc::from("Cell stress"),
        unit: Some(Arc::from("MPa")),
        values: (0..mesh.triangles.len())
            .map(|index| 20.0 + (index as f64 * 0.9).sin() * 12.0)
            .collect::<Vec<_>>()
            .into(),
        association: ScalarAssociation::Cell,
        valid: Some(
            (0..mesh.triangles.len())
                .map(|index| index != 4)
                .collect::<Vec<_>>()
                .into(),
        ),
    }
}

fn revolved_profile() -> TriangleMesh {
    let profile = TriangleMesh {
        id: Arc::from("axisymmetric-profile"),
        positions: vec![
            [0.0, 0.0, -1.0],
            [0.72, 0.0, -1.0],
            [0.88, 0.0, 0.0],
            [0.55, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ]
        .into(),
        triangles: vec![[0, 1, 2], [0, 2, 3], [0, 3, 4]].into(),
        vertex_ids: None,
        cell_ids: None,
    };
    revolve(
        &profile,
        &RevolveSpec {
            segments: 32,
            ..RevolveSpec::default()
        },
    )
    .expect("showcase revolve profile is valid")
    .mesh
}

fn example(
    mesh: TriangleMesh,
    field: Option<ScalarField>,
    wireframe: bool,
    isolines: f32,
) -> MeshExample {
    MeshExample::new(
        mesh,
        field,
        MeshColorConfig {
            colormap: 0,
            range: [-1.5, 1.5],
            wireframe,
            isoline_step: isolines,
            isoline_width_px: 1.0,
            unlit: true,
        },
    )
}

fn card(
    title: &'static str,
    description: &'static str,
    caption: &'static str,
    plot: Div,
    theme: &gpui_ui_kit::theme::Theme,
) -> Div {
    div()
        .flex_1()
        .min_w(px(280.0))
        .bg(theme.surface)
        .border_1()
        .border_color(theme.border)
        .rounded_lg()
        .p_4()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.text_muted)
                .child(description),
        )
        .child(
            div()
                .mt_2()
                .h(px(220.0))
                .bg(theme.background)
                .rounded_md()
                .overflow_hidden()
                .child(plot),
        )
        .child(div().text_xs().text_color(theme.text_muted).child(caption))
}

fn interactive_mesh_plot(
    app: &mut ShowcaseApp,
    cx: &mut Context<ShowcaseApp>,
    example: &MeshExample,
    plot_index: usize,
) -> Div {
    example.set_camera(&app.mesh_plot_camera);
    div()
        .size_full()
        .cursor_crosshair()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                this.mesh_plot_drag = Some((plot_index, event.position));
            }),
        )
        .on_mouse_move(
            cx.listener(move |this, event: &MouseMoveEvent, _window, cx| {
                let Some((active_index, previous)) = this.mesh_plot_drag else {
                    return;
                };
                if active_index != plot_index {
                    return;
                }
                cx.stop_propagation();
                let dx: f64 = (event.position.x - previous.x).into();
                let dy: f64 = (event.position.y - previous.y).into();
                this.mesh_plot_camera.apply_drag(dx, dy);
                this.mesh_plot_drag = Some((plot_index, event.position));
                cx.notify();
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |this, _event, _window, cx| {
                if this
                    .mesh_plot_drag
                    .is_some_and(|(active_index, _)| active_index == plot_index)
                {
                    cx.stop_propagation();
                    this.mesh_plot_drag = None;
                }
            }),
        )
        .on_scroll_wheel(
            cx.listener(move |this, event: &ScrollWheelEvent, _window, cx| {
                cx.stop_propagation();
                let delta_y: f32 = match event.delta {
                    ScrollDelta::Lines(lines) => lines.y,
                    ScrollDelta::Pixels(pixels) => pixels.y.into(),
                };
                this.mesh_plot_camera
                    .apply_scroll(f64::from(delta_y) / 50.0);
                cx.notify();
            }),
        )
        .child(example.element())
}

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let theme = cx.theme();
    let mesh = scalar_mesh();
    let vertex_values = scalar_field(&mesh);
    let cell_values = cell_field(&mesh);
    let scalar = example(mesh.clone(), Some(vertex_values.clone()), true, 0.0);
    let modes = example(mesh.clone(), Some(vertex_values), false, 0.35);
    let axisymmetric = example(revolved_profile(), None, true, 0.0);
    let inspection = example(mesh.clone(), Some(cell_values), true, 0.0);

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(div().text_2xl().font_weight(FontWeight::BOLD).child("Triangle Meshes"))
        .child(div().text_sm().text_color(theme.text_muted).child(
            "Unstructured geometry, scalar fields, contours, axisymmetric data, and inspection-ready picks.",
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_4()
                .child(card(
                    "Irregular scalar mesh",
                    "Vertex-associated values on an indexed triangle mesh.",
                    "10 triangles • smooth scalar field • wireframe overlay",
                    interactive_mesh_plot(app, cx, &scalar, 0),
                    &theme,
                ))
                .child(card(
                    "Field display modes",
                    "The same field can drive fills, contours, and isolines.",
                    "Scalar fill • isoline step 0.35 • flat geometry preserved",
                    interactive_mesh_plot(app, cx, &modes, 1),
                    &theme,
                )),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_4()
                .child(card(
                    "Axisymmetric section",
                    "A radial/axial section is ready to be revolved into a 3D body.",
                    "Radial and axial coordinates • equal-aspect section • revolve-ready",
                    interactive_mesh_plot(app, cx, &axisymmetric, 2),
                    &theme,
                ))
                .child(card(
                    "Inspection & missing data",
                    "Cell values, stable IDs, and an explicitly masked missing sample.",
                    "Cell-associated field • masked NaN • cell/vertex IDs available to picks",
                    interactive_mesh_plot(app, cx, &inspection, 3),
                    &theme,
                )),
        )
        .child(
            div()
                .mt_2()
                .p_4()
                .bg(theme.surface)
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .text_sm()
                .child("Mesh data is validated before upload; geometry and scalar revisions are retained separately so field updates do not re-upload the mesh."),
        )
}
