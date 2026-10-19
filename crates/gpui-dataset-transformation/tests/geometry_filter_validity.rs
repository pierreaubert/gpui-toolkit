// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    ThresholdMode, append_datasets, clip_sphere, decimate, threshold,
};
use gpui_mesh::{Association, CellType, Cells, DataSet, Field, FieldData};
use std::sync::Arc;

fn shaped(name: &str, shape: usize, values: &[f64], mask: Option<&[bool]>) -> Field {
    let mut field = match shape {
        1 => Field::scalar(name, values.to_vec()),
        3 => Field::vector(name, values.iter().map(|&v| [v; 3]).collect::<Vec<_>>()),
        6 => Field::tensor(name, values.iter().map(|&v| [v; 6]).collect::<Vec<_>>()),
        _ => Field::complex(name, values.iter().map(|&v| [v; 2]).collect::<Vec<_>>()),
    };
    field.valid = mask.map(Arc::from);
    field.label = format!("Label {name}").into();
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

fn vertices(points: Vec<[f64; 3]>) -> DataSet {
    let ids = (0..points.len() as u32).map(|i| [i]).collect::<Vec<_>>();
    let cells = ids
        .iter()
        .map(|id| (CellType::Vertex, id.as_slice()))
        .collect::<Vec<_>>();
    DataSet::new("vertices", points, Cells::from_cells(&cells))
}

#[test]
fn decimation_averages_supported_shapes_and_subsets_cell_masks() {
    let mut data = DataSet::new(
        "clusters",
        vec![
            [0.1, 0.0, 0.0],
            [0.2, 0.0, 0.0],
            [1.1, 0.0, 0.0],
            [1.2, 0.0, 0.0],
            [2.1, 0.0, 0.0],
            [2.2, 0.0, 0.0],
        ],
        Cells::from_cells(&[
            (CellType::Line, &[0, 1]),
            (CellType::Line, &[1, 2]),
            (CellType::Triangle, &[0, 2, 4]),
            (CellType::Vertex, &[5]),
        ]),
    );
    for shape in [1, 3, 6, 2] {
        let name = format!("f-{shape}");
        data.point_data.push(shaped(
            &name,
            shape,
            &[2.0, 99.0, 4.0, 8.0, 99.0, f64::NAN],
            Some(&[true, false, true, true, false, false]),
        ));
        data.cell_data.push(
            shaped(
                &name,
                shape,
                &[10.0, 99.0, 30.0, 40.0],
                Some(&[true, false, true, true]),
            )
            .on_cells(),
        );
    }
    let out = decimate(&data, 1.0).unwrap();
    out.validate().unwrap();
    assert_eq!(out.point_count(), 3);
    assert_eq!(out.cell_count(), 3);
    assert_eq!(
        out.cells.types.as_ref(),
        [CellType::Line, CellType::Triangle, CellType::Vertex]
    );
    assert_eq!(out.cells.cell(0).unwrap(), [0, 1]);
    assert_eq!(out.cells.cell(1).unwrap(), [0, 1, 2]);
    assert_eq!(out.cells.cell(2).unwrap(), [2]);
    for (position, expected) in out.positions.iter().zip([0.15, 1.15, 2.15]) {
        assert!((position[0] - expected).abs() < 1e-14);
    }
    for field in &out.point_data {
        assert_eq!(field.valid.as_deref(), Some([true, true, false].as_slice()));
        assert!(row(field, 0).iter().all(|&v| v == 2.0));
        assert!(row(field, 1).iter().all(|&v| v == 6.0));
        assert!(row(field, 2).iter().all(|v| v.is_nan()));
        assert_eq!(field.unit.as_deref(), Some("u"));
    }
    for field in &out.cell_data {
        assert_eq!(field.data.len(), 3);
        assert_eq!(field.valid.as_deref(), Some([false, true, true].as_slice()));
        assert!(row(field, 1).iter().all(|&v| v == 30.0));
        assert!(row(field, 2).iter().all(|&v| v == 40.0));
    }
    assert_eq!(data.point_count(), 6);
    assert_eq!(data.cell_count(), 4);
}

#[test]
fn decimation_uses_finite_compensated_and_exact_cluster_means() {
    for (values, expected) in [
        (vec![f64::MAX; 2], f64::MAX),
        (
            vec![f64::MAX, f64::MAX, -f64::MAX, -f64::MAX, 1e-300],
            1e-300 / 5.0,
        ),
        (vec![1e16, 3.0, -1e16], 1.0),
        (vec![f64::from_bits(1); 3], f64::from_bits(1)),
    ] {
        let mut data = vertices(vec![[0.0; 3]; values.len()]);
        for shape in [1, 3, 6, 2] {
            data.point_data
                .push(shaped(&format!("f-{shape}"), shape, &values, None));
        }
        let out = decimate(&data, 1.0).unwrap();
        out.validate().unwrap();
        assert_eq!(out.point_count(), 1);
        assert_eq!(out.cell_count(), values.len());
        for field in &out.point_data {
            for value in row(field, 0) {
                assert_eq!(value.to_bits(), expected.to_bits());
            }
        }
    }
    let data = vertices(vec![[f64::MAX, 0.0, 0.0]; 2]);
    assert_eq!(
        decimate(&data, f64::MAX).unwrap().positions.as_ref(),
        [[f64::MAX, 0.0, 0.0]]
    );
}

#[test]
fn decimation_never_saturates_large_grid_coordinates_or_infinite_quotients() {
    for (first, second, size) in [
        (1e20, f64::from_bits(1e20_f64.to_bits() + 1), 1.0),
        (-f64::from_bits(1), f64::from_bits(1), f64::MAX),
        (
            1.0,
            f64::from_bits(1.0_f64.to_bits() + 1),
            f64::from_bits(1),
        ),
    ] {
        let data = vertices(vec![[first, 0.0, 0.0], [second, 0.0, 0.0]])
            .with_field(Field::scalar("p", vec![1.0, 2.0]));
        let out = decimate(&data, size).unwrap();
        out.validate().unwrap();
        assert_eq!(out.point_count(), 2);
        assert!(Arc::ptr_eq(&out.positions, &data.positions));
        assert!(Arc::ptr_eq(
            &out.cells.connectivity,
            &data.cells.connectivity
        ));
        let (FieldData::Scalar(a), FieldData::Scalar(b)) =
            (&out.point_data[0].data, &data.point_data[0].data)
        else {
            panic!()
        };
        assert!(Arc::ptr_eq(a, b));
    }
    let out = decimate(&vertices(vec![[-0.0; 3], [0.0; 3]]), 1.0).unwrap();
    assert_eq!(out.point_count(), 1);
    assert_eq!(
        decimate(&vertices(vec![[-0.25, 0.0, 0.0], [0.25, 0.0, 0.0]]), 1.0)
            .unwrap()
            .point_count(),
        2
    );
}

#[test]
fn decimation_preserves_vertices_and_handles_large_collapsed_polygons() {
    let nodes = (0..32).collect::<Vec<u32>>();
    let data = DataSet::new(
        "polygon",
        (0..32)
            .map(|i| [f64::from(i) * 2.0, 0.0, 0.0])
            .collect::<Vec<_>>(),
        Cells::from_cells(&[(CellType::Polygon, &nodes), (CellType::Vertex, &[0])]),
    )
    .with_field(Field::tensor("cells", vec![[1.0; 6], [2.0; 6]]).on_cells());
    let out = decimate(&data, 1.0).unwrap();
    out.validate().unwrap();
    assert_eq!(out.cell_count(), 2);
    let out = decimate(&data, 1000.0).unwrap();
    out.validate().unwrap();
    assert_eq!(out.cell_count(), 1);
    assert_eq!(out.cells.types[0], CellType::Vertex);
    assert_eq!(row(&out.cell_data[0], 0), [2.0; 6]);
}

#[test]
fn append_preserves_all_shapes_masks_names_and_missing_sides() {
    let mut a = vertices(vec![[0.0; 3], [1.0, 0.0, 0.0]]);
    let mut b = vertices(vec![[2.0, 0.0, 0.0], [3.0, 0.0, 0.0], [4.0, 0.0, 0.0]]);
    for shape in [1, 3, 6, 2] {
        for association in [Association::Point, Association::Cell] {
            let mut shared_a = shaped(
                &format!("shared-{shape}"),
                shape,
                &[1.0, 99.0],
                Some(&[true, false]),
            );
            shared_a.association = association;
            let mut shared_b = shaped(
                &format!("shared-{shape}"),
                shape,
                &[f64::NAN, 4.0, 5.0],
                Some(&[false, true, true]),
            );
            shared_b.association = association;
            shared_b.label = "Later".into();
            let mut only_a = shaped(&format!("a-{shape}"), shape, &[7.0, 8.0], None);
            only_a.association = association;
            let mut only_b = shaped(&format!("b-{shape}"), shape, &[2.0, 3.0, 4.0], None);
            only_b.association = association;
            match association {
                Association::Point => {
                    a.point_data.extend([shared_a, only_a]);
                    b.point_data.extend([shared_b, only_b]);
                }
                Association::Cell => {
                    a.cell_data.extend([shared_a, only_a]);
                    b.cell_data.extend([shared_b, only_b]);
                }
            }
        }
    }
    b.point_data.reverse();
    b.cell_data.reverse();
    let out = append_datasets(&a, &b).unwrap();
    out.validate().unwrap();
    assert_eq!(out.point_count(), 5);
    assert_eq!(out.cell_count(), 5);
    assert_eq!(out.cells.cell(4).unwrap(), [4]);
    for association in [Association::Point, Association::Cell] {
        for shape in [1, 3, 6, 2] {
            let field = out.field(association, &format!("shared-{shape}")).unwrap();
            assert_eq!(
                field.valid.as_deref(),
                Some([true, false, false, true, true].as_slice())
            );
            assert!(row(field, 0).iter().all(|&v| v == 1.0));
            assert!(row(field, 4).iter().all(|&v| v == 5.0));
            assert_eq!(field.label.as_ref(), format!("Label shared-{shape}"));
            let first = out.field(association, &format!("a-{shape}")).unwrap();
            assert_eq!(
                first.valid.as_deref(),
                Some([true, true, false, false, false].as_slice())
            );
            assert!(row(first, 4).iter().all(|v| v.is_nan()));
            let second = out.field(association, &format!("b-{shape}")).unwrap();
            assert_eq!(
                second.valid.as_deref(),
                Some([false, false, true, true, true].as_slice())
            );
            assert!(row(second, 0).iter().all(|v| v.is_nan()));
        }
    }
}

#[test]
fn append_shares_empty_side_storage_and_handles_empty_schema_definitions() {
    let empty = DataSet::new("empty", vec![], Cells::from_cells(&[]));
    let data = vertices(vec![[0.0; 3], [1.0, 0.0, 0.0]]).with_field(shaped(
        "p",
        1,
        &[1.0, f64::NAN],
        Some(&[true, false]),
    ));
    for out in [
        append_datasets(&empty, &data).unwrap(),
        append_datasets(&data, &empty).unwrap(),
    ] {
        out.validate().unwrap();
        assert!(Arc::ptr_eq(&out.positions, &data.positions));
        assert!(Arc::ptr_eq(
            &out.cells.connectivity,
            &data.cells.connectivity
        ));
        let (FieldData::Scalar(a), FieldData::Scalar(b)) =
            (&out.point_data[0].data, &data.point_data[0].data)
        else {
            panic!()
        };
        assert!(Arc::ptr_eq(a, b));
        assert!(Arc::ptr_eq(
            out.point_data[0].valid.as_ref().unwrap(),
            data.point_data[0].valid.as_ref().unwrap()
        ));
    }
    let mut schema = empty.clone();
    for shape in [1, 3, 6, 2] {
        schema
            .point_data
            .push(shaped(&format!("missing-{shape}"), shape, &[], None));
    }
    for out in [
        append_datasets(&schema, &data).unwrap(),
        append_datasets(&data, &schema).unwrap(),
    ] {
        out.validate().unwrap();
        for shape in [1, 3, 6, 2] {
            let field = out
                .field(Association::Point, &format!("missing-{shape}"))
                .unwrap();
            assert_eq!(field.valid.as_deref(), Some([false, false].as_slice()));
            assert!(row(field, 0).iter().all(|v| v.is_nan()));
        }
    }
    let pointcloud = DataSet::new("unconnected", vec![[2.0; 3]], Cells::from_cells(&[]));
    let out = append_datasets(&pointcloud, &data).unwrap();
    out.validate().unwrap();
    assert_eq!(out.cells.cell(0).unwrap(), [1]);
    assert_eq!(out.cells.cell(1).unwrap(), [2]);
}

#[test]
fn append_matches_large_reordered_schemas_and_rejects_incompatible_units() {
    let mut a = vertices(vec![[0.0; 3]]);
    let mut b = vertices(vec![[1.0; 3]]);
    for i in 0..40 {
        a.point_data
            .push(Field::scalar(format!("f-{i}"), vec![f64::from(i)]));
        b.point_data
            .push(Field::scalar(format!("f-{i}"), vec![f64::from(i) + 100.0]));
    }
    b.point_data.reverse();
    let out = append_datasets(&a, &b).unwrap();
    out.validate().unwrap();
    for i in 0..40 {
        assert_eq!(
            out.field(Association::Point, &format!("f-{i}"))
                .unwrap()
                .as_scalar()
                .unwrap(),
            [f64::from(i), f64::from(i) + 100.0]
        );
    }
    b.point_data[0].unit = Some("incompatible".into());
    append_datasets(&a, &b).unwrap_err();
    b.point_data[0].unit = None;
    b.point_data[0].data = FieldData::Vector(vec![[1.0; 3]].into());
    append_datasets(&a, &b).unwrap_err();
}

#[test]
fn packed_selection_preserves_first_use_order_and_identity_storage() {
    let mut data = DataSet::surface(
        "selection",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
        &[[2, 0, 1], [2, 1, 3]],
    )
    .with_field(Field::scalar("select", vec![0.0, 1.0]).on_cells());
    for shape in [1, 3, 6, 2] {
        data.point_data.push(shaped(
            &format!("f-{shape}"),
            shape,
            &[1.0, 99.0, 3.0, 4.0],
            Some(&[true, false, true, true]),
        ));
    }
    let out = threshold(
        &data,
        "select",
        Association::Cell,
        1.0,
        1.0,
        ThresholdMode::All,
    )
    .unwrap();
    out.validate().unwrap();
    assert_eq!(
        out.positions.as_ref(),
        [data.positions[2], data.positions[1], data.positions[3]]
    );
    assert_eq!(out.cells.cell(0).unwrap(), [0, 1, 2]);
    for field in &out.point_data {
        assert_eq!(field.valid.as_deref(), Some([true, false, true].as_slice()));
        assert!(row(field, 0).iter().all(|&v| v == 3.0));
    }
    let identity = vertices(vec![[0.0; 3], [1.0; 3]]).with_field(Field::scalar("p", vec![1.0; 2]));
    let out = threshold(
        &identity,
        "p",
        Association::Point,
        f64::NEG_INFINITY,
        f64::INFINITY,
        ThresholdMode::All,
    )
    .unwrap();
    assert!(Arc::ptr_eq(&out.positions, &identity.positions));
    assert!(Arc::ptr_eq(
        &out.cells.connectivity,
        &identity.cells.connectivity
    ));
    let (FieldData::Scalar(a), FieldData::Scalar(b)) =
        (&out.point_data[0].data, &identity.point_data[0].data)
    else {
        panic!()
    };
    assert!(Arc::ptr_eq(a, b));
}

#[test]
fn sphere_selection_preserves_reserved_names_and_extreme_distance_ordering() {
    let data = vertices(vec![[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]]).with_field(shaped(
        "__sphere_distance",
        1,
        &[99.0, 2.0, 3.0],
        Some(&[false, true, true]),
    ));
    for (inside, expected) in [
        (true, vec![[0.0; 3], [1.0, 0.0, 0.0]]),
        (false, vec![[1.0, 0.0, 0.0], [2.0, 0.0, 0.0]]),
    ] {
        let out = clip_sphere(&data, [0.0; 3], 1.0, inside, ThresholdMode::All).unwrap();
        out.validate().unwrap();
        assert_eq!(out.positions.as_ref(), expected);
        assert!(out.field(Association::Point, "__sphere_distance").is_some());
    }
    for (position, radius, inside) in [
        ([f64::MAX, 0.0, 0.0], f64::MAX, true),
        ([f64::from_bits(3), 0.0, 0.0], f64::from_bits(2), false),
    ] {
        let out = clip_sphere(
            &vertices(vec![position]),
            [0.0; 3],
            radius,
            inside,
            ThresholdMode::All,
        )
        .unwrap();
        assert_eq!(out.point_count(), 1);
    }
    let out = clip_sphere(
        &vertices(vec![[f64::MAX, 0.0, 0.0]]),
        [-f64::MAX, 0.0, 0.0],
        f64::MAX,
        false,
        ThresholdMode::All,
    )
    .unwrap();
    assert_eq!(out.point_count(), 1);
    assert_eq!(
        clip_sphere(&data, [0.0; 3], 0.0, true, ThresholdMode::All)
            .unwrap()
            .point_count(),
        1
    );
}

#[test]
fn geometry_filters_reject_invalid_inputs_and_accept_empty_data() {
    let data = vertices(vec![[0.0; 3]]).with_field(Field::scalar("p", vec![1.0]));
    for size in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        decimate(&data, size).unwrap_err();
    }
    for radius in [-1.0, f64::NAN, f64::INFINITY] {
        clip_sphere(&data, [0.0; 3], radius, true, ThresholdMode::Any).unwrap_err();
    }
    clip_sphere(&data, [f64::NAN; 3], 1.0, true, ThresholdMode::Any).unwrap_err();
    for (lower, upper) in [(f64::NAN, 1.0), (2.0, 1.0)] {
        threshold(
            &data,
            "p",
            Association::Point,
            lower,
            upper,
            ThresholdMode::All,
        )
        .unwrap_err();
    }
    let mut invalid = data.clone();
    invalid.point_data[0].valid = Some(vec![].into());
    decimate(&invalid, 1.0).unwrap_err();
    append_datasets(&data, &invalid).unwrap_err();
    clip_sphere(&invalid, [0.0; 3], 1.0, true, ThresholdMode::All).unwrap_err();
    let empty = DataSet::new("empty", vec![], Cells::from_cells(&[]))
        .with_field(Field::complex("p", vec![]));
    decimate(&empty, 1.0).unwrap().validate().unwrap();
    append_datasets(&empty, &empty).unwrap().validate().unwrap();
    clip_sphere(&empty, [0.0; 3], 1.0, true, ThresholdMode::All)
        .unwrap()
        .validate()
        .unwrap();
}
