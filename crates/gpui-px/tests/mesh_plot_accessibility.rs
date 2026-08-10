use d3rs::mesh::{ScalarAssociation, ScalarField, TriangleMesh};
use gpui::{
    AppContext as _, Context, ElementId, IntoElement, ParentElement, Render, TestAppContext,
    Window, div,
};
use gpui_px::{FieldInterpolation, MeshRenderMode, mesh_plot};
use gpui_ui_kit::accessibility::{AccessibilityTree, AriaRole};
use std::sync::Arc;

struct MeshPlotAccessibilityView;

impl Render for MeshPlotAccessibilityView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mesh = TriangleMesh {
            id: "accessibility-mesh".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let field = ScalarField {
            id: "accessibility-pressure".into(),
            label: "Pressure".into(),
            unit: Some("Pa".into()),
            values: Arc::from([0.0, 0.5, 1.0]),
            association: ScalarAssociation::Vertex,
            valid: None,
        };
        let plot = mesh_plot(mesh)
            .field(field)
            .title("Pressure field")
            .mode(MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            })
            .size(240.0, 180.0)
            .build()
            .expect("MeshPlot accessibility fixture should build");
        div().child(plot)
    }
}

#[gpui::test]
async fn mesh_plot_registers_native_accessibility_summary(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_global(AccessibilityTree::new()));
    let _window = cx.add_window(|_window, _cx| MeshPlotAccessibilityView);

    cx.update(|cx| {
        let tree = cx.global::<AccessibilityTree>();
        let node = tree
            .get(&ElementId::Name("mesh-plot-accessibility-mesh".into()))
            .expect("MeshPlot should register an accessibility node");
        assert_eq!(node.props.role, AriaRole::Img);
        assert_eq!(node.label.as_ref(), "Pressure field");
        assert!(node.props.value_text.as_deref().is_some_and(|value| {
            value.contains("1 series") && value.contains("3 data points")
        }));
        let description = node
            .props
            .description
            .as_deref()
            .expect("MeshPlot should expose a structured description");
        assert!(description.contains("planar view with 3 vertices and 1 triangles"));
        assert!(description.contains("Field Pressure (Pa), vertex association, 3 values."));
        assert!(description.contains("Displayed value range 0.000 to 1.000."));
        assert!(description.contains("No mesh element is selected."));
        assert!(description.contains("Available controls: inspect, select, pan, zoom, fit, and reset."));

        let snapshot = tree.to_bridge_snapshot();
        assert_eq!(
            snapshot.schema_version,
            gpui_ui_kit::ACCESSIBILITY_BRIDGE_SCHEMA_VERSION
        );
        assert_eq!(
            snapshot.report_type,
            gpui_ui_kit::ACCESSIBILITY_BRIDGE_REPORT_TYPE
        );
        assert_eq!(snapshot.nodes.len(), 1);
        assert!(snapshot.all_nodes_named());

        let bridge_node = &snapshot.nodes[0];
        assert_eq!(
            bridge_node.element_id,
            ElementId::Name("mesh-plot-accessibility-mesh".into())
        );
        assert_eq!(bridge_node.role, AriaRole::Img);
        assert_eq!(bridge_node.role_name, "img");
        assert_eq!(bridge_node.label.as_ref(), "Pressure field");
        assert_eq!(
            bridge_node.description.as_deref(),
            node.props.description.as_deref()
        );
        assert_eq!(
            bridge_node.value.text.as_deref(),
            node.props.value_text.as_deref()
        );
    });
}
