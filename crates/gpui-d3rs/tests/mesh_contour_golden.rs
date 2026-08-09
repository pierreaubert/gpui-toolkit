//! Golden corpus for marching triangles (MeshPlot Task 5).
//!
//! Table-driven integration corpus: single triangle, two shared triangles,
//! square with hole, disconnected islands, saddle field (v = x² - y² sampled
//! on a 3x3 grid), constant field, masked triangle, exact-on-level vertex and
//! edge, reversed winding, and input-order permutation invariance.
//!
//! Every case asserts exact segment counts and band areas for the levels
//! [-0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75] plus crack-free shared
//! endpoints (near-coincident endpoints from different triangles must be
//! bitwise identical). All fixtures use dyadic coordinates so exact area
//! values are representable in f64.

use d3rs::mesh::{
    ContourBand, CoordinateAxis, IsolineSegment, MarchingTriangles, MeshTopology,
    ScalarAssociation, ScalarField, TriangleMesh,
};

const LEVELS: [f64; 7] = [-0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75];
const TOL: f64 = 1e-9;

fn mesh_of(positions: &[[f64; 3]], triangles: &[[u32; 3]]) -> TriangleMesh {
    TriangleMesh {
        id: "golden".into(),
        positions: positions.to_vec().into(),
        triangles: triangles.to_vec().into(),
        vertex_ids: None,
        cell_ids: None,
    }
}

fn field_of(values: &[f64]) -> ScalarField {
    ScalarField {
        id: "v".into(),
        label: "v".into(),
        unit: None,
        values: values.to_vec().into(),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}

struct Corpus {
    mesh: TriangleMesh,
    field: ScalarField,
    topo: MeshTopology,
}

impl Corpus {
    fn new(positions: &[[f64; 3]], triangles: &[[u32; 3]], values: &[f64]) -> Self {
        let mesh = mesh_of(positions, triangles);
        let field = field_of(values);
        let topo = MeshTopology::build(&mesh.triangles);
        Corpus { mesh, field, topo }
    }

    fn segments(&self) -> Vec<IsolineSegment> {
        MarchingTriangles::new(
            &self.mesh,
            &self.field,
            &self.topo,
            CoordinateAxis::X,
            CoordinateAxis::Y,
        )
        .unwrap()
        .isolines(&LEVELS)
    }

    fn bands(&self) -> Vec<ContourBand> {
        MarchingTriangles::new(
            &self.mesh,
            &self.field,
            &self.topo,
            CoordinateAxis::X,
            CoordinateAxis::Y,
        )
        .unwrap()
        .filled_bands(&LEVELS)
    }
}

fn counts_per_level(segs: &[IsolineSegment]) -> [usize; 7] {
    let mut counts = [0usize; 7];
    for s in segs {
        let i = LEVELS
            .iter()
            .position(|&l| l == s.level)
            .expect("segment level must be one of LEVELS");
        counts[i] += 1;
    }
    counts
}

fn band_area(band: &ContourBand) -> f64 {
    band.triangles
        .iter()
        .map(|t| {
            let p = |i: u32| band.positions[i as usize];
            let [a, b, c] = [p(t[0]), p(t[1]), p(t[2])];
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.0
        })
        .sum()
}

/// Crack contract: endpoints from different segments at the same level are
/// either bitwise identical (shared unique-edge intersection) or clearly
/// distinct — never merely close.
fn assert_crack_free(segs: &[IsolineSegment]) {
    for (i, a) in segs.iter().enumerate() {
        for b in &segs[i + 1..] {
            if a.level != b.level {
                continue;
            }
            for p in [a.start, a.end] {
                for q in [b.start, b.end] {
                    let d2 = (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2);
                    if d2 < 1e-18 {
                        assert!(
                            p == q,
                            "crack: near-coincident endpoints not bitwise equal: {p:?} vs {q:?}"
                        );
                    }
                }
            }
        }
    }
}

fn assert_case(c: &Corpus, want_counts: [usize; 7], want_areas: [f64; 6]) {
    let segs = c.segments();
    assert_eq!(
        counts_per_level(&segs),
        want_counts,
        "segment counts per level {LEVELS:?}"
    );
    assert_crack_free(&segs);
    let bands = c.bands();
    assert_eq!(bands.len(), 6, "windows of 7 levels yield 6 bands");
    for (band, &want) in bands.iter().zip(want_areas.iter()) {
        let area = band_area(band);
        assert!(
            (area - want).abs() < TOL,
            "band [{:?}, {:?}) area must be {want}, got {area}",
            band.lower,
            band.upper
        );
    }
}

/// Canonical, order-independent encoding of a segment set (bitwise, with
/// endpoints normalized so segment direction does not matter).
fn sorted_segment_bits(segs: &[IsolineSegment]) -> Vec<(u64, [u64; 2], [u64; 2])> {
    let mut v: Vec<_> = segs
        .iter()
        .map(|s| {
            let a = [s.start[0].to_bits(), s.start[1].to_bits()];
            let b = [s.end[0].to_bits(), s.end[1].to_bits()];
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            (s.level.to_bits(), lo, hi)
        })
        .collect();
    v.sort();
    v
}

// ---- Fixtures --------------------------------------------------------------

/// Single triangle (0,0),(1,0),(0,1) with v = x + y.
fn single_triangle() -> Corpus {
    Corpus::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0, 1, 2]],
        &[0.0, 1.0, 1.0],
    )
}

/// Unit square, triangles [0,1,2] and [1,3,2], linear field v = x.
fn square_linear() -> Corpus {
    Corpus::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        &[[0, 1, 2], [1, 3, 2]],
        &[0.0, 1.0, 0.0, 1.0],
    )
}

/// Square annulus [0,2]^2 minus open hole (0.5,1.5)^2, 8 triangles, v = x - 1.
/// The hole boundary edges are interior edges shared by two triangles.
fn square_with_hole() -> Corpus {
    Corpus::new(
        &[
            [0.0, 0.0, 0.0], // 0
            [2.0, 0.0, 0.0], // 1
            [2.0, 2.0, 0.0], // 2
            [0.0, 2.0, 0.0], // 3
            [0.5, 0.5, 0.0], // 4
            [1.5, 0.5, 0.0], // 5
            [1.5, 1.5, 0.0], // 6
            [0.5, 1.5, 0.0], // 7
        ],
        &[
            [0, 1, 5],
            [0, 5, 4], // bottom strip
            [1, 2, 6],
            [1, 6, 5], // right strip
            [2, 3, 7],
            [2, 7, 6], // top strip
            [3, 0, 4],
            [3, 4, 7], // left strip
        ],
        &[-1.0, 1.0, 1.0, -1.0, -0.5, 0.5, 0.5, -0.5],
    )
}

/// Two disjoint triangles: v = x on the first, v = x - 11 on the second.
fn disconnected_islands() -> Corpus {
    Corpus::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [10.0, 10.0, 0.0],
            [11.0, 10.0, 0.0],
            [10.0, 11.0, 0.0],
        ],
        &[[0, 1, 2], [3, 4, 5]],
        &[0.0, 1.0, 0.0, -1.0, 0.0, -1.0],
    )
}

/// Saddle v = x^2 - y^2 sampled on a 3x3 grid over [-1,1]^2 (8 triangles).
fn saddle_field() -> Corpus {
    Corpus::new(
        &[
            [-1.0, -1.0, 0.0], // 0
            [0.0, -1.0, 0.0],  // 1
            [1.0, -1.0, 0.0],  // 2
            [-1.0, 0.0, 0.0],  // 3
            [0.0, 0.0, 0.0],   // 4
            [1.0, 0.0, 0.0],   // 5
            [-1.0, 1.0, 0.0],  // 6
            [0.0, 1.0, 0.0],   // 7
            [1.0, 1.0, 0.0],   // 8
        ],
        &[
            [0, 1, 4],
            [0, 4, 3],
            [1, 2, 5],
            [1, 5, 4],
            [3, 4, 7],
            [3, 7, 6],
            [4, 5, 8],
            [4, 8, 7],
        ],
        &[0.0, -1.0, 0.0, 1.0, 0.0, 1.0, 0.0, -1.0, 0.0],
    )
}

// ---- Corpus cases -----------------------------------------------------------

#[test]
fn golden_single_triangle() {
    // v = x + y: one segment per level strictly inside (0, 1).
    // Level 0.0 coincides with vertex 0 (tie-break: above) -> no segment.
    assert_case(
        &single_triangle(),
        [0, 0, 0, 0, 1, 1, 1],
        [0.0, 0.0, 0.0, 0.03125, 0.09375, 0.15625],
    );
}

#[test]
fn golden_two_shared_triangles() {
    // v = x on the unit square: each interior level cuts both triangles.
    assert_case(
        &square_linear(),
        [0, 0, 0, 0, 2, 2, 2],
        [0.0, 0.0, 0.0, 0.25, 0.25, 0.25],
    );
    // Shared-edge intersection point must be referenced bitwise by both
    // segments at every interior level.
    let segs = square_linear().segments();
    for (i, a) in segs.iter().enumerate() {
        for b in &segs[i + 1..] {
            if a.level == b.level {
                assert!(
                    [a.start, a.end].iter().any(|p| p == &b.start || p == &b.end),
                    "segments at level {} must share one endpoint bitwise: {a:?} vs {b:?}",
                    a.level
                );
            }
        }
    }
}

#[test]
fn golden_square_with_hole() {
    // v = x - 1 on the annulus. Levels |l| >= 0.5 produce a single contour
    // piece (line x = 1 + l misses the hole); |l| < 0.5 produces two pieces.
    // Counts derive from per-triangle straddling (holes emit no triangles):
    // [-0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75] -> [5, 5, 4, 4, 4, 4, 5].
    assert_case(
        &square_with_hole(),
        [5, 5, 4, 4, 4, 4, 5],
        [0.5, 0.25, 0.25, 0.25, 0.25, 0.5],
    );
}

#[test]
fn golden_disconnected_islands() {
    // Each island contributes exactly one segment at the levels inside its
    // own range (tri A: (0,1); tri B: [-1,0], with the on-level vertex at
    // level 0.0 counted above -> one degenerate segment).
    assert_case(
        &disconnected_islands(),
        [1, 1, 1, 1, 1, 1, 1],
        [0.15625, 0.09375, 0.03125, 0.21875, 0.15625, 0.09375],
    );
}

#[test]
fn golden_saddle_field() {
    // v = x^2 - y^2 on a 3x3 grid. At every level exactly 6 of 8 triangles
    // straddle. Band areas below are the exact piecewise-linear areas of the
    // sampled field (verified by hand per triangle type; dyadic-exact).
    assert_case(
        &saddle_field(),
        [6, 6, 6, 6, 6, 6, 6],
        [0.375, 0.625, 0.875, 0.875, 0.625, 0.375],
    );
}

#[test]
fn golden_constant_field() {
    // One color everywhere: no isolines, no band geometry.
    let c = Corpus::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        &[[0, 1, 2], [1, 3, 2]],
        &[3.0, 3.0, 3.0, 3.0],
    );
    assert!(c.segments().is_empty(), "constant field yields no isolines");
    for band in c.bands() {
        assert!(band.positions.is_empty() && band.triangles.is_empty());
    }
}

#[test]
fn golden_masked_triangle() {
    // Mask vertex 0 -> only triangle [0,1,2] touches it, so that triangle is
    // excluded entirely; triangle [1,3,2] still contours.
    let mut c = square_linear();
    c.field.valid = Some(vec![false, true, true, true].into());
    assert_case(
        &c,
        [0, 0, 0, 0, 1, 1, 1],
        [0.0, 0.0, 0.0, 0.03125, 0.09375, 0.15625],
    );
}

#[test]
fn golden_exact_on_level_vertex_and_edge() {
    // Vertex exactly on level: v0 = 0.5 at level 0.5 is treated as above,
    // yielding exactly one segment (documented tie-break).
    let vertex_case = Corpus::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0, 1, 2]],
        &[0.5, 0.0, 1.0],
    );
    let segs = vertex_case.segments();
    assert_eq!(counts_per_level(&segs), [0, 0, 0, 0, 1, 1, 1]);
    assert_crack_free(&segs);

    // Edge exactly on level: edge (0,1) sits at v = 0.25. Both endpoints
    // classify as above at level 0.25, so the contour jumps over the edge:
    // no segment at 0.25, one segment at each of 0.5 and 0.75.
    let edge_case = Corpus::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0, 1, 2]],
        &[0.25, 0.25, 1.0],
    );
    let segs = edge_case.segments();
    assert_eq!(counts_per_level(&segs), [0, 0, 0, 0, 0, 1, 1]);
    assert_crack_free(&segs);
}

#[test]
fn golden_reversed_winding() {
    // Reversing triangle winding must not change the segment set (after
    // normalizing segment direction) nor the band areas.
    let forward = Corpus::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0, 1, 2]],
        &[0.0, 1.0, 0.0],
    );
    let reversed = Corpus::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0, 2, 1]],
        &[0.0, 1.0, 0.0],
    );
    let fsegs = forward.segments();
    let rsegs = reversed.segments();
    assert_eq!(counts_per_level(&fsegs), [0, 0, 0, 0, 1, 1, 1]);
    assert_eq!(
        sorted_segment_bits(&fsegs),
        sorted_segment_bits(&rsegs),
        "winding reversal must preserve the segment set bitwise"
    );
    let fbands = forward.bands();
    let rbands = reversed.bands();
    for (f, r) in fbands.iter().zip(rbands.iter()) {
        assert!((band_area(f) - band_area(r)).abs() < TOL);
    }
}

#[test]
fn golden_permutation_invariance() {
    // Shuffling input triangle order must yield the identical sorted segment
    // set (bitwise) and identical band areas.
    for make in [square_with_hole as fn() -> Corpus, saddle_field] {
        let base = make();
        let n = base.mesh.triangles.len();

        // Reverse the triangle order.
        let rev_tris: Vec<[u32; 3]> = base.mesh.triangles.iter().rev().copied().collect();
        let rev = Corpus::new(&base.mesh.positions, &rev_tris, &base.field.values);
        assert_eq!(
            sorted_segment_bits(&base.segments()),
            sorted_segment_bits(&rev.segments()),
            "reversed triangle order must preserve the segment set bitwise"
        );
        for (b, r) in base.bands().iter().zip(rev.bands().iter()) {
            assert_eq!(b.lower, r.lower);
            assert_eq!(b.upper, r.upper);
            assert!((band_area(b) - band_area(r)).abs() < TOL);
        }

        // Rotate the triangle order by 3.
        let rotated: Vec<[u32; 3]> = (0..n)
            .map(|i| base.mesh.triangles[(i + 3) % n])
            .collect();
        let rot = Corpus::new(&base.mesh.positions, &rotated, &base.field.values);
        assert_eq!(
            sorted_segment_bits(&base.segments()),
            sorted_segment_bits(&rot.segments()),
            "rotated triangle order must preserve the segment set bitwise"
        );
    }
}
