//! Geo module demonstration
//!
//! Run with: cargo run --example geo_demo --no-default-features

use d3rs::geo::{
    Albers, ConicEqualArea, Equirectangular, GeoJsonGeometry, GeoPath, Graticule, Mercator,
    Orthographic, Projection, Rotation, Stereographic, TransverseMercator, geo_bounds,
    geo_centroid, geo_contains, geo_distance, geo_interpolate, geo_length,
};

fn main() {
    println!("=== d3-geo Demonstration ===\n");

    // =========================================================================
    // Utility Functions
    // =========================================================================

    println!("--- Utility Functions ---\n");

    // Great circle distance
    let new_york = (-74.0, 40.7); // (lon, lat)
    let london = (0.0, 51.5);
    let distance = geo_distance(new_york.0, new_york.1, london.0, london.1);
    let distance_km = distance * 6371.0; // Earth radius in km
    println!(
        "Great circle distance NYC -> London: {:.0} km ({:.4} radians)",
        distance_km, distance
    );

    // Path length
    let path = vec![(-74.0, 40.7), (-73.0, 41.0), (-72.0, 41.5)];
    let length = geo_length(&path);
    println!("Path length: {:.4} radians", length);

    // Spherical interpolation
    let (mid_lon, mid_lat) = geo_interpolate(-74.0, 40.7, 0.0, 51.5, 0.5);
    println!("Midpoint NYC-London: ({:.2}, {:.2})", mid_lon, mid_lat);

    // Bounds
    let coords = vec![(0.0, 0.0), (10.0, 10.0), (20.0, 5.0)];
    let ((min_lon, min_lat), (max_lon, max_lat)) = geo_bounds(&coords);
    println!(
        "Bounds: lon [{:.1}, {:.1}], lat [{:.1}, {:.1}]",
        min_lon, max_lon, min_lat, max_lat
    );

    // Centroid
    let (cx, cy) = geo_centroid(&coords);
    println!("Centroid: ({:.2}, {:.2})", cx, cy);

    // Point-in-polygon
    let polygon = vec![
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
        (0.0, 0.0),
    ];
    println!("Contains (5, 5): {}", geo_contains(&polygon, 5.0, 5.0));
    println!("Contains (15, 5): {}", geo_contains(&polygon, 15.0, 5.0));

    // Rotation
    let rot = Rotation::new().angles(90.0, 0.0, 0.0);
    let (rlon, rlat) = rot.rotate(0.0, 45.0);
    println!("Rotate (0, 45) by 90 degrees: ({:.2}, {:.2})", rlon, rlat);

    println!();

    // =========================================================================
    // Projections
    // =========================================================================

    println!("--- Projections ---\n");

    // Mercator
    let mercator = Mercator::new().scale(100.0).translate(400.0, 300.0);
    let (x, y) = mercator.project(0.0, 0.0);
    println!("Mercator (0, 0) -> ({:.2}, {:.2})", x, y);
    let (x, y) = mercator.project(-74.0, 40.7);
    println!("Mercator NYC -> ({:.2}, {:.2})", x, y);

    // Equirectangular
    let equirect = Equirectangular::new().scale(100.0).translate(400.0, 300.0);
    let (x, y) = equirect.project(0.0, 0.0);
    println!("Equirectangular (0, 0) -> ({:.2}, {:.2})", x, y);

    // Orthographic (globe view)
    let ortho = Orthographic::new()
        .scale(200.0)
        .translate(400.0, 300.0)
        .rotate(-74.0, -40.7, 0.0); // Center on NYC
    let (x, y) = ortho.project(-74.0, 40.7);
    println!("Orthographic NYC (centered) -> ({:.2}, {:.2})", x, y);

    // Stereographic
    let stereo = Stereographic::new().scale(100.0).translate(400.0, 300.0);
    let (x, y) = stereo.project(0.0, 0.0);
    println!("Stereographic (0, 0) -> ({:.2}, {:.2})", x, y);

    // Transverse Mercator
    let tm = TransverseMercator::new()
        .scale(100.0)
        .translate(400.0, 300.0);
    let (x, y) = tm.project(0.0, 0.0);
    println!("Transverse Mercator (0, 0) -> ({:.2}, {:.2})", x, y);

    // Conic Equal-Area
    let conic = ConicEqualArea::with_parallels(30.0, 60.0)
        .scale(100.0)
        .translate(400.0, 300.0);
    let (x, y) = conic.project(0.0, 45.0);
    println!("Conic Equal-Area (0, 45) -> ({:.2}, {:.2})", x, y);

    // Albers USA
    let albers = Albers::new();
    let (x, y) = albers.project(-98.0, 39.0); // Kansas City
    println!("Albers USA (-98, 39) -> ({:.2}, {:.2})", x, y);

    // Inversion test
    println!("\n--- Inversion Test ---\n");
    let proj = Mercator::new().scale(100.0).translate(400.0, 300.0);
    let (x, y) = proj.project(45.0, 30.0);
    if let Some((lon, lat)) = proj.invert(x, y) {
        println!(
            "Mercator: (45, 30) -> ({:.2}, {:.2}) -> ({:.4}, {:.4})",
            x, y, lon, lat
        );
    }

    println!();

    // =========================================================================
    // Graticule
    // =========================================================================

    println!("--- Graticule ---\n");

    let graticule = Graticule::new().step([30.0, 30.0]);
    let lines = graticule.lines();
    println!("Graticule with 30-degree steps: {} lines", lines.len());

    let outline = graticule.outline();
    println!("Graticule outline: {} points", outline.len());

    // Custom graticule
    let custom = Graticule::new()
        .extent([[-90.0, -45.0], [90.0, 45.0]])
        .step([10.0, 10.0]);
    let lines = custom.lines();
    println!(
        "Custom graticule (10-degree, +-90/45): {} lines",
        lines.len()
    );

    println!();

    // =========================================================================
    // GeoPath
    // =========================================================================

    println!("--- GeoPath ---\n");

    let proj = Equirectangular::new().scale(100.0).translate(400.0, 300.0);
    let path = GeoPath::new(proj);

    // Point
    let point = GeoJsonGeometry::Point(-74.0, 40.7);
    let svg = path.render(&point);
    println!("Point SVG: {} chars", svg.len());
    println!("  {}", &svg[..svg.len().min(80)]);

    // LineString
    let line = GeoJsonGeometry::LineString(vec![(-74.0, 40.7), (-73.5, 41.0), (-73.0, 41.5)]);
    let svg = path.render(&line);
    println!("LineString SVG: {} chars", svg.len());
    println!("  {}", &svg[..svg.len().min(80)]);

    // Polygon (simple triangle)
    let poly =
        GeoJsonGeometry::Polygon(vec![vec![(0.0, 0.0), (10.0, 0.0), (5.0, 10.0), (0.0, 0.0)]]);
    let svg = path.render(&poly);
    println!("Polygon SVG: {} chars", svg.len());
    println!("  {}", svg);

    // Bounds and centroid
    let bbox = path.bounds(&line);
    println!(
        "LineString bounds: ({:.2}, {:.2}) to ({:.2}, {:.2})",
        bbox.0.0, bbox.0.1, bbox.1.0, bbox.1.1
    );

    let (cx, cy) = path.centroid(&line);
    println!("LineString centroid: ({:.2}, {:.2})", cx, cy);

    println!();

    // =========================================================================
    // Complete Example: World Map Grid
    // =========================================================================

    println!("--- World Map Grid Example ---\n");

    // Create a projection for a world map
    let world_proj = Equirectangular::new().scale(150.0).translate(500.0, 300.0);

    let world_path = GeoPath::new(world_proj.clone());

    // Generate graticule lines
    let graticule = Graticule::new().step([15.0, 15.0]);
    let lines = graticule.lines();

    // Count total path length
    let mut total_svg_len = 0;
    for line_coords in &lines {
        let geom = GeoJsonGeometry::LineString(line_coords.clone());
        let svg = world_path.render(&geom);
        total_svg_len += svg.len();
    }
    println!(
        "World graticule: {} lines, {} SVG chars total",
        lines.len(),
        total_svg_len
    );

    // Project some major cities
    let cities = [
        ("New York", -74.0, 40.7),
        ("London", 0.0, 51.5),
        ("Tokyo", 139.7, 35.7),
        ("Sydney", 151.2, -33.9),
        ("Rio de Janeiro", -43.2, -22.9),
    ];

    println!("\nProjected city locations:");
    for (name, lon, lat) in &cities {
        let (x, y) = world_proj.project(*lon, *lat);
        println!("  {}: ({:.1}, {:.1})", name, x, y);
    }

    println!("\n=== Demo Complete ===");
}
