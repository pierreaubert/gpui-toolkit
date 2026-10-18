//! Marching-tetrahedra isosurfaces over volume datasets.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use crate::interp::{Weights, interpolate_point_fields};
use gpui_mesh::{Association, CellType, Cells, DataSet};
use std::collections::HashMap;

/// Tetrahedralizes one volume cell into dataset node-index tetrahedra.
fn tetrahedralize(kind: CellType, n: &[u32]) -> Vec<[u32; 4]> {
    match kind {
        CellType::Tetra => vec![[n[0], n[1], n[2], n[3]]],
        // Six tetrahedra around the 0-6 body diagonal. Every face diagonal
        // touches node 0 or node 6, so identically oriented neighbours
        // triangulate their shared face the same way and the extracted
        // surface stays watertight.
        CellType::Hexahedron => vec![
            [n[0], n[1], n[2], n[6]],
            [n[0], n[2], n[3], n[6]],
            [n[0], n[3], n[7], n[6]],
            [n[0], n[7], n[4], n[6]],
            [n[0], n[4], n[5], n[6]],
            [n[0], n[5], n[1], n[6]],
        ],
        CellType::Wedge => vec![
            [n[0], n[1], n[2], n[3]],
            [n[1], n[2], n[3], n[4]],
            [n[2], n[3], n[4], n[5]],
        ],
        CellType::Pyramid => vec![[n[0], n[1], n[2], n[4]], [n[0], n[2], n[3], n[4]]],
        _ => Vec::new(),
    }
}

struct IsoBuilder<'a> {
    dataset: &'a DataSet,
    scalars: &'a [f64],
    value: f64,
    positions: Vec<[f64; 3]>,
    weights: Vec<Weights>,
    triangles: Vec<[u32; 3]>,
    edge_points: HashMap<(u32, u32), u32>,
    /// Edge key per created point, for cross-builder merging.
    point_edges: Vec<(u32, u32)>,
}

impl IsoBuilder<'_> {
    fn point(&mut self, a: u32, b: u32) -> u32 {
        let key = (a.min(b), a.max(b));
        if let Some(id) = self.edge_points.get(&key) {
            return *id;
        }
        let va = self.scalars[a as usize];
        let vb = self.scalars[b as usize];
        let span = vb - va;
        let t = if span == 0.0 {
            0.5
        } else if span.is_finite() {
            (self.value - va) / span
        } else {
            // Straddling finite endpoints can have an overflowing difference.
            // Halving preserves their interpolation ratio without infinities.
            (self.value * 0.5 - va * 0.5) / (vb * 0.5 - va * 0.5)
        };
        let pa = self.dataset.positions[a as usize];
        let pb = self.dataset.positions[b as usize];
        let p = std::array::from_fn(|axis| {
            let (a, b) = (pa[axis], pb[axis]);
            if a.is_sign_negative() == b.is_sign_negative() {
                if t <= 0.5 {
                    a + (b - a) * t
                } else {
                    b + (a - b) * (1.0 - t)
                }
            } else {
                a * (1.0 - t) + b * t
            }
        });
        let id = self.positions.len() as u32;
        self.positions.push(p);
        self.weights.push(vec![(a, 1.0 - t), (b, t)]);
        self.edge_points.insert(key, id);
        self.point_edges.push(key);
        id
    }

    fn march_tet(&mut self, tet: [u32; 4]) {
        let inside: Vec<usize> = (0..4)
            .filter(|&i| self.scalars[tet[i] as usize] >= self.value)
            .collect();
        match inside.len() {
            0 | 4 => {}
            1 => {
                let i = inside[0];
                let o: Vec<usize> = (0..4).filter(|&j| j != i).collect();
                let tri = [
                    self.point(tet[i], tet[o[0]]),
                    self.point(tet[i], tet[o[1]]),
                    self.point(tet[i], tet[o[2]]),
                ];
                self.triangles.push(tri);
            }
            3 => {
                let outside = (0..4).find(|&j| !inside.contains(&j)).unwrap_or(0);
                let tri = [
                    self.point(tet[outside], tet[inside[0]]),
                    self.point(tet[outside], tet[inside[1]]),
                    self.point(tet[outside], tet[inside[2]]),
                ];
                self.triangles.push(tri);
            }
            2 => {
                let outside: Vec<usize> = (0..4).filter(|&j| !inside.contains(&j)).collect();
                let (i0, i1) = (inside[0], inside[1]);
                let (o0, o1) = (outside[0], outside[1]);
                let a = self.point(tet[i0], tet[o0]);
                let b = self.point(tet[i0], tet[o1]);
                let c = self.point(tet[i1], tet[o1]);
                let d = self.point(tet[i1], tet[o0]);
                self.triangles.push([a, b, c]);
                self.triangles.push([a, c, d]);
            }
            _ => {}
        }
    }
}

/// Extracts the isosurface of a scalar point field at `value`.
///
/// Volume cells are tetrahedralized and processed with marching
/// tetrahedra. Interpolated points are deduplicated per edge, so the
/// output is a watertight indexed triangle surface for a closed, fully valid
/// input. Tetrahedra with any missing scalar node are omitted; missing regions
/// may therefore leave open boundaries.
///
/// # Errors
///
/// Returns an error for a nonfinite level, invalid dataset, missing/non-scalar
/// field, or a dataset with no volume cells.
pub fn isosurface(
    dataset: &DataSet,
    field_name: &str,
    value: f64,
) -> Result<DataSet, TransformError> {
    if !value.is_finite() {
        return Err(TransformError::Degenerate(
            "isosurface value must be finite".into(),
        ));
    }
    dataset.validate()?;
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    let scalars = field
        .as_scalar()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "isosurface requires a scalar field".into(),
        })?;
    // March each volume cell in parallel with a local builder, then
    // merge by edge key so the surface stays watertight and indexed
    // exactly like the sequential result.
    let volume_cells: Vec<usize> = (0..dataset.cell_count())
        .filter(|&cell| dataset.cells.types[cell].is_volume())
        .collect();
    let locals = crate::parallel::parallel_ranges(volume_cells.len(), |range| {
        let mut builder = IsoBuilder {
            dataset,
            scalars,
            value,
            positions: Vec::new(),
            weights: Vec::new(),
            triangles: Vec::new(),
            edge_points: HashMap::new(),
            point_edges: Vec::new(),
        };
        for index in range {
            let cell = volume_cells[index];
            let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
            for tet in tetrahedralize(dataset.cells.types[cell], nodes) {
                if field.valid.is_none() || tet.iter().all(|&node| field.is_valid(node as usize)) {
                    builder.march_tet(tet);
                }
            }
        }
        builder
    });
    let mut builder = IsoBuilder {
        dataset,
        scalars,
        value,
        positions: Vec::new(),
        weights: Vec::new(),
        triangles: Vec::new(),
        edge_points: HashMap::new(),
        point_edges: Vec::new(),
    };
    for local in locals {
        let mut remap = vec![0u32; local.positions.len()];
        for (local_id, (position, weights)) in
            local.positions.iter().zip(local.weights.iter()).enumerate()
        {
            let key = local.point_edges[local_id];
            let global = if let Some(id) = builder.edge_points.get(&key) {
                *id
            } else {
                let id = builder.positions.len() as u32;
                builder.positions.push(*position);
                builder.weights.push(weights.clone());
                builder.edge_points.insert(key, id);
                id
            };
            remap[local_id] = global;
        }
        for tri in &local.triangles {
            builder.triangles.push([
                remap[tri[0] as usize],
                remap[tri[1] as usize],
                remap[tri[2] as usize],
            ]);
        }
    }
    if volume_cells.is_empty() {
        return Err(TransformError::Degenerate(
            "dataset holds no volume cells".into(),
        ));
    }
    let tri_refs: Vec<(CellType, &[u32])> = builder
        .triangles
        .iter()
        .map(|t| (CellType::Triangle, t.as_slice()))
        .collect();
    let fields = interpolate_point_fields(dataset, &builder.weights);
    let mut result = DataSet::new(
        format!("{}-iso-{value}", dataset.id),
        builder.positions,
        Cells::from_cells(&tri_refs),
    );
    result.point_data = fields;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::Field;

    #[test]
    fn sphere_isosurface_is_closed_and_nonempty() {
        let mut ds = DataSet::structured_grid("grid", [6, 6, 6], [-1.0; 3], [0.4; 3]).unwrap();
        let values: Vec<f64> = ds
            .positions
            .iter()
            .map(|p| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
            .collect();
        ds.point_data
            .push(Field::scalar("radius", values).labelled("Radius", "m"));
        let iso = isosurface(&ds, "radius", 0.8).unwrap();
        assert!(iso.cell_count() > 20, "got {} triangles", iso.cell_count());
        assert_eq!(iso.validate(), Ok(()));
        let mut edge_count: HashMap<(u32, u32), usize> = HashMap::new();
        for cell in 0..iso.cell_count() {
            let tri = iso.cells.cell(cell).unwrap();
            for i in 0..3 {
                let a = tri[i];
                let b = tri[(i + 1) % 3];
                *edge_count.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        assert!(edge_count.values().all(|&c| c == 2));
    }
}
