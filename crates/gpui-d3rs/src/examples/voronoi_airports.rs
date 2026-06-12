//! World Airports Voronoi — <https://observablehq.com/@d3/world-airports-voronoi>
//!
//! The Observable example uses `d3-geo-voronoi` to compute spherical Voronoi cells
//! and renders their edges on an orthographic globe.
//!
//! Our approach: compute Delaunay on stereographic-projected coordinates,
//! then render Voronoi cell edges as great-circle segments through orthographic.
//! Each Voronoi edge is the perpendicular bisector of a Delaunay edge — we draw
//! the circumcenter-to-circumcenter connections for adjacent triangles.

use crate::geo::Projection;
use crate::geo::projection::Orthographic;
use crate::shape::path::{Path, PathBuilder};

#[derive(Debug)]
pub struct VoronoiAirportsResult {
    pub width: f64,
    pub height: f64,
    pub projected_points: Vec<Option<(f64, f64)>>,
    pub voronoi_mesh_path: Path,
    pub globe_outline: Path,
    pub graticule_path: Path,
    pub point_count: usize,
}

/// Parse airports CSV: lon,lat (no header).
pub fn load_csv(csv_str: &str) -> Vec<(f64, f64)> {
    csv_str
        .lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 2 {
                return None;
            }
            let lon: f64 = cols[0].trim().parse().ok()?;
            let lat: f64 = cols[1].trim().parse().ok()?;
            if !(-180.0..=180.0).contains(&lon) || !(-90.0..=90.0).contains(&lat) {
                return None;
            }
            Some((lon, lat))
        })
        .collect()
}

/// Compute the spherical Voronoi mesh on an orthographic globe.
/// `zoom` scales the globe (1.0 = default, 2.0 = double size).
pub fn compute(coords: &[(f64, f64)], rotation: (f64, f64)) -> VoronoiAirportsResult {
    compute_with_zoom(coords, rotation, 1.0)
}

/// Compute with explicit zoom factor.
pub fn compute_with_zoom(
    coords: &[(f64, f64)],
    rotation: (f64, f64),
    zoom: f64,
) -> VoronoiAirportsResult {
    use crate::delaunay::Delaunay;
    use std::f64::consts::PI;

    let width: f64 = 600.0;
    let height: f64 = 600.0;
    let base_scale = width.min(height) / 2.0 - 10.0;
    let scale = base_scale * zoom;

    let mut ortho = Orthographic::new();
    ortho.set_scale(scale);
    ortho.set_translate(width / 2.0, height / 2.0);
    ortho.set_rotate(rotation.0, rotation.1, 0.0);

    // Stereographic projection for Delaunay computation.
    // Center the stereographic on the same rotation as the ortho view
    // so that the visible hemisphere has the densest/best triangulation.
    let rot_q = crate::geo::versor::Versor::from_angles(rotation.0, rotation.1, 0.0);

    let stereo_points: Vec<(f64, f64)> = coords
        .iter()
        .map(|&(lon, lat)| {
            // Rotate the point to match the view
            let (rl, rp) = rot_q.rotate_spherical(lon.to_radians(), lat.to_radians());
            let cos_p = rp.cos();
            let k = 1.0 + cos_p * rl.cos();
            if k < 0.01 {
                (rl.sin() * 1000.0, rp.sin() * 1000.0)
            } else {
                (cos_p * rl.sin() / k, rp.sin() / k)
            }
        })
        .collect();

    let delaunay = Delaunay::new(&stereo_points);

    // Build Voronoi mesh by drawing circumcenter connections.
    // For each pair of adjacent triangles (sharing a halfedge), draw a line
    // between their circumcenters projected through orthographic.
    // The circumcenters in stereographic space need to be inverse-projected
    // back to (lon, lat), then projected through orthographic.
    let mut mesh_builder = PathBuilder::new();
    let n_tri = delaunay.inner().triangles().len();

    for e in 0..n_tri {
        let he = delaunay.inner().halfedges()[e];
        if he == delaunator::EMPTY || e > he {
            continue;
        } // each edge once

        let t0 = e / 3;
        let t1 = he / 3;

        // Get circumcenters of both triangles in stereographic space
        let (sx0, sy0) = delaunay.inner().circumcenter(t0);
        let (sx1, sy1) = delaunay.inner().circumcenter(t1);

        // Inverse stereographic → (lon, lat) → orthographic projection
        let inv_stereo = |sx: f64, sy: f64| -> Option<(f64, f64)> {
            let rho = (sx * sx + sy * sy).sqrt();
            let c = 2.0 * rho.atan();
            if rho < 1e-15 {
                // Center of stereographic = center of rotation
                return ortho.project(rotation.0, rotation.1).into();
            }
            let sin_c = c.sin();
            let cos_c = c.cos();
            // Spherical coords in rotated frame
            let rp = (sy * sin_c / rho).asin();
            let rl = (sx * sin_c).atan2(rho * cos_c);
            // Inverse rotation to get geographic (lon, lat)
            let inv_q = rot_q.conjugate();
            let (lon_r, lat_r) = inv_q.rotate_spherical(rl, rp);
            let lon = lon_r * 180.0 / PI;
            let lat = lat_r * 180.0 / PI;
            let (px, py) = ortho.project(lon, lat);
            if px.is_finite() && py.is_finite() {
                // Check if point is on visible hemisphere (within globe circle)
                let dx = px - width / 2.0;
                let dy = py - height / 2.0;
                if dx * dx + dy * dy <= scale * scale * 0.92 {
                    return Some((px, py));
                }
            }
            None
        };

        if let (Some((x0, y0)), Some((x1, y1))) = (inv_stereo(sx0, sy0), inv_stereo(sx1, sy1)) {
            // Skip edges that are too long — these are circumcenters of
            // degenerate/large triangles wrapping around the sphere
            let dx = x1 - x0;
            let dy = y1 - y0;
            let len_sq = dx * dx + dy * dy;
            let max_edge = scale * 0.25; // edges shouldn't span more than 25% of the globe
            if len_sq < max_edge * max_edge {
                mesh_builder = mesh_builder.move_to(x0, y0).line_to(x1, y1);
            }
        }
    }
    let voronoi_mesh_path = mesh_builder.build();

    // Project airports
    let projected_points: Vec<Option<(f64, f64)>> = coords
        .iter()
        .map(|&(lon, lat)| {
            let (px, py) = ortho.project(lon, lat);
            if !px.is_finite() || !py.is_finite() {
                return None;
            }
            let dx = px - width / 2.0;
            let dy = py - height / 2.0;
            if dx * dx + dy * dy > scale * scale * 1.01 {
                return None;
            }
            Some((px, py))
        })
        .collect();

    // Globe outline
    let n_sides = 64;
    let mut globe_builder = PathBuilder::new();
    for v in 0..n_sides {
        let angle = std::f64::consts::TAU * v as f64 / n_sides as f64;
        let x = width / 2.0 + scale * angle.cos();
        let y = height / 2.0 + scale * angle.sin();
        if v == 0 {
            globe_builder = globe_builder.move_to(x, y);
        } else {
            globe_builder = globe_builder.line_to(x, y);
        }
    }
    globe_builder = globe_builder.close_path();

    // Graticule
    let mut grat_builder = PathBuilder::new();
    for lon_i in (-180..180).step_by(30) {
        let lon = lon_i as f64;
        let mut first = true;
        for lat_i in (-90..=90).step_by(3) {
            let lat = lat_i as f64;
            let (px, py) = ortho.project(lon, lat);
            if px.is_finite() && py.is_finite() {
                let dx = px - width / 2.0;
                let dy = py - height / 2.0;
                if dx * dx + dy * dy <= scale * scale * 0.92 {
                    if first {
                        grat_builder = grat_builder.move_to(px, py);
                        first = false;
                    } else {
                        grat_builder = grat_builder.line_to(px, py);
                    }
                    continue;
                }
            }
            first = true;
        }
    }
    for lat_i in (-60..=60).step_by(30) {
        let lat = lat_i as f64;
        let mut first = true;
        for lon_i in (-180..=180).step_by(3) {
            let lon = lon_i as f64;
            let (px, py) = ortho.project(lon, lat);
            if px.is_finite() && py.is_finite() {
                let dx = px - width / 2.0;
                let dy = py - height / 2.0;
                if dx * dx + dy * dy <= scale * scale * 0.92 {
                    if first {
                        grat_builder = grat_builder.move_to(px, py);
                        first = false;
                    } else {
                        grat_builder = grat_builder.line_to(px, py);
                    }
                    continue;
                }
            }
            first = true;
        }
    }

    VoronoiAirportsResult {
        width,
        height,
        projected_points,
        voronoi_mesh_path,
        globe_outline: globe_builder.build(),
        graticule_path: grat_builder.build(),
        point_count: coords.len(),
    }
}
