//! Grid construction topology and invalid-input oracles using existing APIs.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, DataSet};

#[test]
fn constructors_preserve_all_active_axes_and_x_fastest_cell_order() {
    let origin = [0.25, -1.0, 2.0];
    let spacing = [0.5, 1.25, -0.75];
    for dims in [
        [3, 4, 5],
        [3, 4, 1],
        [3, 1, 5],
        [1, 4, 5],
        [3, 1, 1],
        [1, 4, 1],
        [1, 1, 5],
        [1, 1, 1],
    ] {
        let axes: [Vec<f64>; 3] = std::array::from_fn(|a| {
            (0..dims[a])
                .map(|i| origin[a] + i as f64 * spacing[a])
                .collect()
        });
        let affine = DataSet::structured_grid("grid", dims, origin, spacing).unwrap();
        let rect = DataSet::rectilinear_grid("grid", &axes[0], &axes[1], &axes[2]).unwrap();
        let curved = DataSet::curvilinear_grid("grid", dims, affine.positions.to_vec()).unwrap();
        assert_eq!(affine, rect);
        assert_eq!(affine, curved);
        affine.validate().unwrap();
        let dimension = dims.iter().filter(|&&n| n > 1).count();
        let kind = [
            CellType::Vertex,
            CellType::Line,
            CellType::Quad,
            CellType::Hexahedron,
        ][dimension];
        let cell_dims = dims.map(|n| n.saturating_sub(1).max(1));
        assert_eq!(
            affine.cell_count(),
            cell_dims.into_iter().product::<usize>(),
            "missing topology for {dims:?}"
        );
        for (cell, &ty) in affine.cells.types.iter().enumerate() {
            assert_eq!(ty, kind);
            let nodes = affine.cells.cell(cell).unwrap();
            assert_eq!(nodes.len(), 1 << dimension);
            let mut unique = nodes.to_vec();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), nodes.len());
            for axis in 0..3 {
                let stride = dims[..axis].iter().product::<usize>();
                let coords = nodes
                    .iter()
                    .map(|&id| (id as usize / stride) % dims[axis])
                    .collect::<Vec<_>>();
                let min = *coords.iter().min().unwrap();
                let max = *coords.iter().max().unwrap();
                assert_eq!(max - min, usize::from(dims[axis] > 1));
                let expected =
                    (cell / cell_dims[..axis].iter().product::<usize>()) % cell_dims[axis];
                assert_eq!(
                    min, expected,
                    "cell order differs for {dims:?}, cell {cell}, axis {axis}"
                );
            }
        }
    }
    let hex = DataSet::structured_grid("hex", [3, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
    assert_eq!(hex.cells.cell(0).unwrap(), [0, 1, 4, 3, 6, 7, 10, 9]);
    assert_eq!(hex.cells.cell(1).unwrap(), [1, 2, 5, 4, 7, 8, 11, 10]);
    for dims in [[3, 2, 1], [3, 1, 2], [1, 3, 2]] {
        let quad = DataSet::structured_grid("quad", dims, [0.0; 3], [1.0; 3]).unwrap();
        assert_eq!(quad.cells.cell(0).unwrap(), [0, 1, 4, 3]);
        assert_eq!(quad.cells.cell(1).unwrap(), [1, 2, 5, 4]);
    }
}

#[test]
fn impossible_dimensions_return_an_error_without_capacity_panics() {
    DataSet::structured_grid("huge", [usize::MAX, 2, 2], [0.0; 3], [1.0; 3]).unwrap_err();
    DataSet::curvilinear_grid("huge", [usize::MAX, 2, 2], vec![]).unwrap_err();
    DataSet::structured_grid("zero", [0, usize::MAX, 2], [0.0; 3], [1.0; 3]).unwrap_err();
}

#[test]
fn constructors_reject_nonfinite_coordinates() {
    DataSet::structured_grid("bad", [2; 3], [f64::NAN, 0.0, 0.0], [1.0; 3]).unwrap_err();
    DataSet::structured_grid("bad", [2; 3], [0.0; 3], [f64::INFINITY, 1.0, 1.0]).unwrap_err();
    DataSet::structured_grid(
        "overflow",
        [3, 1, 1],
        [-f64::MAX, 0.0, 0.0],
        [f64::MAX, 1.0, 1.0],
    )
    .unwrap_err();
    DataSet::rectilinear_grid("bad", &[0.0, f64::INFINITY], &[0.0], &[0.0]).unwrap_err();
    let mut positions = vec![[0.0; 3]; 8];
    positions[5][2] = f64::NAN;
    DataSet::curvilinear_grid("bad", [2; 3], positions).unwrap_err();
}
