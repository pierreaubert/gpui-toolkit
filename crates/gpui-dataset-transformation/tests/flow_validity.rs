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
