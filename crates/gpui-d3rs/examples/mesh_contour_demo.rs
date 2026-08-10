//! Unstructured triangle mesh contour demonstration (mesh module).
//!
//! Compute-only example for the feature-independent `d3rs::mesh` module:
//! - Builds an annulus triangle mesh (no grid assumptions)
//! - Attaches a vertex scalar field (radius from the origin)
//! - Resolves nice-number contour levels over the field range
//! - Runs marching triangles: isoline segments + filled bands
//!
//! Run with: cargo run --example mesh_contour_demo

use d3rs::mesh::{
    ContourLevels, CoordinateAxis, MarchingTriangles, MeshTopology, ScalarAssociation, ScalarField,
    TriangleMesh,
};

/// Annulus mesh: `rings` concentric rings x `sectors` angular sectors,
/// triangulated into 2 triangles per quad. Radius linearly interpolates
/// from `r_inner` to `r_outer`.
fn annulus_mesh(r_inner: f64, r_outer: f64, rings: usize, sectors: usize) -> TriangleMesh {
    assert!(rings >= 2 && sectors >= 3);
    let mut positions = Vec::with_capacity(rings * sectors);
    for i in 0..rings {
        let t = i as f64 / (rings - 1) as f64;
        let r = r_inner + t * (r_outer - r_inner);
        for j in 0..sectors {
            let theta = 2.0 * std::f64::consts::PI * j as f64 / sectors as f64;
            positions.push([r * theta.cos(), r * theta.sin(), 0.0]);
        }
    }
    let mut triangles = Vec::with_capacity((rings - 1) * sectors * 2);
    for i in 0..rings - 1 {
        for j in 0..sectors {
            let j1 = (j + 1) % sectors;
            let a = (i * sectors + j) as u32;
            let b = (i * sectors + j1) as u32;
            let c = ((i + 1) * sectors + j) as u32;
            let d = ((i + 1) * sectors + j1) as u32;
            triangles.push([a, b, d]);
            triangles.push([a, d, c]);
        }
    }
    TriangleMesh {
        id: "annulus".into(),
        positions: positions.into(),
        triangles: triangles.into(),
        vertex_ids: None,
        cell_ids: None,
    }
}

/// Signed area of a 2D triangle soup.
fn soup_area(positions: &[[f64; 2]], triangles: &[[u32; 3]]) -> f64 {
    triangles
        .iter()
        .map(|t| {
            let [a, b, c] = [
                positions[t[0] as usize],
                positions[t[1] as usize],
                positions[t[2] as usize],
            ];
            0.5 * ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs()
        })
        .sum()
}

fn main() {
    println!("=== d3rs mesh: marching triangles demonstration ===\n");

    // ========================================
    // Mesh + scalar field
    // ========================================
    let mesh = annulus_mesh(0.5, 2.0, 8, 48);
    mesh.validate().expect("generated mesh must be valid");
    println!(
        "Annulus mesh: {} vertices, {} triangles",
        mesh.positions.len(),
        mesh.triangles.len()
    );

    // Field: radius from the origin — isolines are concentric circles.
    let values: Vec<f64> = mesh
        .positions
        .iter()
        .map(|p| (p[0] * p[0] + p[1] * p[1]).sqrt())
        .collect();
    let range = [
        values.iter().copied().fold(f64::INFINITY, f64::min),
        values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    ];
    let field = ScalarField {
        id: "radius".into(),
        label: "radius".into(),
        unit: None,
        values: values.into(),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    field.validate(&mesh).expect("field must match the mesh");
    println!(
        "Scalar field: radius, range [{:.3}, {:.3}]",
        range[0], range[1]
    );

    // ========================================
    // Levels + topology
    // ========================================
    let levels = ContourLevels::Count(6)
        .resolve(range)
        .expect("finite increasing levels");
    println!(
        "Contour levels (Count(6) over range): {:?}",
        levels.as_ref()
    );

    let topo = MeshTopology::build(&mesh.triangles);
    println!("Topology: {} unique edges", topo.unique_edges.len());

    let mt = MarchingTriangles::new(&mesh, &field, &topo, CoordinateAxis::X, CoordinateAxis::Y)
        .expect("vertex field supports contours");

    // ========================================
    // Isolines
    // ========================================
    println!("\n--- Isoline segments ---");
    let segments = mt.isolines(&levels);
    for &level in levels.iter() {
        let n = segments.iter().filter(|s| s.level == level).count();
        println!("  level {:>5.2}: {} segments", level, n);
    }
    println!("  total: {} segments", segments.len());

    // ========================================
    // Filled bands
    // ========================================
    println!("\n--- Filled bands ---");
    let bands = mt.filled_bands(&levels);
    let total_area: f64 = bands
        .iter()
        .map(|b| soup_area(&b.positions, &b.triangles))
        .sum();
    for band in &bands {
        let (Some(lower), Some(upper)) = (band.lower, band.upper) else {
            println!(
                "  open-ended band: {} triangles, {} vertices, area {:.4}",
                band.triangles.len(),
                band.positions.len(),
                soup_area(&band.positions, &band.triangles)
            );
            continue;
        };
        println!(
            "  [{:.2}, {:.2}]: {} triangles, {} vertices, area {:.4}",
            lower,
            upper,
            band.triangles.len(),
            band.positions.len(),
            soup_area(&band.positions, &band.triangles)
        );
    }
    let band_annulus_area = std::f64::consts::PI * (1.5_f64.powi(2) - 1.0_f64.powi(2));
    println!(
        "  total band area: {:.4} (exact annulus band {:.4}; polygonal boundary explains the gap)",
        total_area, band_annulus_area
    );
}
