//! Constant-space boundary traversal after verifying canonical hexahedral connectivity.

// Rust guideline compliant 2026-02-21

use crate::{CellType, DataSet};

#[derive(Debug, Clone, Copy)]
pub(crate) struct CanonicalHexGrid {
    dims: [usize; 3],
    plane: usize,
}

impl CanonicalHexGrid {
    pub(crate) fn detect(dataset: &DataSet) -> Option<Self> {
        let cells = &dataset.cells;
        if cells.types.first() != Some(&CellType::Hexahedron) {
            return None;
        }
        let count = cells.types.len();
        let connectivity = count.checked_mul(8)?;
        if cells.offsets.len() != count.checked_add(1)?
            || cells.connectivity.len() != connectivity
            || cells.offsets.last().copied() != Some(u64::try_from(connectivity).ok()?)
            || u64::try_from(dataset.point_count()).ok()? > u64::from(u32::MAX) + 1
        {
            return None;
        }
        let first = cells.connectivity.as_chunks::<8>().0.first()?;
        let nx = first[3] as usize;
        let plane = first[4] as usize;
        if nx < 2 || plane == 0 || !plane.is_multiple_of(nx) {
            return None;
        }
        let ny = plane / nx;
        if ny < 2 || !dataset.point_count().is_multiple_of(plane) {
            return None;
        }
        let nz = dataset.point_count() / plane;
        if nz < 2 {
            return None;
        }
        let cell_plane = (nx - 1).checked_mul(ny - 1)?;
        if cell_plane.checked_mul(nz - 1)? != count {
            return None;
        }
        // Every type, offset and node must match. A constructor name, spatial
        // arrangement or canonical prefix is insufficient evidence of topology.
        for (cell, nodes) in cells.connectivity.as_chunks::<8>().0.iter().enumerate() {
            if cells.types[cell] != CellType::Hexahedron || cells.offsets[cell] != (cell * 8) as u64
            {
                return None;
            }
            let i = cell % (nx - 1);
            let j = (cell / (nx - 1)) % (ny - 1);
            let k = cell / cell_plane;
            let p = (k * ny + j) * nx + i;
            let expected = [
                p,
                p + 1,
                p + nx + 1,
                p + nx,
                p + plane,
                p + plane + 1,
                p + plane + nx + 1,
                p + plane + nx,
            ];
            if nodes.iter().zip(expected).any(|(&a, b)| a as usize != b) {
                return None;
            }
        }
        Some(Self {
            dims: [nx, ny, nz],
            plane,
        })
    }

    pub(crate) fn visit<E>(
        self,
        mut visit: impl FnMut(usize, u8) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut cursors = [
            FaceCursor::new(&self, Axis::Xy),
            FaceCursor::new(&self, Axis::Xz),
            FaceCursor::new(&self, Axis::Yz),
        ];
        let mut heads = cursors.each_mut().map(|cursor| cursor.next(&self));
        loop {
            // Face keys first compare their minimum point. At a shared minimum,
            // XY precedes XZ (nx < plane), then YZ (its second point is p+nx).
            let Some(index) = heads
                .iter()
                .enumerate()
                .filter_map(|(i, head)| head.map(|face| ((face.minimum, i), i)))
                .min_by_key(|(key, _)| *key)
                .map(|(_, i)| i)
            else {
                return Ok(());
            };
            let face = heads[index].take().expect("selected face cursor");
            visit(face.cell, face.face)?;
            heads[index] = cursors[index].next(&self);
        }
    }
}

#[derive(Clone, Copy)]
enum Axis {
    Xy,
    Xz,
    Yz,
}

#[derive(Clone, Copy)]
struct Candidate {
    minimum: usize,
    cell: usize,
    face: u8,
}

struct FaceCursor {
    axis: Axis,
    next: usize,
    end: usize,
}

impl FaceCursor {
    fn new(grid: &CanonicalHexGrid, axis: Axis) -> Self {
        let [cx, cy, cz] = grid.dims.map(|n| n - 1);
        // Each pair of opposite sides contains no more than point_count faces.
        // Detection proved dimensions >=2 and their full product representable.
        let end = match axis {
            Axis::Xy => 2 * cx * cy,
            Axis::Xz => 2 * cx * cz,
            Axis::Yz => 2 * cy * cz,
        };
        Self { axis, next: 0, end }
    }
    fn next(&mut self, grid: &CanonicalHexGrid) -> Option<Candidate> {
        if self.next == self.end {
            return None;
        }
        let ordinal = self.next;
        self.next += 1;
        let [nx, ny, nz] = grid.dims;
        let [cx, cy, cz] = [nx - 1, ny - 1, nz - 1];
        let (i, j, k, face) = match self.axis {
            Axis::Xy => {
                let side = ordinal / (cx * cy);
                let p = ordinal % (cx * cy);
                (
                    p % cx,
                    p / cx,
                    if side == 0 { 0 } else { nz - 1 },
                    u8::from(side != 0),
                )
            }
            Axis::Xz => {
                let k = ordinal / (2 * cx);
                let p = ordinal % (2 * cx);
                let side = p / cx;
                (
                    p % cx,
                    if side == 0 { 0 } else { ny - 1 },
                    k,
                    if side == 0 { 2 } else { 4 },
                )
            }
            Axis::Yz => {
                let k = ordinal / (2 * cy);
                let p = ordinal % (2 * cy);
                let side = p % 2;
                (
                    if side == 0 { 0 } else { nx - 1 },
                    p / 2,
                    k,
                    if side == 0 { 5 } else { 3 },
                )
            }
        };
        Some(Candidate {
            minimum: k * grid.plane + j * nx + i,
            cell: (k.min(cz - 1) * cy + j.min(cy - 1)) * cx + i.min(cx - 1),
            face,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn cursor_arithmetic_reaches_the_last_u32_point_without_large_allocations() {
        let long = 1usize << 30;
        for dims in [[long, 2, 2], [2, long, 2], [2, 2, long]] {
            let grid = CanonicalHexGrid {
                dims,
                plane: dims[0] * dims[1],
            };
            let cells = dims.map(|d| d - 1).into_iter().product::<usize>();
            for axis in [Axis::Xy, Axis::Xz, Axis::Yz] {
                let mut cursor = FaceCursor::new(&grid, axis);
                let first = cursor.next(&grid).unwrap();
                cursor.next = cursor.end - 1;
                let last = cursor.next(&grid).unwrap();
                assert!(cursor.next(&grid).is_none());
                let offset = match axis {
                    Axis::Xy => dims[0] + 1,
                    Axis::Xz => grid.plane + 1,
                    Axis::Yz => grid.plane + dims[0],
                };
                for face in [first, last] {
                    assert!(face.cell < cells);
                    u32::try_from(face.minimum.checked_add(offset).unwrap()).unwrap();
                }
            }
        }
    }

    #[test]
    fn detection_uses_topology_and_handles_arbitrary_grid_positions() {
        let xs = [3.0, 2.0, 0.0];
        let ys = [-1.0, 0.5, 0.75, 5.0];
        let zs = [0.0, 2.0];
        let rect = DataSet::rectilinear_grid("rect", &xs, &ys, &zs).unwrap();
        assert_eq!(CanonicalHexGrid::detect(&rect).unwrap().dims, [3, 4, 2]);
        let warped = DataSet::curvilinear_grid(
            "warped",
            [3, 4, 2],
            rect.positions
                .iter()
                .map(|p| [p[0] + p[1] * p[2], p[1] - p[0] * 0.1, p[2] + p[0] * p[0]])
                .collect(),
        )
        .unwrap();
        assert_eq!(CanonicalHexGrid::detect(&warped).unwrap().dims, [3, 4, 2]);
    }

    #[test]
    fn detection_rejects_invalid_offsets_types_and_inferred_dimensions() {
        let original = DataSet::structured_grid("grid", [3; 3], [0.0; 3], [1.0; 3]).unwrap();
        for edit in 0..6 {
            let mut dataset = original.clone();
            match edit {
                0 => Arc::make_mut(&mut dataset.cells.offsets)[1] = 7,
                1 => Arc::make_mut(&mut dataset.cells.offsets)[0] = 1,
                2 => {
                    let n = dataset.cells.types.len();
                    Arc::make_mut(&mut dataset.cells.types)[n - 1] = CellType::Quad;
                }
                3 => Arc::make_mut(&mut dataset.cells.connectivity)[3] = 0,
                4 => Arc::make_mut(&mut dataset.cells.connectivity)[4] = u32::MAX,
                _ => dataset.cells.connectivity = vec![u32::MAX; 8].into(),
            }
            assert!(
                CanonicalHexGrid::detect(&dataset).is_none(),
                "accepted edit {edit}"
            );
        }
        let empty = DataSet::new("empty", vec![], crate::Cells::from_cells(&[]));
        assert!(CanonicalHexGrid::detect(&empty).is_none());
    }
}
