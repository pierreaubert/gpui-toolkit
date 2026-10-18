//! Deterministic decomposition of linear volume cells.

// Rust guideline compliant 2026-02-21

use crate::CellType;

/// Iterates the toolkit's linear-cell tetrahedra without allocating.
///
/// Hexahedra use six tetrahedra around the local 0–6 diagonal, wedges use three,
/// and pyramids use two. Neighboring cells need consistent face orientation for
/// conforming shared diagonals. Non-volume kinds and incorrect arities are empty.
/// Returned indices are the supplied global node IDs; their bounds are validated
/// by [`crate::DataSet::validate`].
///
/// # Examples
///
/// ```
/// use gpui_mesh::{CellType, tetrahedra};
/// let nodes = [0, 1, 2, 3, 4, 5, 6, 7];
/// assert_eq!(tetrahedra(CellType::Hexahedron, &nodes).len(), 6);
/// ```
#[inline]
pub fn tetrahedra(kind: CellType, nodes: &[u32]) -> impl ExactSizeIterator<Item = [u32; 4]> + '_ {
    let patterns: &[[usize; 4]] = match kind {
        CellType::Tetra => &[[0, 1, 2, 3]],
        CellType::Hexahedron => &[
            [0, 1, 2, 6],
            [0, 2, 3, 6],
            [0, 3, 7, 6],
            [0, 7, 4, 6],
            [0, 4, 5, 6],
            [0, 5, 1, 6],
        ],
        CellType::Wedge => &[[0, 1, 2, 3], [1, 2, 3, 4], [2, 3, 4, 5]],
        CellType::Pyramid => &[[0, 1, 2, 4], [0, 2, 3, 4]],
        _ => &[],
    };
    let patterns = if nodes.len() == kind.node_count() {
        patterns
    } else {
        &[]
    };
    patterns
        .iter()
        .map(move |tet| (*tet).map(|index| nodes[index]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume(points: [[f64; 3]; 4]) -> f64 {
        let [a, b, c, d] = points;
        let x = std::array::from_fn::<_, 3, _>(|axis| b[axis] - a[axis]);
        let y = std::array::from_fn::<_, 3, _>(|axis| c[axis] - a[axis]);
        let z = std::array::from_fn::<_, 3, _>(|axis| d[axis] - a[axis]);
        (x[0] * (y[1] * z[2] - y[2] * z[1]) - x[1] * (y[0] * z[2] - y[2] * z[0])
            + x[2] * (y[0] * z[1] - y[1] * z[0]))
            .abs()
            / 6.0
    }

    #[test]
    fn decomposition_preserves_simple_cell_volumes_and_rejects_wrong_arities() {
        let cases = [
            (
                CellType::Tetra,
                vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                1,
                1.0 / 6.0,
            ),
            (
                CellType::Hexahedron,
                vec![
                    [0.0; 3],
                    [1.0, 0.0, 0.0],
                    [1.0, 1.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [0.0, 0.0, 1.0],
                    [1.0, 0.0, 1.0],
                    [1.0; 3],
                    [0.0, 1.0, 1.0],
                ],
                6,
                1.0,
            ),
            (
                CellType::Wedge,
                vec![
                    [0.0; 3],
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [0.0, 0.0, 1.0],
                    [1.0, 0.0, 1.0],
                    [0.0, 1.0, 1.0],
                ],
                3,
                0.5,
            ),
            (
                CellType::Pyramid,
                vec![
                    [0.0; 3],
                    [1.0, 0.0, 0.0],
                    [1.0, 1.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [0.5, 0.5, 1.0],
                ],
                2,
                1.0 / 3.0,
            ),
        ];
        for (kind, points, count, expected) in cases {
            let nodes: Vec<_> = (0..points.len() as u32).collect();
            assert_eq!(tetrahedra(kind, &nodes).len(), count);
            let actual: f64 = tetrahedra(kind, &nodes)
                .map(|tet| volume(tet.map(|node| points[node as usize])))
                .sum();
            assert!((actual - expected).abs() < 1e-14);
            assert_eq!(tetrahedra(kind, &nodes[..nodes.len() - 1]).len(), 0);
        }
        assert_eq!(tetrahedra(CellType::Triangle, &[0, 1, 2]).len(), 0);
        assert_eq!(
            tetrahedra(CellType::Tetra, &[7, 11, 13, 17]).next(),
            Some([7, 11, 13, 17])
        );
    }
}
