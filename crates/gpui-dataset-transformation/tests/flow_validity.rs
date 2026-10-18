// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    divergence, flow_diagnostics, lambda2, q_criterion, swirling_strength, vorticity,
    vorticity_magnitude,
};
use gpui_mesh::{Association, CellType, Cells, DataSet, Field};

fn rotating(masked: f64) -> DataSet {
    let positions = vec![
        [0.0; 3],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
        [5.0; 3],
    ];
    let mut velocity = Field::vector(
        "velocity",
        positions
            .iter()
            .map(|p| [-2.0 * p[1], 2.0 * p[0], 0.0])
            .collect::<Vec<_>>(),
    );
    let gpui_mesh::FieldData::Vector(values) = &mut velocity.data else {
        unreachable!()
    };
    std::sync::Arc::make_mut(values)[3] = [masked; 3];
    velocity.valid = Some(vec![true, true, true, false, true, true].into());
    DataSet::new(
        "rotation",
        positions,
        Cells::from_cells(&[
            (CellType::Tetra, &[0, 1, 2, 3]),
            (CellType::Tetra, &[0, 1, 2, 4]),
        ]),
    )
    .with_field(velocity)
}

#[test]
fn skewed_affine_cells_preserve_gradient_under_orientation_translation_and_scaling() {
    for scale in [0.125, 1.0, 8.0] {
        for origin in [0.0, 1e6] {
            for nodes in [[0, 1, 2, 3], [0, 2, 1, 3]] {
                let positions = [[0.0; 3], [2.0, 0.5, 0.25], [1.0, 3.0, 0.5], [0.5, 1.0, 4.0]]
                    .map(|p| p.map(|v| origin + scale * v));
                let vectors = positions.map(|p| {
                    [
                        p[0] + 2.0 * p[1] + 3.0 * p[2] + 0.25,
                        4.0 * p[0] + 5.0 * p[1] + 6.0 * p[2] - 0.5,
                        7.0 * p[0] + 8.0 * p[1] + 9.0 * p[2] + 1.0,
                    ]
                });
                let dataset = DataSet::new(
                    "affine",
                    positions.to_vec(),
                    Cells::from_cells(&[(CellType::Tetra, &nodes)]),
                )
                .with_field(Field::vector("velocity", vectors.to_vec()));
                let output = flow_diagnostics(&dataset, "velocity").unwrap();
                output.validate().unwrap();
                let curl = output.field(Association::Point, "vorticity").unwrap();
                let q = output.field(Association::Point, "q_criterion").unwrap();
                let div = output.field(Association::Point, "divergence").unwrap();
                for point in 0..4 {
                    assert!(curl.is_valid(point) && q.is_valid(point) && div.is_valid(point));
                    for (actual, expected) in curl.as_vector().unwrap()[point]
                        .iter()
                        .zip([2.0, -4.0, 2.0])
                    {
                        assert!((actual - expected).abs() < 1e-10);
                    }
                    assert!((q.as_scalar().unwrap()[point] + 130.5).abs() < 1e-10);
                    assert!((div.as_scalar().unwrap()[point] - 15.0).abs() < 1e-10);
                }
            }
        }
    }
}

#[test]
fn unequal_incident_volumes_average_jacobians_before_nonlinear_diagnostics() {
    let dataset = DataSet::new(
        "unequal-volumes",
        vec![
            [0.0; 3],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -2.0],
        ],
        Cells::from_cells(&[
            (CellType::Tetra, &[0, 1, 2, 3]),
            (CellType::Tetra, &[0, 2, 1, 4]),
        ]),
    )
    .with_field(Field::vector(
        "velocity",
        vec![
            [0.0; 3],
            [1.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.0, 0.0, 3.0],
            [0.0, 0.0, -10.0],
        ],
    ));
    let output = flow_diagnostics(&dataset, "velocity").unwrap();
    output.validate().unwrap();
    let div = output
        .field(Association::Point, "divergence")
        .unwrap()
        .as_scalar()
        .unwrap();
    let q = output
        .field(Association::Point, "q_criterion")
        .unwrap()
        .as_scalar()
        .unwrap();
    for point in 0..3 {
        assert!((div[point] - 22.0 / 3.0).abs() < 1e-12);
        assert!((q[point] + 107.0 / 9.0).abs() < 1e-12);
    }
    assert_eq!(div[3], 6.0);
    assert_eq!(div[4], 8.0);
    assert_eq!(q[3], -7.0);
    assert_eq!(q[4], -15.0);
}

#[test]
fn lambda2_uses_coupled_strain_eigenvalues() {
    let positions = vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let velocity = positions
        .iter()
        .map(|p| [2.0 * p[0] + p[1], p[0] + 2.0 * p[1], 0.0])
        .collect::<Vec<_>>();
    let dataset = DataSet::new(
        "coupled-strain",
        positions,
        Cells::from_cells(&[(CellType::Tetra, &[0, 1, 2, 3])]),
    )
    .with_field(Field::vector("velocity", velocity));
    let output = lambda2(&dataset, "velocity", "lambda2").unwrap();
    output.validate().unwrap();
    // J eigenvalues are 0,1,3; S² + Ω² has 0,1,9, so lambda-2 is 1.
    assert!(
        output
            .field(Association::Point, "lambda2")
            .unwrap()
            .as_scalar()
            .unwrap()
            .iter()
            .all(|value| (value - 1.0).abs() < 1e-12)
    );
}

#[test]
fn diagnostics_require_complete_stencils_and_mark_unsupported_points_missing() {
    for missing in [99.0, f64::NAN] {
        let dataset = rotating(missing);
        let expected_mask = [true, true, true, false, true, false];
        for (function, expected) in [
            (divergence as fn(&DataSet, &str, &str) -> _, 0.0),
            (q_criterion, 4.0),
            (lambda2, -4.0),
            (swirling_strength, 2.0),
            (vorticity_magnitude, 4.0),
        ] {
            let output = function(&dataset, "velocity", "out").unwrap();
            output.validate().unwrap();
            let field = output.field(Association::Point, "out").unwrap();
            for (point, &valid) in expected_mask.iter().enumerate() {
                assert_eq!(field.is_valid(point), valid);
                let value = field.as_scalar().unwrap()[point];
                if valid {
                    assert!((value - expected).abs() < 1e-12);
                } else {
                    assert!(value.is_nan());
                }
            }
        }
        let output = vorticity(&dataset, "velocity", "out").unwrap();
        output.validate().unwrap();
        let field = output.field(Association::Point, "out").unwrap();
        for (point, &valid) in expected_mask.iter().enumerate() {
            assert_eq!(field.is_valid(point), valid);
            let value = field.as_vector().unwrap()[point];
            if valid {
                assert_eq!(value, [0.0, 0.0, 4.0]);
            } else {
                assert!(value.iter().all(|v| v.is_nan()));
            }
        }
        let combined = flow_diagnostics(&dataset, "velocity").unwrap();
        combined.validate().unwrap();
        for (name, function) in [
            ("vorticity", vorticity as fn(&DataSet, &str, &str) -> _),
            ("divergence", divergence),
            ("q_criterion", q_criterion),
        ] {
            let separate = function(&dataset, "velocity", name).unwrap();
            let combined = combined.field(Association::Point, name).unwrap();
            let separate = separate.field(Association::Point, name).unwrap();
            assert_eq!(combined.valid, separate.valid);
            assert_eq!(combined.label, separate.label);
            assert_eq!(combined.unit, separate.unit);
            for &point in &[0, 1, 2, 4] {
                match (&combined.data, &separate.data) {
                    (gpui_mesh::FieldData::Scalar(a), gpui_mesh::FieldData::Scalar(b)) => {
                        assert_eq!(a[point], b[point]);
                    }
                    (gpui_mesh::FieldData::Vector(a), gpui_mesh::FieldData::Vector(b)) => {
                        assert_eq!(a[point], b[point]);
                    }
                    _ => panic!("wrong diagnostic shape"),
                }
            }
        }
    }
}

#[test]
fn diagnostics_preserve_user_fields_replace_outputs_and_validate_input() {
    let dataset = rotating(99.0)
        .with_field(Field::scalar("__vorticity_tmp", vec![42.0; 6]))
        .with_field(Field::scalar("out", vec![-1.0; 6]));
    let output = vorticity_magnitude(&dataset, "velocity", "out").unwrap();
    output.validate().unwrap();
    assert_eq!(
        output
            .field(Association::Point, "__vorticity_tmp")
            .unwrap()
            .as_scalar()
            .unwrap(),
        &[42.0; 6]
    );
    assert_eq!(
        output
            .point_data
            .iter()
            .filter(|f| f.name.as_ref() == "out")
            .count(),
        1
    );
    let again =
        flow_diagnostics(&flow_diagnostics(&dataset, "velocity").unwrap(), "velocity").unwrap();
    again.validate().unwrap();
    let mut missing = rotating(99.0);
    missing.point_data[0].valid = Some(vec![false; 6].into());
    let output = flow_diagnostics(&missing, "velocity").unwrap();
    output.validate().unwrap();
    for name in ["vorticity", "q_criterion", "divergence"] {
        assert!((0..6).all(|point| {
            !output
                .field(Association::Point, name)
                .unwrap()
                .is_valid(point)
        }));
    }
    missing.point_data[0].data = gpui_mesh::FieldData::Vector(vec![[0.0; 3]].into());
    flow_diagnostics(&missing, "velocity").unwrap_err();
    vorticity(&missing, "velocity", "out").unwrap_err();
}

#[cfg(all(feature = "gpu", not(target_family = "wasm")))]
#[test]
fn metal_flow_kernels_preserve_missing_stencils_and_reject_invalid_updates_atomically() {
    use gpui_dataset_transformation::gpu::{GpuError, GpuFlow, PreparedFlow};
    let gpu = match GpuFlow::new() {
        Ok(gpu) => gpu,
        Err(GpuError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => return,
        Err(error) => panic!("required flow device failed: {error}"),
    };
    for missing in [99.0, f64::NAN] {
        let dataset = rotating(missing);
        let expected = flow_diagnostics(&dataset, "velocity").unwrap();
        let mut prepared = PreparedFlow::new(&gpu, &dataset, "velocity").unwrap();
        let outputs = [
            gpu.flow_diagnostics(&dataset, "velocity").unwrap(),
            prepared.diagnostics(None).unwrap(),
        ];
        for output in outputs {
            output.validate().unwrap();
            for name in ["vorticity", "q_criterion", "divergence"] {
                let actual = output.field(Association::Point, name).unwrap();
                let expected = expected.field(Association::Point, name).unwrap();
                assert_eq!(actual.valid, expected.valid);
                assert_eq!(actual.unit, expected.unit);
                for point in [0, 1, 2, 4] {
                    match (&actual.data, &expected.data) {
                        (gpui_mesh::FieldData::Scalar(a), gpui_mesh::FieldData::Scalar(b)) => {
                            assert!((a[point] - b[point]).abs() < 1e-4);
                        }
                        (gpui_mesh::FieldData::Vector(a), gpui_mesh::FieldData::Vector(b)) => {
                            for axis in 0..3 {
                                assert!((a[point][axis] - b[point][axis]).abs() < 1e-4);
                            }
                        }
                        _ => panic!("wrong diagnostic shape"),
                    }
                }
            }
        }
        // Repeated cold computations replace existing output names.
        gpu.flow_diagnostics(&expected, "velocity")
            .unwrap()
            .validate()
            .unwrap();
        for invalid in [f64::NAN, f64::INFINITY, f64::MAX] {
            let mut update = dataset.point_data[0].as_vector().unwrap().to_vec();
            update[0][0] = invalid;
            prepared.diagnostics(Some(&update)).unwrap_err();
            let repeated = prepared.diagnostics(None).unwrap();
            assert_eq!(
                repeated.point_data[0].as_vector().unwrap()[0],
                dataset.point_data[0].as_vector().unwrap()[0]
            );
            assert_eq!(
                repeated
                    .field(Association::Point, "vorticity")
                    .unwrap()
                    .as_vector()
                    .unwrap()[0],
                [0.0, 0.0, 4.0]
            );
        }
        let mut update = vec![[0.0; 3]; 6];
        update[3] = [f64::NAN; 3];
        let updated = prepared.diagnostics(Some(&update)).unwrap();
        updated.validate().unwrap();
        assert!(
            !updated
                .field(Association::Point, "vorticity")
                .unwrap()
                .is_valid(3)
        );
    }
    let mut missing = rotating(99.0);
    missing.point_data[0].valid = Some(vec![false; 6].into());
    let mut degenerate = rotating(99.0);
    degenerate.positions = vec![[0.0; 3]; 6].into();
    for dataset in [missing, degenerate] {
        let mut prepared = PreparedFlow::new(&gpu, &dataset, "velocity").unwrap();
        for output in [
            gpu.flow_diagnostics(&dataset, "velocity").unwrap(),
            prepared.diagnostics(None).unwrap(),
        ] {
            output.validate().unwrap();
            for name in ["vorticity", "q_criterion", "divergence"] {
                assert!((0..6).all(|point| {
                    !output
                        .field(Association::Point, name)
                        .unwrap()
                        .is_valid(point)
                }));
            }
        }
    }
}
