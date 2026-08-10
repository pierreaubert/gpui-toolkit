//! Allocation and retained-state contracts for MeshPlot hot paths.

use d3rs::mesh::{
    CoordinateAxis, ScalarAssociation, ScalarField, TriGridIndex, TriangleMesh, project_2d,
};
use gpui_profiler::{AllocProbe, AllocationBudget};
use gpui_px::mesh_plot::{MeshPlotState, pick_2d};
use std::hint::black_box;
use std::sync::Arc;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn skip_instrumented_runs() -> bool {
    std::env::var_os("CARGO_LLVM_COV").is_some()
}

#[test]
fn navigation_after_warmup_does_not_grow_zoom_history_or_allocate() {
    let _guard = TEST_LOCK.lock().unwrap();
    if skip_instrumented_runs() {
        return;
    }

    let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
    state.set_viewport_without_history(0.1, 0.9, 0.1, 0.9);
    assert_eq!(state.interaction.zoom_level(), 0);

    let mut probe = AllocProbe::new();
    probe.reset();
    for index in 0..1_000 {
        let offset = (index % 10) as f64 * 0.001;
        state.set_viewport_without_history(0.1 + offset, 0.9 + offset, 0.1, 0.9);
        black_box(state.interaction.x_domain());
    }

    AllocationBudget::zero("mesh-plot-navigation-after-warmup")
        .assert_contains(probe.sample("mesh-plot-navigation-after-warmup"));
    assert_eq!(state.interaction.zoom_level(), 0);
}

#[test]
fn alternating_field_updates_reuse_retained_capacity_and_preserve_state() {
    let _guard = TEST_LOCK.lock().unwrap();
    if skip_instrumented_runs() {
        return;
    }

    const VALUES: usize = 100_000;
    let values_a = vec![0.25f32; VALUES];
    let values_b = vec![0.75f32; VALUES];
    let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
    state.reserve_field_capacity(VALUES);
    state.replace_field_values(1, &values_a);
    let retained_len = state.field_values().len();
    assert_eq!(retained_len, VALUES);
    let geometry_revision = state.geometry_revision;
    let viewport = state.interaction.x_domain();

    let mut probe = AllocProbe::new();
    probe.reset();
    for revision in 2..=1_001 {
        let values = if revision % 2 == 0 {
            &values_b
        } else {
            &values_a
        };
        state.replace_field_values(revision, values);
        black_box(state.field_values());
    }

    AllocationBudget::zero("mesh-plot-field-replace-1000x")
        .assert_contains(probe.sample("mesh-plot-field-replace-1000x"));
    assert_eq!(state.field_values().len(), retained_len);
    assert_eq!(state.geometry_revision, geometry_revision);
    assert_eq!(state.interaction.x_domain(), viewport);
    assert!(state.selection.is_none());
}

#[test]
fn field_replacement_preserves_zoom_and_a_real_selection() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mesh = TriangleMesh {
        id: "square".into(),
        positions: Arc::from([
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
        vertex_ids: None,
        cell_ids: None,
    };
    let positions = mesh
        .positions
        .iter()
        .map(|&point| project_2d(CoordinateAxis::X, CoordinateAxis::Y, point))
        .collect::<Vec<_>>();
    let index = TriGridIndex::build(&positions, &mesh.triangles);
    let field = ScalarField {
        id: "pressure".into(),
        label: "Pressure".into(),
        unit: None,
        values: Arc::from([0.0, 1.0, 2.0, 1.0]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    let pick = pick_2d(
        &mesh,
        Some(&field),
        &index,
        CoordinateAxis::X,
        CoordinateAxis::Y,
        [0.4, 0.3],
        "plot",
    )
    .unwrap();

    let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
    state.interaction.zoom_around_pixel(40.0, 30.0, 1.5);
    state.selection = Some(pick);
    let viewport = (state.interaction.x_domain(), state.interaction.y_domain());
    let geometry_revision = state.geometry_revision;
    state.replace_field_values(2, &[4.0, 5.0, 6.0, 7.0]);

    assert_eq!(state.geometry_revision, geometry_revision);
    assert_eq!(
        (state.interaction.x_domain(), state.interaction.y_domain()),
        viewport
    );
    assert!(state.selection.is_some());
}
