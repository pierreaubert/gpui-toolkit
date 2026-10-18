//! Scientific missing-value contracts across public transformations.

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    ThresholdMode, cell_data_to_point_data, cell_data_to_point_data_validated, complex_magnitude,
    complex_phase, isosurface, point_data_to_cell_data, point_data_to_cell_data_validated, probe,
    threshold, vector_magnitude,
};
use gpui_mesh::{Association, CellType, Cells, DataSet, Field, FieldData};

fn tetrahedron() -> DataSet {
    DataSet::new(
        "tet",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        Cells::from_cells(&[(CellType::Tetra, &[0, 1, 2, 3])]),
    )
}

fn component_fields(values: &[f64], association: Association) -> Vec<Field> {
    let mut fields = vec![
        Field::scalar("scalar", values.to_vec()),
        Field::vector(
            "vector",
            values
                .iter()
                .map(|v| [*v, 2.0 * v, 3.0 * v])
                .collect::<Vec<_>>(),
        ),
        Field::tensor(
            "tensor",
            values
                .iter()
                .map(|v| std::array::from_fn(|axis| (axis + 1) as f64 * v))
                .collect::<Vec<_>>(),
        ),
        Field::complex(
            "complex",
            values.iter().map(|v| [*v, 2.0 * v]).collect::<Vec<_>>(),
        ),
    ];
    for field in &mut fields {
        field.association = association;
        field.unit = Some("Pa".into());
    }
    fields
}

fn normalized_components(field: &Field, index: usize) -> Vec<f64> {
    let components: Vec<f64> = match &field.data {
        FieldData::Scalar(values) => vec![values[index]],
        FieldData::Vector(values) => values[index].to_vec(),
        FieldData::Tensor(values) => values[index].to_vec(),
        FieldData::Complex(values) => values[index].to_vec(),
    };
    components
        .into_iter()
        .enumerate()
        .map(|(axis, value)| value / (axis + 1) as f64)
        .collect()
}

#[test]
fn point_to_cell_averages_only_valid_nodes_in_every_component_shape() {
    let mut dataset = tetrahedron();
    dataset.point_data = component_fields(&[1.0, 999.0, f64::NAN, 3.0], Association::Point);
    for field in &mut dataset.point_data {
        field.valid = Some(vec![true, false, false, true].into());
    }
    dataset.validate().unwrap();
    for source in &dataset.point_data {
        let output = point_data_to_cell_data(&dataset, &source.name, "mean").unwrap();
        output.validate().unwrap();
        let mean = output.field(Association::Cell, "mean").unwrap();
        assert!(mean.is_valid(0));
        assert!(
            normalized_components(mean, 0)
                .iter()
                .all(|&value| value == 2.0)
        );
        assert_eq!(mean.unit.as_deref(), Some("Pa"));
        let repeated = point_data_to_cell_data(&output, &source.name, "mean").unwrap();
        assert_eq!(
            repeated
                .cell_data
                .iter()
                .filter(|field| field.name.as_ref() == "mean")
                .count(),
            1
        );
    }
    for field in &mut dataset.point_data {
        field.valid = Some(vec![false; 4].into());
    }
    for source in &dataset.point_data {
        let output = point_data_to_cell_data(&dataset, &source.name, "mean").unwrap();
        output.validate().unwrap();
        let mean = output.field(Association::Cell, "mean").unwrap();
        assert!(!mean.is_valid(0));
        assert!(
            normalized_components(mean, 0)
                .iter()
                .all(|value| value.is_nan())
        );
    }
    let huge = tetrahedron().with_field(Field::scalar("p", vec![f64::MAX; 4]));
    let output = point_data_to_cell_data(&huge, "p", "mean").unwrap();
    assert_eq!(output.cell_data[0].as_scalar().unwrap(), &[f64::MAX]);
}

#[test]
fn cell_to_point_uses_valid_positive_measure_and_leaves_unsupported_points_missing() {
    let mut dataset = DataSet::surface(
        "weighted",
        vec![
            [0.0; 3],
            [1.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, -6.0, 0.0],
            [9.0; 3],
        ],
        &[[0, 1, 2], [0, 3, 4]],
    );
    dataset.cell_data = component_fields(&[2.0, 10.0], Association::Cell);
    for source in &dataset.cell_data {
        let output = cell_data_to_point_data(&dataset, &source.name, "mean").unwrap();
        output.validate().unwrap();
        let mean = output.field(Association::Point, "mean").unwrap();
        for (index, expected) in [8.0, 2.0, 2.0, 10.0, 10.0].into_iter().enumerate() {
            assert!(
                normalized_components(mean, index)
                    .iter()
                    .all(|value| (value - expected).abs() < 1e-12)
            );
        }
        assert!(!mean.is_valid(5));
        assert_eq!(mean.unit.as_deref(), Some("Pa"));
        let repeated = cell_data_to_point_data(&output, &source.name, "mean").unwrap();
        assert_eq!(
            repeated
                .point_data
                .iter()
                .filter(|field| field.name.as_ref() == "mean")
                .count(),
            1
        );
    }
    for placeholder in [999.0, f64::NAN] {
        dataset.cell_data = component_fields(&[2.0, placeholder], Association::Cell);
        for field in &mut dataset.cell_data {
            field.valid = Some(vec![true, false].into());
        }
        for source in &dataset.cell_data {
            let output = cell_data_to_point_data(&dataset, &source.name, "mean").unwrap();
            output.validate().unwrap();
            let mean = output.field(Association::Point, "mean").unwrap();
            for index in 0..3 {
                assert!(
                    normalized_components(mean, index)
                        .iter()
                        .all(|&value| value == 2.0)
                );
            }
            assert!((3..6).all(|index| !mean.is_valid(index)));
        }
    }
    let degenerate = DataSet::surface(
        "zero-area",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        &[[0, 1, 2]],
    )
    .with_field(Field::scalar("p", vec![99.0]).on_cells());
    let output = cell_data_to_point_data(&degenerate, "p", "mean").unwrap();
    output.validate().unwrap();
    assert!((0..3).all(|index| !output.point_data[0].is_valid(index)));
    for field in &mut dataset.cell_data {
        field.valid = Some(vec![false; 2].into());
    }
    for source in &dataset.cell_data {
        let output = cell_data_to_point_data(&dataset, &source.name, "mean").unwrap();
        output.validate().unwrap();
        assert!((0..6).all(|index| !output.point_data[0].is_valid(index)));
    }
}

#[test]
fn cell_means_use_the_highest_supported_dimension_at_each_point_in_either_order() {
    let positions = vec![
        [0.0; 3],
        [1.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, -6.0, 0.0],
        [9.0; 3],
    ];
    let cells = [
        (CellType::Vertex, &[0][..]),
        (CellType::Line, &[0, 1][..]),
        (CellType::Triangle, &[0, 1, 2][..]),
        (CellType::Triangle, &[0, 3, 4][..]),
        (CellType::Vertex, &[5][..]),
    ];
    for reversed in [false, true] {
        let mut cells = cells.to_vec();
        let mut values = vec![999.0, 777.0, 2.0, 10.0, 42.0];
        if reversed {
            cells.reverse();
            values.reverse();
        }
        let dataset = DataSet::new("mixed", positions.clone(), Cells::from_cells(&cells))
            .with_field(Field::scalar("p", values).on_cells());
        let output = cell_data_to_point_data(&dataset, "p", "mean").unwrap();
        output.validate().unwrap();
        let values = output.point_data[0].as_scalar().unwrap();
        for (&actual, expected) in values.iter().zip([8.0, 2.0, 2.0, 10.0, 10.0, 42.0]) {
            assert!((actual - expected).abs() < 1e-12);
        }
    }
    let lines = DataSet::new(
        "lines",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [-3.0, 0.0, 0.0]],
        Cells::from_cells(&[(CellType::Line, &[0, 1]), (CellType::Line, &[0, 2])]),
    )
    .with_field(Field::scalar("p", vec![2.0, 10.0]).on_cells());
    assert!(
        (cell_data_to_point_data(&lines, "p", "mean")
            .unwrap()
            .point_data[0]
            .as_scalar()
            .unwrap()[0]
            - 8.0)
            .abs()
            < 1e-12
    );
}

#[test]
fn conversion_weights_handle_concave_polygons_and_extreme_representable_measures() {
    let positions = vec![
        [0.0; 3],
        [3.0, 0.0, 0.0],
        [3.0, 3.0, 0.0],
        [2.0, 3.0, 0.0],
        [2.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, 3.0, 0.0],
        [0.0, 3.0, 0.0],
        [0.0, -2.0, 0.0],
    ];
    for polygon in [vec![0, 1, 2, 3, 4, 5, 6, 7], vec![7, 6, 5, 4, 3, 2, 1, 0]] {
        let dataset = DataSet::new(
            "concave",
            positions.clone(),
            Cells::from_cells(&[
                (CellType::Polygon, &polygon),
                (CellType::Triangle, &[0, 1, 8]),
            ]),
        )
        .with_field(Field::scalar("p", vec![10.0, 2.0]).on_cells());
        let output = cell_data_to_point_data(&dataset, "p", "mean").unwrap();
        assert!((output.point_data[0].as_scalar().unwrap()[0] - 7.6).abs() < 1e-12);
    }
    for scale in [1e-100, 1e100] {
        let points: Vec<_> = [
            [0.0; 3],
            [1.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, -6.0, 0.0],
        ]
        .into_iter()
        .map(|point| point.map(|value| value * scale))
        .collect();
        let dataset = DataSet::surface("scaled", points, &[[0, 1, 2], [0, 3, 4]])
            .with_field(Field::scalar("p", vec![2.0, 10.0]).on_cells());
        assert!(
            (cell_data_to_point_data(&dataset, "p", "mean")
                .unwrap()
                .point_data[0]
                .as_scalar()
                .unwrap()[0]
                - 8.0)
                .abs()
                < 1e-12
        );
    }
    let dataset = DataSet::surface(
        "large-measures",
        vec![
            [0.0; 3],
            [2e154, 0.0, 0.0],
            [0.0, 1e154, 0.0],
            [-2e154, 0.0, 0.0],
            [0.0, -1e154, 0.0],
        ],
        &[[0, 1, 2], [0, 3, 4]],
    )
    .with_field(Field::scalar("p", vec![f64::MAX; 2]).on_cells());
    let output = cell_data_to_point_data(&dataset, "p", "mean").unwrap();
    output.validate().unwrap();
    assert_eq!(output.point_data[0].as_scalar().unwrap(), &[f64::MAX; 5]);
}

#[test]
fn field_association_and_names_are_validated_before_conversions_index_data() {
    let mut misplaced = tetrahedron();
    misplaced
        .point_data
        .push(Field::scalar("p", vec![1.0]).on_cells());
    assert!(matches!(
        misplaced.validate(),
        Err(gpui_mesh::MeshError::FieldAssociationMismatch { .. })
    ));
    assert!(point_data_to_cell_data(&misplaced, "p", "mean").is_err());
    misplaced.point_data.clear();
    misplaced.cell_data.push(Field::scalar("p", vec![1.0; 4]));
    assert!(matches!(
        misplaced.validate(),
        Err(gpui_mesh::MeshError::FieldAssociationMismatch { .. })
    ));
    assert!(cell_data_to_point_data(&misplaced, "p", "mean").is_err());
    let mut names = tetrahedron()
        .with_field(Field::scalar("p", vec![1.0; 4]))
        .with_field(Field::scalar("p", vec![1.0]).on_cells());
    names.validate().unwrap();
    names.point_data.push(Field::scalar("p", vec![2.0; 4]));
    assert!(matches!(
        names.validate(),
        Err(gpui_mesh::MeshError::DuplicateField {
            association: Association::Point,
            ..
        })
    ));
    let mut wide = DataSet::new("metadata", Vec::<[f64; 3]>::new(), Cells::from_cells(&[]));
    for index in 0..32 {
        wide.point_data
            .push(Field::scalar(format!("field-{index}"), Vec::<f64>::new()));
    }
    wide.validate().unwrap();
    let duplicate = wide.point_data[0].name.clone();
    wide.point_data.last_mut().unwrap().name = duplicate;
    assert!(matches!(
        wide.validate(),
        Err(gpui_mesh::MeshError::DuplicateField { .. })
    ));
}

#[test]
fn validated_views_reuse_one_check_without_changing_conversion_semantics() {
    let dataset = tetrahedron()
        .with_field(Field::scalar("p", vec![1.0, 2.0, 3.0, 4.0]))
        .with_field(Field::vector("v", vec![[2.0, 4.0, 6.0]]).on_cells());
    let checked = dataset.validated().unwrap();
    assert!(std::ptr::eq(checked.as_dataset(), &dataset));
    assert_eq!(checked.uniform_cell_dimension(), Some(3));
    assert_eq!(
        point_data_to_cell_data(&dataset, "p", "mean").unwrap(),
        point_data_to_cell_data_validated(checked, "p", "mean").unwrap()
    );
    assert_eq!(
        cell_data_to_point_data(&dataset, "v", "mean").unwrap(),
        cell_data_to_point_data_validated(checked, "v", "mean").unwrap()
    );
    assert!(point_data_to_cell_data_validated(checked, "missing", "out").is_err());
    assert!(cell_data_to_point_data_validated(checked, "missing", "out").is_err());

    let surface = DataSet::new(
        "surface-kinds",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
        Cells::from_cells(&[
            (CellType::Triangle, &[0, 1, 2]),
            (CellType::Quad, &[0, 1, 3, 2]),
        ]),
    );
    assert_eq!(
        surface.validated().unwrap().uniform_cell_dimension(),
        Some(2)
    );
    let mut mixed = surface;
    mixed.cells = Cells::from_cells(&[
        (CellType::Line, &[0, 1]),
        (CellType::Triangle, &[0, 1, 2]),
        (CellType::Line, &[1, 2]),
    ]);
    assert_eq!(mixed.validated().unwrap().uniform_cell_dimension(), None);
    mixed.cells = Cells::from_cells(&[]);
    assert_eq!(mixed.validated().unwrap().uniform_cell_dimension(), None);
}

#[test]
fn calculators_preserve_masks_and_do_not_overflow_representable_magnitudes() {
    let mut vectors = Field::vector(
        "v",
        vec![
            [3.0, 4.0, 0.0],
            [99.0; 3],
            [f64::NAN; 3],
            [3e200, 4e200, 0.0],
        ],
    );
    vectors.valid = Some(vec![true, false, false, true].into());
    let mut complex = Field::complex(
        "z",
        vec![[3.0, 4.0], [99.0; 2], [f64::NAN; 2], [3e200, 4e200]],
    );
    complex.valid = vectors.valid.clone();
    let dataset = tetrahedron()
        .with_field(vectors.labelled("Velocity", "m/s"))
        .with_field(complex.labelled("Pressure", "Pa"));
    dataset.validate().unwrap();
    for (field, transform, unit) in [
        (
            "v",
            vector_magnitude as fn(&DataSet, &str, &str) -> _,
            "m/s",
        ),
        ("z", complex_magnitude, "Pa"),
        ("z", complex_phase, "rad"),
    ] {
        let output = transform(&dataset, field, "derived").unwrap();
        output.validate().unwrap();
        let derived = output.field(Association::Point, "derived").unwrap();
        assert_eq!(derived.unit.as_deref(), Some(unit));
        assert_eq!(
            derived.valid.as_deref(),
            Some([true, false, false, true].as_slice())
        );
        assert!(std::sync::Arc::ptr_eq(
            derived.valid.as_ref().unwrap(),
            dataset
                .field(Association::Point, field)
                .unwrap()
                .valid
                .as_ref()
                .unwrap(),
        ));
        let values = derived.as_scalar().unwrap();
        assert!(values[1].is_nan() && values[2].is_nan());
        assert!(values[0].is_finite() && values[3].is_finite());
        if unit == "rad" {
            assert!((values[0].tan() - 4.0 / 3.0).abs() < 1e-14);
            assert!((values[3] - values[0]).abs() < 1e-14);
        } else {
            assert_eq!(values[0], 5.0);
            assert!((values[3] / 5e200 - 1.0).abs() < 1e-15);
        }
    }
    assert_eq!(dataset.point_data[0].as_vector().unwrap()[1], [99.0; 3]);
    let overflow = tetrahedron().with_field(Field::complex("z", vec![[1.7e308; 2]; 4]));
    let output = complex_magnitude(&overflow, "z", "magnitude").unwrap();
    output.validate().unwrap();
    assert!((0..4).all(|index| !output.point_data[1].is_valid(index)));
    let tiny = tetrahedron().with_field(Field::vector("v", vec![[3e-200, 4e-200, 0.0]; 4]));
    let output = vector_magnitude(&tiny, "v", "magnitude").unwrap();
    assert!((output.point_data[1].as_scalar().unwrap()[0] / 5e-200 - 1.0).abs() < 1e-15);
    let mut changed = dataset.clone();
    let gpui_mesh::FieldData::Complex(values) = &mut changed.point_data[1].data else {
        unreachable!()
    };
    std::sync::Arc::make_mut(values)[3] = [1.7e308; 2];
    let output = complex_magnitude(&changed, "z", "magnitude").unwrap();
    output.validate().unwrap();
    assert_eq!(
        output.point_data[2].valid.as_deref(),
        Some([true, false, false, false].as_slice())
    );
    assert!(changed.point_data[1].is_valid(3));
}

#[test]
fn probe_and_isosurface_require_valid_tetrahedral_support() {
    let mut field = Field::scalar("p", vec![-1.0, 1.0, 1.0, 1.0]);
    let mut dataset = tetrahedron().with_field(field.clone());
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(isosurface(&dataset, "p", value).is_err());
    }
    assert!(!isosurface(&dataset, "p", 0.0).unwrap().cells.is_empty());
    for placeholder in [123.0, f64::NAN] {
        field.valid = Some(vec![true, false, true, true].into());
        field.data = gpui_mesh::FieldData::Scalar(vec![-1.0, placeholder, 1.0, 1.0].into());
        dataset.point_data[0] = field.clone();
        dataset.validate().unwrap();
        assert_eq!(
            probe(&dataset, "p", &[[0.1; 3], [0.0; 3], [f64::NAN; 3]]).unwrap(),
            vec![None; 3]
        );
        let empty = isosurface(&dataset, "p", 0.0).unwrap();
        assert_eq!(empty.point_count(), 0);
        assert_eq!(empty.cell_count(), 0);
        empty.validate().unwrap();
    }
    let surface = DataSet::surface(
        "surface",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0, 1, 2]],
    )
    .with_field({
        let mut p = Field::scalar("p", vec![99.0, 2.0, 3.0]);
        p.valid = Some(vec![false, true, true].into());
        p
    });
    assert_eq!(
        probe(&surface, "p", &[[0.0; 3], [1.0, 0.0, 0.0]]).unwrap(),
        vec![None, Some(2.0)]
    );
}

#[test]
fn empty_selections_validate_and_remain_empty_through_further_filters() {
    let dataset = tetrahedron().with_field(Field::scalar("p", vec![0.0; 4]));
    let empty = threshold(
        &dataset,
        "p",
        Association::Point,
        1.0,
        2.0,
        ThresholdMode::Any,
    )
    .unwrap();
    empty.validate().unwrap();
    assert_eq!(empty.bounds(), None);
    let again = threshold(
        &empty,
        "p",
        Association::Point,
        -1.0,
        1.0,
        ThresholdMode::All,
    )
    .unwrap();
    again.validate().unwrap();
    assert_eq!(probe(&again, "p", &[[0.0; 3]]).unwrap(), vec![None]);
    let mut malformed = again;
    malformed.cells = Cells::from_cells(&[(CellType::Vertex, &[0])]);
    assert!(matches!(
        malformed.validate(),
        Err(gpui_mesh::MeshError::EmptyPositions)
    ));
}

#[test]
fn isosurface_geometry_is_invariant_to_scalar_units_and_avoids_position_overflow() {
    let reference = isosurface(
        &tetrahedron().with_field(Field::scalar("p", vec![-1.0, 1.0, 1.0, 1.0])),
        "p",
        -0.5,
    )
    .unwrap();
    for scale in [1e-200, f64::from_bits(16), f64::MAX] {
        let dataset =
            tetrahedron().with_field(Field::scalar("p", vec![-scale, scale, scale, scale]));
        let output = isosurface(&dataset, "p", -0.5 * scale).unwrap();
        output.validate().unwrap();
        assert_eq!(output.positions.len(), reference.positions.len());
        for (actual, expected) in output.positions.iter().zip(reference.positions.iter()) {
            for axis in 0..3 {
                assert!(
                    (actual[axis] - expected[axis]).abs() < 1e-14,
                    "scale {scale}: {actual:?} != {expected:?}"
                );
            }
        }
        for &value in output.point_data[0].as_scalar().unwrap() {
            assert!((value / scale + 0.5).abs() < 1e-14);
        }
    }
    let mut wide_geometry = tetrahedron().with_field(Field::scalar("p", vec![-1.0, 1.0, 1.0, 1.0]));
    wide_geometry.positions = vec![
        [-f64::MAX, 0.0, 0.0],
        [f64::MAX, 0.0, 0.0],
        [-f64::MAX, 1.0, 0.0],
        [-f64::MAX, 0.0, 1.0],
    ]
    .into();
    let output = isosurface(&wide_geometry, "p", -0.5).unwrap();
    output.validate().unwrap();
    assert!((output.positions[0][0] / f64::MAX + 0.5).abs() < 1e-14);
    assert!(output.positions.iter().flatten().all(|v| v.is_finite()));
    let mut exact_endpoint = wide_geometry;
    std::sync::Arc::make_mut(&mut exact_endpoint.positions)[1][0] = -1e-200;
    let output = isosurface(&exact_endpoint, "p", 1.0).unwrap();
    assert!(output.positions.contains(&[-1e-200, 0.0, 0.0]));
}
