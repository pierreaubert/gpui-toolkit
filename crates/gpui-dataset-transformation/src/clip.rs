//! Half-space clipping and plane slicing.

// Rust guideline compliant 2026-02-21

use crate::interp::{Weights, blend, interpolate_point_fields, single};
use crate::{Plane, TransformError};
use gpui_mesh::{CellType, Cells, DataSet};
use std::collections::HashMap;

/// A geometry vertex carrying interpolation provenance.
#[derive(Debug, Clone)]
struct V {
    pos: [f64; 3],
    weights: Weights,
}

fn lerp_v(a: &V, b: &V, t: f64) -> V {
    V {
        pos: Plane::lerp(a.pos, b.pos, t),
        weights: blend(&a.weights, &b.weights, t),
    }
}

/// Builder that deduplicates output points by exact position bits.
struct Builder {
    positions: Vec<[f64; 3]>,
    weights: Vec<Weights>,
    index: HashMap<[u64; 3], u32>,
    cells: Vec<(CellType, Vec<u32>)>,
}

impl Builder {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            weights: Vec::new(),
            index: HashMap::new(),
            cells: Vec::new(),
        }
    }

    fn point(&mut self, v: &V) -> u32 {
        let key = [v.pos[0].to_bits(), v.pos[1].to_bits(), v.pos[2].to_bits()];
        if let Some(id) = self.index.get(&key) {
            return *id;
        }
        let id = self.positions.len() as u32;
        self.positions.push(v.pos);
        self.weights.push(v.weights.clone());
        self.index.insert(key, id);
        id
    }

    fn triangle(&mut self, tri: [&V; 3]) {
        let positions = [tri[0].pos, tri[1].pos, tri[2].pos];
        if signed_area2(positions) <= 1e-30 {
            return;
        }
        let ids = [self.point(tri[0]), self.point(tri[1]), self.point(tri[2])];
        if ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2] {
            self.cells.push((CellType::Triangle, ids.to_vec()));
        }
    }

    fn finish(self, dataset: &DataSet, id: &str) -> DataSet {
        let fields = interpolate_point_fields(dataset, &self.weights);
        let refs: Vec<(CellType, &[u32])> =
            self.cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
        let mut result = DataSet::new(id, self.positions, Cells::from_cells(&refs));
        result.point_data = fields;
        result
    }
}

fn signed_area2(tri: [[f64; 3]; 3]) -> f64 {
    let ab = [
        tri[1][0] - tri[0][0],
        tri[1][1] - tri[0][1],
        tri[1][2] - tri[0][2],
    ];
    let ac = [
        tri[2][0] - tri[0][0],
        tri[2][1] - tri[0][1],
        tri[2][2] - tri[0][2],
    ];
    let n = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    n[0] * n[0] + n[1] * n[1] + n[2] * n[2]
}

/// Clips a polygon (vertices in order) to the kept half-space.
fn clip_polygon(poly: &[V], plane: &Plane, keep_positive: bool) -> Vec<V> {
    if poly.is_empty() {
        return Vec::new();
    }
    let sign = if keep_positive { 1.0 } else { -1.0 };
    let mut out: Vec<V> = Vec::new();
    for i in 0..poly.len() {
        let a = &poly[i];
        let b = &poly[(i + 1) % poly.len()];
        let da = sign * plane.distance(a.pos);
        let db = sign * plane.distance(b.pos);
        if da >= 0.0 {
            out.push(a.clone());
        }
        if (da >= 0.0) != (db >= 0.0) {
            out.push(lerp_v(a, b, Plane::crossing(da, db)));
        }
    }
    out
}

fn emit_polygon(builder: &mut Builder, poly: &[V]) {
    if poly.len() < 3 {
        return;
    }
    for i in 1..poly.len() - 1 {
        builder.triangle([&poly[0], &poly[i], &poly[i + 1]]);
    }
}

/// Faces of a tetrahedron as oriented triples of local indices.
const TET_FACES: [[usize; 3]; 4] = [[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]];

/// Clips a dataset to one side of a plane.
///
/// Surface triangles and quads are polygon-clipped. Tetrahedra are clipped
/// as polyhedra and re-triangulated on their boundary. Point fields are
/// interpolated onto created points; cell fields do not survive. Other
/// volume kinds are reported as [`TransformError::UnsupportedCell`];
/// tetrahedralize or extract the surface first for those.
///
/// Validity: geometry is position-only — masked field values never move
/// or drop vertices. An output point's field value is valid only when
/// every nonzero-weight contributor is valid (see `interp`); a created
/// point interpolated from a masked endpoint is a masked NaN, while an
/// unmodified valid vertex keeps its exact value.
///
/// # Errors
///
/// Returns an error when the input is invalid or holds unsupported cells.
pub fn clip_halfspace(
    dataset: &DataSet,
    plane: &Plane,
    keep_positive: bool,
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    let mut builder = Builder::new();
    for cell in 0..dataset.cell_count() {
        let kind = dataset.cells.types[cell];
        let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
        let pts: Vec<V> = nodes
            .iter()
            .map(|&n| V {
                pos: dataset.positions[n as usize],
                weights: single(n),
            })
            .collect();
        match kind {
            CellType::Triangle | CellType::Quad | CellType::Polygon => {
                emit_polygon(&mut builder, &clip_polygon(&pts, plane, keep_positive));
            }
            CellType::Tetra => {
                clip_tetra(&mut builder, &pts, plane, keep_positive);
            }
            CellType::Vertex | CellType::Line => {}
            other => return Err(TransformError::UnsupportedCell(other)),
        }
    }
    Ok(builder.finish(dataset, &format!("{}-clipped", dataset.id)))
}

fn clip_tetra(builder: &mut Builder, tet: &[V], plane: &Plane, keep_positive: bool) {
    let mut cap: Vec<V> = Vec::new();
    for face in TET_FACES {
        let poly: Vec<V> = face.iter().map(|&i| tet[i].clone()).collect();
        let clipped = clip_polygon(&poly, plane, keep_positive);
        emit_polygon(builder, &clipped);
    }
    const EDGES: [[usize; 2]; 6] = [[0, 1], [1, 2], [2, 0], [0, 3], [1, 3], [2, 3]];
    for [a, b] in EDGES {
        let da = plane.distance(tet[a].pos);
        let db = plane.distance(tet[b].pos);
        if (da >= 0.0) != (db >= 0.0) {
            cap.push(lerp_v(&tet[a], &tet[b], Plane::crossing(da, db)));
        }
    }
    if cap.len() >= 3 {
        let positions: Vec<[f64; 3]> = cap.iter().map(|v| v.pos).collect();
        let centre = centroid(&positions);
        let u = plane_basis(plane.normal);
        let v = cross(plane.normal, u);
        cap.sort_by(|a, b| {
            angle(a.pos, centre, u, v)
                .partial_cmp(&angle(b.pos, centre, u, v))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        emit_polygon(builder, &cap);
    }
}

fn centroid(points: &[[f64; 3]]) -> [f64; 3] {
    let mut c = [0.0; 3];
    for p in points {
        c[0] += p[0];
        c[1] += p[1];
        c[2] += p[2];
    }
    let n = points.len() as f64;
    [c[0] / n, c[1] / n, c[2] / n]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn plane_basis(normal: [f64; 3]) -> [f64; 3] {
    let a = if normal[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let c = cross(normal, a);
    let len = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    [c[0] / len, c[1] / len, c[2] / len]
}

fn angle(p: [f64; 3], centre: [f64; 3], u: [f64; 3], v: [f64; 3]) -> f64 {
    let d = [p[0] - centre[0], p[1] - centre[1], p[2] - centre[2]];
    let x = d[0] * u[0] + d[1] * u[1] + d[2] * u[2];
    let y = d[0] * v[0] + d[1] * v[1] + d[2] * v[2];
    y.atan2(x)
}

/// Intersects a dataset with a plane (a VTK-style cut).
///
/// Volume cells produce filled cut polygons triangulated as triangles;
/// surface cells produce line segments. Point fields are interpolated
/// onto the cut, so a slice can be coloured by pressure directly; cell
/// fields do not survive.
///
/// Validity: same contract as [`clip_halfspace`] — geometry is
/// position-only, and a cut point interpolated from any masked endpoint
/// carries a masked NaN instead of a blended placeholder.
///
/// # Errors
///
/// Returns an error when the input is invalid or holds unsupported cells.
pub fn slice(dataset: &DataSet, plane: &Plane) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    let mut builder = Builder::new();
    let mut lines: Vec<(CellType, Vec<u32>)> = Vec::new();
    for cell in 0..dataset.cell_count() {
        let kind = dataset.cells.types[cell];
        let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
        let pts: Vec<V> = nodes
            .iter()
            .map(|&n| V {
                pos: dataset.positions[n as usize],
                weights: single(n),
            })
            .collect();
        match kind {
            CellType::Tetra | CellType::Hexahedron | CellType::Wedge | CellType::Pyramid => {
                slice_volume_cell(&mut builder, &pts, kind, plane);
            }
            CellType::Triangle | CellType::Quad | CellType::Polygon => {
                let mut hits: Vec<V> = Vec::new();
                for i in 0..pts.len() {
                    let a = &pts[i];
                    let b = &pts[(i + 1) % pts.len()];
                    let da = plane.distance(a.pos);
                    let db = plane.distance(b.pos);
                    if (da >= 0.0) != (db >= 0.0) {
                        hits.push(lerp_v(a, b, Plane::crossing(da, db)));
                    }
                }
                if hits.len() == 2 {
                    let a = builder.point(&hits[0]);
                    let b = builder.point(&hits[1]);
                    lines.push((CellType::Line, vec![a, b]));
                }
            }
            CellType::Vertex | CellType::Line => {}
        }
    }
    builder.cells.extend(lines);
    Ok(builder.finish(dataset, &format!("{}-slice", dataset.id)))
}

fn slice_volume_cell(builder: &mut Builder, pts: &[V], kind: CellType, plane: &Plane) {
    let faces = volume_face_list(kind);
    let mut hits: Vec<V> = Vec::new();
    let mut seen: Vec<([f64; 3], [f64; 3])> = Vec::new();
    for face in &faces {
        for i in 0..face.len() {
            let a = &pts[face[i]];
            let b = &pts[face[(i + 1) % face.len()]];
            if seen
                .iter()
                .any(|(x, y)| (*x == a.pos && *y == b.pos) || (*x == b.pos && *y == a.pos))
            {
                continue;
            }
            seen.push((a.pos, b.pos));
            let da = plane.distance(a.pos);
            let db = plane.distance(b.pos);
            if (da >= 0.0) != (db >= 0.0) {
                hits.push(lerp_v(a, b, Plane::crossing(da, db)));
            }
        }
    }
    if hits.len() < 3 {
        return;
    }
    let positions: Vec<[f64; 3]> = hits.iter().map(|v| v.pos).collect();
    let centre = centroid(&positions);
    let u = plane_basis(plane.normal);
    let v = cross(plane.normal, u);
    hits.sort_by(|a, b| {
        angle(a.pos, centre, u, v)
            .partial_cmp(&angle(b.pos, centre, u, v))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    emit_polygon(builder, &hits);
}

/// Face node lists per volume kind, in dataset node ordering.
fn volume_face_list(kind: CellType) -> Vec<Vec<usize>> {
    match kind {
        CellType::Tetra => TET_FACES.iter().map(|f| f.to_vec()).collect(),
        CellType::Hexahedron => vec![
            vec![0, 3, 2, 1],
            vec![4, 5, 6, 7],
            vec![0, 1, 5, 4],
            vec![1, 2, 6, 5],
            vec![2, 3, 7, 6],
            vec![3, 0, 4, 7],
        ],
        CellType::Wedge => vec![
            vec![0, 2, 1],
            vec![3, 4, 5],
            vec![0, 1, 4, 3],
            vec![1, 2, 5, 4],
            vec![2, 0, 3, 5],
        ],
        CellType::Pyramid => vec![
            vec![0, 3, 2, 1],
            vec![0, 1, 4],
            vec![1, 2, 4],
            vec![2, 3, 4],
            vec![3, 0, 4],
        ],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::Association;
    use gpui_mesh::{Cells, Field};

    fn tet_dataset() -> DataSet {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        DataSet::new(
            "tet",
            positions,
            Cells::from_cells(&[(CellType::Tetra, &[0, 1, 2, 3][..])]),
        )
        .with_field(Field::scalar("p", vec![0.0, 0.0, 0.0, 10.0]))
    }

    #[test]
    fn slicing_a_tet_at_mid_height_gives_one_triangle() {
        let out = slice(&tet_dataset(), &Plane::axis(2, 0.5).unwrap()).unwrap();
        assert_eq!(out.cell_count(), 1);
        assert_eq!(out.validate(), Ok(()));
    }

    #[test]
    fn slice_interpolates_point_fields() {
        let out = slice(&tet_dataset(), &Plane::axis(2, 0.5).unwrap()).unwrap();
        let values = out
            .field(Association::Point, "p")
            .unwrap()
            .as_scalar()
            .unwrap()
            .to_vec();
        // Every cut point lies halfway to the apex value of 10 or on the base.
        assert!(values.iter().all(|v| *v == 5.0), "got {values:?}");
    }

    #[test]
    fn slicing_a_cube_gives_a_quad_as_two_triangles() {
        let cube = DataSet::structured_grid("cube", [2, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
        let out = slice(&cube, &Plane::axis(2, 0.5).unwrap()).unwrap();
        assert_eq!(out.cell_count(), 2);
        assert_eq!(out.validate(), Ok(()));
    }

    #[test]
    fn clipping_a_triangle_keeps_the_positive_part() {
        let ds = DataSet::surface(
            "tri",
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        let out = clip_halfspace(&ds, &Plane::axis(0, 0.5).unwrap(), true).unwrap();
        assert_eq!(out.cell_count(), 1);
        assert!(out.positions.iter().all(|p| p[0] >= 0.5 - 1e-12));
        assert_eq!(out.validate(), Ok(()));
    }

    fn masked_tet_dataset() -> DataSet {
        let mut dataset = tet_dataset();
        // Apex value 10 becomes a masked placeholder.
        dataset.point_data[0].valid = Some(vec![true, true, true, false].into());
        dataset
    }

    #[test]
    fn slice_from_masked_endpoint_is_masked_nan_not_blended_placeholder() {
        let out = slice(&masked_tet_dataset(), &Plane::axis(2, 0.5).unwrap()).unwrap();
        assert_eq!(out.cell_count(), 1, "geometry is position-only");
        let field = out.field(Association::Point, "p").unwrap();
        // Every cut point interpolates toward the masked apex.
        assert!(
            (0..out.point_count()).all(|index| !field.is_valid(index)),
            "cut values must be missing, got {:?}",
            field.as_scalar().unwrap()
        );
        assert!(field.as_scalar().unwrap().iter().all(|v| v.is_nan()));
        assert_eq!(out.validate(), Ok(()));
    }

    #[test]
    fn clip_keeps_exact_valid_vertices_and_masks_created_points() {
        let out =
            clip_halfspace(&masked_tet_dataset(), &Plane::axis(2, 0.5).unwrap(), false).unwrap();
        let field = out.field(Association::Point, "p").unwrap();
        let values = field.as_scalar().unwrap();
        for (index, position) in out.positions.iter().enumerate() {
            if position[2] == 0.0 {
                assert!(field.is_valid(index), "base vertices stay exact");
                assert_eq!(values[index], 0.0);
            } else {
                assert!(
                    !field.is_valid(index),
                    "points created toward the masked apex stay missing at {position:?}"
                );
            }
        }
        assert_eq!(out.validate(), Ok(()));
    }

    #[test]
    fn clipping_a_tet_produces_a_closed_piece() {
        let out = clip_halfspace(&tet_dataset(), &Plane::axis(2, 0.5).unwrap(), true).unwrap();
        assert!(out.cell_count() >= 1);
        assert!(out.positions.iter().all(|p| p[2] >= 0.5 - 1e-12));
        assert_eq!(out.validate(), Ok(()));
    }
}
