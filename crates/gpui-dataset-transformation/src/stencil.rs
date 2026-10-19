//! Compact cell lookup for validated scalar/vector/tensor stencil sampling.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, DataSet, tetrahedra};

const LEAF_CELLS: usize = 32;

#[derive(Clone, Copy, Default)]
struct Node {
    min: [f64; 3],
    max: [f64; 3],
    first: usize,
    count: usize,
}

pub(crate) struct StencilLocator<'a> {
    dataset: &'a DataSet,
    valid: Option<&'a [bool]>,
    cells: Vec<usize>,
    nodes: Vec<Node>,
    has_volume: bool,
    first_cell: usize,
}

fn bounds(dataset: &DataSet, cell: usize) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for &node in dataset.cells.cell(cell).unwrap_or(&[]) {
        for axis in 0..3 {
            min[axis] = min[axis].min(dataset.positions[node as usize][axis]);
            max[axis] = max[axis].max(dataset.positions[node as usize][axis]);
        }
    }
    (min, max)
}

fn expanded(min: [f64; 3], max: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    // Several slightly negative barycentric weights can enlarge a coordinate
    // beyond one epsilon of the box. Keep pruning conservative for all four.
    let pad = std::array::from_fn::<_, 3, _>(|axis| (max[axis] - min[axis]) * 4e-9);
    (
        std::array::from_fn(|axis| min[axis] - pad[axis]),
        std::array::from_fn(|axis| max[axis] + pad[axis]),
    )
}

fn contains(min: [f64; 3], max: [f64; 3], p: [f64; 3]) -> bool {
    p[0] >= min[0]
        && p[0] <= max[0]
        && p[1] >= min[1]
        && p[1] <= max[1]
        && p[2] >= min[2]
        && p[2] <= max[2]
}

fn node_count(count: usize) -> usize {
    if count <= LEAF_CELLS {
        1
    } else {
        1 + node_count(count / 2) + node_count(count - count / 2)
    }
}

impl<'a> StencilLocator<'a> {
    /// The dataset/field must be validated before constructing this borrow.
    pub(crate) fn new(dataset: &'a DataSet, valid: Option<&'a [bool]>) -> Self {
        let has_volume = dataset.cells.types.iter().any(|kind| kind.is_volume());
        let all_missing = valid.is_some_and(|mask| mask.iter().all(|v| !v));
        let valid = valid.filter(|mask| !mask.iter().all(|v| *v));
        let supported = |cell: usize| {
            let kind = dataset.cells.types[cell];
            kind.is_volume()
                && valid.is_none_or(|mask| {
                    tetrahedra(kind, dataset.cells.cell(cell).unwrap_or(&[]))
                        .any(|tet| tet.iter().all(|&node| mask[node as usize]))
                })
        };
        let count = if all_missing {
            0
        } else {
            (0..dataset.cell_count())
                .filter(|&cell| supported(cell))
                .count()
        };
        let mut cells = Vec::with_capacity(count);
        if count > 0 {
            cells.extend((0..dataset.cell_count()).filter(|&cell| supported(cell)));
        }
        let first_cell = cells.first().copied().unwrap_or(usize::MAX);
        let mut locator = Self {
            dataset,
            valid,
            cells,
            nodes: Vec::with_capacity(if count > 0 { node_count(count) } else { 0 }),
            has_volume,
            first_cell,
        };
        if count > 0 {
            locator.nodes.push(Node::default());
            locator.build(0, 0, count);
        }
        locator
    }

    fn build(&mut self, node: usize, first: usize, count: usize) {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for &cell in &self.cells[first..first + count] {
            let (lo, hi) = bounds(self.dataset, cell);
            for axis in 0..3 {
                min[axis] = min[axis].min(lo[axis]);
                max[axis] = max[axis].max(hi[axis]);
            }
        }
        let (lo, hi) = expanded(min, max);
        if count <= LEAF_CELLS {
            self.nodes[node] = Node {
                min: lo,
                max: hi,
                first,
                count,
            };
            return;
        }
        let axis = (0..3)
            .max_by(|&a, &b| (max[a] - min[a]).total_cmp(&(max[b] - min[b])))
            .unwrap_or(0);
        let dataset = self.dataset;
        let center = |cell: usize| {
            let (min, max) = bounds(dataset, cell);
            f64::midpoint(min[axis], max[axis])
        };
        let mid = count / 2;
        let _ = self.cells[first..first + count].select_nth_unstable_by(mid, |&a, &b| {
            center(a).total_cmp(&center(b)).then_with(|| a.cmp(&b))
        });
        let left = self.nodes.len();
        self.nodes.extend([Node::default(); 2]);
        self.nodes[node] = Node {
            min: lo,
            max: hi,
            first: left,
            count: 0,
        };
        self.build(left, first, mid);
        self.build(left + 1, first + mid, count - mid);
    }

    pub(crate) fn has_volume(&self) -> bool {
        self.has_volume
    }

    pub(crate) fn weights(&self, p: [f64; 3]) -> Option<([u32; 4], [f64; 4])> {
        if self.nodes.is_empty() || !p.iter().all(|v| v.is_finite()) {
            return None;
        }
        // Median splits bound depth by log2(cell count). An allocatable usize
        // slice cannot reach depth 64, including the pending sibling stack.
        let mut stack = [0usize; 64];
        let mut len = 1;
        let mut best = None;
        let mut best_cell = usize::MAX;
        while len > 0 {
            len -= 1;
            let node = self.nodes[stack[len]];
            if !contains(node.min, node.max, p) {
                continue;
            }
            if node.count == 0 {
                stack[len] = node.first + 1;
                stack[len + 1] = node.first;
                len += 2;
                continue;
            }
            for &cell in &self.cells[node.first..node.first + node.count] {
                if cell >= best_cell {
                    continue;
                }
                let kind = self.dataset.cells.types[cell];
                if kind != CellType::Tetra {
                    let (min, max) = bounds(self.dataset, cell);
                    let (min, max) = expanded(min, max);
                    if !contains(min, max, p) {
                        continue;
                    }
                }
                for tet in tetrahedra(kind, self.dataset.cells.cell(cell).unwrap_or(&[])) {
                    if self
                        .valid
                        .is_some_and(|mask| tet.iter().any(|&node| !mask[node as usize]))
                    {
                        continue;
                    }
                    let corners = tet.map(|node| self.dataset.positions[node as usize]);
                    if let Some(weights) = barycentric(corners, p) {
                        best = Some((tet, weights));
                        best_cell = cell;
                        if cell == self.first_cell {
                            return best;
                        }
                        break;
                    }
                }
            }
        }
        best
    }
}

/// Ordered, allocation-free lookup for batches too small to amortize a BVH.
pub(crate) fn linear_weights(
    dataset: &DataSet,
    valid: Option<&[bool]>,
    p: [f64; 3],
) -> Option<([u32; 4], [f64; 4])> {
    if !p.iter().all(|value| value.is_finite()) {
        return None;
    }
    for cell in 0..dataset.cell_count() {
        let kind = dataset.cells.types[cell];
        if !kind.is_volume() {
            continue;
        }
        let (min, max) = bounds(dataset, cell);
        let (min, max) = expanded(min, max);
        if !contains(min, max, p) {
            continue;
        }
        for tet in tetrahedra(kind, dataset.cells.cell(cell).unwrap_or(&[])) {
            if valid.is_some_and(|mask| tet.iter().any(|&node| !mask[node as usize])) {
                continue;
            }
            if let Some(weights) = barycentric(tet.map(|node| dataset.positions[node as usize]), p)
            {
                return Some((tet, weights));
            }
        }
    }
    None
}

/// Barycentric weights of `p` in a tetrahedron, when inside (within tolerance).
pub(crate) fn barycentric(tet: [[f64; 3]; 4], p: [f64; 3]) -> Option<[f64; 4]> {
    if !p.iter().all(|v| v.is_finite()) {
        return None;
    }
    let (a, b, c, d) = (tet[0], tet[1], tet[2], tet[3]);
    let m = [
        [b[0] - a[0], c[0] - a[0], d[0] - a[0]],
        [b[1] - a[1], c[1] - a[1], d[1] - a[1]],
        [b[2] - a[2], c[2] - a[2], d[2] - a[2]],
    ];
    let rhs = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if !det.is_finite() || det.abs() < 1e-300 {
        return None;
    }
    // Cramer's rule for the 3x3 system.
    let solve = |col: usize| -> f64 {
        let mut mm = m;
        for row in 0..3 {
            mm[row][col] = rhs[row];
        }
        (mm[0][0] * (mm[1][1] * mm[2][2] - mm[1][2] * mm[2][1])
            - mm[0][1] * (mm[1][0] * mm[2][2] - mm[1][2] * mm[2][0])
            + mm[0][2] * (mm[1][0] * mm[2][1] - mm[1][1] * mm[2][0]))
            / det
    };
    let w = [solve(0), solve(1), solve(2)];
    let weights = [1.0 - w[0] - w[1] - w[2], w[0], w[1], w[2]];
    if !weights
        .iter()
        .all(|&x| x.is_finite() && (-1e-9..=1.0 + 1e-9).contains(&x))
    {
        return None;
    }
    let weights = weights.map(|w| w.max(0.0));
    let sum: f64 = weights.iter().sum();
    Some(weights.map(|w| w / sum))
}

pub(crate) fn sample_values<const N: usize>(
    values: &[[f64; N]],
    nodes: [u32; 4],
    weights: [f64; 4],
) -> Option<[f64; N]> {
    let mut result = [0.0; N];
    let mut needs_fallback = false;
    for (node, weight) in nodes.into_iter().zip(weights) {
        if weight == 0.0 {
            continue;
        }
        for (target, source) in result.iter_mut().zip(values[node as usize]) {
            let product = source * weight;
            needs_fallback |= source != 0.0 && !product.is_normal();
            *target += product;
        }
    }
    if !needs_fallback && result.iter().all(|v| v.is_finite()) {
        return Some(result);
    }
    // Retry overflow/underflow in a finite convex combination.
    let mut result = [0.0; N];
    let mut total = 0.0;
    for (node, weight) in nodes.into_iter().zip(weights) {
        if weight == 0.0 {
            continue;
        }
        total += weight;
        for (target, source) in result.iter_mut().zip(values[node as usize]) {
            *target = crate::convert::blend(*target, source, weight / total);
        }
    }
    result.iter().all(|v| v.is_finite()).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::Cells;

    fn reference(
        dataset: &DataSet,
        mask: Option<&[bool]>,
        p: [f64; 3],
    ) -> Option<([u32; 4], [f64; 4])> {
        for cell in 0..dataset.cell_count() {
            for tet in tetrahedra(dataset.cells.types[cell], dataset.cells.cell(cell).unwrap()) {
                if mask.is_some_and(|mask| tet.iter().any(|&node| !mask[node as usize])) {
                    continue;
                }
                if let Some(weights) =
                    barycentric(tet.map(|node| dataset.positions[node as usize]), p)
                {
                    return Some((tet, weights));
                }
            }
        }
        None
    }

    #[test]
    fn bvh_agrees_with_ordered_linear_reference_at_boundaries_and_missing_regions() {
        let dataset = DataSet::structured_grid("reference", [5; 3], [0.0; 3], [0.5; 3]).unwrap();
        let mask = (0..dataset.point_count())
            .map(|point| point % 7 != 0)
            .collect::<Vec<_>>();
        for mask in [None, Some(mask.as_slice())] {
            let locator = StencilLocator::new(&dataset, mask);
            for x in -1..10 {
                for y in -1..10 {
                    for z in -1..10 {
                        let p = [
                            f64::from(x) * 0.25,
                            f64::from(y) * 0.25,
                            f64::from(z) * 0.25,
                        ];
                        assert_eq!(
                            locator.weights(p),
                            reference(&dataset, mask, p),
                            "point {p:?}"
                        );
                        assert_eq!(
                            linear_weights(&dataset, mask, p),
                            reference(&dataset, mask, p),
                            "linear point {p:?}"
                        );
                    }
                }
            }
            assert_eq!(locator.weights([f64::NAN; 3]), None);
        }
        let dataset = DataSet::new(
            "tolerant",
            vec![[0.0; 3], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0]],
            Cells::from_cells(&[(CellType::Tetra, &[0, 1, 2, 3])]),
        );
        let locator = StencilLocator::new(&dataset, None);
        let p = [1.0 + 1.5e-9, -0.75e-9, 0.5 + 0.75e-9];
        assert!(reference(&dataset, None, p).is_some());
        assert_eq!(locator.weights(p), reference(&dataset, None, p));
    }

    #[test]
    fn overlapping_cells_keep_original_priority_after_spatial_reordering() {
        let mut positions = Vec::new();
        let mut cells = Vec::new();
        for cell in 0..40u32 {
            let base = positions.len() as u32;
            let extent = if cell == 0 { 4.0 } else { 1.0 };
            positions.extend([
                [0.0; 3],
                [extent, 0.0, 0.0],
                [0.0, extent, 0.0],
                [0.0, 0.0, extent],
            ]);
            cells.push([base, base + 1, base + 2, base + 3]);
        }
        let refs = cells
            .iter()
            .map(|nodes| (CellType::Tetra, nodes.as_slice()))
            .collect::<Vec<_>>();
        let dataset = DataSet::new("overlap", positions, Cells::from_cells(&refs));
        let p = [0.2; 3];
        assert_eq!(
            StencilLocator::new(&dataset, None).weights(p).unwrap().0,
            cells[0]
        );
        let mut mask = vec![true; dataset.point_count()];
        mask[0] = false;
        assert_eq!(
            StencilLocator::new(&dataset, Some(&mask))
                .weights(p)
                .unwrap()
                .0,
            cells[1]
        );
    }

    #[test]
    fn retained_index_is_smaller_than_tet_materialization_and_missing_input_has_no_index() {
        let nodes = [0, 1, 2, 3];
        let cells = vec![(CellType::Tetra, nodes.as_slice()); 4096];
        let dataset = DataSet::new(
            "footprint",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            Cells::from_cells(&cells),
        );
        let locator = StencilLocator::new(&dataset, None);
        let bytes = std::mem::size_of_val(&locator)
            + locator.cells.capacity() * std::mem::size_of::<usize>()
            + locator.nodes.capacity() * std::mem::size_of::<Node>();
        assert!(bytes < dataset.cell_count() * std::mem::size_of::<[u32; 4]>());
        let missing = [false; 4];
        let locator = StencilLocator::new(&dataset, Some(&missing));
        assert!(locator.has_volume());
        assert_eq!(locator.cells.capacity(), 0);
        assert_eq!(locator.nodes.capacity(), 0);
    }
}
