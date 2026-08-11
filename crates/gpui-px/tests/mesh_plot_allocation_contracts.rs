//! Allocation and retained-state contracts for MeshPlot hot paths.

use d3rs::mesh::{
    CoordinateAxis, ScalarAssociation, ScalarField, TriGridIndex, TriangleMesh, project_2d,
};
use gpui_profiler::{AllocProbe, AllocationBudget};
use gpui_px::mesh_plot::{MeshPlotState, pick_2d};
use std::hint::black_box;
use std::sync::Arc;
use std::sync::Mutex;

#[cfg(feature = "gpu-3d")]
use d3rs::gpu3d::Camera3D;
#[cfg(feature = "gpu-3d")]
use d3rs::mesh::{MeshBvh, RevolveSpec, revolve, revolve_field};
#[cfg(feature = "gpu-3d")]
use gpui_px::mesh_plot::picking3d::{pick_3d_with_bvh, pick_revolved_3d_with_bvh};

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

#[cfg(feature = "gpu-3d")]
#[test]
fn retained_3d_picking_reuses_the_bvh_and_stable_plot_id_without_allocating() {
    let _guard = TEST_LOCK.lock().unwrap();
    if skip_instrumented_runs() {
        return;
    }
    let mesh = TriangleMesh {
        id: "surface".into(),
        positions: Arc::from([
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
        vertex_ids: None,
        cell_ids: None,
    };
    let field = ScalarField {
        id: "height".into(),
        label: "Height".into(),
        unit: None,
        values: Arc::from([0.0, 1.0, 2.0, 1.0]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    let bvh = MeshBvh::build(&mesh);
    let camera = Camera3D::default().with_position(glam::Vec3::new(0.0, 0.0, 3.0));
    let plot_id: Arc<str> = Arc::from("retained-plot");
    assert!(
        pick_3d_with_bvh(
            &mesh,
            Some(&field),
            &bvh,
            &camera,
            [50.0, 50.0],
            [100.0, 100.0],
            plot_id.clone(),
        )
        .is_some()
    );
    // Settle debug/runtime one-time state before measuring the steady-state
    // retained picker contract.
    for _ in 0..1_000 {
        black_box(bvh.ray_cast([0.0, 0.0, 3.0], [0.0, 0.0, -1.0]));
    }

    let mut probe = AllocProbe::new();
    probe.reset();
    for _ in 0..1_000 {
        black_box(bvh.ray_cast([0.0, 0.0, 3.0], [0.0, 0.0, -1.0]));
    }
    AllocationBudget::zero("mesh-plot-retained-3d-bvh-query-1000x")
        .assert_contains(probe.sample("mesh-plot-retained-3d-bvh-query-1000x"));

    probe.reset();
    for _ in 0..1_000 {
        black_box(pick_3d_with_bvh(
            &mesh,
            Some(&field),
            &bvh,
            &camera,
            [50.0, 50.0],
            [100.0, 100.0],
            plot_id.clone(),
        ));
    }
    AllocationBudget::zero("mesh-plot-retained-3d-pick-1000x")
        .assert_contains(probe.sample("mesh-plot-retained-3d-pick-1000x"));
}

#[cfg(feature = "gpu-3d")]
#[test]
fn retained_revolved_picking_reuses_derived_geometry_and_source_ids_without_allocating() {
    let _guard = TEST_LOCK.lock().unwrap();
    if skip_instrumented_runs() {
        return;
    }
    let profile = TriangleMesh {
        id: "profile".into(),
        positions: Arc::from([[0.0, 0.0, -1.0], [1.0, 0.0, -1.0], [1.0, 0.0, 1.0]]),
        triangles: Arc::from([[0, 1, 2]]),
        vertex_ids: Some(Arc::from([10, 20, 30])),
        cell_ids: Some(Arc::from([40])),
    };
    let source_field = ScalarField {
        id: "pressure".into(),
        label: "Pressure".into(),
        unit: None,
        values: Arc::from([0.0, 1.0, 2.0]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    let revolved = revolve(&profile, &RevolveSpec::default()).expect("valid profile");
    let derived_field = ScalarField {
        values: revolve_field(&source_field, &revolved).into(),
        ..source_field.clone()
    };
    let bvh = MeshBvh::build(&revolved.mesh);
    let camera = Camera3D::default().with_position(glam::Vec3::new(0.0, 0.0, 3.0));
    let plot_id: Arc<str> = Arc::from("retained-revolve");
    let pick = pick_revolved_3d_with_bvh(
        &profile,
        &revolved,
        Some(&derived_field),
        &bvh,
        Some(source_field.id.clone()),
        &camera,
        [50.0, 50.0],
        [100.0, 100.0],
        plot_id.clone(),
    )
    .expect("center ray intersects the retained revolution surface");
    assert_eq!(pick.cell_id, Some(40));
    assert!(matches!(pick.vertex_id, Some(10 | 20 | 30)));

    for _ in 0..1_000 {
        black_box(pick_revolved_3d_with_bvh(
            &profile,
            &revolved,
            Some(&derived_field),
            &bvh,
            Some(source_field.id.clone()),
            &camera,
            [50.0, 50.0],
            [100.0, 100.0],
            plot_id.clone(),
        ));
    }
    let mut probe = AllocProbe::new();
    probe.reset();
    for _ in 0..1_000 {
        black_box(pick_revolved_3d_with_bvh(
            &profile,
            &revolved,
            Some(&derived_field),
            &bvh,
            Some(source_field.id.clone()),
            &camera,
            [50.0, 50.0],
            [100.0, 100.0],
            plot_id.clone(),
        ));
    }
    AllocationBudget::zero("mesh-plot-retained-revolved-pick-1000x")
        .assert_contains(probe.sample("mesh-plot-retained-revolved-pick-1000x"));
}
