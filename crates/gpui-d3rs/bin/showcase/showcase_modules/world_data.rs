use d3rs::geo::GeoJsonGeometry;
use std::sync::OnceLock;

static SMALL_DATA: OnceLock<GeoJsonGeometry> = OnceLock::new();
static LARGE_DATA: OnceLock<GeoJsonGeometry> = OnceLock::new();

pub fn get_world_data(use_large: bool) -> &'static GeoJsonGeometry {
    if use_large {
        LARGE_DATA.get_or_init(|| {
            // Load large data from JSON
            // Path relative to this file: ../../data/land-50m.json
            // But include_str is relative to current file.
            let json = include_str!("../data/land-50m.json");
            super::topojson_utils::parse_topojson(json).expect("Failed to parse large world data")
        })
    } else {
        SMALL_DATA.get_or_init(generate_small_data)
    }
}

// Kept for compatibility if needed, but demos should move to get_world_data
#[allow(dead_code)]
pub fn world_continents() -> GeoJsonGeometry {
    generate_small_data()
}

fn generate_small_data() -> GeoJsonGeometry {
    GeoJsonGeometry::MultiPolygon(vec![
        // Simplified Africa
        vec![vec![
            (32.5, 30.5),
            (51.3, 11.5),
            (42.5, -20.5),
            (20.5, -34.8),
            (0.0, 5.0),
            (-17.5, 14.5),
            (-5.5, 35.8),
            (10.0, 37.0),
            (32.5, 30.5),
        ]],
        // Simplified Antarctica
        vec![vec![
            (-180.0, -90.0),
            (180.0, -90.0),
            (180.0, -60.0),
            (90.0, -65.0),
            (0.0, -70.0),
            (-90.0, -65.0),
            (-180.0, -60.0),
            (-180.0, -90.0),
        ]],
        // Simplified Asia
        vec![vec![
            (26.0, 40.0),
            (60.0, 60.0),
            (100.0, 75.0),
            (170.0, 70.0),
            (145.0, 40.0),
            (120.0, 30.0),
            (100.0, 10.0),
            (80.0, 15.0),
            (50.5, 25.5),
            (26.0, 40.0),
        ]],
        // Simplified Australia
        vec![vec![
            (113.0, -22.0),
            (153.0, -28.0),
            (145.0, -39.0),
            (115.0, -35.0),
            (113.0, -22.0),
        ]],
        // Simplified Europe
        vec![vec![
            (-10.0, 36.0),
            (0.0, 42.0),
            (20.0, 40.0),
            (30.0, 60.0),
            (40.0, 70.0),
            (10.0, 70.0),
            (-10.0, 60.0),
            (-10.0, 36.0),
        ]],
        // Simplified North America
        vec![vec![
            (-170.0, 20.0),
            (-120.0, 50.0),
            (-80.0, 80.0),
            (-60.0, 50.0),
            (-90.0, 20.0),
            (-110.0, 20.0),
            (-170.0, 20.0),
        ]],
        // Simplified South America
        vec![vec![
            (-80.0, 10.0),
            (-35.0, -5.0),
            (-40.0, -25.0),
            (-60.0, -55.0),
            (-80.0, -5.0),
            (-80.0, 10.0),
        ]],
    ])
}
