// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    CellToPointPlan, cell_data_to_point_data, cell_data_to_point_data_validated,
};
use gpui_mesh::{Association, CellType, Cells, DataSet, Field, FieldData};

fn star(pairs: &[(f64, f64)]) -> DataSet {
    let mut positions = vec![[0.0; 3]];
    positions.extend(pairs.iter().map(|&(_, weight)| [weight, 0.0, 0.0]));
    let nodes = (1..positions.len() as u32)
        .map(|node| [0, node])
        .collect::<Vec<_>>();
    let cells = nodes
        .iter()
        .map(|node| (CellType::Line, node.as_slice()))
        .collect::<Vec<_>>();
    DataSet::new("weighted", positions, Cells::from_cells(&cells)).with_field(
        Field::scalar("p", pairs.iter().map(|&(v, _)| v).collect::<Vec<_>>()).on_cells(),
    )
}

fn first(field: &Field) -> Vec<f64> {
    match &field.data {
        FieldData::Scalar(v) => vec![v[0]],
        FieldData::Vector(v) => v[0].to_vec(),
        FieldData::Tensor(v) => v[0].to_vec(),
        FieldData::Complex(v) => v[0].to_vec(),
    }
}

#[test]
fn weighted_conversion_matches_independent_rational_oracle_in_both_orders() {
    let mut checked = 0;
    for (index, line) in include_str!("fixtures/weighted_mean.tsv")
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.starts_with('#'))
    {
        let words = line
            .split_whitespace()
            .map(|word| u64::from_str_radix(word, 16).unwrap())
            .collect::<Vec<_>>();
        assert!(words.len() >= 3 && words.len() % 2 == 1);
        checked += 1;
        let pairs = words[1..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| (f64::from_bits(pair[0]), f64::from_bits(pair[1])))
            .collect::<Vec<_>>();
        for pairs in [pairs.clone(), pairs.iter().copied().rev().collect()] {
            let data = star(&pairs);
            let mut plan = CellToPointPlan::new(data.validated().unwrap()).unwrap();
            for out in [
                cell_data_to_point_data(&data, "p", "mean").unwrap(),
                plan.convert_field("p", "mean").unwrap(),
            ] {
                out.validate().unwrap();
                let actual = out
                    .field(Association::Point, "mean")
                    .unwrap()
                    .as_scalar()
                    .unwrap();
                assert_eq!(
                    actual[0].to_bits(),
                    words[0],
                    "public weighted oracle row {index}"
                );
                for (actual, &(expected, _)) in actual[1..].iter().zip(&pairs) {
                    // Exact rational zero is normalized to +0; a one-cell point
                    // otherwise retains its finite source value exactly.
                    assert_eq!(*actual, expected);
                }
            }
        }
    }
    assert_eq!(checked, 590);
}

#[test]
fn every_weighted_shape_preserves_masks_units_and_unsupported_points() {
    for weights in [[1.0, 1.0, 1.0, 1.0], [1.0, 2.0, 1.0, 4.0]] {
        for shape in [1, 3, 6, 2] {
            let values = [1e16, 3.0, -1e16, f64::NAN];
            let pairs = values.into_iter().zip(weights).collect::<Vec<_>>();
            let mut data = star(&pairs);
            let mut field = match shape {
                1 => Field::scalar("p", values.to_vec()),
                3 => Field::vector("p", values.map(|v| [v; 3]).to_vec()),
                6 => Field::tensor("p", values.map(|v| [v; 6]).to_vec()),
                _ => Field::complex("p", values.map(|v| [v; 2]).to_vec()),
            }
            .on_cells();
            field.label = "Weighted samples".into();
            field.unit = Some("u".into());
            field.valid = Some(vec![true, true, true, false].into());
            data.cell_data[0] = field;
            let checked = data.validated().unwrap();
            for out in [
                cell_data_to_point_data(&data, "p", "mean").unwrap(),
                cell_data_to_point_data_validated(checked, "p", "mean").unwrap(),
            ] {
                out.validate().unwrap();
                let field = out.field(Association::Point, "mean").unwrap();
                let expected: f64 = if weights[1] == 1.0 { 1.0 } else { 1.5 };
                assert!(
                    first(field)
                        .iter()
                        .all(|v| v.to_bits() == expected.to_bits())
                );
                assert_eq!(field.label.as_ref(), "Weighted samples");
                assert_eq!(field.unit.as_deref(), Some("u"));
                assert!(!field.is_valid(4));
            }
        }
    }
}

#[test]
fn repeated_polygon_nodes_preserve_incidence_multiplicity_and_division_rounding() {
    let data = DataSet::new(
        "repeated-incidence",
        vec![[0.0; 3], [2.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        Cells::from_cells(&[
            (CellType::Polygon, &[0, 1, 2, 0]),
            (CellType::Triangle, &[0, 1, 2]),
        ]),
    )
    .with_field(Field::vector("p", vec![[3.0; 3], [-1.0; 3]]).on_cells());
    let out = cell_data_to_point_data(&data, "p", "mean").unwrap();
    out.validate().unwrap();
    let values = out
        .field(Association::Point, "mean")
        .unwrap()
        .as_vector()
        .unwrap();
    assert!(
        values[0]
            .iter()
            .all(|v| v.to_bits() == 0x3ffa_aaaa_aaaa_aaab)
    );
    assert_eq!(&values[1..], &[[1.0; 3]; 2]);
}

#[test]
fn dimension_priority_is_per_point_and_independent_of_cell_order() {
    let positions = vec![
        [0.0; 3],
        [2.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [4.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [10.0, 0.0, 0.0],
    ];
    let nodes = [
        (CellType::Vertex, vec![0]),
        (CellType::Line, vec![0, 5]),
        (CellType::Triangle, vec![0, 1, 2]),
        (CellType::Triangle, vec![0, 3, 4]),
    ];
    let values = [-1e308, 1e308, 3.0, -3.0];
    for order in [[0, 1, 2, 3], [3, 2, 1, 0]] {
        for mask in [
            [true; 4],
            [true, true, true, false],
            [true, true, false, false],
            [true, false, false, false],
        ] {
            let cells = order
                .iter()
                .map(|&i| (nodes[i].0, nodes[i].1.as_slice()))
                .collect::<Vec<_>>();
            let mut field = Field::scalar("p", order.map(|i| values[i]).to_vec()).on_cells();
            field.valid = Some(order.map(|i| mask[i]).to_vec().into());
            let data = DataSet::new("dimensions", positions.clone(), Cells::from_cells(&cells))
                .with_field(field);
            let out = cell_data_to_point_data(&data, "p", "mean").unwrap();
            out.validate().unwrap();
            let actual = out
                .field(Association::Point, "mean")
                .unwrap()
                .as_scalar()
                .unwrap()[0];
            let expected = if mask[3] {
                -1.0
            } else if mask[2] {
                3.0
            } else if mask[1] {
                1e308
            } else {
                -1e308
            };
            assert_eq!(actual, expected);
        }
    }
}
