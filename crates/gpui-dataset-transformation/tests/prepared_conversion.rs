// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    CellToPointLimits, CellToPointPlan, TransformError, cell_data_to_point_data,
};
use gpui_mesh::{Association, CellType, Cells, DataSet, Field, FieldData};
use std::sync::Arc;

fn shaped(name: &str, shape: usize, values: &[f64]) -> Field {
    match shape {
        1 => Field::scalar(name, values.to_vec()),
        3 => Field::vector(name, values.iter().map(|&v| [v; 3]).collect::<Vec<_>>()),
        6 => Field::tensor(name, values.iter().map(|&v| [v; 6]).collect::<Vec<_>>()),
        _ => Field::complex(name, values.iter().map(|&v| [v; 2]).collect::<Vec<_>>()),
    }
    .on_cells()
}

fn data() -> DataSet {
    let cells = Cells::from_cells(&[
        (CellType::Line, &[0, 1]),
        (CellType::Line, &[0, 2]),
        (CellType::Line, &[0, 3]),
        (CellType::Vertex, &[0]),
    ]);
    let mut data = DataSet::new(
        "prepared",
        vec![
            [0.0; 3],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [7.0; 3],
        ],
        cells,
    );
    for (name, shape, mask) in [
        ("s", 1, [true; 4]),
        ("v", 3, [true, true, false, true]),
        ("t", 6, [false, false, true, true]),
        ("z", 2, [true, false, false, true]),
    ] {
        let mut field = shaped(name, shape, &[1e16, 3.0, -1e16, 9.0]);
        field.label = format!("{name} samples").into();
        field.unit = Some("u".into());
        field.valid = Some(mask.to_vec().into());
        data.cell_data.push(field);
    }
    data.with_field(Field::scalar("existing", vec![7.0; 5]))
}

fn bits(field: &Field) -> Vec<u64> {
    match &field.data {
        FieldData::Scalar(v) => v.iter().map(|v| v.to_bits()).collect(),
        FieldData::Vector(v) => v.iter().flatten().map(|v| v.to_bits()).collect(),
        FieldData::Tensor(v) => v.iter().flatten().map(|v| v.to_bits()).collect(),
        FieldData::Complex(v) => v.iter().flatten().map(|v| v.to_bits()).collect(),
    }
}

fn shared(a: &Field, b: &Field) -> bool {
    match (&a.data, &b.data) {
        (FieldData::Scalar(a), FieldData::Scalar(b)) => Arc::ptr_eq(a, b),
        (FieldData::Vector(a), FieldData::Vector(b)) => Arc::ptr_eq(a, b),
        (FieldData::Tensor(a), FieldData::Tensor(b)) => Arc::ptr_eq(a, b),
        (FieldData::Complex(a), FieldData::Complex(b)) => Arc::ptr_eq(a, b),
        _ => false,
    }
}

fn change_first(field: &mut Field) {
    match &mut field.data {
        FieldData::Scalar(v) => Arc::make_mut(v)[0] = 99.0,
        FieldData::Vector(v) => Arc::make_mut(v)[0][0] = 99.0,
        FieldData::Tensor(v) => Arc::make_mut(v)[0][0] = 99.0,
        FieldData::Complex(v) => Arc::make_mut(v)[0][0] = 99.0,
    }
}

#[test]
fn batch_preserves_each_shape_mask_metadata_and_ordered_output_replacement() {
    let data = data();
    let requests = [
        ("s", "shared"),
        ("v", "vec"),
        ("t", "ten"),
        ("z", "cx"),
        ("s", "vec"),
    ];
    let mut expected = data.clone();
    for (source, target) in requests {
        expected = cell_data_to_point_data(&expected, source, target).unwrap();
    }
    let mut plan = CellToPointPlan::new(data.validated().unwrap()).unwrap();
    let actual = plan.convert_fields(&requests).unwrap();
    actual.validate().unwrap();
    assert!(Arc::ptr_eq(&actual.positions, &data.positions));
    assert!(Arc::ptr_eq(
        &actual.cells.connectivity,
        &data.cells.connectivity
    ));
    assert_eq!(actual.point_data.len(), expected.point_data.len());
    for (a, b) in actual.point_data.iter().zip(&expected.point_data) {
        assert_eq!(
            (&a.name, &a.label, &a.unit, a.association),
            (&b.name, &b.label, &b.unit, b.association)
        );
        assert_eq!(a.valid, b.valid);
        assert_eq!(bits(a), bits(b));
    }
    assert_eq!(plan.output_cache_payload_bytes(), 0);
}

#[test]
fn cached_shapes_share_payload_and_masks_without_sharing_mutable_results() {
    let data = data();
    let mut plan = CellToPointPlan::with_limits(
        data.validated().unwrap(),
        CellToPointLimits {
            max_output_cache_bytes: 4096,
            ..Default::default()
        },
    )
    .unwrap();
    for name in ["s", "v", "t", "z"] {
        let mut first = plan.convert_field(name, "first").unwrap();
        let second = plan.convert_field(name, "second").unwrap();
        let a = first.field(Association::Point, "first").unwrap();
        let b = second.field(Association::Point, "second").unwrap();
        assert!(shared(a, b));
        assert!(Arc::ptr_eq(
            a.valid.as_ref().unwrap(),
            b.valid.as_ref().unwrap()
        ));
        let expected = bits(b);
        let charged = b.data.byte_len() as usize + b.valid.as_ref().unwrap().len();
        assert_eq!(plan.output_cache_payload_bytes(), charged);
        change_first(
            first
                .point_data
                .iter_mut()
                .find(|f| f.name.as_ref() == "first")
                .unwrap(),
        );
        let third = plan.convert_field(name, "third").unwrap();
        assert_eq!(
            bits(third.field(Association::Point, "third").unwrap()),
            expected
        );
        assert!(shared(b, third.field(Association::Point, "third").unwrap()));
        let geometry = plan.geometry_payload_bytes();
        plan.clear_output_cache();
        assert_eq!(plan.output_cache_payload_bytes(), 0);
        assert_eq!(plan.geometry_payload_bytes(), geometry);
        let fresh = plan.convert_field(name, "fresh").unwrap();
        fresh.validate().unwrap();
        assert!(!shared(
            b,
            fresh.field(Association::Point, "fresh").unwrap()
        ));
        assert_eq!(
            bits(fresh.field(Association::Point, "fresh").unwrap()),
            expected
        );
    }
}

#[test]
fn payload_limits_include_masks_and_do_not_prevent_uncached_conversion() {
    let data = data();
    let expected = cell_data_to_point_data(&data, "s", "mean").unwrap();
    let mean = expected.field(Association::Point, "mean").unwrap();
    let bytes = mean.data.byte_len() as usize + mean.valid.as_ref().unwrap().len();
    for budget in [0, bytes - 1, bytes] {
        let mut plan = CellToPointPlan::with_limits(
            data.validated().unwrap(),
            CellToPointLimits {
                max_output_cache_bytes: budget,
                ..Default::default()
            },
        )
        .unwrap();
        let a = plan.convert_field("s", "a").unwrap();
        let b = plan.convert_field("s", "b").unwrap();
        assert_eq!(bits(a.field(Association::Point, "a").unwrap()), bits(mean));
        assert_eq!(
            shared(
                a.field(Association::Point, "a").unwrap(),
                b.field(Association::Point, "b").unwrap()
            ),
            budget == bytes
        );
        assert_eq!(
            plan.output_cache_payload_bytes(),
            if budget == bytes { bytes } else { 0 }
        );
    }
    let mut plan = CellToPointPlan::with_limits(
        data.validated().unwrap(),
        CellToPointLimits {
            max_output_cache_bytes: bytes,
            ..Default::default()
        },
    )
    .unwrap();
    let first = plan.convert_field("s", "s").unwrap();
    let FieldData::Scalar(values) = &first.point_data.last().unwrap().data else {
        panic!("expected scalar output");
    };
    let weak = Arc::downgrade(values);
    drop(first);
    assert!(weak.upgrade().is_some());
    // An oversized miss must release the old entry even though the new output
    // cannot be retained. No returned dataset keeps the old payload alive.
    let oversized = plan.convert_field("v", "v").unwrap();
    assert_eq!(plan.output_cache_payload_bytes(), 0);
    assert!(weak.upgrade().is_none());
    drop(oversized);
    let next = plan.convert_field("s", "next").unwrap();
    let FieldData::Scalar(values) = &next.point_data.last().unwrap().data else {
        panic!("expected scalar output");
    };
    let weak = Arc::downgrade(values);
    drop(next);
    assert!(weak.upgrade().is_some());
    plan.clear_output_cache();
    assert!(weak.upgrade().is_none());
}

#[test]
fn preparation_limits_cover_bitmap_to_dense_peak_before_allocating() {
    let data = DataSet::new(
        "promotion",
        vec![[0.0; 3], [0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        Cells::from_cells(&[
            (CellType::Line, &[0, 1]),
            (CellType::Line, &[0, 2]),
            (CellType::Line, &[0, 3]),
        ]),
    )
    .with_field(Field::scalar("p", vec![1.0, 2.0, 3.0]).on_cells());
    for (limit, required) in [(0, 8), (7, 8), (8, 32), (31, 32)] {
        let result = CellToPointPlan::with_limits(
            data.validated().unwrap(),
            CellToPointLimits {
                max_geometry_bytes: limit,
                max_output_cache_bytes: 0,
            },
        );
        assert!(
            matches!(result,Err(TransformError::ResourceLimit {required:actual,limit:actual_limit,..}) if actual==required && actual_limit==limit)
        );
    }
    let mut plan = CellToPointPlan::with_limits(
        data.validated().unwrap(),
        CellToPointLimits {
            max_geometry_bytes: 32,
            max_output_cache_bytes: 0,
        },
    )
    .unwrap();
    assert_eq!(plan.geometry_payload_bytes(), 24);
    let out = plan.convert_field("p", "mean").unwrap();
    out.validate().unwrap();
    assert_eq!(
        out.field(Association::Point, "mean")
            .unwrap()
            .as_scalar()
            .unwrap()[0],
        8.0 / 3.0
    );
}

#[test]
fn rebuilding_after_geometry_value_and_mask_changes_uses_the_new_snapshot() {
    let mut data = DataSet::new(
        "snapshots",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        Cells::from_cells(&[(CellType::Line, &[0, 1]), (CellType::Line, &[0, 2])]),
    )
    .with_field(Field::scalar("p", vec![2.0, 4.0]).on_cells());
    let old = {
        let mut plan = CellToPointPlan::new(data.validated().unwrap()).unwrap();
        plan.convert_field("p", "mean").unwrap()
    };
    Arc::make_mut(&mut data.positions)[2][0] = 4.0;
    let changed = {
        let mut plan = CellToPointPlan::new(data.validated().unwrap()).unwrap();
        plan.convert_field("p", "mean").unwrap()
    };
    assert_eq!(
        changed
            .field(Association::Point, "mean")
            .unwrap()
            .as_scalar()
            .unwrap()[0],
        18.0 / 5.0
    );
    data.cell_data[0] = Field::scalar("p", vec![2.0, 8.0]).on_cells();
    let changed = {
        let mut plan = CellToPointPlan::new(data.validated().unwrap()).unwrap();
        plan.convert_field("p", "mean").unwrap()
    };
    assert_eq!(
        changed
            .field(Association::Point, "mean")
            .unwrap()
            .as_scalar()
            .unwrap()[0],
        34.0 / 5.0
    );
    data.cell_data[0].valid = Some(vec![true, false].into());
    let changed = {
        let mut plan = CellToPointPlan::new(data.validated().unwrap()).unwrap();
        plan.convert_field("p", "mean").unwrap()
    };
    assert_eq!(
        changed
            .field(Association::Point, "mean")
            .unwrap()
            .as_scalar()
            .unwrap()[0],
        2.0
    );
    assert!(
        !changed
            .field(Association::Point, "mean")
            .unwrap()
            .is_valid(2)
    );
    assert_eq!(
        old.field(Association::Point, "mean")
            .unwrap()
            .as_scalar()
            .unwrap()[0],
        10.0 / 3.0
    );
}

#[test]
fn missing_fields_and_empty_batches_preserve_the_cached_snapshot() {
    let data = data();
    let mut plan = CellToPointPlan::with_limits(
        data.validated().unwrap(),
        CellToPointLimits {
            max_output_cache_bytes: 4096,
            ..Default::default()
        },
    )
    .unwrap();
    let first = plan.convert_field("s", "first").unwrap();
    let bytes = plan.output_cache_payload_bytes();
    assert!(
        matches!(plan.convert_fields(&[("v","v"),("missing","bad")]),Err(TransformError::FieldNotFound(name)) if name=="missing")
    );
    assert_eq!(plan.output_cache_payload_bytes(), bytes);
    assert!(matches!(
        plan.convert_field("existing", "bad"),
        Err(TransformError::FieldNotFound(name)) if name == "existing"
    ));
    let empty = plan.convert_fields(&[]).unwrap();
    assert_eq!(empty.point_data.len(), data.point_data.len());
    let again = plan.convert_field("s", "again").unwrap();
    assert!(shared(
        first.field(Association::Point, "first").unwrap(),
        again.field(Association::Point, "again").unwrap()
    ));
}
