use d3rs::geo::GeoJsonGeometry;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Transform {
    pub scale: [f64; 2],
    pub translate: [f64; 2],
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum GeometryObject {
    MultiPolygon { arcs: Vec<Vec<Vec<i32>>> },
    Polygon { arcs: Vec<Vec<i32>> },
    GeometryCollection { geometries: Vec<GeometryObject> },
}

#[derive(Deserialize, Debug)]
pub struct Objects {
    pub land: GeometryObject,
}

#[derive(Deserialize, Debug)]
pub struct Topology {
    pub objects: Objects,
    pub arcs: Vec<Vec<[i32; 2]>>,
    pub transform: Transform,
}

pub fn parse_topojson(json_str: &str) -> Option<GeoJsonGeometry> {
    let topology: Topology = serde_json::from_str(json_str).ok()?;

    // Decode all arcs first
    let scale = topology.transform.scale;
    let translate = topology.transform.translate;

    let decoded_arcs: Vec<Vec<(f64, f64)>> = topology
        .arcs
        .iter()
        .map(|arc| {
            let mut x = 0;
            let mut y = 0;
            let mut points = Vec::with_capacity(arc.len());
            for point in arc {
                x += point[0];
                y += point[1];
                let px = x as f64 * scale[0] + translate[0];
                let py = y as f64 * scale[1] + translate[1];
                points.push((px, py));
            }
            points
        })
        .collect();

    // Extract the geometry from GeometryCollection if needed
    let geometry = match &topology.objects.land {
        GeometryObject::GeometryCollection { geometries } => {
            // Take the first geometry from the collection
            geometries.first()?
        }
        other => other,
    };

    // Convert Geometry to GeoJsonGeometry
    match geometry {
        GeometryObject::MultiPolygon { arcs } => {
            let mut multi_polygon = Vec::new();
            for polygon_arcs in arcs {
                let mut polygon = Vec::new();
                for ring_arcs in polygon_arcs {
                    let mut stitched_ring = Vec::new();
                    for (i, &arc_idx) in ring_arcs.iter().enumerate() {
                        let arc_opt = if arc_idx >= 0 {
                            decoded_arcs.get(arc_idx as usize)
                        } else {
                            decoded_arcs.get((!arc_idx) as usize)
                        };

                        if let Some(arc) = arc_opt {
                            if arc_idx < 0 {
                                for (j, p) in arc.iter().rev().enumerate() {
                                    if i > 0 && j == 0 {
                                        continue;
                                    }
                                    stitched_ring.push(*p);
                                }
                            } else {
                                for (j, p) in arc.iter().enumerate() {
                                    if i > 0 && j == 0 {
                                        continue;
                                    }
                                    stitched_ring.push(*p);
                                }
                            }
                        } else {
                            eprintln!(
                                "Warning: Invalid arc index {} in MultiPolygon topology",
                                arc_idx
                            );
                        }
                    }
                    polygon.push(stitched_ring);
                }
                multi_polygon.push(polygon);
            }
            Some(GeoJsonGeometry::MultiPolygon(multi_polygon))
        }
        GeometryObject::Polygon { arcs } => {
            // Handle single polygon same as above but one level less
            let mut polygon = Vec::new();
            for ring_arcs in arcs {
                let mut stitched_ring = Vec::new();
                for (i, &arc_idx) in ring_arcs.iter().enumerate() {
                    let arc_opt = if arc_idx >= 0 {
                        decoded_arcs.get(arc_idx as usize)
                    } else {
                        decoded_arcs.get((!arc_idx) as usize)
                    };

                    if let Some(arc) = arc_opt {
                        if arc_idx < 0 {
                            for (j, p) in arc.iter().rev().enumerate() {
                                if i > 0 && j == 0 {
                                    continue;
                                }
                                stitched_ring.push(*p);
                            }
                        } else {
                            for (j, p) in arc.iter().enumerate() {
                                if i > 0 && j == 0 {
                                    continue;
                                }
                                stitched_ring.push(*p);
                            }
                        }
                    } else {
                        eprintln!("Warning: Invalid arc index {} in topology", arc_idx);
                    }
                }
                polygon.push(stitched_ring);
            }
            Some(GeoJsonGeometry::Polygon(polygon))
        }
        GeometryObject::GeometryCollection { .. } => {
            // This case should not be reached since we extract the first geometry above
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_land_50m() {
        // Path relative to this file
        let json = include_str!("../data/land-50m.json");
        let result = parse_topojson(json);
        assert!(result.is_some(), "Failed to parse land-50m.json");

        // Verify we got data
        if let Some(geom) = result {
            match geom {
                d3rs::geo::GeoJsonGeometry::MultiPolygon(mp) => {
                    println!("Parsed MultiPolygon with {} polygons", mp.len());
                    assert!(
                        mp.len() > 0,
                        "MultiPolygon should have at least one polygon"
                    );
                }
                d3rs::geo::GeoJsonGeometry::Polygon(p) => {
                    println!("Parsed Polygon with {} rings", p.len());
                    assert!(p.len() > 0, "Polygon should have at least one ring");
                }
                _ => panic!("Unexpected geometry type"),
            }
        }
    }
}
