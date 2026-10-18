//! Scientific missing-value contracts across public transformations.

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    ThresholdMode, complex_magnitude, complex_phase, isosurface, probe, threshold, vector_magnitude,
};
use gpui_mesh::{Association, CellType, Cells, DataSet, Field};

fn tetrahedron() -> DataSet {
    DataSet::new(
        "tet",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        Cells::from_cells(&[(CellType::Tetra, &[0, 1, 2, 3])]),
    )
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
