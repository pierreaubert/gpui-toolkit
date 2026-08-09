//! Marching triangles on unstructured 2D-projected meshes (spec §7.1).
//!
//! Determinism contract:
//! - intersections are computed once per (unique edge, level) and cached;
//! - a vertex exactly on a level classifies as "above" (documented tie-break);
//! - masked triangles are skipped entirely and never emit geometry.

use super::{
    CoordinateAxis, MeshTopology, MeshValidationError, ScalarAssociation, ScalarField,
    TriangleMesh, project_2d,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IsolineSegment {
    pub level: f64,
    pub start: [f64; 2],
    pub end: [f64; 2],
}

/// One filled band as an indexed triangle soup in view coordinates —
/// directly uploadable to the GPU fill pipeline (design §3).
#[derive(Debug, Clone, PartialEq)]
pub struct ContourBand {
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub positions: Vec<[f64; 2]>,
    pub triangles: Vec<[u32; 3]>,
}

pub struct MarchingTriangles<'m> {
    mesh: &'m TriangleMesh,
    values: &'m [f64],
    valid: Option<&'m [bool]>,
    topo: &'m MeshTopology,
    horizontal: CoordinateAxis,
    vertical: CoordinateAxis,
}

impl<'m> MarchingTriangles<'m> {
    pub fn new(
        mesh: &'m TriangleMesh,
        field: &'m ScalarField,
        topo: &'m MeshTopology,
        horizontal: CoordinateAxis,
        vertical: CoordinateAxis,
    ) -> Result<Self, MeshValidationError> {
        if field.association != ScalarAssociation::Vertex {
            return Err(MeshValidationError::ContoursRequireVertexField);
        }
        Ok(Self {
            mesh,
            values: &field.values,
            valid: field.valid.as_deref(),
            topo,
            horizontal,
            vertical,
        })
    }

    fn point(&self, v: u32) -> [f64; 2] {
        project_2d(self.horizontal, self.vertical, self.mesh.positions[v as usize])
    }

    fn tri_masked(&self, tri: [u32; 3]) -> bool {
        match self.valid {
            Some(mask) => tri.iter().any(|&i| !mask[i as usize]),
            None => false,
        }
    }

    /// Intersection on unique edge `ei` at `level`, computed once and cached.
    fn edge_hit(
        &self,
        ei: u32,
        level: f64,
        cache: &mut [Option<[f64; 2]>],
    ) -> Option<[f64; 2]> {
        if let Some(p) = cache[ei as usize] {
            return Some(p);
        }
        let [a, b] = self.topo.unique_edges[ei as usize];
        let (va, vb) = (self.values[a as usize], self.values[b as usize]);
        // straddle test with documented tie-break: on-level counts as above
        let ha = va >= level;
        let hb = vb >= level;
        if ha == hb {
            return None;
        }
        let t = (level - va) / (vb - va);
        let pa = self.point(a);
        let pb = self.point(b);
        let p = [pa[0] + t * (pb[0] - pa[0]), pa[1] + t * (pb[1] - pa[1])];
        cache[ei as usize] = Some(p);
        Some(p)
    }

    pub fn isolines(&self, levels: &[f64]) -> Vec<IsolineSegment> {
        let mut out = Vec::new();
        for &level in levels {
            let mut cache = vec![None; self.topo.unique_edges.len()];
            for (t, tri) in self.mesh.triangles.iter().enumerate() {
                if self.tri_masked(*tri) {
                    continue;
                }
                let mut hits = Vec::with_capacity(2);
                for &ei in &self.topo.triangle_edges[t] {
                    if let Some(p) = self.edge_hit(ei, level, &mut cache) {
                        hits.push(p);
                    }
                }
                if hits.len() == 2 {
                    out.push(IsolineSegment { level, start: hits[0], end: hits[1] });
                }
            }
        }
        out
    }

    pub fn filled_bands(&self, levels: &[f64]) -> Vec<ContourBand> {
        let mut bands: Vec<ContourBand> = levels
            .windows(2)
            .map(|w| ContourBand {
                lower: Some(w[0]),
                upper: Some(w[1]),
                positions: Vec::new(),
                triangles: Vec::new(),
            })
            .collect();
        for tri in self.mesh.triangles.iter() {
            if self.tri_masked(*tri) {
                continue;
            }
            let pts = [
                (self.point(tri[0]), self.values[tri[0] as usize]),
                (self.point(tri[1]), self.values[tri[1] as usize]),
                (self.point(tri[2]), self.values[tri[2] as usize]),
            ];
            for band in &mut bands {
                let lo = band.lower.unwrap();
                let hi = band.upper.unwrap();
                let poly = clip_band(&pts, lo, hi);
                if poly.len() >= 3 {
                    let base = band.positions.len() as u32;
                    band.positions.extend(poly.iter().map(|(p, _)| *p));
                    for i in 1..poly.len() as u32 - 1 {
                        band.triangles.push([base, base + i, base + i + 1]);
                    }
                }
            }
        }
        bands
    }
}

/// Clip a value-carrying triangle to the slab lo <= v <= hi
/// (Sutherland–Hodgman, two passes). On-boundary vertices are kept
/// (documented tie-break: boundaries inclusive on both sides; the crack
/// contract is carried by interpolation determinism).
fn clip_band(tri: &[([f64; 2], f64); 3], lo: f64, hi: f64) -> Vec<([f64; 2], f64)> {
    fn clip_against(
        input: &[([f64; 2], f64)],
        keep_above: bool,
        level: f64,
    ) -> Vec<([f64; 2], f64)> {
        let mut out = Vec::new();
        if input.is_empty() {
            return out;
        }
        let inside = |v: f64| if keep_above { v >= level } else { v <= level };
        let mut prev = *input.last().unwrap();
        for &cur in input {
            let cur_in = inside(cur.1);
            let prev_in = inside(prev.1);
            if cur_in != prev_in {
                let t = (level - prev.1) / (cur.1 - prev.1);
                out.push((
                    [
                        prev.0[0] + t * (cur.0[0] - prev.0[0]),
                        prev.0[1] + t * (cur.0[1] - prev.0[1]),
                    ],
                    level,
                ));
            }
            if cur_in {
                out.push(cur);
            }
            prev = cur;
        }
        out
    }
    let lower = clip_against(tri, true, lo);
    clip_against(&lower, false, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh_of(positions: &[[f64; 3]], triangles: &[[u32; 3]]) -> TriangleMesh {
        TriangleMesh {
            id: "m".into(),
            positions: positions.to_vec().into(),
            triangles: triangles.to_vec().into(),
            vertex_ids: None,
            cell_ids: None,
        }
    }

    fn field_of(values: &[f64]) -> ScalarField {
        ScalarField {
            id: "f".into(),
            label: "f".into(),
            unit: None,
            values: values.to_vec().into(),
            association: ScalarAssociation::Vertex,
            valid: None,
        }
    }

    fn with_topo(mesh: TriangleMesh, field: ScalarField) -> (TriangleMesh, ScalarField, MeshTopology) {
        let topo = MeshTopology::build(&mesh.triangles);
        (mesh, field, topo)
    }

    /// Single triangle (0,0),(1,0),(0,1) with v = x -> values [0.0, 1.0, 0.0].
    fn single_tri_fixture() -> (TriangleMesh, ScalarField, MeshTopology) {
        let mesh = mesh_of(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        with_topo(mesh, field_of(&[0.0, 1.0, 0.0]))
    }

    /// Unit square, triangles [0,1,2] and [1,3,2], linear field v = x.
    fn square_fixture_linear() -> (TriangleMesh, ScalarField, MeshTopology) {
        let mesh = mesh_of(
            &[
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            &[[0, 1, 2], [1, 3, 2]],
        );
        with_topo(mesh, field_of(&[0.0, 1.0, 0.0, 1.0]))
    }

    /// Single triangle with caller-supplied vertex values.
    fn fixture_with_values(values: &[f64]) -> (TriangleMesh, ScalarField, MeshTopology) {
        let mesh = mesh_of(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        with_topo(mesh, field_of(values))
    }

    // Unit: single triangle, linear field v = x → isoline at 0.5 is the segment x=0.5
    #[test]
    fn single_triangle_linear_field_isoline() {
        let (mesh, field, topo) = single_tri_fixture(); // (0,0),(1,0),(0,1); v = [0.0, 1.0, 0.0]
        let mt = MarchingTriangles::new(&mesh, &field, &topo, CoordinateAxis::X, CoordinateAxis::Y).unwrap();
        let segs = mt.isolines(&[0.5]);
        assert_eq!(segs.len(), 1);
        let s = segs[0];
        for p in [s.start, s.end] {
            assert!((p[0] - 0.5).abs() < 1e-12, "x must be 0.5, got {p:?}");
        }
        // endpoints lie on the two straddling edges
        let ys = [s.start[1], s.end[1]];
        assert!(ys.iter().any(|&y| (y - 0.0).abs() < 1e-12));
        assert!(ys.iter().any(|&y| (y - 0.5).abs() < 1e-12));
    }

    #[test]
    fn shared_edge_no_cracks() {
        // two triangles sharing an edge; the intersection point on the shared edge
        // must appear exactly once in the cache → both triangles agree bitwise
        let (mesh, field, topo) = square_fixture_linear(); // v = x over unit square, tris [0,1,2],[1,3,2]
        let mt = MarchingTriangles::new(&mesh, &field, &topo, CoordinateAxis::X, CoordinateAxis::Y).unwrap();
        let segs = mt.isolines(&[0.5]);
        assert_eq!(segs.len(), 2);
        // the two segments share an endpoint bitwise (computed once per unique edge)
        let pts: Vec<[f64; 2]> = segs.iter().flat_map(|s| [s.start, s.end]).collect();
        let shared = pts.iter().filter(|p| (p[0] - 0.5).abs() < 1e-15 && (p[1] - 0.5).abs() < 1e-15).count();
        assert_eq!(shared, 2, "both segments must reference the same shared-edge point");
    }

    #[test]
    fn exact_on_level_vertex_tiebreak() {
        // v = [0.5, 0.0, 1.0], level 0.5: vertex 0 is exactly on level → treated as above
        let (mesh, field, topo) = fixture_with_values(&[0.5, 0.0, 1.0]);
        let mt = MarchingTriangles::new(&mesh, &field, &topo, CoordinateAxis::X, CoordinateAxis::Y).unwrap();
        let segs = mt.isolines(&[0.5]);
        assert_eq!(segs.len(), 1, "documented tie-break must yield exactly one segment");
    }

    #[test]
    fn masked_triangle_excluded() {
        let (mesh, mut field, topo) = square_fixture_linear();
        field.valid = Some(vec![true, false, true, true].into()); // vertex 1 masked → both tris touch it
        let mt = MarchingTriangles::new(&mesh, &field, &topo, CoordinateAxis::X, CoordinateAxis::Y).unwrap();
        assert!(mt.isolines(&[0.5]).is_empty());
    }

    #[test]
    fn filled_band_triangle_count_matches_area() {
        // v = x on unit square, band [0.25, 0.75): covered area = 0.5
        let (mesh, field, topo) = square_fixture_linear();
        let mt = MarchingTriangles::new(&mesh, &field, &topo, CoordinateAxis::X, CoordinateAxis::Y).unwrap();
        let bands = mt.filled_bands(&[0.25, 0.75]);
        assert_eq!(bands.len(), 1);
        let area: f64 = bands[0].triangles.iter().map(|t| {
            let p = |i: u32| bands[0].positions[i as usize];
            let [a, b, c] = [p(t[0]), p(t[1]), p(t[2])];
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.0
        }).sum();
        assert!((area - 0.5).abs() < 1e-9, "band area must be 0.5, got {area}");
    }

    #[test]
    fn cell_field_contours_rejected() {
        let (mesh, mut field, topo) = square_fixture_linear();
        field.association = ScalarAssociation::Cell;
        field.values = vec![0.2, 0.8].into();
        let err = MarchingTriangles::new(&mesh, &field, &topo, CoordinateAxis::X, CoordinateAxis::Y)
            .err()
            .unwrap();
        assert_eq!(err, MeshValidationError::ContoursRequireVertexField);
    }
}
