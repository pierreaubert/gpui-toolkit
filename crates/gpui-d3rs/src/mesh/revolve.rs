//! Axisymmetric r-z revolve into a retained 3D triangle surface (spec §7.2).
//!
//! The generated surface is the boundary of the solid of revolution:
//! lateral ribbons from swept boundary edges, plus optional end caps
//! (the 2D section at start/end angle) for partial sweeps.

use std::f64::consts::TAU;

use super::{
    CoordinateAxis, MeshTopology, MeshValidationError, ScalarAssociation, ScalarField, TriangleMesh,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RevolveSpec {
    pub radial: CoordinateAxis,
    pub axial: CoordinateAxis,
    pub start_angle: f64,
    pub sweep_angle: f64,
    pub segments: u32,
    pub end_caps: bool,
}

impl Default for RevolveSpec {
    fn default() -> Self {
        Self {
            radial: CoordinateAxis::X,
            axial: CoordinateAxis::Z,
            start_angle: 0.0,
            sweep_angle: TAU,
            segments: 64,
            end_caps: false,
        }
    }
}

/// Retained derivative of a 2D mesh (spec §7.2): geometry rebuilds only on
/// geometry/spec revision change; fields replicate by lookup, never rebuild.
#[derive(Debug, Clone, PartialEq)]
pub struct RevolvedMesh {
    pub mesh: TriangleMesh,
    /// derived vertex → source profile vertex (vertex-field replication)
    pub source_vertex: Vec<u32>,
    /// derived triangle → source cell (cell-field replication)
    pub source_triangle: Vec<u32>,
    /// smooth area-weighted per-vertex normals
    pub normals: Vec<[f32; 3]>,
}

const AXIS_TOL: f64 = 1e-12;

pub fn revolve(
    mesh: &TriangleMesh,
    spec: &RevolveSpec,
) -> Result<RevolvedMesh, MeshValidationError> {
    // The construction below indexes positions, triangles, and topology
    // mappings directly. Validate the caller-owned mesh before deriving any
    // retained state so malformed input returns a structured error rather
    // than reaching an unchecked index.
    mesh.validate()?;
    if spec.segments < 3
        || !spec.start_angle.is_finite()
        || !spec.sweep_angle.is_finite()
        || spec.sweep_angle <= 0.0
        || spec.sweep_angle > TAU
    {
        return Err(MeshValidationError::InvalidRevolveSpec);
    }
    let full = (spec.sweep_angle - TAU).abs() < 1e-12;
    // full sweep welds the seam; partial sweep has segments+1 columns
    let cols = if full {
        spec.segments as usize
    } else {
        spec.segments as usize + 1
    };
    let caps = spec.end_caps && !full;

    let radius = |i: usize| spec.radial.component(mesh.positions[i]);
    for i in 0..mesh.positions.len() {
        if radius(i) < -AXIS_TOL {
            return Err(MeshValidationError::InvalidRadius {
                index: i,
                value: radius(i),
            });
        }
    }
    let on_axis: Vec<bool> = (0..mesh.positions.len())
        .map(|i| radius(i).abs() <= AXIS_TOL)
        .collect();
    let topo = MeshTopology::build(&mesh.triangles);

    // vertices needing columns: boundary endpoints; all vertices when caps
    let mut needed = vec![false; mesh.positions.len()];
    for &ei in &topo.boundary_edges {
        let [a, b] = topo.unique_edges[ei as usize];
        if on_axis[a as usize] && on_axis[b as usize] {
            // edges fully on the axis emit no ribbons; a vertex touched only
            // by such edges (e.g. interior to a subdivided axis-aligned
            // boundary chain) must not get an orphan column
            continue;
        }
        needed[a as usize] = true;
        needed[b as usize] = true;
    }
    if caps {
        needed.fill(true);
    }

    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut source_vertex: Vec<u32> = Vec::new();
    let mut derived: Vec<Vec<u32>> = vec![Vec::new(); mesh.positions.len()];
    for (vi, p) in mesh.positions.iter().enumerate() {
        if !needed[vi] {
            continue;
        }
        let r = radius(vi).max(0.0);
        let z = spec.axial.component(*p);
        let count = if on_axis[vi] { 1 } else { cols };
        for c in 0..count {
            let theta = spec.start_angle + spec.sweep_angle * (c as f64 / spec.segments as f64);
            positions.push([r * theta.cos(), r * theta.sin(), z]);
            source_vertex.push(vi as u32);
            derived[vi].push(positions.len() as u32 - 1);
        }
    }
    let col = |vi: u32, c: usize| -> u32 {
        let d = &derived[vi as usize];
        debug_assert!(!d.is_empty(), "vertex {vi} must have columns");
        if d.len() == 1 { d[0] } else { d[c % d.len()] }
    };

    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let mut source_triangle: Vec<u32> = Vec::new();
    let push_tri =
        |triangles: &mut Vec<[u32; 3]>, source_triangle: &mut Vec<u32>, t: [u32; 3], cell: u32| {
            if t[0] != t[1] && t[1] != t[2] && t[0] != t[2] {
                triangles.push(t);
                source_triangle.push(cell);
            }
        };

    // Lateral ribbons: each boundary edge sweeps a quad strip. Winding is
    // chosen so the closed surface has outward orientation; the signed-volume
    // test in this module is the arbiter.
    for &ei in &topo.boundary_edges {
        let [a, b] = topo.unique_edges[ei as usize];
        if on_axis[a as usize] && on_axis[b as usize] {
            continue; // edge lies on the axis: emits no ribbon
        }
        let cell = topo.edge_triangles[ei as usize][0];
        let (tail, head) = ribbon_direction(mesh.triangles[cell as usize], a, b);
        for s in 0..spec.segments as usize {
            let (t0, t1) = (col(tail, s), col(tail, s + 1));
            let (h0, h1) = (col(head, s), col(head, s + 1));
            push_tri(&mut triangles, &mut source_triangle, [t0, h1, h0], cell);
            push_tri(&mut triangles, &mut source_triangle, [t0, t1, h1], cell);
        }
    }

    // End caps: full 2D section at start (source winding) and end (reversed).
    if caps {
        let last = spec.segments as usize;
        for (ci, tri) in mesh.triangles.iter().enumerate() {
            push_tri(
                &mut triangles,
                &mut source_triangle,
                [col(tri[0], 0), col(tri[1], 0), col(tri[2], 0)],
                ci as u32,
            );
            push_tri(
                &mut triangles,
                &mut source_triangle,
                [col(tri[2], last), col(tri[1], last), col(tri[0], last)],
                ci as u32,
            );
        }
    }

    let revolved = TriangleMesh {
        id: format!("{}-revolved", mesh.id).into(),
        positions: positions.into(),
        triangles: triangles.into(),
        vertex_ids: None,
        cell_ids: None,
    };
    let normals = smooth_normals(&revolved);
    Ok(RevolvedMesh {
        mesh: revolved,
        source_vertex,
        source_triangle,
        normals,
    })
}

/// Order (a,b) as (tail, head) following the edge's direction in the
/// incident cell's cyclic winding, so ribbons inherit the source
/// orientation. The signed-volume test in this module is the arbiter that
/// the resulting winding is outward for the default axis convention.
fn ribbon_direction(cell_tri: [u32; 3], a: u32, b: u32) -> (u32, u32) {
    for i in 0..3 {
        if cell_tri[i] == a && cell_tri[(i + 1) % 3] == b {
            return (a, b);
        }
    }
    (b, a)
}

/// Replicate a scalar field onto a revolved mesh — pure lookup, no rebuild.
///
/// The normal contract is that `field` has been validated against the source
/// profile before this function is called. A malformed hand-built
/// `RevolvedMesh` is still kept total: an out-of-range source mapping becomes
/// the NaN sentinel used by GPU uploads instead of panicking.
pub fn revolve_field(field: &ScalarField, revolved: &RevolvedMesh) -> Vec<f64> {
    match field.association {
        ScalarAssociation::Vertex => revolved
            .source_vertex
            .iter()
            .map(|&s| field.values.get(s as usize).copied().unwrap_or(f64::NAN))
            .collect(),
        ScalarAssociation::Cell => revolved
            .source_triangle
            .iter()
            .map(|&s| field.values.get(s as usize).copied().unwrap_or(f64::NAN))
            .collect(),
    }
}

fn smooth_normals(mesh: &TriangleMesh) -> Vec<[f32; 3]> {
    let mut acc = vec![[0.0f64; 3]; mesh.positions.len()];
    for tri in mesh.triangles.iter() {
        let [a, b, c] = [
            mesh.positions[tri[0] as usize],
            mesh.positions[tri[1] as usize],
            mesh.positions[tri[2] as usize],
        ];
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        for &vi in tri {
            let a = &mut acc[vi as usize];
            a[0] += n[0];
            a[1] += n[1];
            a[2] += n[2];
        }
    }
    acc.iter()
        .map(|n| {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-20);
            [
                (n[0] / len) as f32,
                (n[1] / len) as f32,
                (n[2] / len) as f32,
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile_square() -> TriangleMesh {
        // unit square in r-z: r on X, z on Z, Y = 0
        TriangleMesh {
            id: "square".into(),
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
            ]
            .into(),
            triangles: vec![[0, 1, 2], [1, 3, 2]].into(),
            vertex_ids: None,
            cell_ids: None,
        }
    }

    #[test]
    fn full_sweep_of_square_profile() {
        // profile: unit square in r-z, verts (0,0),(1,0),(0,1),(1,1), tris [0,1,2],[1,3,2]
        let mesh = profile_square();
        let out = revolve(
            &mesh,
            &RevolveSpec {
                segments: 8,
                ..RevolveSpec::default()
            },
        )
        .unwrap();
        // 2 axis verts → 1 column each; 2 off-axis verts → 8 columns each
        assert_eq!(out.mesh.positions.len(), 2 + 2 * 8);
        // lateral surface: cylinder ribbon 8×2 + bottom cone 8 + top cone 8 = 32
        assert_eq!(out.mesh.triangles.len(), 32);
        assert!(out.mesh.validate().is_ok(), "no degenerate triangles");
        assert_eq!(out.source_vertex.len(), out.mesh.positions.len());
        assert_eq!(out.source_triangle.len(), out.mesh.triangles.len());
    }

    #[test]
    fn full_sweep_signed_volume_matches_cylinder() {
        let mesh = profile_square();
        let out = revolve(
            &mesh,
            &RevolveSpec {
                segments: 64,
                ..RevolveSpec::default()
            },
        )
        .unwrap();
        // signed volume via divergence theorem: Σ dot(a, cross(b, c)) / 6 ≈ π r² h = π
        let volume: f64 = out
            .mesh
            .triangles
            .iter()
            .map(|t| {
                let p = |i: u32| out.mesh.positions[i as usize];
                let [a, b, c] = [p(t[0]), p(t[1]), p(t[2])];
                let cross = [
                    b[1] * c[2] - b[2] * c[1],
                    b[2] * c[0] - b[0] * c[2],
                    b[0] * c[1] - b[1] * c[0],
                ];
                (a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]) / 6.0
            })
            .sum();
        assert!(
            (volume - std::f64::consts::PI).abs() < 0.02,
            "volume must be ≈ +π (outward winding), got {volume}"
        );
    }

    #[test]
    fn vertex_field_replicates_per_segment() {
        let mesh = profile_square();
        let field = ScalarField {
            id: "f".into(),
            label: "f".into(),
            unit: None,
            values: vec![0.0, 1.0, 2.0, 3.0].into(),
            association: ScalarAssociation::Vertex,
            valid: None,
        };
        let out = revolve(
            &mesh,
            &RevolveSpec {
                segments: 8,
                ..RevolveSpec::default()
            },
        )
        .unwrap();
        let values = revolve_field(&field, &out);
        for (i, &v) in values.iter().enumerate() {
            assert_eq!(v, field.values[out.source_vertex[i] as usize]);
        }
    }

    #[test]
    fn cell_field_maps_via_source_triangle() {
        let mesh = profile_square();
        let field = ScalarField {
            id: "f".into(),
            label: "f".into(),
            unit: None,
            values: vec![10.0, 20.0].into(),
            association: ScalarAssociation::Cell,
            valid: None,
        };
        let out = revolve(
            &mesh,
            &RevolveSpec {
                segments: 8,
                ..RevolveSpec::default()
            },
        )
        .unwrap();
        let values = revolve_field(&field, &out);
        for (i, &v) in values.iter().enumerate() {
            assert_eq!(v, field.values[out.source_triangle[i] as usize]);
        }
    }

    #[test]
    fn negative_radius_rejected() {
        let mut mesh = profile_square();
        let mut positions = mesh.positions.to_vec();
        positions[0][0] = -0.5; // r = -0.5
        mesh.positions = positions.into();
        let err = revolve(&mesh, &RevolveSpec::default()).err().unwrap();
        assert!(matches!(err, MeshValidationError::InvalidRadius { .. }));
    }

    #[test]
    fn malformed_mesh_is_rejected_before_revolve_indexing() {
        let mesh = TriangleMesh {
            id: "malformed".into(),
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].into(),
            triangles: vec![[0, 1, 3]].into(),
            vertex_ids: None,
            cell_ids: None,
        };
        assert!(matches!(
            revolve(&mesh, &RevolveSpec::default()),
            Err(MeshValidationError::IndexOutOfRange { .. })
        ));
    }

    #[test]
    fn invalid_revolve_field_mapping_uses_nan_sentinel() {
        let mesh = profile_square();
        let mut revolved = revolve(&mesh, &RevolveSpec::default()).unwrap();
        revolved.source_vertex[0] = u32::MAX;
        let field = ScalarField {
            id: "f".into(),
            label: "f".into(),
            unit: None,
            values: vec![1.0; 4].into(),
            association: ScalarAssociation::Vertex,
            valid: None,
        };
        assert!(revolve_field(&field, &revolved)[0].is_nan());
    }

    #[test]
    fn partial_sweep_open_seam_and_caps() {
        let mesh = profile_square();
        let spec = RevolveSpec {
            sweep_angle: std::f64::consts::PI,
            segments: 4,
            ..RevolveSpec::default()
        };
        let out = revolve(&mesh, &spec).unwrap();
        // open seam: off-axis verts get segments+1 = 5 columns
        assert_eq!(out.mesh.positions.len(), 2 + 2 * 5);
        // no caps by default: only lateral ribbons
        let lateral = out.mesh.triangles.len();
        let with_caps = revolve(
            &mesh,
            &RevolveSpec {
                end_caps: true,
                ..spec
            },
        )
        .unwrap();
        // caps add the 2 source triangles × 2 ends
        assert_eq!(with_caps.mesh.triangles.len(), lateral + 4);
    }

    #[test]
    fn normals_are_unit() {
        let mesh = profile_square();
        let out = revolve(
            &mesh,
            &RevolveSpec {
                segments: 16,
                ..RevolveSpec::default()
            },
        )
        .unwrap();
        for n in &out.normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "normal must be unit, got {len}");
        }
    }

    #[test]
    fn invalid_segment_count_rejected() {
        let mesh = profile_square();
        let spec = RevolveSpec {
            segments: 2,
            ..RevolveSpec::default()
        };
        assert_eq!(
            revolve(&mesh, &spec),
            Err(MeshValidationError::InvalidRevolveSpec)
        );
    }

    #[test]
    fn axis_chain_interior_vertices_not_orphaned() {
        // axis boundary is subdivided: vertex 1 (0,0,0.5) is interior to the
        // axis-aligned boundary chain and touched only by axis-on-axis
        // boundary edges, so it must not get an orphan derived column
        let mesh = TriangleMesh {
            id: "axis-chain".into(),
            positions: vec![
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.5],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 0.5],
                [1.0, 0.0, 1.0],
            ]
            .into(),
            triangles: vec![[0, 1, 4], [0, 4, 3], [1, 2, 5], [1, 5, 4]].into(),
            vertex_ids: None,
            cell_ids: None,
        };
        assert!(mesh.validate().is_ok(), "fixture must be a valid mesh");
        let out = revolve(
            &mesh,
            &RevolveSpec {
                segments: 8,
                ..RevolveSpec::default()
            },
        )
        .unwrap();
        // 2 axis-column endpoints (verts 0, 2) + 3 off-axis verts × 8 columns;
        // interior axis vertex 1 contributes nothing
        assert_eq!(out.mesh.positions.len(), 2 + 3 * 8);
        // every derived vertex has at least one incident triangle
        let mut incident = vec![false; out.mesh.positions.len()];
        for t in out.mesh.triangles.iter() {
            for &i in t {
                incident[i as usize] = true;
            }
        }
        assert!(
            incident.iter().all(|&i| i),
            "every derived vertex must have an incident triangle"
        );
        // no zero normals from orphaned vertices
        for n in &out.normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "normal must be unit, got {len}");
        }
        assert!(out.mesh.validate().is_ok(), "output must be a valid mesh");
    }

    #[test]
    fn nan_sweep_rejected() {
        let mesh = profile_square();
        let spec = RevolveSpec {
            sweep_angle: f64::NAN,
            ..RevolveSpec::default()
        };
        assert_eq!(
            revolve(&mesh, &spec),
            Err(MeshValidationError::InvalidRevolveSpec)
        );
    }
}
