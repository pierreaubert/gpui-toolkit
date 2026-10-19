// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    calculator, field_statistics, histogram, probe, resample_to_grid,
};
use gpui_mesh::{Association, CellType, Cells, DataSet, Field};

fn cloud(values: Vec<f64>) -> DataSet {
    DataSet::new(
        "values",
        vec![[0.0; 3]; values.len()],
        Cells::from_cells(&[]),
    )
    .with_field(Field::scalar("p", values))
}

#[test]
fn statistics_and_histograms_exclude_masked_finite_placeholders() {
    let mut data = cloud(vec![1.0, 99.0, 3.0, f64::NAN]);
    data.point_data[0].valid = Some(vec![true, false, true, false].into());
    data.cells = Cells::lines(&[[0, 1], [1, 2], [2, 3]]);
    let mut cell = Field::scalar("p", vec![2.0, 100.0, 6.0]).on_cells();
    cell.valid = Some(vec![true, false, true].into());
    data.cell_data.push(cell);
    for (association, mean, deviation) in [
        (Association::Point, 2.0, 1.0),
        (Association::Cell, 4.0, 2.0),
    ] {
        let stats = field_statistics(&data, "p", association).unwrap();
        assert_eq!(stats.count, 2);
        assert_eq!(stats.mean, mean);
        assert_eq!(stats.median, mean);
        assert_eq!(stats.std_dev, deviation);
    }
    assert_eq!(histogram(&data, "p", 2).unwrap(), vec![(1.5, 1), (2.5, 1)]);
    assert_eq!(data.point_data[0].as_scalar().unwrap()[1], 99.0);
}

#[test]
fn statistics_preserve_extreme_ranges_small_spreads_and_subnormals() {
    let tiny = f64::from_bits(1);
    for (values, mean, deviation, median) in [
        (vec![f64::MAX; 2], f64::MAX, 0.0, f64::MAX),
        (vec![-f64::MAX, f64::MAX], 0.0, f64::MAX, 0.0),
        (
            vec![-f64::MAX, f64::MAX, 1.0],
            1.0 / 3.0,
            f64::MAX * (2.0_f64 / 3.0).sqrt(),
            1.0,
        ),
        (
            vec![f64::MAX, 1.0, -f64::MAX],
            1.0 / 3.0,
            f64::MAX * (2.0_f64 / 3.0).sqrt(),
            1.0,
        ),
        (vec![1e16, 1e16 + 2.0], 1e16, 1.0, 1e16),
        (vec![0.0, 2.0 * tiny], tiny, tiny, tiny),
        (vec![3.0, 1.0, 2.0, 4.0], 2.5, 1.25_f64.sqrt(), 2.5),
    ] {
        let stats = field_statistics(&cloud(values), "p", Association::Point).unwrap();
        for (actual, expected) in [
            (stats.mean, mean),
            (stats.std_dev, deviation),
            (stats.median, median),
        ] {
            assert!(actual.is_finite());
            if expected == 0.0 || expected.is_subnormal() {
                assert_eq!(actual, expected);
            } else {
                assert!(
                    (actual / expected - 1.0).abs() < 2e-14,
                    "{actual} != {expected}"
                );
            }
        }
    }
}

#[test]
fn histograms_handle_overflowing_ranges_subnormal_widths_and_bounded_bins() {
    let bins = histogram(&cloud(vec![-f64::MAX, 0.0, f64::MAX]), "p", 2).unwrap();
    assert_eq!(
        bins.iter().map(|(_, count)| *count).collect::<Vec<_>>(),
        [1, 2]
    );
    for ((actual, _), expected) in bins.iter().zip([-f64::MAX * 0.5, f64::MAX * 0.5]) {
        assert!((actual / expected - 1.0).abs() <= 2.0 * f64::EPSILON);
    }
    let bins = histogram(&cloud(vec![0.0, f64::from_bits(2)]), "p", 4).unwrap();
    assert_eq!(
        bins.iter().map(|(_, n)| *n).collect::<Vec<_>>(),
        [1, 0, 0, 1]
    );
    assert!(bins.iter().all(|(centre, _)| centre.is_finite()));
    assert_eq!(
        histogram(&cloud(vec![2.0; 3]), "p", 3).unwrap(),
        vec![(2.0, 3), (2.0, 0), (2.0, 0)]
    );
    histogram(&cloud(vec![1.0]), "p", 0).unwrap_err();
    histogram(&cloud(vec![1.0]), "p", usize::MAX).unwrap_err();
}

#[test]
fn calculator_propagates_dependencies_and_nonfinite_intermediates() {
    let mut data = DataSet::new("calc", vec![[1.0; 3]; 4], Cells::from_cells(&[]));
    let mut p = Field::scalar("p", vec![2.0, 99.0, f64::NAN, 8.0]);
    p.valid = Some(vec![true, false, false, true].into());
    let mut q = Field::scalar("q", vec![3.0, 4.0, 5.0, 99.0]);
    q.valid = Some(vec![true, true, true, false].into());
    data.point_data.extend([p, q]);
    let out = calculator(&data, "min(p,q)+sqrt(x)", "p").unwrap();
    out.validate().unwrap();
    let p = out.field(Association::Point, "p").unwrap();
    assert_eq!(p.as_scalar().unwrap()[0], 3.0);
    assert_eq!(
        p.valid.as_deref(),
        Some([true, false, false, false].as_slice())
    );
    assert!(p.as_scalar().unwrap()[1..].iter().all(|v| v.is_nan()));
    for expression in ["min(sqrt(-1),1)", "max(0/0,1)", "min(exp(1000),1)", "p*0"] {
        let out = calculator(&data, expression, "result").unwrap();
        out.validate().unwrap();
        let result = out.field(Association::Point, "result").unwrap();
        if expression == "p*0" {
            assert_eq!(
                result.valid.as_deref(),
                Some([true, false, false, true].as_slice())
            );
        } else {
            assert!((0..4).all(|i| !result.is_valid(i)));
        }
    }
    let out = calculator(&data, "7", "result").unwrap();
    assert!(
        out.field(Association::Point, "result")
            .unwrap()
            .valid
            .is_none()
    );
    assert_eq!(data.point_data[0].as_scalar().unwrap()[1], 99.0);
}

#[test]
fn calculator_binds_empty_inputs_and_uses_standard_power_precedence() {
    let data = cloud(vec![0.0]).with_field(Field::scalar("π", vec![2.0]));
    let out = calculator(&data, "-2^2 + 2^-2 + 2^3^2 + 1e-2 + π", "result").unwrap();
    assert_eq!(
        out.field(Association::Point, "result")
            .unwrap()
            .as_scalar()
            .unwrap(),
        &[510.26]
    );
    let empty = cloud(vec![]);
    for expression in ["absent", "sin(1,2)", "unknown(1)", "1e999", "2+", "sin()"] {
        calculator(&empty, expression, "result").unwrap_err();
    }
    let out = calculator(&empty, "2+3", "result").unwrap();
    out.validate().unwrap();
}

#[test]
fn calculator_rejects_excessive_source_depth_and_instruction_counts() {
    let data = cloud(vec![1.0]);
    for expression in [
        format!("{}1{}", "(".repeat(4096), ")".repeat(4096)),
        format!("{}1", "-".repeat(4096)),
        "1+".repeat(32_768),
        "1^".repeat(4096),
        format!("{}1", "1+".repeat(2048)),
    ] {
        calculator(&data, &expression, "result").unwrap_err();
    }
    let out = calculator(&data, &format!("{}1", "1+".repeat(2047)), "result").unwrap();
    assert_eq!(
        out.field(Association::Point, "result")
            .unwrap()
            .as_scalar()
            .unwrap(),
        &[2048.0]
    );
}

#[test]
fn analysis_rejects_malformed_data_and_wholly_missing_populations() {
    let mut missing = cloud(vec![99.0, f64::NAN]);
    missing.point_data[0].valid = Some(vec![false; 2].into());
    field_statistics(&missing, "p", Association::Point).unwrap_err();
    histogram(&missing, "p", 4).unwrap_err();
    for data in [cloud(vec![f64::INFINITY]), cloud(vec![f64::NAN]), {
        let mut data = cloud(vec![1.0, 2.0]);
        data.point_data[0].valid = Some(vec![true].into());
        data
    }] {
        field_statistics(&data, "p", Association::Point).unwrap_err();
        histogram(&data, "p", 4).unwrap_err();
        calculator(&data, "1", "result").unwrap_err();
        resample_to_grid(&data, [2; 3], [0.0; 3], [1.0; 3], 0.0).unwrap_err();
    }
    let empty = cloud(vec![]).with_field(Field::vector("v", vec![]));
    calculator(&empty, "v", "result").unwrap_err();
}

#[test]
fn resampling_preserves_missing_support_and_field_metadata() {
    let mut data = DataSet::structured_grid("source", [2; 3], [0.0; 3], [1.0; 3]).unwrap();
    let mut field = Field::scalar("p", vec![3.0; 8]);
    field.label = "Pressure".into();
    field.unit = Some("Pa".into());
    data.point_data.push(field);
    let mut missing = Field::scalar("missing", vec![99.0; 8]);
    missing.valid = Some(vec![false; 8].into());
    data.point_data.push(missing);
    for fill in [-7.0, f64::NAN] {
        let out = resample_to_grid(&data, [3; 3], [0.0; 3], [1.0; 3], fill).unwrap();
        out.validate().unwrap();
        let field = out.field(Association::Point, "p").unwrap();
        assert_eq!(field.label.as_ref(), "Pressure");
        assert_eq!(field.unit.as_deref(), Some("Pa"));
        for (i, position) in out.positions.iter().enumerate() {
            let supported = position.iter().all(|v| *v <= 1.0);
            assert_eq!(field.is_valid(i), supported);
            if supported {
                assert_eq!(field.as_scalar().unwrap()[i], 3.0);
            } else if fill.is_nan() {
                assert!(field.as_scalar().unwrap()[i].is_nan());
            } else {
                assert_eq!(field.as_scalar().unwrap()[i], fill);
            }
        }
        let missing = out.field(Association::Point, "missing").unwrap();
        assert!((0..out.point_count()).all(|i| !missing.is_valid(i)));
    }
    resample_to_grid(&data, [2; 3], [0.0; 3], [1.0; 3], f64::INFINITY).unwrap_err();
}

#[test]
fn probing_preserves_extreme_and_subnormal_constants_and_nearest_point_order() {
    let data = DataSet::structured_grid("probe", [2; 3], [0.0; 3], [1.0; 3]).unwrap();
    for value in [f64::MAX, f64::from_bits(1)] {
        let data = data.clone().with_field(Field::scalar("p", vec![value; 8]));
        assert_eq!(probe(&data, "p", &[[0.2; 3]]).unwrap(), vec![Some(value)]);
    }
    for (positions, query) in [
        (
            vec![[-f64::MAX, 0.0, 0.0], [f64::MAX * 0.5, 0.0, 0.0]],
            [f64::MAX, 0.0, 0.0],
        ),
        (
            vec![[0.0; 3], [f64::from_bits(1), 0.0, 0.0]],
            [f64::from_bits(1), 0.0, 0.0],
        ),
    ] {
        let data = DataSet::new("nearest", positions, Cells::from_cells(&[]))
            .with_field(Field::scalar("p", vec![1.0, 2.0]));
        assert_eq!(probe(&data, "p", &[query]).unwrap(), vec![Some(2.0)]);
    }
}

#[test]
fn probing_batch_sizes_preserve_mask_support_and_original_cell_priority() {
    let tet = [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut data = DataSet::new(
        "overlap",
        tet.repeat(2),
        Cells::from_cells(&[
            (CellType::Tetra, &[0, 1, 2, 3]),
            (CellType::Tetra, &[4, 5, 6, 7]),
        ]),
    )
    .with_field(Field::scalar(
        "p",
        vec![10.0, 10.0, 10.0, 10.0, 20.0, 20.0, 20.0, 20.0],
    ));
    for expected in [10.0, 20.0] {
        if expected == 20.0 {
            data.point_data[0].valid =
                Some(vec![false, false, false, false, true, true, true, true].into());
        }
        for count in [1, 8, 9, 64, 65, 128] {
            assert_eq!(
                probe(&data, "p", &vec![[0.1; 3]; count]).unwrap(),
                vec![Some(expected); count]
            );
            assert_eq!(
                probe(&data, "p", &vec![[2.0; 3]; count]).unwrap(),
                vec![None; count]
            );
        }
    }
}
