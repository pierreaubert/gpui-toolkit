// Rust guideline compliant 2026-02-21

use gpui_mesh::{Association, CellType, Cells, DataSet, Field, FieldData, TimeSeries};
use std::sync::Arc;

fn tetrahedron() -> DataSet {
    DataSet::new(
        "tet",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        Cells::from_cells(&[(CellType::Tetra, &[0, 1, 2, 3])]),
    )
}

fn shaped(name: &str, shape: usize, values: &[f64], mask: Option<&[bool]>, missing: f64) -> Field {
    let base = |index: usize| {
        if mask.is_some_and(|m| !m[index]) {
            missing
        } else {
            values[index]
        }
    };
    let rows = |index: usize, component: usize| {
        if mask.is_some_and(|m| !m[index]) {
            missing
        } else {
            base(index) + component as f64 * 0.25
        }
    };
    let mut field = match shape {
        1 => Field::scalar(name, (0..values.len()).map(base).collect::<Vec<_>>()),
        3 => Field::vector(
            name,
            (0..values.len())
                .map(|i| std::array::from_fn(|c| rows(i, c)))
                .collect::<Vec<_>>(),
        ),
        6 => Field::tensor(
            name,
            (0..values.len())
                .map(|i| std::array::from_fn(|c| rows(i, c)))
                .collect::<Vec<_>>(),
        ),
        _ => Field::complex(
            name,
            (0..values.len())
                .map(|i| std::array::from_fn(|c| rows(i, c)))
                .collect::<Vec<_>>(),
        ),
    };
    field.valid = mask.map(Arc::from);
    field.unit = Some("u".into());
    field
}

fn row(field: &Field, index: usize) -> Vec<f64> {
    match &field.data {
        FieldData::Scalar(v) => vec![v[index]],
        FieldData::Vector(v) => v[index].to_vec(),
        FieldData::Tensor(v) => v[index].to_vec(),
        FieldData::Complex(v) => v[index].to_vec(),
    }
}

#[test]
fn all_field_shapes_and_associations_match_names_order_and_intersect_masks() {
    for missing in [99.0, f64::NAN] {
        let mut a = tetrahedron();
        let mut b = tetrahedron();
        b.positions = vec![
            [0.0, 0.0, 2.0],
            [1.0, 0.0, 2.0],
            [0.0, 1.0, 2.0],
            [0.0, 0.0, 3.0],
        ]
        .into();
        for shape in [1, 3, 6, 2] {
            let name = format!("shape-{shape}");
            let mut first = shaped(
                &name,
                shape,
                &[1.0, 2.0, 3.0, 4.0],
                Some(&[true, false, true, false]),
                missing,
            );
            first.label = "early".into();
            let mut second = shaped(
                &name,
                shape,
                &[5.0, 6.0, 7.0, 8.0],
                Some(&[true, true, false, false]),
                missing,
            );
            second.label = "late".into();
            a.point_data.push(first);
            b.point_data.push(second);
            a.cell_data
                .push(shaped(&name, shape, &[2.0], None, missing).on_cells());
            b.cell_data
                .push(shaped(&name, shape, &[6.0], None, missing).on_cells());
        }
        b.point_data.reverse();
        b.cell_data.reverse();
        let series = TimeSeries::new(vec![a, b], vec![0.0, 1.0]).unwrap();
        let sampler = series.sampler().unwrap();
        let prepared = series.clone().prepare().unwrap();
        for t in [0.25, 0.5, 0.75] {
            for output in [
                series.sample(t).unwrap(),
                sampler.sample(t).unwrap(),
                prepared.sample(t).unwrap(),
            ] {
                output.validate().unwrap();
                assert_eq!(output.positions[0], [0.0, 0.0, 2.0 * t]);
                let near = if t < 0.5 {
                    &series.steps[0]
                } else {
                    &series.steps[1]
                };
                assert_eq!(
                    output
                        .point_data
                        .iter()
                        .map(|f| f.name.as_ref())
                        .collect::<Vec<_>>(),
                    near.point_data
                        .iter()
                        .map(|f| f.name.as_ref())
                        .collect::<Vec<_>>()
                );
                for field in &output.point_data {
                    assert_eq!(field.label.as_ref(), if t < 0.5 { "early" } else { "late" });
                    assert_eq!(field.unit.as_deref(), Some("u"));
                    assert_eq!(
                        field.valid.as_deref(),
                        Some([true, false, false, false].as_slice())
                    );
                    for (component, actual) in row(field, 0).iter().enumerate() {
                        assert!((actual - (1.0 + 4.0 * t + component as f64 * 0.25)).abs() < 1e-12);
                    }
                    for index in 1..4 {
                        assert!(row(field, index).iter().all(|v| v.is_nan()));
                    }
                }
                for field in &output.cell_data {
                    assert!(field.valid.is_none());
                    for (component, actual) in row(field, 0).iter().enumerate() {
                        assert!((actual - (2.0 + 4.0 * t + component as f64 * 0.25)).abs() < 1e-12);
                    }
                }
            }
        }
        for (time, index) in [(-1.0, 0), (0.0, 0), (1.0, 1), (2.0, 1)] {
            let output = sampler.sample(time).unwrap();
            assert!(Arc::ptr_eq(
                &output.positions,
                &series.steps[index].positions
            ));
        }
    }
}

#[test]
fn one_sided_fields_follow_nearest_step_and_large_reordered_schemas_match_by_name() {
    let mut a = tetrahedron();
    let mut b = tetrahedron();
    for i in 0..32 {
        a.point_data
            .push(Field::scalar(format!("f-{i}"), vec![f64::from(i); 4]));
        b.point_data
            .push(Field::scalar(format!("f-{i}"), vec![f64::from(i) + 4.0; 4]));
    }
    a.point_data.push(Field::scalar("only-a", vec![7.0; 4]));
    b.point_data.push(Field::scalar("only-b", vec![9.0; 4]));
    b.point_data.reverse();
    let series = TimeSeries::new(vec![a, b], vec![0.0, 1.0]).unwrap();
    let prepared = series.clone().prepare().unwrap();
    for t in [0.25, 0.5, 0.75] {
        let output = prepared.sample(t).unwrap();
        output.validate().unwrap();
        assert_eq!(
            output.field(Association::Point, "only-a").is_some(),
            t < 0.5
        );
        assert_eq!(
            output.field(Association::Point, "only-b").is_some(),
            t >= 0.5
        );
        for i in 0..32 {
            assert_eq!(
                output
                    .field(Association::Point, &format!("f-{i}"))
                    .unwrap()
                    .as_scalar()
                    .unwrap(),
                &[f64::from(i) + 4.0 * t; 4]
            );
        }
    }
}

#[test]
fn cell_masks_intersect_for_scalar_vector_tensor_and_complex_fields() {
    let geometry = DataSet::new(
        "cells",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [3.0, 0.0, 0.0]],
        Cells::lines(&[[0, 1], [1, 2], [2, 3]]),
    );
    let mut a = geometry.clone();
    let mut b = geometry;
    for shape in [1, 3, 6, 2] {
        a.cell_data.push(
            shaped(
                &format!("f-{shape}"),
                shape,
                &[1.0, 2.0, 3.0],
                Some(&[true, false, true]),
                99.0,
            )
            .on_cells(),
        );
        b.cell_data.push(
            shaped(
                &format!("f-{shape}"),
                shape,
                &[5.0, 6.0, 7.0],
                Some(&[true, true, false]),
                f64::NAN,
            )
            .on_cells(),
        );
    }
    b.cell_data.reverse();
    let series = TimeSeries::new(vec![a, b], vec![0.0, 1.0]).unwrap();
    let prepared = series.prepare().unwrap();
    for t in [0.25, 0.75] {
        let output = prepared.sample(t).unwrap();
        output.validate().unwrap();
        for field in &output.cell_data {
            assert_eq!(
                field.valid.as_deref(),
                Some([true, false, false].as_slice())
            );
            for (component, value) in row(field, 0).iter().enumerate() {
                assert!((value - (1.0 + 4.0 * t + component as f64 * 0.25)).abs() < 1e-12);
            }
            assert!(
                row(field, 1)
                    .iter()
                    .chain(row(field, 2).iter())
                    .all(|v| v.is_nan())
            );
        }
    }
}

#[test]
fn unchanged_payloads_and_subset_masks_share_storage() {
    let a = tetrahedron().with_field(Field::scalar("p", vec![1.0; 4]));
    let mut b = a.clone();
    b.point_data[0].valid = Some(vec![true, false, true, false].into());
    let series = TimeSeries::new(vec![a, b], vec![0.0, 1.0]).unwrap();
    let prepared = series.clone().prepare().unwrap();
    let cloned = prepared.clone();
    assert!(std::ptr::eq(prepared.as_series(), cloned.as_series()));
    for t in [0.25, 0.75] {
        let output = prepared.sample(t).unwrap();
        output.validate().unwrap();
        assert!(Arc::ptr_eq(&output.positions, &series.steps[0].positions));
        let FieldData::Scalar(values) = &output.point_data[0].data else {
            panic!("wrong shape")
        };
        let FieldData::Scalar(original) = &series.steps[0].point_data[0].data else {
            panic!("wrong shape")
        };
        assert!(Arc::ptr_eq(values, original));
        assert!(Arc::ptr_eq(
            output.point_data[0].valid.as_ref().unwrap(),
            series.steps[1].point_data[0].valid.as_ref().unwrap()
        ));
    }
    for (mask_a, mask_b, expected) in [
        (
            vec![true, false, false, true],
            vec![true, true, false, true],
            0,
        ),
        (
            vec![true, true, false, true],
            vec![true, false, false, true],
            1,
        ),
    ] {
        let mut a = tetrahedron().with_field(Field::scalar("p", vec![1.0; 4]));
        a.point_data[0].valid = Some(mask_a.into());
        let mut b = tetrahedron().with_field(Field::scalar("p", vec![3.0; 4]));
        b.point_data[0].valid = Some(mask_b.into());
        let series = TimeSeries::new(vec![a, b], vec![0.0, 1.0]).unwrap();
        let sampler = series.sampler().unwrap();
        for t in [0.25, 0.75] {
            let output = sampler.sample(t).unwrap();
            output.validate().unwrap();
            assert!(Arc::ptr_eq(
                output.point_data[0].valid.as_ref().unwrap(),
                series.steps[expected].point_data[0].valid.as_ref().unwrap()
            ));
        }
    }
    let mut a = tetrahedron().with_field(Field::scalar("p", vec![1.0; 4]));
    let mut b = tetrahedron().with_field(Field::scalar("p", vec![1.0; 4]));
    // Equal values in independent allocations still share after plan preparation.
    a.id = "a".into();
    b.id = "b".into();
    let series = TimeSeries::new(vec![a, b], vec![0.0, 1.0]).unwrap();
    let output = series.sampler().unwrap().sample(0.75).unwrap();
    assert_eq!(output.id.as_ref(), "b");
    assert!(Arc::ptr_eq(&output.positions, &series.steps[0].positions));
    let FieldData::Scalar(values) = &output.point_data[0].data else {
        panic!("wrong shape")
    };
    let FieldData::Scalar(source) = &series.steps[0].point_data[0].data else {
        panic!("wrong shape")
    };
    assert!(Arc::ptr_eq(values, source));
}

#[test]
fn finite_convex_interpolation_handles_extremes_and_subnormals() {
    for (a, b, t, expected) in [
        (-f64::MAX, f64::MAX, 0.5, 0.0),
        (-f64::MAX, f64::MAX, 0.25, -0.5 * f64::MAX),
        (0.0, f64::from_bits(2), 0.5, f64::from_bits(1)),
        (f64::MAX, f64::MAX, 0.5, f64::MAX),
    ] {
        let first = DataSet::new("a", vec![[a, 0.0, 0.0]], Cells::from_cells(&[]))
            .with_field(Field::scalar("p", vec![a]));
        let second = DataSet::new("b", vec![[b, 0.0, 0.0]], Cells::from_cells(&[]))
            .with_field(Field::scalar("p", vec![b]));
        let series = TimeSeries::new(vec![first, second], vec![-f64::MAX, f64::MAX]).unwrap();
        let axis = series.time_axis().unwrap();
        let time = axis.at_fraction(t).unwrap();
        let output = series.sampler().unwrap().sample(time).unwrap();
        output.validate().unwrap();
        for actual in [
            output.positions[0][0],
            output.point_data[0].as_scalar().unwrap()[0],
        ] {
            if expected == 0.0 || expected.is_subnormal() {
                assert_eq!(actual, expected);
            } else {
                assert!(
                    (actual / expected - 1.0).abs() <= 4.0 * f64::EPSILON,
                    "{actual} != {expected}"
                );
            }
        }
    }
}

#[test]
fn ordinary_interpolation_preserves_small_values_near_zero_endpoints() {
    for (a, b, t, expected) in [
        (
            1e150,
            0.0,
            1.0 - f64::EPSILON / 2.0,
            1e150 * (f64::EPSILON / 2.0),
        ),
        (0.0, 1e150, f64::EPSILON / 2.0, 1e150 * (f64::EPSILON / 2.0)),
    ] {
        let first = DataSet::new("a", vec![[a, 0.0, 0.0]], Cells::from_cells(&[]))
            .with_field(Field::scalar("p", vec![a]));
        let second = DataSet::new("b", vec![[b, 0.0, 0.0]], Cells::from_cells(&[]))
            .with_field(Field::scalar("p", vec![b]));
        let prepared = TimeSeries::new(vec![first, second], vec![0.0, 1.0])
            .unwrap()
            .prepare()
            .unwrap();
        let output = prepared.sample(t).unwrap();
        output.validate().unwrap();
        assert_eq!(output.positions[0][0], expected);
        assert_eq!(output.point_data[0].as_scalar().unwrap()[0], expected);
    }
}

#[test]
fn invalid_time_axes_queries_and_incompatible_fields_fail_explicitly() {
    let series = TimeSeries::new(vec![tetrahedron(), tetrahedron()], vec![0.0, 1.0]).unwrap();
    let sampler = series.sampler().unwrap();
    let prepared = series.clone().prepare().unwrap();
    for time in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        series.locate(time).unwrap_err();
        series.sample(time).unwrap_err();
        sampler.sample(time).unwrap_err();
        prepared.sample(time).unwrap_err();
        series.time_axis().unwrap().at_fraction(time).unwrap_err();
    }
    for times in [
        vec![],
        vec![0.0],
        vec![0.0, 0.0],
        vec![1.0, 0.0],
        vec![0.0, f64::NAN],
    ] {
        let mut malformed = series.clone();
        malformed.times = times;
        malformed.locate(0.5).unwrap_err();
        malformed.sample(0.5).unwrap_err();
        malformed.sampler().unwrap_err();
        malformed.prepare().unwrap_err();
    }
    for mismatch in [0, 1, 2] {
        let a = tetrahedron().with_field(Field::scalar("p", vec![1.0; 4]));
        let mut b = tetrahedron().with_field(Field::scalar("p", vec![2.0; 4]));
        match mismatch {
            0 => b.point_data[0].data = FieldData::Vector(vec![[2.0; 3]; 4].into()),
            1 => b.point_data[0].unit = Some("Pa".into()),
            _ => b.point_data[0].data = FieldData::Scalar(vec![2.0].into()),
        }
        let series = TimeSeries {
            steps: vec![a, b],
            times: vec![0.0, 1.0],
        };
        series.sample(0.5).unwrap_err();
        series.sampler().unwrap_err();
        series.prepare().unwrap_err();
    }
    let empty = TimeSeries {
        steps: vec![],
        times: vec![],
    };
    empty.locate(0.0).unwrap_err();
    empty.sample(0.0).unwrap_err();
}

#[test]
fn prepared_ownership_releases_payloads_when_the_last_owner_and_sample_drop() {
    let data = tetrahedron().with_field(Field::scalar("p", vec![1.0; 4]));
    let FieldData::Scalar(values) = &data.point_data[0].data else {
        panic!("wrong shape")
    };
    let weak = Arc::downgrade(values);
    let prepared = TimeSeries::new(vec![data.clone(), data], vec![0.0, 1.0])
        .unwrap()
        .prepare()
        .unwrap();
    let clone = prepared.clone();
    let output = clone.sample(0.5).unwrap();
    drop(prepared);
    drop(clone);
    assert!(weak.upgrade().is_some());
    drop(output);
    assert!(weak.upgrade().is_none());
}
