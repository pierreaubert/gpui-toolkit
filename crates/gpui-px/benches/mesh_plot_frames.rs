use criterion::{Criterion, criterion_group, criterion_main};
use d3rs::mesh::TriangleMesh;
#[cfg(feature = "gpu-3d")]
use gpui_px::MeshPlotView;
use gpui_px::mesh_plot::MeshPlotState;
use gpui_px::{MeshRenderMode, mesh_plot};
use std::hint::black_box;
use std::sync::Arc;

#[cfg(feature = "gpu-3d")]
use d3rs::gpu3d::Camera3D;
#[cfg(feature = "gpu-3d")]
use d3rs::mesh::{MeshBounds, MeshBvh, RevolveSpec, revolve, revolve_field};
#[cfg(feature = "gpu-3d")]
use d3rs::mesh::{ScalarAssociation, ScalarField};
#[cfg(feature = "gpu-3d")]
use gpui_px::FieldInterpolation;

#[cfg(feature = "gpu-3d")]
fn connected_grid_mesh(triangle_count: usize, id: &str) -> TriangleMesh {
    let side = ((triangle_count as f64 / 2.0).sqrt().ceil() as usize).saturating_add(1);
    let positions = (0..side)
        .flat_map(|y| (0..side).map(move |x| [x as f64, y as f64, 0.0]))
        .collect::<Vec<_>>();
    let mut triangles = Vec::with_capacity(triangle_count);
    'rows: for y in 0..side - 1 {
        for x in 0..side - 1 {
            let a = (y * side + x) as u32;
            let b = a + 1;
            let c = a + side as u32;
            let d = c + 1;
            triangles.push([a, b, c]);
            if triangles.len() == triangle_count {
                break 'rows;
            }
            triangles.push([b, d, c]);
            if triangles.len() == triangle_count {
                break 'rows;
            }
        }
    }
    TriangleMesh {
        id: id.into(),
        positions: positions.into(),
        triangles: triangles.into(),
        vertex_ids: None,
        cell_ids: None,
    }
}

#[cfg(feature = "gpu-3d")]
fn connected_axisymmetric_profile(triangle_count: usize) -> TriangleMesh {
    let side = ((triangle_count as f64 / 2.0).sqrt().ceil() as usize).saturating_add(1);
    let positions = (0..side)
        .flat_map(|z| (0..side).map(move |r| [r as f64 / side as f64, 0.0, z as f64 / side as f64]))
        .collect::<Vec<_>>();
    let mut triangles = Vec::with_capacity(triangle_count);
    'rows: for z in 0..side - 1 {
        for r in 0..side - 1 {
            let a = (z * side + r) as u32;
            let b = a + 1;
            let c = a + side as u32;
            let d = c + 1;
            triangles.push([a, c, b]);
            if triangles.len() == triangle_count {
                break 'rows;
            }
            triangles.push([b, c, d]);
            if triangles.len() == triangle_count {
                break 'rows;
            }
        }
    }
    TriangleMesh {
        id: "revolve-bench".into(),
        positions: positions.into(),
        triangles: triangles.into(),
        vertex_ids: None,
        cell_ids: None,
    }
}

#[cfg(feature = "gpu-3d")]
fn vertex_field(mesh: &TriangleMesh, id: &str) -> ScalarField {
    ScalarField {
        id: id.into(),
        label: id.into(),
        unit: None,
        values: mesh
            .positions
            .iter()
            .map(|position| position[0] * 0.5 + position[1] * 0.001 + position[2])
            .collect::<Vec<_>>()
            .into(),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}
#[cfg(feature = "gpu-3d")]
use gpui_px::mesh_plot::picking3d::{pick_3d_with_bvh, pick_revolved_3d_with_bvh};

#[inline(never)]
fn replace_field_values_for_bench(
    state: &mut MeshPlotState,
    revision: u64,
    values: &[f32],
) -> (bool, u64) {
    let accepted = black_box(&mut *state).replace_field_values(revision, black_box(values));
    // Observe the retained payload so the optimizer cannot replace the whole
    // benchmark with only a revision increment. The checksum is deliberately
    // allocation-free and models the readback a renderer/cache inspector uses.
    let checksum = state.field_values().iter().fold(0_u64, |sum, value| {
        sum.wrapping_add(u64::from(value.to_bits()))
    });
    (accepted, checksum)
}

#[inline(never)]
fn camera_frame_for_bench(state: &mut MeshPlotState, phase: f64) -> f64 {
    let inset = 0.05 + phase * 0.001;
    state.set_viewport_without_history(inset, 1.0 - inset, inset, 1.0 - inset);
    state.interaction.x_domain().0
}

fn bench(c: &mut Criterion) {
    #[cfg(feature = "gpu-3d")]
    let mesh = connected_grid_mesh(200_000, "bench-surface");
    c.bench_function("mesh_plot_svg_frame", |b| {
        let small = TriangleMesh {
            id: "bench-svg".into(),
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
        b.iter(|| black_box(mesh_plot(small.clone()).mode(MeshRenderMode::Mesh).to_svg()))
    });

    #[cfg(feature = "gpu-3d")]
    c.bench_function("mesh_plot_png_frame", |b| {
        let small = TriangleMesh {
            id: "bench-png".into(),
            positions: Arc::from([
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.15],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, -0.15],
            ]),
            triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let field = vertex_field(&small, "bench-png-field");
        let plot = mesh_plot(small)
            .field(field)
            .view(MeshPlotView::Surface3d)
            .mode(MeshRenderMode::ScalarFill {
                interpolation: FieldInterpolation::Smooth,
            });
        b.iter(|| black_box(plot.to_png(1.0)))
    });

    #[cfg(feature = "gpu-3d")]
    let field = vertex_field(&mesh, "bench-field");
    #[cfg(feature = "gpu-3d")]
    c.bench_function("mesh_plot_build_200000_triangles", |b| {
        b.iter(|| {
            black_box(
                mesh_plot(mesh.clone())
                    .field(field.clone())
                    .mode(MeshRenderMode::ScalarFill {
                        interpolation: FieldInterpolation::Smooth,
                    })
                    .build(),
            )
        })
    });

    #[cfg(feature = "gpu-3d")]
    c.bench_function("mesh_plot_fit_200000_triangles", |b| {
        let bounds = MeshBounds::from_positions(&mesh.positions);
        b.iter(|| {
            let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
            state.fit_camera_to_bounds(black_box(bounds), 16.0 / 9.0);
            black_box(state.camera.position)
        })
    });

    let mut group = c.benchmark_group("mesh_plot_retained_frames");
    for value_count in [100_000, 1_000_000] {
        let values_a = vec![0.25_f32; value_count];
        let values_b = vec![0.75_f32; value_count];

        let mut camera_state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        camera_state.reserve_field_capacity(value_count);
        camera_state.replace_field_values(1, &values_a);
        group.bench_function(format!("camera_{value_count}_values"), |b| {
            let mut phase = 0.0;
            b.iter(|| {
                phase = (phase + 1.0) % 100.0;
                black_box(camera_frame_for_bench(&mut camera_state, phase));
            })
        });

        let mut field_state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        field_state.reserve_field_capacity(value_count);
        field_state.replace_field_values(1, &values_a);
        group.bench_function(format!("field_replace_{value_count}_values"), |b| {
            let mut revision = 2;
            b.iter(|| {
                let values = if revision % 2 == 0 {
                    &values_b
                } else {
                    &values_a
                };
                let accepted = replace_field_values_for_bench(
                    black_box(&mut field_state),
                    revision,
                    black_box(values),
                );
                revision += 1;
                black_box((accepted, field_state.field_revision));
            })
        });
    }
    group.finish();

    #[cfg(feature = "gpu-3d")]
    retained_pick_bench(c);
}

#[cfg(feature = "gpu-3d")]
fn retained_pick_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_plot_retained_picking");
    let surface = connected_grid_mesh(200_000, "surface");
    let field = vertex_field(&surface, "height");
    let camera = Camera3D::default().with_position(glam::Vec3::new(0.0, 0.0, 3.0));
    let bvh = MeshBvh::build(&surface);
    let plot_id: Arc<str> = Arc::from("bench-surface");
    group.bench_function("surface_bvh_pick", |b| {
        b.iter(|| {
            black_box(pick_3d_with_bvh(
                &surface,
                Some(&field),
                &bvh,
                &camera,
                [50.0, 50.0],
                [100.0, 100.0],
                plot_id.clone(),
            ))
        })
    });

    let profile = connected_axisymmetric_profile(2_000);
    let source_field = vertex_field(&profile, "pressure");
    let revolve_spec = RevolveSpec {
        radial: d3rs::mesh::CoordinateAxis::X,
        axial: d3rs::mesh::CoordinateAxis::Z,
        ..RevolveSpec::default()
    };
    let revolved = revolve(&profile, &revolve_spec).expect("valid benchmark profile");
    let derived_field = ScalarField {
        values: revolve_field(&source_field, &revolved).into(),
        ..source_field.clone()
    };
    let revolved_bvh = MeshBvh::build(&revolved.mesh);
    let revolved_id: Arc<str> = Arc::from("bench-revolve");
    group.bench_function("revolved_bvh_pick", |b| {
        b.iter(|| {
            black_box(pick_revolved_3d_with_bvh(
                &profile,
                &revolved,
                Some(&derived_field),
                &revolved_bvh,
                Some(source_field.id.clone()),
                &camera,
                [50.0, 50.0],
                [100.0, 100.0],
                revolved_id.clone(),
            ))
        })
    });
    group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
