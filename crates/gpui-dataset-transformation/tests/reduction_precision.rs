// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    decimate, field_statistics, point_data_to_cell_data, point_data_to_cell_data_validated,
};
use gpui_mesh::{Association, CellType, Cells, DataSet, Field, FieldData};

fn dataset(values: Vec<f64>) -> DataSet {
    let nodes = (0..values.len() as u32).collect::<Vec<_>>();
    let kind = match nodes.len() {
        1 => CellType::Vertex,
        2 => CellType::Line,
        3 => CellType::Triangle,
        4 => CellType::Quad,
        5 => CellType::Pyramid,
        6 => CellType::Wedge,
        8 => CellType::Hexahedron,
        _ => CellType::Polygon,
    };
    DataSet::new(
        "average",
        vec![[0.0; 3]; values.len()],
        Cells::from_cells(&[(kind, &nodes)]),
    )
    .with_field(Field::scalar("p", values))
}

fn shaped(shape: usize, values: &[f64]) -> Field {
    match shape {
        1 => Field::scalar("p", values.to_vec()),
        3 => Field::vector("p", values.iter().map(|&v| [v; 3]).collect::<Vec<_>>()),
        6 => Field::tensor("p", values.iter().map(|&v| [v; 6]).collect::<Vec<_>>()),
        _ => Field::complex("p", values.iter().map(|&v| [v; 2]).collect::<Vec<_>>()),
    }
}

fn row(field: &Field) -> Vec<f64> {
    match &field.data {
        FieldData::Scalar(v) => vec![v[0]],
        FieldData::Vector(v) => v[0].to_vec(),
        FieldData::Tensor(v) => v[0].to_vec(),
        FieldData::Complex(v) => v[0].to_vec(),
    }
}

#[test]
fn public_means_match_independent_rational_oracle_for_mixed_signs() {
    let mut checked = 0;
    for line in include_str!("fixtures/exact_mean.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let mut words = line
            .split_whitespace()
            .map(|word| u64::from_str_radix(word, 16).unwrap());
        let expected = words.next().unwrap();
        let values = words.map(f64::from_bits).collect::<Vec<_>>();
        if !values.iter().any(|v| *v < 0.0) || !values.iter().any(|v| *v > 0.0) {
            continue;
        }
        let data = dataset(values.clone());
        assert_eq!(
            field_statistics(&data, "p", Association::Point)
                .unwrap()
                .mean
                .to_bits(),
            expected
        );
        let out = point_data_to_cell_data(&data, "p", "mean").unwrap();
        out.validate().unwrap();
        assert_eq!(
            out.field(Association::Cell, "mean")
                .unwrap()
                .as_scalar()
                .unwrap()[0]
                .to_bits(),
            expected
        );
        let reversed = dataset(values.into_iter().rev().collect());
        assert_eq!(
            field_statistics(&reversed, "p", Association::Point)
                .unwrap()
                .mean
                .to_bits(),
            expected
        );
        let out = point_data_to_cell_data(&reversed, "p", "mean").unwrap();
        assert_eq!(
            out.field(Association::Cell, "mean")
                .unwrap()
                .as_scalar()
                .unwrap()[0]
                .to_bits(),
            expected
        );
        checked += 1;
    }
    assert!(checked > 400);
}

#[test]
fn averages_preserve_masked_dependencies_and_metadata_for_every_shape() {
    for missing in [99.0, f64::NAN] {
        for shape in [1, 3, 6, 2] {
            let mut data = dataset(vec![1e16, 3.0, -1e16, missing]);
            let mut field = shaped(shape, &[1e16, 3.0, -1e16, missing]);
            field.valid = Some(vec![true, true, true, false].into());
            field.label = "Samples".into();
            field.unit = Some("u".into());
            data.point_data[0] = field;
            for out in [
                point_data_to_cell_data(&data, "p", "mean").unwrap(),
                point_data_to_cell_data_validated(data.validated().unwrap(), "p", "mean").unwrap(),
            ] {
                out.validate().unwrap();
                let field = out.field(Association::Cell, "mean").unwrap();
                assert!(row(field).iter().all(|v| v.to_bits() == 1.0_f64.to_bits()));
                assert!(field.valid.is_none());
                assert_eq!(field.label.as_ref(), "Samples");
                assert_eq!(field.unit.as_deref(), Some("u"));
            }
        }
    }
}

#[test]
fn clustering_retains_residuals_lost_by_finite_compensated_sums() {
    let values = [1e308, 1.0, 1e-300, -1e308, -1.0];
    let expected = 1e-300_f64 / 5.0;
    for shape in [1, 3, 6, 2] {
        let mut data = dataset(values.to_vec());
        data.point_data[0] = shaped(shape, &values);
        let out = decimate(&data, 1.0).unwrap();
        out.validate().unwrap();
        assert_eq!(out.point_count(), 1);
        assert!(
            row(&out.point_data[0])
                .iter()
                .all(|v| v.to_bits() == expected.to_bits())
        );
    }
}

#[test]
fn sparse_and_dense_cells_keep_local_sign_and_support_semantics() {
    let mut values = vec![f64::NAN; 4096];
    values[..3].copy_from_slice(&[1e16, 3.0, -1e16]);
    let mut mask = vec![false; values.len()];
    mask[..3].fill(true);
    let mut data = DataSet::new(
        "sparse",
        vec![[0.0; 3]; values.len()],
        Cells::triangles(&[[0, 1, 2]]),
    )
    .with_field(Field::scalar("p", values));
    data.point_data[0].valid = Some(mask.into());
    let out = point_data_to_cell_data_validated(data.validated().unwrap(), "p", "mean").unwrap();
    out.validate().unwrap();
    assert_eq!(
        out.field(Association::Cell, "mean")
            .unwrap()
            .as_scalar()
            .unwrap(),
        [1.0]
    );
    let dense = DataSet::new(
        "separate signs",
        vec![[0.0; 3]; 6],
        Cells::triangles(&[[0, 1, 2], [3, 4, 5]]),
    )
    .with_field(Field::scalar("p", vec![1.0, 2.0, 3.0, -1.0, -2.0, -3.0]));
    let out = point_data_to_cell_data(&dense, "p", "mean").unwrap();
    assert_eq!(
        out.field(Association::Cell, "mean")
            .unwrap()
            .as_scalar()
            .unwrap(),
        [2.0, -2.0]
    );
}

#[test]
fn statistics_preserve_cancellation_residuals_for_point_and_cell_fields() {
    for values in [
        vec![f64::MAX, -f64::MAX, 1e-300],
        vec![1e308, 1.0, 1e-300, -1e308, -1.0],
    ] {
        let expected = 1e-300 / values.len() as f64;
        let nodes = (0..values.len() as u32).map(|i| [i]).collect::<Vec<_>>();
        let cells = nodes
            .iter()
            .map(|node| (CellType::Vertex, node.as_slice()))
            .collect::<Vec<_>>();
        let data = DataSet::new(
            "stats",
            vec![[0.0; 3]; values.len()],
            Cells::from_cells(&cells),
        )
        .with_field(Field::scalar("p", values.clone()))
        .with_field(Field::scalar("p", values).on_cells());
        for association in [Association::Point, Association::Cell] {
            assert_eq!(
                field_statistics(&data, "p", association)
                    .unwrap()
                    .mean
                    .to_bits(),
                expected.to_bits()
            );
        }
    }
}
