use d3rs::mesh::{CoordinateAxis, ScalarAssociation, ScalarField, TriangleMesh};
use gpui::{
    AppContext as _, Context, ElementId, IntoElement, ParentElement, Render, TestAppContext,
    Window, div,
};
use gpui_px::{
    Axes2d, FieldInterpolation, MeshPlotPick, MeshPlotView, MeshRenderMode, PlotInteractions,
    Wireframe, mesh_plot,
};
use gpui_ui_kit::accessibility::{AccessibilityTree, AriaRole};
use std::sync::Arc;

struct MeshPlotAccessibilityView;
struct MeshPlotToolbarAccessibilityView;

#[test]
fn mesh_plot_accessibility_summary_covers_selection_view_ranges_and_disabled_controls() {
    let mesh = TriangleMesh {
        id: "summary-mesh".into(),
        positions: Arc::from([[0.0, 0.0, 0.0], [2.0, 0.0, 4.0], [0.0, 0.0, 4.0]]),
        triangles: Arc::from([[0, 1, 2]]),
        vertex_ids: Some(Arc::from([10, 11, 12])),
        cell_ids: Some(Arc::from([42])),
    };
    let field = ScalarField {
        id: "summary-field".into(),
        label: "Temperature".into(),
        unit: Some("K".into()),
        values: Arc::from([10.0, 20.0, 30.0]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    let plot = mesh_plot(mesh)
        .field(field)
        .title("Thermal section")
        .view(MeshPlotView::AxisymmetricSection {
            radial: CoordinateAxis::X,
            axial: CoordinateAxis::Z,
        })
        .axes(
            Axes2d::equal_aspect()
                .labels("radius", "height")
                .unit("m")
                .ranges([0.0, 2.0], [0.0, 4.0])
                .grid(false),
        )
        .wireframe(Wireframe::hidden())
        .interactions(PlotInteractions::none())
        .selection(MeshPlotPick {
            plot_id: "summary-plot".into(),
            mesh_id: "summary-mesh".into(),
            cell_index: 0,
            cell_id: Some(42),
            nearest_vertex_index: Some(1),
            vertex_id: Some(11),
            world_position: [1.0, 0.0, 4.0],
            displayed_value: Some(20.0),
            field_id: Some("summary-field".into()),
        });

    let summary = plot.accessibility_summary();
    assert_eq!(summary.title.as_deref(), Some("Thermal section"));
    assert_eq!(summary.series_count, 1);
    assert_eq!(summary.datum_count, 3);
    assert_eq!(summary.x_range, Some([0.0, 2.0]));
    assert_eq!(summary.y_range, Some([0.0, 4.0]));
    assert_eq!(summary.value_range, Some([10.0, 30.0]));
    assert_eq!(summary.series_labels, vec!["Temperature".to_string()]);
    assert_eq!(summary.description.matches("Field Temperature").count(), 1);
    assert!(summary.description.contains("axisymmetric-section view"));
    assert!(summary.description.contains("radius (m)"));
    assert!(
        summary
            .description
            .contains("Selected cell 0 (id 42); value 20.000.")
    );
    assert!(summary.description.contains("Wireframe hidden."));
    assert!(summary.description.contains("Available controls: none."));
}

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

impl Render for MeshPlotToolbarAccessibilityView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mesh = TriangleMesh {
            id: "toolbar-accessibility-mesh".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let field = ScalarField {
            id: "toolbar-accessibility-field".into(),
            label: "Pressure".into(),
            unit: Some("Pa".into()),
            values: Arc::from([0.0, 0.5, 1.0]),
            association: ScalarAssociation::Vertex,
            valid: None,
        };
        let plot = mesh_plot(mesh)
            .field(field)
            .title("Toolbar plot")
            .mode(MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            })
            .toolbar(true)
            .size(320.0, 220.0)
            .build()
            .expect("MeshPlot toolbar accessibility fixture should build");
        div().child(plot)
    }
}

#[gpui::test]
async fn mesh_plot_registers_native_accessibility_summary(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_global(AccessibilityTree::new()));
    let window = cx.add_window(|_window, _cx| MeshPlotAccessibilityView);

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
        assert!(
            description.contains("Available controls: inspect, select, pan, zoom, fit, and reset.")
        );

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

    cx.update_window(window.into(), |_, window, _| window.remove_window())
        .expect("close MeshPlot accessibility test window");
    cx.run_until_parked();
}

#[gpui::test]
async fn mesh_plot_toolbar_exports_named_controls_in_keyboard_order(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_global(AccessibilityTree::new()));
    let window = cx.add_window(|_window, _cx| MeshPlotToolbarAccessibilityView);

    cx.update(|cx| {
        let tree = cx.global::<AccessibilityTree>();
        let ordered_ids: Vec<_> = tree
            .nodes_in_order()
            .into_iter()
            .map(|node| node.element_id.clone())
            .collect();
        let position = |id: &str| {
            ordered_ids
                .iter()
                .position(|candidate| *candidate == ElementId::Name(id.into()))
                .unwrap_or_else(|| panic!("missing accessibility node {id}"))
        };

        let toolbar = tree
            .get(&ElementId::Name("mesh-plot-toolbar".into()))
            .expect("MeshPlot toolbar should register an accessibility node");
        assert_eq!(toolbar.props.role, AriaRole::Toolbar);
        assert_eq!(toolbar.label.as_ref(), "Plot controls");

        let visible_controls = [
            "mesh-plot-toolbar-fit",
            "mesh-plot-toolbar-reset",
            "mesh-plot-toolbar-mode",
            "mesh-plot-toolbar-wireframe",
            "mesh-plot-toolbar-color-range",
            "mesh-plot-toolbar-export",
        ];
        let positions: Vec<_> = visible_controls.iter().map(|id| position(id)).collect();
        assert!(
            positions.windows(2).all(|pair| pair[0] < pair[1]),
            "toolbar controls must remain in keyboard/render order: {positions:?}"
        );
        assert!(
            tree.get(&ElementId::Name("mesh-plot-toolbar-view".into()))
                .is_none(),
            "planar plots must not expose the 3D view menu"
        );

        let export = tree
            .get(&ElementId::Name("mesh-plot-toolbar-export".into()))
            .expect("export control should be present even when no callback is configured");
        assert_eq!(export.props.role, AriaRole::Button);
        assert_eq!(export.label.as_ref(), "Export plot");

        let snapshot = tree.to_bridge_snapshot();
        assert!(snapshot.all_nodes_named());
        let export_bridge_node = snapshot
            .nodes
            .iter()
            .find(|node| node.element_id == ElementId::Name("mesh-plot-toolbar-export".into()))
            .expect("export control should be present in the bridge snapshot");
        assert!(export_bridge_node.is_disabled());
    });

    cx.update_window(window.into(), |_, window, _| window.remove_window())
        .expect("close MeshPlot toolbar accessibility test window");
    cx.run_until_parked();
}
